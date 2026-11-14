//! The sun of a view: its settings through `view_set`/`view_get`, where it
//! stands (`sun_position`, `sun_frames`), the project's presets and the
//! site's offset from UTC.

// Test helpers outside #[test] functions may fail loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cc_engine::commands::sun::{sun_arg, sun_json};
use cc_engine::{ErrorCode, Session};
use serde_json::{Value, json};

fn run(s: &mut Session, name: &str, args: Value) -> Value {
    s.execute(name, args).unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

/// The command fails and leaves no undo step.
fn refused(s: &mut Session, name: &str, args: Value) -> ErrorCode {
    let depth = s.doc.undo_depth();
    let code = s.execute(name, args.clone()).map(|v| panic!("{name} {args} should fail, gave {v}")).unwrap_err().code;
    assert_eq!(s.doc.undo_depth(), depth, "{name} {args} left an undo step");
    code
}

fn three_d(s: &mut Session) -> u64 {
    run(s, "view_create", json!({"kind": "three_d"}))["id"].as_u64().unwrap()
}

fn sun_of(s: &mut Session, view: u64) -> Value {
    run(s, "view_get", json!({"view": view}))["sun"].clone()
}

fn set_sun(s: &mut Session, view: u64, sun: Value) -> Value {
    run(s, "view_set", json!({"view": view, "sun": sun}))["sun"].clone()
}

#[test]
fn each_mode_round_trips() {
    let mut s = Session::default();
    let v = three_d(&mut s);
    assert_eq!(sun_of(&mut s, v), Value::Null);

    let still = set_sun(&mut s, v, json!({"mode": "still", "date": "2026-03-20", "time": "09:30"}));
    assert_eq!(still["mode"], "still");
    assert_eq!(still["date"], "2026-03-20");
    assert_eq!(still["time"], "09:30");
    assert!(still.get("from").is_none() && still.get("azimuth").is_none());
    assert_eq!(still["show_path"], true);
    assert_eq!(still["ground"], Value::Null);
    let derived = &still["derived"];
    assert_eq!(derived["date"], "2026-03-20");
    assert_eq!(derived["time"], "09:30");
    assert_eq!(derived["frames"], 1);
    assert_eq!(derived["site_known"], false);
    assert!(derived["sunrise"].is_string() && derived["sunset"].is_string(), "{derived}");
    assert_eq!(derived["above_horizon"], true);

    let single = json!({"mode": "single_day", "date": "2026-12-21", "from": "07:00", "to": "17:00",
        "sunrise_to_sunset": false, "interval": "minutes_30"});
    let got = set_sun(&mut s, v, single.clone());
    for (k, val) in single.as_object().unwrap() {
        assert_eq!(&got[k], val, "{k}");
    }
    assert_eq!(got["derived"]["frames"], 21);
    assert!(got.get("time").is_none());

    let multi = json!({"mode": "multi_day", "from_date": "2026-01-01", "to_date": "2026-12-31", "from": "12:00",
        "to": "12:00", "sunrise_to_sunset": false, "interval": "month"});
    let got = set_sun(&mut s, v, multi.clone());
    for (k, val) in multi.as_object().unwrap() {
        assert_eq!(&got[k], val, "{k}");
    }
    assert_eq!(got["derived"]["frames"], 12);

    let light = set_sun(
        &mut s,
        v,
        json!({"mode": "lighting", "azimuth": -45.0, "altitude": 30.0,
        "relative_to_view": false}),
    );
    assert_eq!(light["azimuth"], 315.0);
    assert_eq!(light["altitude"], 30.0);
    assert_eq!(light["relative_to_view"], false);
    assert_eq!(light["derived"]["azimuth"], 315.0);
    assert_eq!(light["derived"]["frames"], 0);
    assert!(light["derived"].get("time").is_none());
    assert_eq!(light["derived"]["sunrise"], Value::Null);
    assert!(light.get("date").is_none());

    // Null removes the sun.
    assert_eq!(set_sun(&mut s, v, Value::Null), Value::Null);
}

#[test]
fn merge_keeps_untouched_fields_and_mode_switch_carries_the_date() {
    let mut s = Session::default();
    let v = three_d(&mut s);
    set_sun(&mut s, v, json!({"date": "2026-09-01", "time": "15:45", "shadows": true, "frame": 3}));
    let got = set_sun(&mut s, v, json!({"time": "08:00"}));
    assert_eq!(got["mode"], "still");
    assert_eq!(got["date"], "2026-09-01");
    assert_eq!(got["time"], "08:00");
    assert_eq!(got["shadows"], true);
    assert_eq!(got["frame"], 3);
    // Changing the name or scale leaves the sun alone.
    run(&mut s, "view_set", json!({"view": v, "name": "Sun view"}));
    assert_eq!(sun_of(&mut s, v)["time"], "08:00");

    // Still -> single day keeps the date and starts from the mode's defaults.
    let got = set_sun(&mut s, v, json!({"mode": "single_day"}));
    assert_eq!(got["date"], "2026-09-01");
    assert_eq!(got["from"], "06:00");
    assert_eq!(got["interval"], "hour");
    assert_eq!(got["shadows"], true);
    // -> several days: the date is the first one.
    let got = set_sun(&mut s, v, json!({"mode": "multi_day"}));
    assert_eq!(got["from_date"], "2026-09-01");
    assert_eq!(got["to_date"], "2026-12-31");
    // -> still: the first date again.
    let got = set_sun(&mut s, v, json!({"mode": "still"}));
    assert_eq!(got["date"], "2026-09-01");
    assert_eq!(got["time"], "10:00");
    // The same mode keeps its values.
    let got = set_sun(&mut s, v, json!({"mode": "still", "time": "11:11"}));
    assert_eq!(got["date"], "2026-09-01");

    // Undo restores the earlier sun.
    run(&mut s, "undo", json!({}));
    assert_eq!(sun_of(&mut s, v)["time"], "10:00");
}

#[test]
fn bad_input_is_refused_without_an_undo_step() {
    let mut s = Session::default();
    let v = three_d(&mut s);
    set_sun(&mut s, v, json!({"mode": "single_day", "from": "08:00", "to": "16:00"}));
    let bad = [
        json!({"mode": "dusk"}),
        json!({"interval": "fortnight"}),
        json!({"preset": "Nope"}),
        json!({"ground": "No such level"}),
        json!({"date": "2026-02-30"}),
        json!({"date": "21/06/2026"}),
        json!({"from": "8h"}),
        json!({"from": "24:00"}),
        json!({"to": "12:60"}),
        json!({"time": "10:00"}),
        json!({"azimuth": 10.0}),
        json!({"from_date": "2026-01-01"}),
        json!({"interval": "day"}),
        json!({"interval": "month"}),
        json!({"from": "17:00"}),
        json!({"mode": "multi_day", "from_date": "2026-05-01", "to_date": "2026-04-01"}),
        json!({"mode": "lighting", "altitude": 95.0}),
        json!({"mode": "lighting", "altitude": -1.0}),
        json!({"mode": "lighting", "date": "2026-01-01"}),
        json!({"unknown_field": 1}),
    ];
    for sun in bad {
        refused(&mut s, "view_set", json!({"view": v, "sun": sun}));
    }
    // Non-finite numbers cannot come through JSON; the arguments are refused all the same.
    let sun = sun_of(&mut s, v);
    assert_eq!(sun["from"], "08:00");
    assert_eq!(sun["to"], "16:00");

    // A schedule takes no sun.
    let schedule = run(&mut s, "view_create", json!({"kind": "schedule"}))["id"].as_u64().unwrap();
    refused(&mut s, "view_set", json!({"view": schedule, "sun": {"mode": "still"}}));
    refused(&mut s, "view_set", json!({"view": schedule, "sun": null}));
}

#[test]
fn presets() {
    let mut s = Session::default();
    let v = three_d(&mut s);
    assert_eq!(run(&mut s, "sun_presets_list", json!({})), json!([]));
    set_sun(&mut s, v, json!({"mode": "single_day", "date": "2026-12-21", "interval": "minutes_15", "shadows": true}));
    let saved = run(&mut s, "sun_preset_save", json!({"name": " Summer ", "view": v}));
    assert_eq!(saved["name"], "Summer");
    assert_eq!(saved["mode"], "single_day");
    assert_eq!(saved["interval"], "minutes_15");
    assert!(saved.get("shadows").is_none());
    run(&mut s, "sun_preset_save", json!({"name": "Low", "sun": {"mode": "lighting", "altitude": 10.0}}));
    let list = run(&mut s, "sun_presets_list", json!({}));
    assert_eq!(list.as_array().unwrap().len(), 2);
    assert_eq!(list[1]["altitude"], 10.0);

    // Names are unique; replacing must be asked for.
    refused(&mut s, "sun_preset_save", json!({"name": "Summer", "sun": {"mode": "still"}}));
    refused(&mut s, "sun_preset_save", json!({"name": "  ", "sun": {"mode": "still"}}));
    refused(&mut s, "sun_preset_save", json!({"name": "X"}));
    refused(&mut s, "sun_preset_save", json!({"name": "X", "view": v, "sun": {}}));
    let other = three_d(&mut s);
    refused(&mut s, "sun_preset_save", json!({"name": "X", "view": other}));
    run(
        &mut s,
        "sun_preset_save",
        json!({"name": "Summer", "sun": {"mode": "still", "time": "12:00"}, "replace": true}),
    );
    assert_eq!(run(&mut s, "sun_presets_list", json!({}))[0]["time"], "12:00");

    // A view takes a preset; other fields apply on top.
    let got = set_sun(&mut s, other, json!({"preset": "low", "azimuth": 200.0}));
    assert_eq!(got["mode"], "lighting");
    assert_eq!(got["altitude"], 10.0);
    assert_eq!(got["azimuth"], 200.0);
    refused(&mut s, "view_set", json!({"view": other, "sun": {"preset": "Low", "mode": "still"}}));

    // Rename and delete.
    refused(&mut s, "sun_preset_rename", json!({"name": "Low", "new_name": "Summer"}));
    refused(&mut s, "sun_preset_rename", json!({"name": "Nope", "new_name": "Y"}));
    let renamed = run(&mut s, "sun_preset_rename", json!({"name": "Low", "new_name": "Evening"}));
    assert_eq!(renamed["name"], "Evening");
    run(&mut s, "sun_preset_delete", json!({"name": "Summer"}));
    refused(&mut s, "sun_preset_delete", json!({"name": "Summer"}));
    let list = run(&mut s, "sun_presets_list", json!({}));
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["name"], "Evening");
    // One undo step each.
    run(&mut s, "undo", json!({}));
    assert_eq!(run(&mut s, "sun_presets_list", json!({})).as_array().unwrap().len(), 2);
}

#[test]
fn preset_limit() {
    let mut s = Session::default();
    for i in 0..cc_core::sun::MAX_PRESETS {
        run(&mut s, "sun_preset_save", json!({"name": format!("P{i}"), "sun": {}}));
    }
    refused(&mut s, "sun_preset_save", json!({"name": "One more", "sun": {}}));
    // Replacing one is still allowed.
    run(&mut s, "sun_preset_save", json!({"name": "P0", "sun": {"mode": "lighting"}, "replace": true}));
}

#[test]
fn positions_and_frames() {
    let mut s = Session::default();
    // The equinox at the equator on Greenwich: solar noon is about 12:07
    // (the equation of time), the sun then nearly overhead.
    let at = |s: &mut Session, time: &str| {
        let args = json!({"date": "2026-03-20", "time": time, "latitude": 0.0, "longitude": 0.0, "utc_offset": 0.0});
        run(s, "sun_position", args)
    };
    assert!(at(&mut s, "12:00")["altitude"].as_f64().unwrap() > 88.0);
    let p = at(&mut s, "12:07");
    assert!(p["altitude"].as_f64().unwrap() > 89.0, "{p}");
    assert_eq!(p["above_horizon"], true);
    assert_eq!(p["site_known"], true);
    assert!(p["direction"][2].as_f64().unwrap() > 0.99, "{p}");
    assert!(p["sunrise"].is_string() && p["sunset"].is_string());
    // A polar night.
    let p =
        run(&mut s, "sun_position", json!({"date": "2026-12-21", "time": "12:00", "latitude": 80.0, "longitude": 0.0}));
    assert_eq!(p["above_horizon"], false);
    assert_eq!(p["polar"], "night");
    assert_eq!(p["sunrise"], Value::Null);
    // Without a site: the equator at Greenwich.
    assert_eq!(run(&mut s, "sun_position", json!({"date": "2026-03-20", "time": "12:00"}))["site_known"], false);
    refused(&mut s, "sun_position", json!({"date": "2026-03-20"}));
    refused(&mut s, "sun_position", json!({"date": "2026-03-20", "time": "12:00", "latitude": 91.0}));
    refused(&mut s, "sun_position", json!({"date": "2026-03-20", "time": "12:00", "utc_offset": 15.0}));

    let v = three_d(&mut s);
    refused(&mut s, "sun_position", json!({"view": v}));
    refused(&mut s, "sun_frames", json!({"view": v}));
    set_sun(
        &mut s,
        v,
        json!({"mode": "single_day", "date": "2026-03-20", "from": "06:00", "to": "18:00",
        "interval": "hour", "frame": 6}),
    );
    refused(&mut s, "sun_position", json!({"view": v, "date": "2026-03-20"}));
    let p = run(&mut s, "sun_position", json!({"view": v}));
    assert_eq!(p["time"], "12:00");
    let frames = run(&mut s, "sun_frames", json!({"view": v}));
    assert_eq!(frames["count"], 13);
    assert_eq!(frames["frames"].as_array().unwrap().len(), 13);
    assert_eq!(frames["frames"][6]["time"], "12:00");
    assert_eq!(frames["frames"][6]["altitude"], p["altitude"]);
    let few = run(&mut s, "sun_frames", json!({"view": v, "limit": 3}));
    assert_eq!(few["count"], 13);
    assert_eq!(few["frames"].as_array().unwrap().len(), 3);
    assert_eq!(few["frames"][2]["index"], 2);

    // Every day of a month, hourly from 8 to 10: three frames a day.
    set_sun(
        &mut s,
        v,
        json!({"mode": "multi_day", "from_date": "2026-01-01", "to_date": "2026-01-31",
        "from": "08:00", "to": "10:00", "interval": "hour"}),
    );
    assert_eq!(run(&mut s, "sun_frames", json!({"view": v}))["count"], 93);
}

#[test]
fn site_utc_offset() {
    let mut s = Session::default();
    refused(&mut s, "site_set", json!({"utc_offset": -3.0}));
    let out = run(&mut s, "site_set", json!({"latitude": -23.5, "longitude": -46.6}));
    assert_eq!(out["site"]["utc_offset"], Value::Null);
    assert_eq!(out["site"]["utc_offset_used"], -3.0);
    let out = run(&mut s, "site_set", json!({"utc_offset": 5.5}));
    assert_eq!(out["site"]["utc_offset"], 5.5);
    assert_eq!(out["site"]["utc_offset_used"], 5.5);
    refused(&mut s, "site_set", json!({"utc_offset": 14.5}));
    refused(&mut s, "site_set", json!({"utc_offset": -12.5}));
    refused(&mut s, "site_set", json!({"clear": true, "utc_offset": null}));
    let out = run(&mut s, "site_set", json!({"utc_offset": null}));
    assert_eq!(out["site"]["utc_offset"], Value::Null);
    assert_eq!(run(&mut s, "site_get", json!({}))["site"]["utc_offset_used"], -3.0);
    // The views' suns follow the site.
    let v = three_d(&mut s);
    assert_eq!(set_sun(&mut s, v, json!({}))["derived"]["site_known"], true);
}

#[test]
fn deleting_the_ground_level_clears_it() {
    let mut s = Session::default();
    let v = three_d(&mut s);
    let level = run(&mut s, "level_create", json!({"name": "Terrace", "elevation": 9.0}))["id"].as_u64().unwrap();
    let got = set_sun(&mut s, v, json!({"shadows": true, "ground": "Terrace"}));
    assert_eq!(got["ground"], level);
    run(&mut s, "level_delete", json!({"level": level}));
    assert_eq!(sun_of(&mut s, v)["ground"], Value::Null);
    run(&mut s, "undo", json!({}));
    assert_eq!(sun_of(&mut s, v)["ground"], level);
    assert_eq!(set_sun(&mut s, v, json!({"ground": null}))["ground"], Value::Null);
}

#[test]
fn sun_arg_inverts_the_settings() {
    let mut s = Session::default();
    let a = three_d(&mut s);
    let b = three_d(&mut s);
    let level = run(&mut s, "level_create", json!({"name": "Roof", "elevation": 6.0}))["id"].as_u64().unwrap();
    let suns = [
        json!({"mode": "still", "date": "2027-02-03", "time": "16:20", "frame": 2, "show_path": false, "ground": level}),
        json!({"mode": "single_day", "date": "2026-07-04", "from": "05:15", "to": "19:45", "sunrise_to_sunset": true,
            "interval": "minutes_45", "shadows": true}),
        json!({"mode": "multi_day", "from_date": "2026-03-01", "to_date": "2027-03-01", "from": "09:00",
            "to": "15:00", "interval": "week", "frame": 40}),
        json!({"mode": "lighting", "azimuth": 359.5, "altitude": 0.0, "relative_to_view": false}),
    ];
    // Each mode, applied over every other one, comes back the same.
    for sun in &suns {
        set_sun(&mut s, a, sun.clone());
        let settings = s.project().views[&cc_core::ViewId(a)].sun.clone().unwrap();
        let arg = sun_arg(s.project(), &settings);
        for start in &suns {
            set_sun(&mut s, b, start.clone());
            run(&mut s, "view_set", json!({"view": b, "sun": serde_json::to_value(&arg).unwrap()}));
            let back = s.project().views[&cc_core::ViewId(b)].sun.clone().unwrap();
            assert_eq!(back, settings, "{sun} over {start}");
        }
        // The friendly form is what view_get shows, without `derived`.
        let mut shown = sun_of(&mut s, a);
        shown.as_object_mut().unwrap().remove("derived");
        assert_eq!(shown, sun_json(s.project(), &settings));
    }
    // From no sun at all.
    run(&mut s, "view_set", json!({"view": b, "sun": null}));
    let settings = s.project().views[&cc_core::ViewId(a)].sun.clone().unwrap();
    let arg = sun_arg(s.project(), &settings);
    run(&mut s, "view_set", json!({"view": b, "sun": serde_json::to_value(&arg).unwrap()}));
    assert_eq!(s.project().views[&cc_core::ViewId(b)].sun.clone().unwrap(), settings);
}
