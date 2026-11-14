//! The sun of a view (see `cc_core::sun`): its settings as commands show
//! and take them (`view_get`, `view_set`), where it stands, the frames of a
//! study, and the project's named presets.

use super::edit;
use crate::error::{EngineError, Result};
use crate::refs::{self, Ref};
use crate::registry::{Command, CommandKind};
use crate::session::Session;
use crate::t;
use cc_core::sun::{
    Daylight, MAX_FRAMES, MAX_PRESETS, SunDate, SunInterval, SunMode, SunPreset, SunSettings, SunSite,
    UTC_OFFSET_RANGE, clean_preset_name, sun_direction, view_forward,
};
use cc_core::{LevelId, Project, View};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

/// Frames `sun_frames` lists when no `limit` is given.
const DEFAULT_FRAME_LIMIT: usize = 500;

// ---------------------------------------------------------------- the friendly form

/// A view's sun as commands take it (`view_set` `sun`, `sun_preset_save`):
/// every field is optional and changes only what it names. The fields a
/// mode has: `still` `date` and `time`; `single_day` `date`, `from`, `to`,
/// `sunrise_to_sunset` and `interval`; `multi_day` `from_date`, `to_date`,
/// `from`, `to`, `sunrise_to_sunset` and `interval`; `lighting` `azimuth`,
/// `altitude` and `relative_to_view`. `frame`, `show_path`, `shadows` and
/// `ground` belong to every mode. Dates and times are local to the site.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SunArg {
    /// Take the mode and its values from this project preset (by name); the other fields given apply on top.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    /// `still` (one moment), `single_day` (a study over one day), `multi_day`
    /// (a study over several days) or `lighting` (a light from a direction
    /// given by hand). Another mode starts from its defaults, keeping the date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Still and single day: the date, `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Still: the time of day, `HH:MM`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    /// Multi day: the first date, `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_date: Option<String>,
    /// Multi day: the last date, `YYYY-MM-DD` (not before `from_date`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_date: Option<String>,
    /// Single and multi day: the first time of each day, `HH:MM` (with a
    /// day, week or month step, the time of every frame).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Single and multi day: the last time of each day, `HH:MM` (not before `from`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    /// Single and multi day: from sunrise to sunset instead of `from` to `to`
    /// (12:00 with a day, week or month step).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sunrise_to_sunset: Option<bool>,
    /// Single and multi day: the step between frames: `minutes_15`,
    /// `minutes_30`, `minutes_45` or `hour`; several days also take `day`,
    /// `week` or `month`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    /// Lighting: degrees clockwise seen from above, from true north or, with
    /// `relative_to_view`, from the direction the view looks along (any
    /// number, kept within 0 to 360).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub azimuth: Option<f64>,
    /// Lighting: degrees above the horizon, 0 to 90.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub altitude: Option<f64>,
    /// Lighting: measure the azimuth from the direction the view looks along (default true).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_to_view: Option<bool>,
    /// The frame of a study shown, from 0 (kept within the frames when read).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<u32>,
    /// Draw the sun path diagram.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_path: Option<bool>,
    /// Cast shadows (3D views).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadows: Option<bool>,
    /// The level whose plane receives the shadows (id or name), or `null` for no ground plane.
    #[serde(default, deserialize_with = "refs::nullable", skip_serializing_if = "Option::is_none")]
    pub ground: Option<Option<Ref>>,
}

/// `HH:MM` of a whole minute of the day.
fn time_text(minutes: u16) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// `HH:MM` of a time in minutes, rounded and brought within the day.
fn clock(minutes: f64) -> String {
    let m = if minutes.is_finite() { minutes.round().rem_euclid(1440.0) } else { 0.0 };
    time_text((m as u16).min(1439))
}

/// Reads `HH:MM` (or `H:MM`), 00:00 to 23:59.
fn parse_time(text: &str) -> Result<u16> {
    let bad = || EngineError::invalid(t!("msg-sun-time", "text" => text));
    let (h, m) = text.trim().split_once(':').ok_or_else(bad)?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(h) || h.len() > 2 || !digits(m) || m.len() != 2 {
        return Err(bad());
    }
    let (h, m): (u16, u16) = (h.parse().map_err(|_| bad())?, m.parse().map_err(|_| bad())?);
    if h >= 24 || m >= 60 {
        return Err(bad());
    }
    Ok(h * 60 + m)
}

fn parse_date(text: &str) -> Result<SunDate> {
    SunDate::parse(text).ok_or_else(|| EngineError::invalid(t!("msg-sun-date", "text" => text)))
}

fn parse_interval(text: &str) -> Result<SunInterval> {
    SunInterval::from_name(text).ok_or_else(|| EngineError::invalid(t!("msg-sun-interval-unknown", "name" => text)))
}

/// An azimuth in `[0, 360)`.
fn normalize_azimuth(a: f64) -> f64 {
    let r = a.rem_euclid(360.0) + 0.0;
    if r >= 360.0 { 0.0 } else { r }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// The date a mode places its sun on (a multi-day study's first one).
fn date_of(mode: &SunMode) -> Option<SunDate> {
    match mode {
        SunMode::Still { date, .. } | SunMode::SingleDay { date, .. } => Some(*date),
        SunMode::MultiDay { from_date, .. } => Some(*from_date),
        SunMode::Lighting { .. } => None,
    }
}

/// Put `d` as a mode's date (a multi-day study's first one, its last date
/// moved to the end of that year when it would come before).
fn set_date_of(mode: &mut SunMode, d: SunDate) {
    match mode {
        SunMode::Still { date, .. } | SunMode::SingleDay { date, .. } => *date = d,
        SunMode::MultiDay { from_date, to_date, .. } => {
            *from_date = d;
            if *to_date < d {
                *to_date = SunDate::new(d.year, 12, 31).unwrap_or(d);
            }
        }
        SunMode::Lighting { .. } => {}
    }
}

/// The fields of the friendly form a mode has (besides the common ones).
fn fields_of(mode: &SunMode) -> &'static [&'static str] {
    match mode {
        SunMode::Still { .. } => &["date", "time"],
        SunMode::SingleDay { .. } => &["date", "from", "to", "sunrise_to_sunset", "interval"],
        SunMode::MultiDay { .. } => &["from_date", "to_date", "from", "to", "sunrise_to_sunset", "interval"],
        SunMode::Lighting { .. } => &["azimuth", "altitude", "relative_to_view"],
    }
}

/// The project preset called `name` (exactly, else ignoring case when only one matches).
fn find_preset<'a>(p: &'a Project, name: &str) -> Result<&'a SunPreset> {
    if let Some(preset) = p.sun_preset(name.trim()) {
        return Ok(preset);
    }
    let mut found = p.sun_presets.iter().filter(|x| x.name.eq_ignore_ascii_case(name.trim()));
    match (found.next(), found.next()) {
        (Some(one), None) => Ok(one),
        _ => Err(EngineError::not_found(t!("msg-sun-preset-unknown", "name" => name))),
    }
}

/// `current` (or the default sun) changed by `arg`, checked: unknown names,
/// unreadable dates and times, fields the mode does not have and values out
/// of range are refused, never repaired.
pub(crate) fn apply_sun(p: &Project, current: Option<&SunSettings>, arg: &SunArg) -> Result<SunSettings> {
    let mut sun = current.cloned().unwrap_or_default();
    if let Some(name) = &arg.preset {
        if arg.mode.is_some() {
            return Err(EngineError::invalid(t!("msg-sun-preset-mode")));
        }
        sun.mode = find_preset(p, name)?.mode.clone();
    }
    if let Some(name) = &arg.mode {
        let fresh =
            SunMode::from_name(name).ok_or_else(|| EngineError::invalid(t!("msg-sun-mode-unknown", "name" => name)))?;
        if fresh.name() != sun.mode.name() {
            let carried = date_of(&sun.mode);
            sun.mode = fresh;
            if let Some(d) = carried {
                set_date_of(&mut sun.mode, d);
            }
        }
    }
    let given = [
        ("date", arg.date.is_some()),
        ("time", arg.time.is_some()),
        ("from_date", arg.from_date.is_some()),
        ("to_date", arg.to_date.is_some()),
        ("from", arg.from.is_some()),
        ("to", arg.to.is_some()),
        ("sunrise_to_sunset", arg.sunrise_to_sunset.is_some()),
        ("interval", arg.interval.is_some()),
        ("azimuth", arg.azimuth.is_some()),
        ("altitude", arg.altitude.is_some()),
        ("relative_to_view", arg.relative_to_view.is_some()),
    ];
    let allowed = fields_of(&sun.mode);
    if let Some((field, _)) = given.iter().find(|(f, g)| *g && !allowed.contains(f)) {
        return Err(EngineError::invalid(t!("msg-sun-field", "field" => field, "mode" => sun.mode.name())));
    }
    let date = arg.date.as_deref().map(parse_date).transpose()?;
    let from_date = arg.from_date.as_deref().map(parse_date).transpose()?;
    let to_date = arg.to_date.as_deref().map(parse_date).transpose()?;
    let time = arg.time.as_deref().map(parse_time).transpose()?;
    let from = arg.from.as_deref().map(parse_time).transpose()?;
    let to = arg.to.as_deref().map(parse_time).transpose()?;
    let interval = arg.interval.as_deref().map(parse_interval).transpose()?;
    let time_order = |a: u16, b: u16| -> Result<()> {
        match a > b {
            true => Err(EngineError::invalid(t!("msg-sun-time-order", "from" => time_text(a), "to" => time_text(b)))),
            false => Ok(()),
        }
    };
    match &mut sun.mode {
        SunMode::Still { date: d, time: tm } => {
            if let Some(x) = date {
                *d = x;
            }
            if let Some(x) = time {
                *tm = x;
            }
        }
        SunMode::SingleDay { date: d, from: f, to: tt, sunrise_to_sunset, interval: i } => {
            if let Some(x) = date {
                *d = x;
            }
            if let Some(x) = from {
                *f = x;
            }
            if let Some(x) = to {
                *tt = x;
            }
            if let Some(x) = arg.sunrise_to_sunset {
                *sunrise_to_sunset = x;
            }
            if let Some(x) = interval {
                if !x.is_sub_day() {
                    return Err(EngineError::invalid(t!("msg-sun-interval-day")));
                }
                *i = x;
            }
            time_order(*f, *tt)?;
        }
        SunMode::MultiDay { from_date: fd, to_date: td, from: f, to: tt, sunrise_to_sunset, interval: i } => {
            if let Some(x) = from_date {
                *fd = x;
            }
            if let Some(x) = to_date {
                *td = x;
            }
            if let Some(x) = from {
                *f = x;
            }
            if let Some(x) = to {
                *tt = x;
            }
            if let Some(x) = arg.sunrise_to_sunset {
                *sunrise_to_sunset = x;
            }
            if let Some(x) = interval {
                *i = x;
            }
            if *fd > *td {
                return Err(EngineError::invalid(t!("msg-sun-date-order", "from" => fd.iso(), "to" => td.iso())));
            }
            time_order(*f, *tt)?;
        }
        SunMode::Lighting { azimuth, altitude, relative_to_view } => {
            if let Some(a) = arg.azimuth {
                if !a.is_finite() {
                    return Err(EngineError::invalid(t!("msg-sun-azimuth")));
                }
                *azimuth = normalize_azimuth(a);
            }
            if let Some(a) = arg.altitude {
                if !(a.is_finite() && (0.0..=90.0).contains(&a)) {
                    return Err(EngineError::invalid(t!("msg-sun-altitude")));
                }
                *altitude = a;
            }
            if let Some(x) = arg.relative_to_view {
                *relative_to_view = x;
            }
        }
    }
    if let Some(f) = arg.frame {
        sun.frame = f;
    }
    if let Some(x) = arg.show_path {
        sun.show_path = x;
    }
    if let Some(x) = arg.shadows {
        sun.shadows = x;
    }
    match &arg.ground {
        Some(Some(r)) => sun.ground = Some(refs::level(p, r)?),
        Some(None) => sun.ground = None,
        None => {}
    }
    if !sun.is_valid() {
        return Err(EngineError::invalid(t!("msg-sun-invalid")));
    }
    Ok(sun)
}

/// A mode in the friendly form: `mode` and the fields it has.
pub fn mode_json(mode: &SunMode) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("mode".into(), json!(mode.name()));
    let mut put = |k: &str, v: Value| {
        m.insert(k.into(), v);
    };
    match mode {
        SunMode::Still { date, time } => {
            put("date", json!(date.iso()));
            put("time", json!(time_text(*time)));
        }
        SunMode::SingleDay { date, from, to, sunrise_to_sunset, interval } => {
            put("date", json!(date.iso()));
            put("from", json!(time_text(*from)));
            put("to", json!(time_text(*to)));
            put("sunrise_to_sunset", json!(sunrise_to_sunset));
            put("interval", json!(interval.name()));
        }
        SunMode::MultiDay { from_date, to_date, from, to, sunrise_to_sunset, interval } => {
            put("from_date", json!(from_date.iso()));
            put("to_date", json!(to_date.iso()));
            put("from", json!(time_text(*from)));
            put("to", json!(time_text(*to)));
            put("sunrise_to_sunset", json!(sunrise_to_sunset));
            put("interval", json!(interval.name()));
        }
        SunMode::Lighting { azimuth, altitude, relative_to_view } => {
            put("azimuth", json!(azimuth));
            put("altitude", json!(altitude));
            put("relative_to_view", json!(relative_to_view));
        }
    }
    m
}

/// The ground level, when the project has it.
fn ground_of(p: &Project, sun: &SunSettings) -> Option<LevelId> {
    sun.ground.filter(|l| p.level(*l).is_some())
}

/// A view's sun in the friendly form (what `view_get` shows and `view_set` takes).
pub fn sun_json(project: &Project, sun: &SunSettings) -> Value {
    let mut m = mode_json(&sun.mode);
    m.insert("frame".into(), json!(sun.frame));
    m.insert("show_path".into(), json!(sun.show_path));
    m.insert("shadows".into(), json!(sun.shadows));
    m.insert("ground".into(), json!(ground_of(project, sun).map(|l| l.0)));
    Value::Object(m)
}

/// The argument that makes `sun` from any settings: every field of its
/// mode and the common ones (the inverse of applying a [`SunArg`]).
pub fn sun_arg(project: &Project, sun: &SunSettings) -> SunArg {
    let mut arg = SunArg {
        mode: Some(sun.mode.name().to_string()),
        frame: Some(sun.frame),
        show_path: Some(sun.show_path),
        shadows: Some(sun.shadows),
        ground: Some(ground_of(project, sun).map(|l| Ref::Id(l.0))),
        ..Default::default()
    };
    match &sun.mode {
        SunMode::Still { date, time } => {
            arg.date = Some(date.iso());
            arg.time = Some(time_text(*time));
        }
        SunMode::SingleDay { date, from, to, sunrise_to_sunset, interval } => {
            arg.date = Some(date.iso());
            arg.from = Some(time_text(*from));
            arg.to = Some(time_text(*to));
            arg.sunrise_to_sunset = Some(*sunrise_to_sunset);
            arg.interval = Some(interval.name().to_string());
        }
        SunMode::MultiDay { from_date, to_date, from, to, sunrise_to_sunset, interval } => {
            arg.from_date = Some(from_date.iso());
            arg.to_date = Some(to_date.iso());
            arg.from = Some(time_text(*from));
            arg.to = Some(time_text(*to));
            arg.sunrise_to_sunset = Some(*sunrise_to_sunset);
            arg.interval = Some(interval.name().to_string());
        }
        SunMode::Lighting { azimuth, altitude, relative_to_view } => {
            arg.azimuth = Some(*azimuth);
            arg.altitude = Some(*altitude);
            arg.relative_to_view = Some(*relative_to_view);
        }
    }
    arg
}

/// Sunrise and sunset on `date` at `site` into `out` (`polar` when the sun
/// does not rise or set).
fn daylight_into(out: &mut Map<String, Value>, site: &SunSite, date: Option<SunDate>) {
    let (sunrise, sunset, polar) = match date.map(|d| site.daylight(d)) {
        Some(Daylight::Normal { sunrise, sunset }) => (json!(clock(sunrise)), json!(clock(sunset)), None),
        Some(Daylight::PolarDay) => (Value::Null, Value::Null, Some("day")),
        Some(Daylight::PolarNight) => (Value::Null, Value::Null, Some("night")),
        None => (Value::Null, Value::Null, None),
    };
    out.insert("sunrise".into(), sunrise);
    out.insert("sunset".into(), sunset);
    if let Some(polar) = polar {
        out.insert("polar".into(), json!(polar));
    }
}

/// A view's sun as `view_get` shows it: `null`, or the friendly form plus
/// `derived` (where it stands for the current frame).
pub(crate) fn view_sun_json(p: &Project, v: &View) -> Value {
    let Some(sun) = &v.sun else { return Value::Null };
    let site = p.sun_site();
    let state = sun.state(&site, view_forward(v));
    let mut d = Map::new();
    d.insert("azimuth".into(), json!(round2(state.azimuth)));
    d.insert("altitude".into(), json!(round2(state.altitude)));
    d.insert("above_horizon".into(), json!(state.above_horizon));
    if let Some(f) = state.frame {
        d.insert("date".into(), json!(f.date.iso()));
        d.insert("time".into(), json!(clock(f.minutes)));
    }
    daylight_into(&mut d, &site, state.frame.map(|f| f.date));
    d.insert("frames".into(), json!(sun.frames(&site).len()));
    d.insert("site_known".into(), json!(site.known));
    let mut out = sun_json(p, sun);
    if let Some(o) = out.as_object_mut() {
        o.insert("derived".into(), Value::Object(d));
    }
    out
}

/// The view's sun settings, or an error when it has none.
fn view_sun<'a>(p: &'a Project, r: &Ref) -> Result<(&'a View, &'a SunSettings)> {
    let id = refs::view(p, r)?;
    let v = p.view(id).ok_or_else(|| EngineError::not_found("view"))?;
    let sun = v.sun.as_ref().ok_or_else(|| EngineError::rejected(t!("msg-sun-none")))?;
    Ok((v, sun))
}

// ---------------------------------------------------------------- queries

/// Where the sun stands: for a view with a sun, at its current frame (or
/// its lighting direction); else on `date` at `time` (local) at the
/// project's site, or at `latitude`, `longitude` and `utc_offset` given
/// here (a longitude given without an offset takes its nominal time zone).
/// Returns the azimuth (degrees clockwise from true north), the altitude
/// (degrees above the horizon, no refraction), `above_horizon`, the unit
/// `direction` towards the sun in project coordinates (true north
/// applied), sunrise and sunset that day (`HH:MM`, or null with `polar`
/// `day` or `night`), and `site_known` (false: the equator at Greenwich).
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SunPosition {
    /// A view with a sun (id or name); or give `date` and `time`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<Ref>,
    /// The date, `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// The local time of day, `HH:MM`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    /// Latitude, degrees (-90 to 90, north positive), instead of the site's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    /// Longitude, degrees (-180 to 180, east positive), instead of the site's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    /// Local time's offset from UTC, hours (-12 to 14), instead of the site's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub utc_offset: Option<f64>,
}

/// An offset from UTC within range.
pub(crate) fn utc_offset(v: f64) -> Result<f64> {
    match v.is_finite() && UTC_OFFSET_RANGE.contains(&v) {
        true => Ok(v),
        false => Err(EngineError::invalid(t!("msg-sun-utc-offset"))),
    }
}

fn direction_json(d: glam::DVec3) -> Value {
    let r = |v: f64| (v * 1e6).round() / 1e6;
    json!([r(d.x), r(d.y), r(d.z)])
}

impl Command for SunPosition {
    const NAME: &'static str = "sun_position";
    const KIND: CommandKind = CommandKind::Query;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let mut out = Map::new();
        match &self.view {
            Some(r) => {
                let by_date = self.date.is_some() || self.time.is_some();
                let site_given = self.latitude.is_some() || self.longitude.is_some() || self.utc_offset.is_some();
                if by_date || site_given {
                    return Err(EngineError::invalid(t!("msg-sun-position-args")));
                }
                let (v, sun) = view_sun(p, r)?;
                let site = p.sun_site();
                let state = sun.state(&site, view_forward(v));
                out.insert("azimuth".into(), json!(round2(state.azimuth)));
                out.insert("altitude".into(), json!(round2(state.altitude)));
                out.insert("above_horizon".into(), json!(state.above_horizon));
                out.insert("direction".into(), direction_json(state.direction));
                if let Some(f) = state.frame {
                    out.insert("date".into(), json!(f.date.iso()));
                    out.insert("time".into(), json!(clock(f.minutes)));
                }
                daylight_into(&mut out, &site, state.frame.map(|f| f.date));
                out.insert("site_known".into(), json!(site.known));
            }
            None => {
                let (Some(date), Some(time)) = (&self.date, &self.time) else {
                    return Err(EngineError::invalid(t!("msg-sun-position-args")));
                };
                let (date, minutes) = (parse_date(date)?, parse_time(time)?);
                let mut site = p.sun_site();
                if let Some(lat) = self.latitude {
                    if !(lat.is_finite() && (-90.0..=90.0).contains(&lat)) {
                        return Err(EngineError::invalid("latitude must be between -90 and 90 degrees"));
                    }
                    site.latitude = lat;
                }
                if let Some(lon) = self.longitude {
                    if !(lon.is_finite() && (-180.0..=180.0).contains(&lon)) {
                        return Err(EngineError::invalid("longitude must be between -180 and 180 degrees"));
                    }
                    site.longitude = lon;
                    site.utc_offset = (lon / 15.0).round().clamp(*UTC_OFFSET_RANGE.start(), *UTC_OFFSET_RANGE.end());
                }
                if let Some(o) = self.utc_offset {
                    site.utc_offset = utc_offset(o)?;
                }
                let known = site.known || (self.latitude.is_some() && self.longitude.is_some());
                let pos = site.position(date, f64::from(minutes));
                out.insert("azimuth".into(), json!(round2(pos.azimuth)));
                out.insert("altitude".into(), json!(round2(pos.altitude)));
                out.insert("above_horizon".into(), json!(pos.altitude > 0.0));
                out.insert(
                    "direction".into(),
                    direction_json(sun_direction(site.north_dir(), pos.azimuth, pos.altitude)),
                );
                out.insert("date".into(), json!(date.iso()));
                out.insert("time".into(), json!(time_text(minutes)));
                daylight_into(&mut out, &site, Some(date));
                out.insert("site_known".into(), json!(known));
            }
        }
        Ok(Value::Object(out))
    }
}

/// The frames of a view's sun study, in time order: `count` (all of them,
/// at most 5000; 0 for lighting, 1 for a still sun) and the first `limit`
/// as `{index, date, time, azimuth, altitude}`.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SunFrames {
    /// A view with a sun (id or name).
    pub view: Ref,
    /// Most frames listed (default 500, at most 5000).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

impl Command for SunFrames {
    const NAME: &'static str = "sun_frames";
    const KIND: CommandKind = CommandKind::Query;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let (_, sun) = view_sun(p, &self.view)?;
        let site = p.sun_site();
        let frames = sun.frames(&site);
        let limit = self.limit.unwrap_or(DEFAULT_FRAME_LIMIT).min(MAX_FRAMES);
        let listed: Vec<Value> = frames
            .iter()
            .take(limit)
            .enumerate()
            .map(|(i, f)| {
                let pos = site.position(f.date, f.minutes);
                json!({
                    "index": i,
                    "date": f.date.iso(),
                    "time": clock(f.minutes),
                    "azimuth": round2(pos.azimuth),
                    "altitude": round2(pos.altitude),
                })
            })
            .collect();
        Ok(json!({ "count": frames.len(), "frames": listed }))
    }
}

// ---------------------------------------------------------------- presets

fn preset_json(preset: &SunPreset) -> Value {
    let mut m = Map::new();
    m.insert("name".into(), json!(preset.name));
    m.extend(mode_json(&preset.mode));
    Value::Object(m)
}

/// The project's sun presets: each `name` with its mode in the form
/// `view_set` `sun` takes (a view takes one with `"sun": {"preset": name}`).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SunPresetsList {}

impl Command for SunPresetsList {
    const NAME: &'static str = "sun_presets_list";
    const KIND: CommandKind = CommandKind::Query;
    fn run(self, s: &mut Session) -> Result<Value> {
        Ok(Value::Array(s.project().sun_presets.iter().map(preset_json).collect()))
    }
}

/// Save a sun preset, in one undo step: the mode and its values (dates,
/// times, step or light direction; not the frame, path, shadows or
/// ground) of a view's sun, or of `sun` applied to the default sun. A
/// preset of the same name is replaced only with `replace`; a project
/// keeps at most 200.
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SunPresetSave {
    /// Name of the preset.
    pub name: String,
    /// A view whose sun is saved (id or name); or give `sun`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<Ref>,
    /// The sun to save, as `view_set` takes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sun: Option<SunArg>,
    /// Replace a preset of the same name (default false: refused).
    #[serde(default)]
    pub replace: bool,
}

impl Command for SunPresetSave {
    const NAME: &'static str = "sun_preset_save";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let mode = match (&self.view, &self.sun) {
            (Some(r), None) => view_sun(p, r)?.1.mode.clone(),
            (None, Some(arg)) => apply_sun(p, None, arg)?.mode,
            _ => return Err(EngineError::invalid(t!("msg-sun-preset-source"))),
        };
        let name = clean_preset_name(&self.name).ok_or_else(|| EngineError::invalid(t!("msg-sun-preset-name")))?;
        let existing = p.sun_presets.iter().position(|x| x.name == name);
        match existing {
            Some(_) if !self.replace => {
                return Err(EngineError::rejected(t!("msg-sun-preset-exists", "name" => name)));
            }
            None if p.sun_presets.len() >= MAX_PRESETS => {
                return Err(EngineError::rejected(t!("msg-sun-preset-limit", "max" => MAX_PRESETS)));
            }
            _ => {}
        }
        let preset = SunPreset { name, mode };
        let out = preset_json(&preset);
        edit(s, t!("tx-sun-preset-save"), |e| {
            let presets = e.sun_presets_mut();
            match existing.and_then(|i| presets.get_mut(i)) {
                Some(slot) => *slot = preset,
                None => presets.push(preset),
            }
            Ok(())
        })?;
        Ok(out)
    }
}

/// Rename a sun preset, in one undo step.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SunPresetRename {
    /// The preset's name.
    pub name: String,
    /// Its new name (no other preset's).
    pub new_name: String,
}

impl Command for SunPresetRename {
    const NAME: &'static str = "sun_preset_rename";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let old = find_preset(p, &self.name)?.name.clone();
        let new = clean_preset_name(&self.new_name).ok_or_else(|| EngineError::invalid(t!("msg-sun-preset-name")))?;
        if new != old && p.sun_preset(&new).is_some() {
            return Err(EngineError::rejected(t!("msg-sun-preset-exists", "name" => new)));
        }
        let out = edit(s, t!("tx-sun-preset-rename"), |e| {
            let preset = e
                .sun_presets_mut()
                .iter_mut()
                .find(|x| x.name == old)
                .ok_or_else(|| EngineError::not_found(t!("msg-sun-preset-unknown", "name" => old)))?;
            preset.name = new;
            Ok(preset_json(preset))
        })?;
        Ok(out)
    }
}

/// Delete a sun preset, in one undo step (views that took it keep their sun).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SunPresetDelete {
    /// The preset's name.
    pub name: String,
}

impl Command for SunPresetDelete {
    const NAME: &'static str = "sun_preset_delete";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let name = find_preset(s.project(), &self.name)?.name.clone();
        edit(s, t!("tx-sun-preset-delete"), |e| {
            e.sun_presets_mut().retain(|x| x.name != name);
            Ok(())
        })?;
        Ok(json!({ "deleted": name }))
    }
}
