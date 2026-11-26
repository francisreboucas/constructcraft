//! `ccad`: ConstructCraft without a GUI. Runs engine commands from the
//! command line, from scripts, or for AI agents as an MCP server; all three
//! use the engine's one command catalog, plus the commands of the plugins
//! loaded with `--plugins`.

mod mcp;
mod scaffold;

use cc_engine::{EngineError, Session};
use serde_json::{Value, json};
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
ccad - ConstructCraft without a GUI

Usage:
  ccad [options] commands             list the commands
  ccad [options] schema <command>     the JSON Schema of a command's parameters
  ccad reference                      the command reference in Markdown
  ccad [options] run <command> [args] run one command
  ccad [options] script <file|-> [level=<id|name>]
                                      run a script of commands
  ccad [options] lua <file|-> [args]  run a Lua script
  ccad [options] graph <file> [inputs] run a visual programming graph (.ccgraph)
  ccad [options] plugins              list the plugins loaded with --plugins
  ccad plugins install <zip> <folder> install a plugin archive into a plugins folder
  ccad plugins remove <id> <folder>   remove an installed plugin
  ccad plugins settings <folder> <id> [key=value ...]
                                      show a plugin's settings, or change and
                                      store them (in <folder>/.settings)
  ccad plugins tools <folder> <id> [tool=path ...] [--test]
                                      show the external programs a plugin may
                                      run, or choose them (in <folder>/.tools;
                                      tool= forgets one); --test runs each
                                      one's version check
  ccad plugins new <folder> <id> [name=<name>]
                                      make a new plugin in <folder>/<id>
  ccad [--tool ...] plugins test <plugin folder> [--filter <text>] [--tool ...]
                                      run the plugin's tests/*_test.lua, each
                                      over a new project with the plugin loaded
  ccad [options] mcp                  serve the commands over MCP (stdio)

Arguments of `run`: a JSON object, @file.json, - (standard input), or
key=value pairs whose values are JSON or plain text, e.g.
  ccad -p house.ccad --save run wall_create start=[0,0] end=[5,0] height=2.8

A script is a JSON array of {\"command\": ..., \"args\": {...}} objects, or one
such object per line; it stops at the first error. {\"$step\": n} in its
arguments is the id step n created (n from 1), {\"$level\": \"active\"} the
level given as level=<id|name> (default: the lowest level).

A graph (.ccgraph) takes its named input nodes as key=value pairs, e.g.
  ccad --save-as row.ccad graph examples/graphs/house.ccgraph width=12
It prints each failing node, its output nodes and the elements created,
changed and deleted.

A Lua script calls commands with ccad.run(name, args) and gets its arguments
(same forms as `run`) as ccad.args; what it prints goes to standard output,
what it returns is printed as JSON.

Options:
  -p, --project <file>   open a project (.ccad, or .ifc to import) first
  --save                 save the project afterwards (to its file)
  --save-as <file>       save the project afterwards to <file>
  --plugins <folder>     load the plugin in <folder>, or each plugin in its
                         subfolders (repeatable); their commands are named
                         <plugin>__<command> and get the permissions their
                         plugin.toml asks for
  --tool <plugin>.<tool>=<path>
                         the program a plugin's tool runs, for this run
                         (repeatable; checked like `plugins tools`)
  --allow-files          let a graph run commands that read or write files
                         (open, save, import, export); a graph file cannot
                         allow them itself
  --lang <code>          language of default names (default: the system's):
                         en, pt-BR, es, fr, de, it, lo, pl, uk-UA, ru, zh-TW, zh-CN or ja
  -V, --version          the version
  -h, --help             this help

Environment: CCAD_LANG sets the language like --lang.
";

/// The help of the Revit import, in builds with the `rvt` feature.
#[cfg(feature = "rvt")]
const RVT_USAGE: &str = "
Revit files (this build has the rvt feature):
  ccad rvt-to-ifc <file.rvt> <out.ifc> [<report.json>]
                         convert a Revit file to IFC4; the report (JSON) goes
                         to <report.json> or standard output
  -p also takes a .rvt file, which rvt_import converts in a child process
";
#[cfg(not(feature = "rvt"))]
const RVT_USAGE: &str = "";

struct Options {
    project: Option<PathBuf>,
    save: bool,
    save_as: Option<PathBuf>,
    plugins: Vec<PathBuf>,
    /// `--tool`: plugin id, tool id, program.
    tools: Vec<(String, String, PathBuf)>,
    /// `graph` may run file commands (`graph_run`'s `allow_files`).
    allow_files: bool,
    rest: Vec<String>,
}

/// `<plugin>.<tool>=<path>` of `--tool`.
fn parse_tool(spec: &str) -> Result<(String, String, PathBuf), String> {
    let bad = || format!("--tool {spec}: expected <plugin>.<tool>=<path>");
    let (names, path) = spec.split_once('=').ok_or_else(bad)?;
    let (plugin, tool) = names.split_once('.').ok_or_else(bad)?;
    if plugin.is_empty() || tool.is_empty() || path.is_empty() {
        return Err(bad());
    }
    let path = std::path::absolute(path).map_err(|e| format!("--tool {spec}: {e}"))?;
    Ok((plugin.to_string(), tool.to_string(), path))
}

fn parse_options(args: Vec<String>) -> Result<Options, String> {
    let mut o = Options {
        project: None,
        save: false,
        save_as: None,
        plugins: Vec::new(),
        tools: Vec::new(),
        allow_files: false,
        rest: Vec::new(),
    };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-p" | "--project" => o.project = Some(it.next().ok_or("--project needs a file")?.into()),
            "--save" => o.save = true,
            "--allow-files" => o.allow_files = true,
            "-V" | "--version" => o.rest.push("--version".into()),
            "--save-as" => o.save_as = Some(it.next().ok_or("--save-as needs a file")?.into()),
            "--plugins" => o.plugins.push(it.next().ok_or("--plugins needs a folder")?.into()),
            "--tool" => o.tools.push(parse_tool(&it.next().ok_or("--tool needs <plugin>.<tool>=<path>")?)?),
            "--lang" => {
                let code = it.next().ok_or("--lang needs a language")?;
                let lang = cc_engine::i18n::Language::from_code(&code).ok_or(format!("unsupported language {code}"))?;
                cc_engine::i18n::set_language(lang);
            }
            _ if o.rest.is_empty() && a.starts_with('-') && a != "-" => return Err(format!("unknown option {a}")),
            _ => {
                o.rest.push(a);
                o.rest.extend(it.by_ref());
            }
        }
    }
    Ok(o)
}

/// The plugins of `--plugins`, each granted what its manifest asks for (the
/// user named the folders), with the programs of `--tool`; plugins that fail
/// to load, and programs that do not fit their tool, are reported on
/// standard error and left out.
fn plugins(o: &Options) -> cc_plugins::PluginHost {
    let mut host = cc_plugins::PluginHost::new();
    for folder in &o.plugins {
        for e in host.load_folder(folder, cc_plugins::Limits::default(), |m| Some(m.requested())) {
            eprintln!("ccad: plugin not loaded: {e}");
        }
    }
    for (plugin, tool, path) in &o.tools {
        if let Err(e) = host.set_tool_path(plugin, tool, Some(path.clone())) {
            eprintln!("ccad: --tool {plugin}.{tool}: {e}");
        }
    }
    host.set_cache_dir(&default_plugin_cache_dir());
    host
}

fn default_plugin_cache_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_CACHE_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(dir).join("constructcraft").join("plugin-cache");
    }
    if let Some(dir) = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty()) {
        return PathBuf::from(dir).join("ConstructCAD").join("plugin-cache");
    }
    if let Some(home) = std::env::var_os("HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(home).join(".cache").join("constructcraft").join("plugin-cache");
    }
    std::env::temp_dir().join("constructcraft-plugin-cache")
}

/// The session for the options: a new project, or the given file (`.ifc`,
/// and `.rvt` with the `rvt` feature, are imported), with the plugins of
/// `--plugins`.
fn session(o: &Options) -> Result<Session, EngineError> {
    let mut s = Session::default();
    if !o.plugins.is_empty() {
        s.set_extension(Some(Box::new(plugins(o))));
        flush_messages(&mut s);
    }
    if let Some(path) = &o.project {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
        let command = match ext.as_str() {
            "ifc" => "ifc_import",
            #[cfg(feature = "rvt")]
            "rvt" => "rvt_import",
            _ => "project_open",
        };
        s.execute(command, json!({ "path": path }))?;
    }
    Ok(s)
}

/// What plugins printed, on standard error (standard output carries results).
fn flush_messages(s: &mut Session) {
    for line in s.take_extension_messages() {
        eprintln!("{line}");
    }
}

fn read_input(spec: &str) -> Result<String, String> {
    if spec == "-" {
        let mut text = String::new();
        std::io::stdin().read_to_string(&mut text).map_err(|e| e.to_string())?;
        Ok(text)
    } else {
        std::fs::read_to_string(spec).map_err(|e| format!("{spec}: {e}"))
    }
}

/// Command arguments from the command line.
fn parse_args(args: &[String]) -> Result<Value, String> {
    match args {
        [] => Ok(json!({})),
        [one] if one.trim_start().starts_with('{') => serde_json::from_str(one).map_err(|e| e.to_string()),
        [one] if one == "-" || one.starts_with('@') => {
            let text = read_input(one.strip_prefix('@').unwrap_or(one))?;
            serde_json::from_str(&text).map_err(|e| e.to_string())
        }
        pairs => {
            let mut map = serde_json::Map::new();
            for pair in pairs {
                let (k, v) = pair.split_once('=').ok_or(format!("expected key=value, got '{pair}'"))?;
                let value = serde_json::from_str(v).unwrap_or_else(|_| Value::String(v.to_string()));
                map.insert(k.to_string(), value);
            }
            Ok(Value::Object(map))
        }
    }
}

fn print(v: &Value) {
    println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
}

fn fail(e: &EngineError) -> ExitCode {
    eprintln!("{}", serde_json::to_string_pretty(&json!({ "error": e })).unwrap_or_default());
    ExitCode::from(1)
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("ccad: {message}\n\n{USAGE}");
    ExitCode::from(2)
}

/// `ccad plugins settings <folder> <id> [key=value ...]`: print the
/// settings of the plugin `id` of the plugins folder `folder` with their
/// current values, after checking and storing the given ones. The plugin's
/// code does not run.
fn plugin_settings(folder: &str, id: &str, pairs: &[String]) -> ExitCode {
    let folder = std::path::Path::new(folder);
    if !cc_plugins::plugin::valid_id(id) {
        return usage_error(&format!("\"{id}\" is not a plugin id"));
    }
    let (manifest, settings) = match cc_plugins::plugin::settings_schema(&folder.join(id)) {
        Ok(found) => found,
        Err(e) => return fail(&EngineError::io(e)),
    };
    if manifest.id != id {
        return fail(&EngineError::io(format!("the folder {id} holds the plugin \"{}\"", manifest.id)));
    }
    // The stored values that still fit, then the new ones.
    let mut stored: serde_json::Map<String, Value> = cc_plugins::store::read_settings(folder, id)
        .into_iter()
        .filter(|(k, v)| settings.iter().any(|s| &s.key == k && s.validate(v).is_ok()))
        .collect();
    for pair in pairs {
        let Some((key, text)) = pair.split_once('=') else {
            return usage_error(&format!("expected key=value, got '{pair}'"));
        };
        let Some(setting) = settings.iter().find(|s| s.key == key) else {
            let known: Vec<&str> = settings.iter().map(|s| s.key.as_str()).collect();
            return usage_error(&format!("the plugin {id} has no setting \"{key}\" (settings: {})", known.join(", ")));
        };
        // Like `run`'s arguments: JSON or plain text; a string setting takes
        // the text as it is.
        let value = match setting.kind {
            cc_plugins::SettingKind::String => Value::String(text.to_string()),
            _ => serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_string())),
        };
        match setting.validate(&value) {
            Ok(v) => {
                stored.insert(key.to_string(), v);
            }
            Err(e) => return usage_error(&format!("invalid setting: {e}")),
        }
    }
    if !pairs.is_empty()
        && let Err(e) = cc_plugins::store::write_settings(folder, id, &stored)
    {
        return fail(&EngineError::io(e));
    }
    let values = cc_plugins::contributions::current_values(&settings, &stored);
    print(&json!({ "id": id, "settings": cc_plugins::contributions::settings_json(&settings, &values) }));
    ExitCode::SUCCESS
}

/// `ccad plugins tools <folder> <id> [tool=path ...] [--test]`: print the
/// external programs of the plugin `id` of the plugins folder `folder`, the
/// path chosen for each (and why it cannot run), a suggestion found on
/// `PATH`, after checking and storing the given paths (`tool=` forgets
/// one); `--test` runs each chosen program's version check. The plugin's
/// code does not run.
fn plugin_tools(folder: &str, id: &str, rest: &[String]) -> ExitCode {
    let folder = std::path::Path::new(folder);
    if !cc_plugins::plugin::valid_id(id) {
        return usage_error(&format!("\"{id}\" is not a plugin id"));
    }
    let (manifest, tools) = match cc_plugins::plugin::tools_schema(&folder.join(id), folder) {
        Ok(found) => found,
        Err(e) => return fail(&EngineError::io(e)),
    };
    if manifest.id != id {
        return fail(&EngineError::io(format!("the folder {id} holds the plugin \"{}\"", manifest.id)));
    }
    let test = rest.iter().any(|a| a == "--test");
    let mut stored: std::collections::BTreeMap<String, PathBuf> = cc_plugins::tools::read_tool_paths(folder, id)
        .into_iter()
        .filter(|(k, _)| tools.iter().any(|t| &t.id == k))
        .collect();
    let pairs: Vec<&String> = rest.iter().filter(|a| *a != "--test").collect();
    for pair in &pairs {
        let Some((key, text)) = pair.split_once('=') else {
            return usage_error(&format!("expected tool=path, got '{pair}'"));
        };
        let Some(tool) = tools.iter().find(|t| t.id == key) else {
            let known: Vec<&str> = tools.iter().map(|t| t.id.as_str()).collect();
            return usage_error(&format!("the plugin {id} has no tool \"{key}\" (tools: {})", known.join(", ")));
        };
        if text.is_empty() {
            stored.remove(key);
            continue;
        }
        let path = match std::path::absolute(text) {
            Ok(p) => p,
            Err(e) => return usage_error(&format!("{text}: {e}")),
        };
        if let Err(e) = cc_plugins::tools::check_path(&tool.executables, &path) {
            return usage_error(&format!("invalid program for {key}: {e}"));
        }
        stored.insert(key.to_string(), path);
    }
    if !pairs.is_empty()
        && let Err(e) = cc_plugins::tools::write_tool_paths(folder, id, &stored)
    {
        return fail(&EngineError::io(e));
    }
    let list: Vec<Value> = tools
        .into_iter()
        .map(|t| {
            let tool = cc_plugins::Tool { path: stored.get(&t.id).cloned(), ..t };
            let problem = tool.problem();
            let mut out = json!({
                "id": tool.id,
                "name": tool.name,
                "executables": tool.executables,
                "homepage": tool.homepage,
                "version_args": tool.version_args,
                "path": tool.path,
                "problem": problem,
                "suggestion": cc_plugins::tools::find_on_path(&tool.executables),
            });
            if let (true, None, Some(path)) = (test, &problem, &tool.path) {
                out["test"] = match cc_plugins::tools::test(id, &tool, path) {
                    Ok(text) => json!({ "ok": true, "output": text }),
                    Err(e) => json!({ "ok": false, "error": e }),
                };
            }
            out
        })
        .collect();
    print(&json!({ "id": id, "tools": list }));
    ExitCode::SUCCESS
}

/// `ccad plugins new <folder> <id> [name=<name>]`: make a new plugin from
/// the template ([`scaffold`]).
fn plugin_new(folder: &str, id: &str, rest: &[String]) -> ExitCode {
    let name = match rest {
        [] => None,
        [one] => match one.strip_prefix("name=") {
            Some(name) => Some(name),
            None => return usage_error(&format!("expected name=<name>, got '{one}'")),
        },
        _ => return usage_error("plugins new takes a folder, an id and at most name=<name>"),
    };
    match scaffold::create(std::path::Path::new(folder), id, name) {
        Ok(dir) => {
            print(&json!({ "created": dir }));
            ExitCode::SUCCESS
        }
        Err(e) => usage_error(&e),
    }
}

/// `ccad plugins test <plugin folder> [--filter <text>] [--tool …]`: run the
/// plugin's tests ([`cc_plugins::testing`]); one line per test (`PASS
/// name`, or `FAIL name: message` and what it printed), then a summary.
/// Exit code 1 when a test failed (or none matched the filter), 2 on usage
/// errors.
fn plugin_test(o: &Options, rest: &[String]) -> ExitCode {
    let mut folder = None;
    let mut options = cc_plugins::testing::TestOptions { tools: o.tools.clone(), ..Default::default() };
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--filter" => match it.next() {
                Some(f) => options.filter = Some(f.clone()),
                None => return usage_error("--filter needs a text"),
            },
            "--tool" => match it.next().map(|t| parse_tool(t)) {
                Some(Ok(tool)) => options.tools.push(tool),
                Some(Err(e)) => return usage_error(&e),
                None => return usage_error("--tool needs <plugin>.<tool>=<path>"),
            },
            _ if a.starts_with("--") => return usage_error(&format!("unknown option {a}")),
            _ if folder.is_none() => folder = Some(a.clone()),
            _ => return usage_error("plugins test takes one plugin folder"),
        }
    }
    let Some(folder) = folder else { return usage_error("plugins test needs a plugin's folder") };
    let results = match cc_plugins::testing::run_tests(std::path::Path::new(&folder), &options) {
        Ok(r) => r,
        Err(e) => return usage_error(&e),
    };
    let failed = results.iter().filter(|r| r.result.is_err()).count();
    for r in &results {
        match &r.result {
            Ok(()) => println!("PASS {}", r.name),
            Err(message) => {
                println!("FAIL {}: {message}", r.name);
                for line in &r.output {
                    println!("    {line}");
                }
            }
        }
    }
    println!("{} passed, {failed} failed", results.len() - failed);
    if results.is_empty() {
        eprintln!("ccad: no test matches");
    }
    if failed > 0 || results.is_empty() { ExitCode::from(1) } else { ExitCode::SUCCESS }
}

/// Save when asked (after the commands ran), then show what plugins printed
/// meanwhile (their `project_saving` and `project_saved` handlers).
fn save(s: &mut Session, o: &Options) -> Result<(), EngineError> {
    let result = if let Some(path) = &o.save_as {
        s.execute("project_save", json!({ "path": path })).map(drop)
    } else if o.save {
        s.execute("project_save", json!({})).map(drop)
    } else {
        Ok(())
    };
    flush_messages(s);
    result
}

fn main() -> ExitCode {
    // The Revit converter that `rvt_import` starts; read before `std::env::args`, which takes UTF-8 only.
    #[cfg(feature = "rvt")]
    {
        let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
        if let Some((first, rest)) = args.split_first()
            && first == cc_engine::rvt::SUBCOMMAND
        {
            return ExitCode::from(cc_engine::rvt::child_main(rest));
        }
    }
    if let Some(lang) = std::env::var("CCAD_LANG").ok().and_then(|c| cc_engine::i18n::Language::from_code(&c)) {
        cc_engine::i18n::set_language(lang);
    } else {
        cc_engine::i18n::set_language(cc_engine::i18n::Language::from_system());
    }
    let o = match parse_options(std::env::args().skip(1).collect()) {
        Ok(o) => o,
        Err(e) => return usage_error(&e),
    };
    let Some((sub, rest)) = o.rest.split_first() else {
        print!("{USAGE}{RVT_USAGE}");
        return ExitCode::SUCCESS;
    };
    match sub.as_str() {
        "--version" => {
            println!("ccad {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        "-h" | "--help" | "help" => {
            print!("{USAGE}{RVT_USAGE}");
            ExitCode::SUCCESS
        }
        "commands" => {
            let mut s = Session::default();
            s.set_extension(Some(Box::new(plugins(&o))));
            let list: Vec<Value> = s
                .commands()
                .iter()
                .map(|c| json!({"name": c.name, "kind": c.kind, "description": c.description}))
                .collect();
            print(&json!(list));
            ExitCode::SUCCESS
        }
        "reference" => {
            print!("{}", cc_engine::reference::markdown());
            ExitCode::SUCCESS
        }
        "schema" => {
            let mut s = Session::default();
            s.set_extension(Some(Box::new(plugins(&o))));
            match rest.first().and_then(|n| s.describe(n)) {
                Some(info) => {
                    print(&serde_json::to_value(&info).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                None => usage_error("schema needs a known command name"),
            }
        }
        "plugins" => match rest {
            [] => {
                let host = plugins(&o);
                print(&host.summary());
                ExitCode::SUCCESS
            }
            [action, archive, folder] if action == "install" => {
                let bytes = match std::fs::read(archive) {
                    Ok(b) => b,
                    Err(e) => return usage_error(&format!("{archive}: {e}")),
                };
                match cc_plugins::store::install_archive(&bytes, std::path::Path::new(folder)) {
                    Ok(dir) => {
                        print(&json!({ "installed": dir }));
                        ExitCode::SUCCESS
                    }
                    Err(e) => fail(&EngineError::new(cc_engine::ErrorCode::Script, e)),
                }
            }
            [action, id, folder] if action == "remove" => {
                match cc_plugins::store::uninstall(std::path::Path::new(folder), id) {
                    Ok(()) => {
                        print(&json!({ "removed": id }));
                        ExitCode::SUCCESS
                    }
                    Err(e) => fail(&EngineError::io(e)),
                }
            }
            [action, folder, id, pairs @ ..] if action == "settings" => plugin_settings(folder, id, pairs),
            [action, folder, id, rest @ ..] if action == "tools" => plugin_tools(folder, id, rest),
            [action, folder, id, rest @ ..] if action == "new" => plugin_new(folder, id, rest),
            [action, rest @ ..] if action == "test" => plugin_test(&o, rest),
            _ => usage_error(
                "plugins takes no arguments, `install <file.zip> <folder>`, `remove <id> <folder>`, `settings \
                 <folder> <id> [key=value ...]`, `tools <folder> <id> [tool=path ...] [--test]`, `new <folder> \
                 <id> [name=<name>]` or `test <plugin folder> [--filter <text>] [--tool <plugin>.<tool>=<path>]`",
            ),
        },
        "run" => {
            let Some((name, args)) = rest.split_first() else { return usage_error("run needs a command") };
            let args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return usage_error(&format!("invalid arguments: {e}")),
            };
            let mut s = match session(&o) {
                Ok(s) => s,
                Err(e) => return fail(&e),
            };
            let result = s.execute(name, args);
            flush_messages(&mut s);
            match result.and_then(|out| save(&mut s, &o).map(|()| out)) {
                Ok(out) => {
                    print(&out);
                    ExitCode::SUCCESS
                }
                Err(e) => fail(&e),
            }
        }
        "script" => {
            let Some(file) = rest.first() else { return usage_error("script needs a file (or -)") };
            let steps = match read_input(file).and_then(|t| cc_engine::script::parse(&t).map_err(|e| e.message)) {
                Ok(s) => s,
                Err(e) => return usage_error(&format!("invalid script: {e}")),
            };
            let level = match rest.get(1..).unwrap_or_default() {
                [] => None,
                [arg] => match arg.strip_prefix("level=") {
                    Some(v) => {
                        Some(v.parse::<u64>().map_or_else(|_| cc_engine::Ref::Name(v.into()), cc_engine::Ref::Id))
                    }
                    None => return usage_error(&format!("unknown script argument {arg} (expected level=<id|name>)")),
                },
                _ => return usage_error("script takes a file and at most level=<id|name>"),
            };
            let mut s = match session(&o) {
                Ok(s) => s,
                Err(e) => return fail(&e),
            };
            if let Some(level) = level {
                match s.level(&level) {
                    Ok(id) => s.set_active_level(Some(id)),
                    Err(e) => return fail(&e),
                }
            }
            let results = cc_engine::script::run(&mut s, &steps);
            flush_messages(&mut s);
            let results = match results {
                Ok(results) => results,
                Err((done, failed)) => {
                    print(&json!(
                        done.iter()
                            .zip(&steps)
                            .map(|(r, st)| json!({"command": st.command, "result": r}))
                            .collect::<Vec<_>>()
                    ));
                    eprintln!("step {} ({}) failed:", failed.index + 1, failed.command);
                    return fail(&failed.error);
                }
            };
            let results: Vec<Value> =
                results.into_iter().zip(&steps).map(|(r, st)| json!({"command": st.command, "result": r})).collect();
            if let Err(e) = save(&mut s, &o) {
                print(&json!(results));
                return fail(&e);
            }
            print(&json!(results));
            ExitCode::SUCCESS
        }
        "lua" => {
            let Some((file, args)) = rest.split_first() else { return usage_error("lua needs a file (or -)") };
            let source = match read_input(file) {
                Ok(t) => t,
                Err(e) => return usage_error(&e),
            };
            let args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return usage_error(&format!("invalid arguments: {e}")),
            };
            let mut s = match session(&o) {
                Ok(s) => s,
                Err(e) => return fail(&e),
            };
            let name = if file == "-" { "stdin" } else { file.as_str() };
            let out = cc_plugins::run_script(&mut s, &source, name, args, cc_plugins::ScriptOptions::default());
            flush_messages(&mut s);
            for line in &out.output {
                println!("{line}");
            }
            match out.result.and_then(|v| save(&mut s, &o).map(|()| v)) {
                Ok(Value::Null) => ExitCode::SUCCESS,
                Ok(v) => {
                    print(&v);
                    ExitCode::SUCCESS
                }
                Err(e) => fail(&e),
            }
        }
        "graph" => {
            let Some((file, args)) = rest.split_first() else { return usage_error("graph needs a .ccgraph file") };
            let inputs = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return usage_error(&format!("invalid arguments: {e}")),
            };
            let mut s = match session(&o) {
                Ok(s) => s,
                Err(e) => return fail(&e),
            };
            let result = s.execute(
                "graph_run",
                json!({ "path": file, "inputs": inputs, "brief": true, "allow_files": o.allow_files }),
            );
            flush_messages(&mut s);
            match result.and_then(|out| save(&mut s, &o).map(|()| out)) {
                Ok(out) => {
                    print(&out);
                    if out["errors"].as_u64().unwrap_or(0) > 0 { ExitCode::from(1) } else { ExitCode::SUCCESS }
                }
                Err(e) => fail(&e),
            }
        }
        "mcp" => {
            let s = match session(&o) {
                Ok(s) => s,
                Err(e) => return fail(&e),
            };
            match mcp::serve(s, std::io::stdin().lock(), std::io::stdout().lock()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("ccad mcp: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => usage_error(&format!("unknown command {other}")),
    }
}
