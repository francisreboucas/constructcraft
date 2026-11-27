//! The `ccad` binary: one-off commands, scripts and an MCP session.

// Test helpers outside #[test] functions may fail loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};

fn ccad(args: &[&str], stdin: &str) -> (i32, Value, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ccad"))
        .args(args)
        .env("CCAD_LANG", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let value = serde_json::from_str(&stdout).unwrap_or(Value::String(stdout));
    (out.status.code().unwrap_or(-1), value, String::from_utf8_lossy(&out.stderr).into_owned())
}

fn scratch(name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("ccad-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name).to_string_lossy().into_owned()
}

#[test]
fn run_saves_and_reopens() {
    let file = scratch("one.ccad");
    let (code, wall, err) =
        ccad(&["--save-as", &file, "run", "wall_create", "start=[0,0]", "end=[5,0]", "height=2.5"], "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(wall["derived"]["length"], 5.0);
    // JSON arguments, on the saved project.
    let (code, got, err) = ccad(&["-p", &file, "run", "element_get", &format!("{{\"id\": {}}}", wall["id"])], "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(got["params"]["height"], 2.5);
    // Errors go to stderr as JSON with exit code 1; usage errors exit with 2.
    let (code, _, err) = ccad(&["run", "element_get", "id=424242"], "");
    assert_eq!(code, 1);
    assert!(err.contains("not_found"), "{err}");
    assert_eq!(ccad(&["run", "wall_create", "oops"], "").0, 2);
    assert_eq!(ccad(&["frobnicate"], "").0, 2);
}

#[test]
fn scripts_run_in_order_and_stop_at_errors() {
    let script = r#"
{"command": "level_create", "args": {"name": "Upper", "elevation": 3}}
{"command": "wall_create", "args": {"start": [0, 0], "end": [4, 0], "top_level": "Upper"}}
{"command": "elements_list", "args": {"category": "wall"}}
"#;
    let (code, out, err) = ccad(&["script", "-"], script);
    assert_eq!(code, 0, "{err}");
    let steps = out.as_array().unwrap();
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[1]["result"]["derived"]["height"], 3.0);
    let failing = r#"[{"command": "undo"}, {"command": "project_info"}]"#;
    let (code, out, err) = ccad(&["script", "-"], failing);
    assert_eq!(code, 1);
    assert_eq!(out, json!([]));
    assert!(err.contains("step 1"), "{err}");
}

#[test]
fn scripts_refer_to_earlier_steps_and_the_given_level() {
    let script = r#"
{"command": "level_create", "args": {"name": "Upper", "elevation": 3}}
{"command": "wall_create", "args": {"level": {"$level": "active"}, "start": [0, 0], "end": [6, 0]}}
{"command": "window_create", "args": {"wall": {"$step": 2}, "offset": 2, "sill": 1}}
"#;
    // No level given: the lowest.
    let (code, out, err) = ccad(&["script", "-"], script);
    assert_eq!(code, 0, "{err}");
    let lowest = out[1]["result"]["level"]["name"].clone();
    assert_eq!(out[2]["result"]["params"]["host"], out[1]["result"]["id"]);
    // A level created before the script runs, by name.
    let file = scratch("levels.ccad");
    let (code, _, err) = ccad(&["--save-as", &file, "run", "level_create", "name=Upper", "elevation=3"], "");
    assert_eq!(code, 0, "{err}");
    let (code, out, err) = ccad(&["-p", &file, "script", "-", "level=Upper"], &script.replace("Upper", "Upper 2"));
    assert_eq!(code, 0, "{err}");
    assert_eq!(out[1]["result"]["level"]["name"], "Upper");
    assert_ne!(lowest, "Upper");
    assert_eq!(ccad(&["-p", &file, "script", "-", "level=Nowhere"], script).0, 1);
    assert_eq!(ccad(&["script", "-", "oops"], script).0, 2);
}

#[test]
fn calculated_schedule_units_follow_the_cli_language() {
    let script = r#"
{"command":"column_create","args":{"position":[0,0]}}
{"command":"connections_create","args":{"kind":"base_plate","members":[{"$step":1}]}}
{"command":"view_create","args":{"kind":"schedule","category":"connection","name":"Connections"}}
{"command":"schedule_table","args":{"view":"Connections"}}
{"command":"beam_create","args":{"start":[0,0],"end":[5,0],"top_offset":3.0}}
{"command":"rebar_typical"}
{"command":"bending_schedule_create","args":{"name":"Bending"}}
{"command":"schedule_table","args":{"view":"Bending"}}
"#;
    for (lang, bolt, weight) in [("en", "Ø 20 mm", " kg"), ("ru", "Ø 20 мм", " кг")] {
        let (code, out, err) = ccad(&["--lang", lang, "script", "-"], script);
        assert_eq!(code, 0, "{err}");
        let connection = out[3]["result"]["rows"][0]["cells"].as_array().unwrap();
        assert!(connection.contains(&json!(bolt)), "{lang}: {connection:?}");
        let bars = out[7]["result"]["rows"].as_array().unwrap();
        assert!(
            bars.iter().any(|row| {
                row["cells"].as_array().unwrap().iter().any(|cell| cell.as_str().is_some_and(|s| s.ends_with(weight)))
            }),
            "{lang}: {bars:?}"
        );
    }
}

#[test]
fn catalog_and_schemas() {
    let (code, list, _) = ccad(&["commands"], "");
    assert_eq!(code, 0);
    assert!(list.as_array().unwrap().iter().any(|c| c["name"] == "wall_create"));
    let (code, schema, _) = ccad(&["schema", "door_create"], "");
    assert_eq!(code, 0);
    assert!(schema["input_schema"]["properties"]["wall"].is_object());
}

#[test]
fn mcp_session() {
    let messages = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "wall_create", "arguments": {"start": [0, 0], "end": [3, 0]}}}),
        json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "elements_list", "arguments": {}}}),
        json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "element_get", "arguments": {"id": 999999}}}),
        json!({"jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": {"name": "nope"}}),
        json!({"jsonrpc": "2.0", "id": 7, "method": "resources/list"}),
        json!({"jsonrpc": "2.0", "id": 8, "method": "ping"}),
    ];
    let input: String = messages.iter().map(|m| format!("{m}\n")).collect::<String>() + "not json\n";
    let mut child = Command::new(env!("CARGO_BIN_EXE_ccad"))
        .args(["mcp"])
        .env("CCAD_LANG", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    let replies: Vec<Value> =
        String::from_utf8_lossy(&out.stdout).lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    // No reply to the notification.
    assert_eq!(replies.len(), 9);
    let by_id = |id: i64| replies.iter().find(|r| r["id"] == id).unwrap();
    assert_eq!(by_id(1)["result"]["protocolVersion"], "2025-06-18");
    assert!(by_id(1)["result"]["capabilities"]["tools"].is_object());
    let tools = by_id(2)["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), cc_count());
    let list = tools.iter().find(|t| t["name"] == "elements_list").unwrap();
    assert_eq!(list["annotations"]["readOnlyHint"], true);
    assert_eq!(list["annotations"]["openWorldHint"], false);
    // Commands that read a file a parameter names are neither read-only nor closed-world.
    for name in ["cad_import", "graph_check", "project_save"] {
        let tool = tools.iter().find(|t| t["name"] == name).unwrap();
        assert_eq!(tool["annotations"]["readOnlyHint"], false, "{name}");
        assert_eq!(tool["annotations"]["openWorldHint"], true, "{name}");
    }
    assert_eq!(by_id(3)["result"]["isError"], false);
    assert_eq!(by_id(3)["result"]["structuredContent"]["category"], "wall");
    // The wall stays in the session for the next call.
    assert_eq!(by_id(4)["result"]["structuredContent"]["result"].as_array().unwrap().len(), 1);
    assert_eq!(by_id(5)["result"]["isError"], true);
    assert_eq!(by_id(6)["error"]["code"], -32602);
    assert_eq!(by_id(7)["error"]["code"], -32601);
    assert_eq!(by_id(8)["result"], json!({}));
    assert_eq!(replies.last().unwrap()["error"]["code"], -32700);
}

fn cc_count() -> usize {
    ccad(&["commands"], "").1.as_array().unwrap().len()
}

#[test]
fn version() {
    for flag in ["--version", "-V"] {
        let (code, out, err) = ccad(&[flag], "");
        assert_eq!(code, 0, "{err}");
        assert_eq!(
            out,
            Value::String(format!(
                "ccad {}
",
                env!("CARGO_PKG_VERSION")
            ))
        );
    }
}

#[test]
fn lua_scripts() {
    let script = r#"
local n = ccad.args.n
for i = 1, n do ccad.run("wall_create", { start = { i * 2, 0 }, ["end"] = { i * 2 + 1, 0 } }) end
print("made " .. n)
return { walls = #ccad.run("elements_list", { category = "wall" }) }
"#;
    let (code, out, err) = ccad(&["lua", "-", "n=3"], script);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, Value::String("made 3\n{\n  \"walls\": 3\n}\n".into()));
    let (code, _, err) = ccad(&["lua", "-"], "error('nope')");
    assert_eq!(code, 1);
    assert!(err.contains("script") && err.contains("nope"), "{err}");
}

#[test]
fn plugins_add_commands() {
    let dir = std::env::temp_dir().join(format!("ccad-cli-{}-plugins", std::process::id()));
    let plugin = dir.join("tools");
    std::fs::create_dir_all(&plugin).unwrap();
    std::fs::write(plugin.join("plugin.toml"), "id = \"tools\"\nname = \"Tools\"\nversion = \"1.0\"\n").unwrap();
    std::fs::write(
        plugin.join("main.lua"),
        r#"ccad.command { name = "pair", description = "Two walls",
             run = function(a)
               ccad.run("wall_create", { start = { 0, 0 }, ["end"] = { 1, 0 } })
               ccad.run("wall_create", { start = { 0, 1 }, ["end"] = { 1, 1 } })
               print("pair done")
               return { ok = true }
             end }"#,
    )
    .unwrap();
    let folder = dir.to_string_lossy().into_owned();
    let (code, list, err) = ccad(&["--plugins", &folder, "commands"], "");
    assert_eq!(code, 0, "{err}");
    let names: Vec<&str> = list.as_array().unwrap().iter().filter_map(|c| c["name"].as_str()).collect();
    assert!(names.contains(&"tools__pair"));
    assert_eq!(names.len(), cc_count() + 1);
    let (code, out, err) = ccad(&["--plugins", &folder, "run", "tools__pair"], "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, json!({"ok": true}));
    assert!(err.contains("[tools] pair done"), "{err}");
    let (code, summary, err) = ccad(&["--plugins", &folder, "plugins"], "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(summary[0]["id"], "tools");
    assert_eq!(summary[0]["commands"][0]["name"], "tools__pair");
    // A plugin that fails to load is reported, the others still run.
    std::fs::create_dir_all(dir.join("broken")).unwrap();
    std::fs::write(dir.join("broken").join("plugin.toml"), "id = 7").unwrap();
    let (code, _, err) = ccad(&["--plugins", &folder, "run", "tools__pair"], "");
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("plugin not loaded"), "{err}");
    // Over MCP, the plugin's command is a tool.
    let messages = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "tools__pair", "arguments": {}}}),
    ];
    let input: String = messages.iter().map(|m| format!("{m}\n")).collect();
    let (_, out, _) = ccad(&["--plugins", &folder, "mcp"], &input);
    let replies: Vec<Value> = out.as_str().unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert!(replies[0]["result"]["tools"].as_array().unwrap().iter().any(|t| t["name"] == "tools__pair"));
    assert_eq!(replies[1]["result"]["isError"], false);
}

#[test]
fn plugin_event_handlers_print_on_standard_error() {
    let dir = std::env::temp_dir().join(format!("ccad-cli-{}-events", std::process::id()));
    let plugin = dir.join("watch");
    std::fs::create_dir_all(&plugin).unwrap();
    std::fs::write(plugin.join("plugin.toml"), "id = \"watch\"\nname = \"Watch\"\nversion = \"1.0\"\napi = 2\n")
        .unwrap();
    std::fs::write(
        plugin.join("main.lua"),
        r#"ccad.on("project_saved", function(e) print("saved to " .. e.path) end)
           ccad.on("model_changed", function(e) print("changed by " .. e.command) end)"#,
    )
    .unwrap();
    let folder = dir.to_string_lossy().into_owned();
    let file = dir.join("watched.ccad").to_string_lossy().into_owned();
    let (code, _, err) =
        ccad(&["--plugins", &folder, "--save-as", &file, "run", "wall_create", "start=[0,0]", "end=[3,0]"], "");
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("[watch] changed by wall_create"), "{err}");
    assert!(err.contains(&format!("[watch] saved to {file}")), "{err}");
    // --save to the project's own file.
    let (code, _, err) = ccad(&["--plugins", &folder, "-p", &file, "--save", "run", "project_info"], "");
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("[watch] saved to"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn plugins_install_and_remove() {
    let dir = std::env::temp_dir().join(format!("ccad-cli-{}-store", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let plugin = dir.join("tools");
    std::fs::create_dir_all(&plugin).unwrap();
    std::fs::write(plugin.join("plugin.toml"), "id = \"tools\"\nname = \"Tools\"\nversion = \"1.0\"\n").unwrap();
    std::fs::write(plugin.join("main.lua"), "").unwrap();
    let folder = dir.to_string_lossy().into_owned();
    let not_zip = dir.join("not.zip");
    std::fs::write(&not_zip, "this is not a zip archive at all, honestly").unwrap();
    let (code, _, err) = ccad(&["plugins", "install", &not_zip.to_string_lossy(), &folder], "");
    assert_eq!(code, 1);
    assert!(err.contains("not a zip"), "{err}");
    let (code, out, err) = ccad(&["plugins", "remove", "tools", &folder], "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(out["removed"], "tools");
    assert!(!plugin.exists());
    assert_eq!(ccad(&["plugins", "remove", "tools", &folder], "").0, 1);
    assert_eq!(ccad(&["plugins", "frobnicate"], "").0, 2);
}

#[test]
fn plugins_settings_are_shown_and_stored() {
    let dir = std::env::temp_dir().join(format!("ccad-cli-{}-settings", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let plugin = dir.join("tools");
    std::fs::create_dir_all(plugin.join("locales")).unwrap();
    std::fs::write(
        plugin.join("plugin.toml"),
        r#"id = "tools"
name = "Tools"
version = "2.0"
api = 2

[settings.height]
type = "number"
default = 2.8
title = "%height%"
minimum = 1

[settings.mode]
type = "string"
default = "fast"
enum = ["fast", "exact"]

[settings.label]
type = "string"
default = "A"

[settings.count]
type = "integer"
default = 1
"#,
    )
    .unwrap();
    std::fs::write(plugin.join("locales").join("en.toml"), "height = \"Wall height\"\n").unwrap();
    // The code does not run to show or change settings.
    std::fs::write(plugin.join("main.lua"), "error('ran')").unwrap();
    let folder = dir.to_string_lossy().into_owned();
    let (code, out, err) = ccad(&["plugins", "settings", &folder, "tools"], "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(out["id"], "tools");
    let settings = out["settings"].as_array().unwrap();
    assert_eq!(settings.len(), 4);
    assert_eq!(settings[0]["key"], "height");
    assert_eq!(settings[0]["title"], "Wall height");
    assert_eq!(settings[0]["value"], 2.8);
    assert_eq!(settings[1]["enum"], json!(["fast", "exact"]));

    let (code, out, err) =
        ccad(&["plugins", "settings", &folder, "tools", "height=3.5", "mode=exact", "label=42", "count=3"], "");
    assert_eq!(code, 0, "{err}");
    let values: Vec<&Value> = out["settings"].as_array().unwrap().iter().map(|s| &s["value"]).collect();
    assert_eq!(values, [&json!(3.5), &json!("exact"), &json!("42"), &json!(3)]);
    let stored: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join(".settings").join("tools.json")).unwrap()).unwrap();
    assert_eq!(stored, json!({"height": 3.5, "mode": "exact", "label": "42", "count": 3}));
    let (_, out, _) = ccad(&["plugins", "settings", &folder, "tools"], "");
    assert_eq!(out["settings"][0]["value"], 3.5);

    // Values that do not fit are refused and nothing is written.
    for bad in ["height=0.5", "mode=slow", "count=1.5", "height=tall", "nope=1", "height"] {
        let (code, _, err) = ccad(&["plugins", "settings", &folder, "tools", bad, "count=5"], "");
        assert_eq!(code, 2, "{bad}: {err}");
    }
    let (_, out, _) = ccad(&["plugins", "settings", &folder, "tools"], "");
    assert_eq!(out["settings"][3]["value"], 3);
    // An unknown plugin or a bad id.
    assert_eq!(ccad(&["plugins", "settings", &folder, "other"], "").0, 1);
    assert_eq!(ccad(&["plugins", "settings", &folder, "Bad Id"], "").0, 2);
    // The plugins of the folder start with the stored values.
    std::fs::write(plugin.join("main.lua"), "").unwrap();
    let (code, summary, err) = ccad(&["--plugins", &folder, "plugins"], "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(summary[0]["settings"][0]["value"], 3.5);
    assert_eq!(summary[0]["api"], 2);
}

#[test]
fn plugin_exporters_write_the_path_given() {
    let dir = std::env::temp_dir().join(format!("ccad-cli-{}-exporters", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let plugin = dir.join("idf");
    std::fs::create_dir_all(&plugin).unwrap();
    let manifest = "id = \"idf\"\nname = \"IDF\"\nversion = \"1.0\"\napi = 2\n\n\
                    [[exporters]]\ncommand = \"export\"\nlabel = \"EnergyPlus model (.idf)\"\n";
    std::fs::write(plugin.join("plugin.toml"), manifest).unwrap();
    std::fs::write(
        plugin.join("main.lua"),
        r#"ccad.command { name = "export", kind = "query",
             params = { required = { "path" }, properties = {
               path = { type = "string", ui = "save_path", extensions = { "idf" } } } },
             run = function(a)
               local levels = ccad.run("levels_list", {})
               ccad.file.write(a.path, { "Version,24.1;\n", "! levels: ", tostring(#levels), "\n" })
               return { path = a.path }
             end }"#,
    )
    .unwrap();
    let folder = dir.to_string_lossy().into_owned();
    let out = dir.join("model.idf");
    let arg = format!("path={}", out.to_string_lossy());
    let (code, _, err) = ccad(&["--plugins", &folder, "run", "idf__export", &arg], "");
    assert_eq!(code, 0, "{err}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.starts_with("Version,24.1;\n! levels: "), "{text}");
    // Another extension is refused before the command runs.
    let bad = format!("path={}", dir.join("model.txt").to_string_lossy());
    let (code, _, err) = ccad(&["--plugins", &folder, "run", "idf__export", &bad], "");
    assert_ne!(code, 0);
    assert!(err.contains(".idf"), "{err}");
    assert!(!dir.join("model.txt").exists());
    // The listing tells the exporter.
    let (code, summary, err) = ccad(&["--plugins", &folder, "plugins"], "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(summary[0]["exporters"][0]["command"], "idf__export");
    assert_eq!(summary[0]["exporters"][0]["extensions"], json!(["idf"]));
    let _ = std::fs::remove_dir_all(&dir);
}

/// `ccad plugins tools` shows and stores the program a plugin's tool runs
/// (here `ccad` itself), `--tool` gives it for one run, and the plugin runs
/// it with `ccad.process.run`, waited for.
#[test]
fn plugin_tools_are_chosen_and_run() {
    let dir = std::env::temp_dir().join(format!("ccad-cli-{}-tools", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let plugin = dir.join("runner");
    std::fs::create_dir_all(&plugin).unwrap();
    std::fs::write(
        plugin.join("plugin.toml"),
        r#"id = "runner"
name = "Runner"
version = "1.0"
api = 2
permissions = ["tools"]

[[tools]]
id = "self"
name = "ConstructCraft CLI"
executables = ["ccad.exe", "ccad"]
version_args = ["--version"]
"#,
    )
    .unwrap();
    std::fs::write(
        plugin.join("main.lua"),
        r#"ccad.command { name = "version", kind = "query", run = function()
  local r = ccad.process.run("self", { args = { "--version" } })
  return { code = r.code, out = r.stdout }
end }"#,
    )
    .unwrap();
    let folder = dir.to_string_lossy().into_owned();
    let exe = env!("CARGO_BIN_EXE_ccad");
    let (code, out, err) = ccad(&["plugins", "tools", &folder, "runner"], "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(out["tools"][0]["id"], "self");
    assert_eq!(out["tools"][0]["path"], Value::Null);
    assert_eq!(out["tools"][0]["problem"], "no program chosen");

    // Not chosen: the command says what to configure.
    let (code, _, err) = ccad(&["--plugins", &folder, "run", "runner__version"], "");
    assert_eq!(code, 1);
    assert!(err.contains("no program was chosen for ConstructCraft CLI (self)"), "{err}");

    // A program of another name is refused, and nothing is stored.
    let manifest = plugin.join("plugin.toml").to_string_lossy().into_owned();
    let (code, _, err) = ccad(&["plugins", "tools", &folder, "runner", &format!("self={manifest}")], "");
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("must be named ccad.exe or ccad"), "{err}");
    assert_eq!(ccad(&["plugins", "tools", &folder, "runner", "other=x"], "").0, 2);
    assert!(!dir.join(".tools").join("runner.json").exists());

    let (code, out, err) = ccad(&["plugins", "tools", &folder, "runner", &format!("self={exe}"), "--test"], "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(out["tools"][0]["problem"], Value::Null);
    assert_eq!(out["tools"][0]["test"]["ok"], true, "{out}");
    assert!(out["tools"][0]["test"]["output"].as_str().unwrap().starts_with("ccad "), "{out}");
    let stored: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join(".tools").join("runner.json")).unwrap()).unwrap();
    assert_eq!(std::path::PathBuf::from(stored["self"].as_str().unwrap()), std::path::PathBuf::from(exe));

    // The folder of plugins runs it, waited for.
    let (code, out, err) = ccad(&["--plugins", &folder, "run", "runner__version"], "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(out["code"], 0);
    assert!(out["out"].as_str().unwrap().starts_with("ccad "), "{out}");
    let (_, summary, _) = ccad(&["--plugins", &folder, "plugins"], "");
    assert_eq!(summary[0]["granted"]["tools"], true);
    assert_eq!(summary[0]["tools"][0]["problem"], Value::Null);

    // `self=` forgets it.
    let (code, out, _) = ccad(&["plugins", "tools", &folder, "runner", "self="], "");
    assert_eq!((code, out["tools"][0]["path"].clone()), (0, Value::Null));
    assert!(!dir.join(".tools").join("runner.json").exists());

    // --tool gives it for one run, also to a plugin loaded from its own folder.
    let single = plugin.to_string_lossy().into_owned();
    let tool = format!("runner.self={exe}");
    let (code, out, err) = ccad(&["--plugins", &single, "--tool", &tool, "run", "runner__version"], "");
    assert_eq!(code, 0, "{err}");
    assert!(out["out"].as_str().unwrap().starts_with("ccad "), "{out}");
    let bad = format!("runner.self={manifest}");
    let (code, _, err) = ccad(&["--plugins", &single, "--tool", &bad, "run", "runner__version"], "");
    assert_eq!(code, 1);
    assert!(err.contains("--tool runner.self") && err.contains("must be named"), "{err}");
    assert_eq!(ccad(&["--tool", "nonsense", "commands"], "").0, 2);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The output of `ccad` as text, its exit code and standard error.
fn ccad_text(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_ccad")).args(args).env("CCAD_LANG", "en").output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn the_example_plugin_passes_its_tests() {
    let folder = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/plugins/quick_rooms");
    let (code, out, err) = ccad_text(&["plugins", "test", folder]);
    assert_eq!(code, 0, "{out}\n{err}");
    assert!(out.contains("PASS quick_rooms_test.lua::test_a_room_has_four_walls_a_floor_and_a_room"), "{out}");
    assert!(out.contains("PASS quick_rooms_test.lua::test_walls_take_the_default_height_setting"), "{out}");
    assert!(out.trim_end().ends_with("5 passed, 0 failed"), "{out}");
    let (code, out, _) = ccad_text(&["plugins", "test", folder, "--filter", "door"]);
    assert_eq!(code, 0);
    assert_eq!(out.lines().count(), 2, "one test and the summary: {out}");
}

#[test]
fn plugin_tests_report_failures_with_exit_codes() {
    let dir = std::env::temp_dir().join(format!("ccad-cli-{}-plugin-tests", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::write(dir.join("plugin.toml"), "id = \"t\"\nname = \"T\"\nversion = \"1\"\napi = 2\n").unwrap();
    std::fs::write(dir.join("main.lua"), "ccad.command { name = 'n', run = function() print('ran n') return 1 end }")
        .unwrap();
    std::fs::write(
        dir.join("tests/n_test.lua"),
        "function test_good() ccad.test.eq(ccad.run('t__n'), 1) end\n\
         function test_bad() ccad.run('t__n') ccad.test.eq(ccad.run('t__n'), 2, 'the answer') end",
    )
    .unwrap();
    let folder = dir.to_string_lossy().into_owned();
    let (code, out, _) = ccad_text(&["plugins", "test", &folder]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("PASS n_test.lua::test_good"), "{out}");
    assert!(out.contains("FAIL n_test.lua::test_bad: ") && out.contains("the answer: expected 2, got 1"), "{out}");
    assert!(out.contains("    [t] ran n"), "what the failed test printed: {out}");
    assert!(out.contains("1 passed, 1 failed"), "{out}");
    let (code, out, _) = ccad_text(&["plugins", "test", &folder, "--filter", "good"]);
    assert_eq!((code, out.contains("1 passed, 0 failed")), (0, true), "{out}");
    let (code, _, err) = ccad_text(&["plugins", "test", &folder, "--filter", "nothing"]);
    assert_eq!(code, 1, "no test matches: {err}");
    // Usage errors.
    assert_eq!(ccad_text(&["plugins", "test"]).0, 2);
    assert_eq!(ccad_text(&["plugins", "test", &folder, "--frobnicate"]).0, 2);
    let empty = dir.join("tests").to_string_lossy().into_owned();
    assert_eq!(ccad_text(&["plugins", "test", &empty]).0, 2, "not a plugin");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_new_plugin_loads_and_passes_its_test() {
    let dir = std::env::temp_dir().join(format!("ccad-cli-{}-new-plugin", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let folder = dir.to_string_lossy().into_owned();
    let (code, out, err) = ccad(&["plugins", "new", &folder, "my_walls", "name=My \"walls\""], "");
    assert_eq!(code, 0, "{err}");
    let plugin = dir.join("my_walls");
    assert_eq!(out["created"].as_str().map(std::path::PathBuf::from), Some(plugin.clone()));
    for file in [
        "plugin.toml",
        "main.lua",
        "locales/en.toml",
        "locales/pt-BR.toml",
        "icons/my_walls.svg",
        "tests/basic_test.lua",
        "README.md",
        ".luarc.json",
        "types/ccad.d.lua",
    ] {
        assert!(plugin.join(file).is_file(), "{file}");
    }
    let luarc: Value = serde_json::from_str(&std::fs::read_to_string(plugin.join(".luarc.json")).unwrap()).unwrap();
    assert_eq!(luarc["workspace.library"], json!(["types"]));
    // It loads, with its name, button and setting.
    let path = plugin.to_string_lossy().into_owned();
    let (code, summary, err) = ccad(&["--plugins", &path, "plugins"], "");
    assert_eq!(code, 0, "{err}");
    assert!(!err.contains("not loaded"), "{err}");
    assert_eq!(summary[0]["name"], "My \"walls\"");
    assert_eq!(summary[0]["buttons"][0]["command"], "my_walls__wall_line");
    assert_eq!(summary[0]["settings"][0]["key"], "wall_height");
    // Its tests pass.
    let (code, out, err) = ccad_text(&["plugins", "test", &path]);
    assert_eq!(code, 0, "{out}\n{err}");
    assert!(out.contains("2 passed, 0 failed"), "{out}");
    // An existing folder or a bad id is refused.
    assert_eq!(ccad(&["plugins", "new", &folder, "my_walls"], "").0, 2);
    assert_eq!(ccad(&["plugins", "new", &folder, "Bad-Id"], "").0, 2);
    assert!(!dir.join("Bad-Id").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// `ccad rvt-to-ifc` converts a Revit file, and `-p file.rvt` imports one through it in a child process.
#[cfg(feature = "rvt")]
#[test]
fn revit_files_are_converted_in_a_child_process() {
    let rvt = concat!(env!("CARGO_MANIFEST_DIR"), "/../cc_rvt/tests/fixtures/synthetic.rvt");
    let (ifc, report) = (scratch("synthetic.ifc"), scratch("synthetic.json"));
    let (code, out, err) = ccad(&["rvt-to-ifc", rvt, &ifc], "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(out["Ok"]["revit_version"], 2024, "{out}");
    assert!(std::fs::read_to_string(&ifc).unwrap().contains("IFCPROJECT("));
    assert_eq!(ccad(&["rvt-to-ifc", rvt, &ifc, &report], "").0, 0);
    let written: Value = serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    assert_eq!(written, out);

    let (code, levels, err) = ccad(&["-p", rvt, "run", "levels_list"], "");
    assert_eq!(code, 0, "{err}");
    assert!(levels.is_array(), "{levels}");

    let (code, out, _) = ccad(&["rvt-to-ifc", &report, &ifc], "");
    assert_eq!(code, 1);
    assert!(out["Err"]["read"].is_string(), "{out}");
    assert_eq!(ccad(&["rvt-to-ifc", rvt], "").0, 2);
}
