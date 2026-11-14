//! The sun of a view: where it stands in the sky for a date, a time and the
//! project's site, and the settings each view keeps to place it.
//!
//! A view that takes a sun ([`ViewKind::takes_sun`]: plans, ceiling plans,
//! 3D views, elevations and sections) may keep [`SunSettings`] in
//! [`View::sun`]: a [`SunMode`] that is either a still moment, a study
//! over one day or several days (a list of [`SunFrame`]s, the current one
//! chosen by [`SunSettings::frame`]), or a lighting direction given by hand.
//! [`SunSettings::state`] (or [`Project::sun_state`] for a view) resolves
//! them into a [`SunState`]: the sun's azimuth and altitude and the unit
//! vector towards it in project coordinates, turned by the site's true
//! north ([`SunSite`], from [`crate::site::SiteLocation`]).
//!
//! The solar position uses the usual public low-precision formulas: the
//! sun's mean longitude and anomaly in Julian centuries from J2000 give its
//! apparent longitude, the declination and the equation of time (good to
//! about a hundredth of a degree and a few seconds for centuries around
//! 2000), the true solar time gives the hour angle, and spherical
//! trigonometry the altitude and azimuth. No atmospheric refraction is applied to positions; sunrise and
//! sunset are when the sun's upper limb meets the horizon, its center at
//! [`HORIZON_ALTITUDE`] (refraction and the sun's radius together).
//!
//! Named [`SunPreset`]s live in [`Project::sun_presets`], tracked as
//! project settings (`EditCtx::sun_presets_mut`).
//!
//! Dates and times are local to the site: a date is a [`SunDate`] and a
//! time of day is minutes after local midnight (`u16`, `0..=`[`MAX_MINUTE`]
//! in settings; `f64` in frames and results). Everything is deterministic:
//! nothing reads the clock.

use crate::{LevelId, Project, View, ViewKind};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// The last minute of a day (23:59).
pub const MAX_MINUTE: u16 = 1439;
/// Most frames a study yields; longer ones are cut.
pub const MAX_FRAMES: usize = 5000;
/// Most days a study walks through (a bound for studies spanning polar nights).
pub const MAX_DAYS: usize = 100_000;
/// Most presets a project keeps.
pub const MAX_PRESETS: usize = 200;
/// Longest preset name, characters.
pub const MAX_PRESET_NAME: usize = 128;
/// Earliest and latest year of a [`SunDate`].
pub const MIN_YEAR: i32 = 1;
pub const MAX_YEAR: i32 = 9999;
/// Accepted offsets from UTC, hours.
pub const UTC_OFFSET_RANGE: std::ops::RangeInclusive<f64> = -12.0..=14.0;
/// Altitude of the sun's center, degrees, when its upper limb touches the
/// horizon (34' of refraction plus 16' of radius).
pub const HORIZON_ALTITUDE: f64 = -0.833;

/// Default time of a still sun: 10:00.
const DEFAULT_TIME: u16 = 600;
/// Default lighting azimuth (behind the viewer, to the right) and altitude, degrees.
const DEFAULT_LIGHT_AZIMUTH: f64 = 135.0;
const DEFAULT_LIGHT_ALTITUDE: f64 = 35.0;

// ---------------------------------------------------------------- dates

/// A calendar date (proleptic Gregorian), local to the site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SunDate {
    pub year: i32,
    /// 1 to 12.
    pub month: u8,
    /// 1 to the month's length.
    pub day: u8,
}

impl Default for SunDate {
    /// June 21, 2026: a fixed date, so a new view is the same everywhere.
    fn default() -> Self {
        SunDate { year: 2026, month: 6, day: 21 }
    }
}

impl std::fmt::Display for SunDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Whether `year` has a February 29.
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Days in `month` (1 to 12) of `year`; 0 for a month out of range.
pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Days from 1970-01-01 to a civil date (any integers; months and days
/// out of range run on into the next ones).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The civil date `days` after 1970-01-01, as (year, month, day).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

impl SunDate {
    /// The date, when it is a real one within [`MIN_YEAR`]..=[`MAX_YEAR`].
    pub fn new(year: i32, month: u8, day: u8) -> Option<SunDate> {
        let d = SunDate { year, month, day };
        d.is_valid().then_some(d)
    }

    /// A real date within [`MIN_YEAR`]..=[`MAX_YEAR`].
    pub fn is_valid(self) -> bool {
        (MIN_YEAR..=MAX_YEAR).contains(&self.year)
            && (1..=12).contains(&self.month)
            && self.day >= 1
            && self.day <= days_in_month(self.year, self.month)
    }

    /// The nearest valid date: year, month and day each brought into range.
    pub fn clamped(self) -> SunDate {
        let year = self.year.clamp(MIN_YEAR, MAX_YEAR);
        let month = self.month.clamp(1, 12);
        let day = self.day.clamp(1, days_in_month(year, month));
        SunDate { year, month, day }
    }

    /// Reads an ISO `YYYY-MM-DD` date (surrounding spaces ignored).
    pub fn parse(text: &str) -> Option<SunDate> {
        let (y, m, d) = crate::title_block::parse_iso_date(text.trim())?;
        SunDate::new(y, u8::try_from(m).ok()?, u8::try_from(d).ok()?)
    }

    /// The ISO `YYYY-MM-DD` form (the same as `to_string`).
    pub fn iso(self) -> String {
        self.to_string()
    }

    /// Days in the date's year: 365 or 366.
    pub fn days_in_year(self) -> u16 {
        if is_leap_year(self.year) { 366 } else { 365 }
    }

    /// The day of the year, 1 for January 1 (of the [clamped](Self::clamped) date).
    pub fn day_of_year(self) -> u16 {
        let d = self.clamped();
        let n = d.to_days() - days_from_civil(i64::from(d.year), 1, 1) + 1;
        u16::try_from(n).unwrap_or(1)
    }

    /// Days from 1970-01-01 (negative before it).
    pub fn to_days(self) -> i64 {
        days_from_civil(i64::from(self.year), i64::from(self.month), i64::from(self.day))
    }

    /// The date `days` after 1970-01-01, when its year is in range.
    pub fn from_days(days: i64) -> Option<SunDate> {
        let (y, m, d) = civil_from_days(days);
        SunDate::new(i32::try_from(y).ok()?, u8::try_from(m).ok()?, u8::try_from(d).ok()?)
    }

    /// The next day (`None` past [`MAX_YEAR`]).
    pub fn next_day(self) -> Option<SunDate> {
        self.add_days(1)
    }

    /// The date `n` days later (earlier when negative), when in range.
    pub fn add_days(self, n: i64) -> Option<SunDate> {
        SunDate::from_days(self.to_days().checked_add(n)?)
    }

    /// The date `n` weeks later, when in range.
    pub fn add_weeks(self, n: i64) -> Option<SunDate> {
        self.add_days(n.checked_mul(7)?)
    }

    /// The date `n` months later, its day cut to the new month's length
    /// (January 31 plus one month is February 28 or 29), when in range.
    pub fn add_months(self, n: i64) -> Option<SunDate> {
        let total = (i64::from(self.year) * 12 + i64::from(self.month) - 1).checked_add(n)?;
        let year = i32::try_from(total.div_euclid(12)).ok()?;
        let month = u8::try_from(total.rem_euclid(12) + 1).ok()?;
        SunDate::new(year, month, self.day.min(days_in_month(year, month)))
    }
}

// ---------------------------------------------------------------- solar position

/// Where the sun stands in the sky, degrees.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SunPosition {
    /// Clockwise from true north seen from above (0 north, 90 east), in `[0, 360)`.
    pub azimuth: f64,
    /// Above the horizon (negative below it), in `[-90, 90]`; no refraction.
    pub altitude: f64,
}

/// When the sun rises and sets on a day.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Daylight {
    /// Sunrise and sunset, minutes after local midnight (may fall outside
    /// `0..1440` when the time zone is far from the longitude).
    Normal { sunrise: f64, sunset: f64 },
    /// The sun never sets.
    PolarDay,
    /// The sun never rises.
    PolarNight,
}

/// Finite inputs within range: latitude clamped to ±90 (0 when not a
/// number), longitude into `(-180, 180]`, the offset into [`UTC_OFFSET_RANGE`].
fn site_inputs(latitude: f64, longitude: f64, utc_offset: f64) -> (f64, f64, f64) {
    let lat = if latitude.is_finite() { latitude.clamp(-90.0, 90.0) } else { 0.0 };
    let lon = if longitude.is_finite() { crate::site::normalize_angle(longitude) } else { 0.0 };
    let tz =
        if utc_offset.is_finite() { utc_offset.clamp(*UTC_OFFSET_RANGE.start(), *UTC_OFFSET_RANGE.end()) } else { 0.0 };
    (lat, lon, tz)
}

/// `a` degrees in `[0, 360)` (0 when not a number).
fn normalize_azimuth(a: f64) -> f64 {
    if !a.is_finite() {
        return 0.0;
    }
    let r = a.rem_euclid(360.0);
    if r >= 360.0 { 0.0 } else { r }
}

/// Declination (radians) and equation of time (minutes) at a moment.
#[derive(Clone, Copy)]
struct Sun {
    decl: f64,
    eqtime: f64,
}

/// The sun's declination and equation of time on `date` at `utc_minutes`
/// after its midnight (UTC): the low-precision solar coordinates in Julian
/// centuries from J2000 (mean longitude and anomaly, equation of center,
/// apparent longitude, obliquity of the ecliptic).
fn sun_at(date: SunDate, utc_minutes: f64) -> Sun {
    // Days from J2000.0 (2000-01-01 12:00 UTC, 10957.5 days after 1970-01-01).
    let days = date.clamped().to_days() as f64 - 10_957.5 + utc_minutes / 1440.0;
    let t = days / 36_525.0;
    let l0 = (280.466_46 + t * (36_000.769_83 + t * 0.000_303_2)).rem_euclid(360.0).to_radians();
    let m = (357.529_11 + t * (35_999.050_29 - 0.000_153_7 * t)).to_radians();
    let e = 0.016_708_634 - t * (0.000_042_037 + 0.000_000_126_7 * t);
    let center = m.sin() * (1.914_602 - t * (0.004_817 + 0.000_014 * t))
        + (2.0 * m).sin() * (0.019_993 - 0.000_101 * t)
        + (3.0 * m).sin() * 0.000_289;
    let omega = (125.04 - 1934.136 * t).to_radians();
    let lambda = (l0.to_degrees() + center - 0.005_69 - 0.004_78 * omega.sin()).to_radians();
    let eps0 = 23.0 + (26.0 + (21.448 - t * (46.815 + t * (0.000_59 - t * 0.001_813))) / 60.0) / 60.0;
    let eps = (eps0 + 0.002_56 * omega.cos()).to_radians();
    let decl = (eps.sin() * lambda.sin()).clamp(-1.0, 1.0).asin();
    let y = (eps / 2.0).tan().powi(2);
    let eqtime = 4.0
        * (y * (2.0 * l0).sin() - 2.0 * e * m.sin() + 4.0 * e * y * m.sin() * (2.0 * l0).cos()
            - 0.5 * y * y * (4.0 * l0).sin()
            - 1.25 * e * e * (2.0 * m).sin())
        .to_degrees();
    Sun { decl, eqtime }
}

/// The sun's position seen from `latitude`, `longitude` (degrees, WGS 84,
/// north and east positive) on `date` at `minutes` after local midnight,
/// local time being `utc_offset_hours` east of UTC. Azimuth clockwise from
/// true north, altitude without atmospheric refraction. Inputs that are not
/// numbers or out of range are brought into range; nothing returns NaN.
pub fn solar_position(
    latitude: f64,
    longitude: f64,
    utc_offset_hours: f64,
    date: SunDate,
    minutes: f64,
) -> SunPosition {
    let (lat, lon, tz) = site_inputs(latitude, longitude, utc_offset_hours);
    let minutes = if minutes.is_finite() { minutes.clamp(-2880.0, 4320.0) } else { 720.0 };
    let s = sun_at(date, minutes - 60.0 * tz);
    // True solar time, then the hour angle (0 at solar noon, positive after it).
    let tst = minutes + s.eqtime + 4.0 * lon - 60.0 * tz;
    let ha = (tst / 4.0 - 180.0).to_radians();
    let phi = lat.to_radians();
    let (sd, cd) = s.decl.sin_cos();
    let sin_alt = (phi.sin() * sd + phi.cos() * cd * ha.cos()).clamp(-1.0, 1.0);
    let altitude = sin_alt.asin().to_degrees();
    let azimuth = (-ha.sin() * cd).atan2(sd * phi.cos() - cd * phi.sin() * ha.cos()).to_degrees();
    SunPosition { azimuth: normalize_azimuth(azimuth), altitude }
}

/// Where the sun's center crosses [`HORIZON_ALTITUDE`]: the hour angle
/// (degrees) of sunset, or whether it stays above or below.
enum Horizon {
    Crosses(f64),
    Above,
    Below,
}

fn horizon(phi: f64, decl: f64) -> Horizon {
    let h = HORIZON_ALTITUDE.to_radians().sin();
    let den = phi.cos() * decl.cos();
    let num = h - phi.sin() * decl.sin();
    if den.abs() < 1e-12 {
        return if phi.sin() * decl.sin() > h { Horizon::Above } else { Horizon::Below };
    }
    let c = num / den;
    if c < -1.0 {
        Horizon::Above
    } else if c > 1.0 {
        Horizon::Below
    } else {
        Horizon::Crosses(c.acos().to_degrees())
    }
}

/// Local minutes at which the hour angle is `ha` degrees.
fn time_of_hour_angle(ha: f64, eqtime: f64, lon: f64, tz: f64) -> f64 {
    720.0 + 4.0 * ha - eqtime - 4.0 * lon + 60.0 * tz
}

/// Local solar noon on `date` at `longitude`, minutes after local midnight.
pub fn solar_noon(longitude: f64, utc_offset_hours: f64, date: SunDate) -> f64 {
    let (_, lon, tz) = site_inputs(0.0, longitude, utc_offset_hours);
    let guess = time_of_hour_angle(0.0, sun_at(date, 720.0 - 60.0 * tz).eqtime, lon, tz);
    time_of_hour_angle(0.0, sun_at(date, guess - 60.0 * tz).eqtime, lon, tz)
}

/// Sunrise and sunset on `date` at the site (see [`Daylight`]).
pub fn daylight(latitude: f64, longitude: f64, utc_offset_hours: f64, date: SunDate) -> Daylight {
    let (lat, lon, tz) = site_inputs(latitude, longitude, utc_offset_hours);
    let phi = lat.to_radians();
    let noon = solar_noon(lon, tz, date);
    let s = sun_at(date, noon - 60.0 * tz);
    let h0 = match horizon(phi, s.decl) {
        Horizon::Crosses(h) => h,
        Horizon::Above => return Daylight::PolarDay,
        Horizon::Below => return Daylight::PolarNight,
    };
    // Each event again with the sun as it is at that time.
    let event = |sign: f64| {
        let guess = time_of_hour_angle(sign * h0, s.eqtime, lon, tz);
        let s2 = sun_at(date, guess - 60.0 * tz);
        match horizon(phi, s2.decl) {
            Horizon::Crosses(h) => time_of_hour_angle(sign * h, s2.eqtime, lon, tz),
            _ => guess,
        }
    };
    Daylight::Normal { sunrise: event(-1.0), sunset: event(1.0) }
}

/// The sun's path over `date` at the site, every `step_minutes` (1 to
/// 1440; 15 when not a number) from 00:00 to 24:00, both included, for sun
/// path diagrams: callers keep the part above the horizon. At most 1441 points.
pub fn day_path(site: &SunSite, date: SunDate, step_minutes: f64) -> Vec<(f64, SunPosition)> {
    let step = if step_minutes.is_finite() && step_minutes > 0.0 { step_minutes.clamp(1.0, 1440.0) } else { 15.0 };
    let count = (1440.0 / step).ceil() as usize;
    (0..=count.min(1440))
        .map(|i| {
            let t = (i as f64 * step).min(1440.0);
            (t, site.position(date, t))
        })
        .collect()
}

/// The unit vector from the model towards the sun, in project
/// coordinates, for a sun at `azimuth` (clockwise from true north) and
/// `altitude` degrees, true north being `north` in plan.
pub fn sun_direction(north: DVec2, azimuth: f64, altitude: f64) -> DVec3 {
    let north = north.try_normalize().unwrap_or(DVec2::Y);
    let east = DVec2::new(north.y, -north.x);
    let (sa, ca) = normalize_azimuth(azimuth).to_radians().sin_cos();
    let alt = if altitude.is_finite() { altitude.clamp(-90.0, 90.0) } else { 0.0 };
    let (sh, ch) = alt.to_radians().sin_cos();
    let plan = (north * ca + east * sa) * ch;
    DVec3::new(plan.x, plan.y, sh)
}

/// Azimuth (clockwise from `north`) of the plan direction `d`.
fn azimuth_of(north: DVec2, d: DVec2) -> f64 {
    let north = north.try_normalize().unwrap_or(DVec2::Y);
    let east = DVec2::new(north.y, -north.x);
    normalize_azimuth(d.dot(east).atan2(d.dot(north)).to_degrees())
}

// ---------------------------------------------------------------- site

/// What the sun needs of the project's site.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunSite {
    /// Degrees, north positive.
    pub latitude: f64,
    /// Degrees, east positive.
    pub longitude: f64,
    /// Local time's offset from UTC, hours east.
    pub utc_offset: f64,
    /// True north in plan (unit vector).
    pub north: DVec2,
    /// The project has a site location (otherwise the values above are
    /// the defaults: the equator at Greenwich, north up).
    pub known: bool,
}

impl Default for SunSite {
    fn default() -> Self {
        SunSite { latitude: 0.0, longitude: 0.0, utc_offset: 0.0, north: DVec2::Y, known: false }
    }
}

impl SunSite {
    /// The sun's position here on `date` at `minutes` after local midnight.
    pub fn position(&self, date: SunDate, minutes: f64) -> SunPosition {
        solar_position(self.latitude, self.longitude, self.utc_offset, date, minutes)
    }

    /// Sunrise and sunset here on `date`.
    pub fn daylight(&self, date: SunDate) -> Daylight {
        daylight(self.latitude, self.longitude, self.utc_offset, date)
    }

    /// True north as a unit vector (+Y when not one).
    pub fn north_dir(&self) -> DVec2 {
        self.north.try_normalize().unwrap_or(DVec2::Y)
    }
}

// ---------------------------------------------------------------- settings

/// The step between frames of a sun study.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SunInterval {
    Minutes15,
    Minutes30,
    Minutes45,
    #[default]
    Hour,
    Day,
    Week,
    Month,
}

impl SunInterval {
    pub const ALL: [SunInterval; 7] = [
        SunInterval::Minutes15,
        SunInterval::Minutes30,
        SunInterval::Minutes45,
        SunInterval::Hour,
        SunInterval::Day,
        SunInterval::Week,
        SunInterval::Month,
    ];

    /// The steps within a day: the only ones a single-day study takes.
    pub const SUB_DAY: [SunInterval; 4] =
        [SunInterval::Minutes15, SunInterval::Minutes30, SunInterval::Minutes45, SunInterval::Hour];

    /// Translation key of its name.
    pub fn key(self) -> &'static str {
        match self {
            SunInterval::Minutes15 => "sun-interval-minutes-15",
            SunInterval::Minutes30 => "sun-interval-minutes-30",
            SunInterval::Minutes45 => "sun-interval-minutes-45",
            SunInterval::Hour => "sun-interval-hour",
            SunInterval::Day => "sun-interval-day",
            SunInterval::Week => "sun-interval-week",
            SunInterval::Month => "sun-interval-month",
        }
    }

    /// The name commands use: `minutes_15`, `minutes_30`, `minutes_45`,
    /// `hour`, `day`, `week` or `month`.
    pub fn name(self) -> &'static str {
        match self {
            SunInterval::Minutes15 => "minutes_15",
            SunInterval::Minutes30 => "minutes_30",
            SunInterval::Minutes45 => "minutes_45",
            SunInterval::Hour => "hour",
            SunInterval::Day => "day",
            SunInterval::Week => "week",
            SunInterval::Month => "month",
        }
    }

    /// The interval called `name` (any case; spaces and `-` read as `_`).
    pub fn from_name(name: &str) -> Option<SunInterval> {
        let name = name.trim().to_ascii_lowercase().replace([' ', '-'], "_");
        SunInterval::ALL.into_iter().find(|i| i.name() == name)
    }

    /// Minutes between frames for a step within a day; `None` for days, weeks and months.
    pub fn minutes(self) -> Option<f64> {
        match self {
            SunInterval::Minutes15 => Some(15.0),
            SunInterval::Minutes30 => Some(30.0),
            SunInterval::Minutes45 => Some(45.0),
            SunInterval::Hour => Some(60.0),
            SunInterval::Day | SunInterval::Week | SunInterval::Month => None,
        }
    }

    /// A step within a day (what a single-day study accepts).
    pub fn is_sub_day(self) -> bool {
        self.minutes().is_some()
    }
}

fn yes() -> bool {
    true
}
fn default_time() -> u16 {
    DEFAULT_TIME
}
fn day_from() -> u16 {
    360
}
fn day_to() -> u16 {
    1080
}
fn noon() -> u16 {
    720
}
fn year_start() -> SunDate {
    SunDate { year: 2026, month: 1, day: 1 }
}
fn year_end() -> SunDate {
    SunDate { year: 2026, month: 12, day: 31 }
}
fn month_interval() -> SunInterval {
    SunInterval::Month
}
fn light_azimuth() -> f64 {
    DEFAULT_LIGHT_AZIMUTH
}
fn light_altitude() -> f64 {
    DEFAULT_LIGHT_ALTITUDE
}

/// How a view places its sun.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SunMode {
    /// One moment.
    Still {
        #[serde(default)]
        date: SunDate,
        /// Minutes after local midnight.
        #[serde(default = "default_time")]
        time: u16,
    },
    /// Frames over one day, from `from` to `to` (or sunrise to sunset)
    /// every `interval` (a step within the day).
    SingleDay {
        #[serde(default)]
        date: SunDate,
        #[serde(default = "day_from")]
        from: u16,
        #[serde(default = "day_to")]
        to: u16,
        #[serde(default)]
        sunrise_to_sunset: bool,
        #[serde(default)]
        interval: SunInterval,
    },
    /// Frames over several days: with a step within a day, every day from
    /// `from_date` to `to_date` as a single-day study; with a day, week or
    /// month, one frame per step at `from` (at 12:00 with `sunrise_to_sunset`).
    MultiDay {
        #[serde(default = "year_start")]
        from_date: SunDate,
        #[serde(default = "year_end")]
        to_date: SunDate,
        #[serde(default = "noon")]
        from: u16,
        #[serde(default = "noon")]
        to: u16,
        #[serde(default)]
        sunrise_to_sunset: bool,
        #[serde(default = "month_interval")]
        interval: SunInterval,
    },
    /// A light from a direction given by hand, with no date.
    Lighting {
        /// Degrees clockwise (seen from above) from true north, or from the
        /// direction the viewer looks along with `relative_to_view`.
        #[serde(default = "light_azimuth")]
        azimuth: f64,
        /// Degrees above the horizon, 0 to 90.
        #[serde(default = "light_altitude")]
        altitude: f64,
        #[serde(default = "yes")]
        relative_to_view: bool,
    },
}

impl Default for SunMode {
    fn default() -> Self {
        SunMode::Still { date: SunDate::default(), time: DEFAULT_TIME }
    }
}

impl SunMode {
    /// The names commands use, in display order.
    pub const NAMES: [&'static str; 4] = ["still", "single_day", "multi_day", "lighting"];

    /// The name commands use: `still`, `single_day`, `multi_day` or `lighting`.
    pub fn name(&self) -> &'static str {
        match self {
            SunMode::Still { .. } => "still",
            SunMode::SingleDay { .. } => "single_day",
            SunMode::MultiDay { .. } => "multi_day",
            SunMode::Lighting { .. } => "lighting",
        }
    }

    /// Translation key of its name.
    pub fn key(&self) -> &'static str {
        match self {
            SunMode::Still { .. } => "sun-mode-still",
            SunMode::SingleDay { .. } => "sun-mode-single-day",
            SunMode::MultiDay { .. } => "sun-mode-multi-day",
            SunMode::Lighting { .. } => "sun-mode-lighting",
        }
    }

    /// A mode of the kind called `name` (any case; spaces and `-` read as
    /// `_`) with its default values.
    pub fn from_name(name: &str) -> Option<SunMode> {
        let name = name.trim().to_ascii_lowercase().replace([' ', '-'], "_");
        Some(match name.as_str() {
            "still" => SunMode::default(),
            "single_day" => SunMode::SingleDay {
                date: SunDate::default(),
                from: day_from(),
                to: day_to(),
                sunrise_to_sunset: false,
                interval: SunInterval::Hour,
            },
            "multi_day" => SunMode::MultiDay {
                from_date: year_start(),
                to_date: year_end(),
                from: noon(),
                to: noon(),
                sunrise_to_sunset: false,
                interval: SunInterval::Month,
            },
            "lighting" => SunMode::Lighting {
                azimuth: DEFAULT_LIGHT_AZIMUTH,
                altitude: DEFAULT_LIGHT_ALTITUDE,
                relative_to_view: true,
            },
            _ => return None,
        })
    }

    /// The step of a study (`None` for a still sun or lighting).
    pub fn interval(&self) -> Option<SunInterval> {
        match self {
            SunMode::SingleDay { interval, .. } | SunMode::MultiDay { interval, .. } => Some(*interval),
            SunMode::Still { .. } | SunMode::Lighting { .. } => None,
        }
    }

    /// Whether the mode places the sun by date and time (all but lighting).
    pub fn is_dated(&self) -> bool {
        !matches!(self, SunMode::Lighting { .. })
    }

    /// Whether the mode's values are all in range (see [`Self::repair`]).
    pub fn is_valid(&self) -> bool {
        self.clone().repair() == 0
    }

    /// Bring every value into range: dates valid and in order, times
    /// within the day and `from <= to`, a single day's step within the
    /// day (else hourly), a light's azimuth in `[0, 360)` and altitude in
    /// `0..=90` (defaults when not numbers). Returns how many it fixed.
    pub fn repair(&mut self) -> usize {
        fn date(d: &mut SunDate) -> usize {
            if d.is_valid() {
                0
            } else {
                *d = d.clamped();
                1
            }
        }
        fn time(t: &mut u16) -> usize {
            if *t > MAX_MINUTE {
                *t = MAX_MINUTE;
                1
            } else {
                0
            }
        }
        fn span(from: &mut u16, to: &mut u16) -> usize {
            let n = time(from) + time(to);
            if *from > *to {
                std::mem::swap(from, to);
                n + 1
            } else {
                n
            }
        }
        match self {
            SunMode::Still { date: d, time: t } => date(d) + time(t),
            SunMode::SingleDay { date: d, from, to, interval, .. } => {
                let mut n = date(d) + span(from, to);
                if !interval.is_sub_day() {
                    *interval = SunInterval::Hour;
                    n += 1;
                }
                n
            }
            SunMode::MultiDay { from_date, to_date, from, to, .. } => {
                let mut n = date(from_date) + date(to_date) + span(from, to);
                if from_date > to_date {
                    std::mem::swap(from_date, to_date);
                    n += 1;
                }
                n
            }
            SunMode::Lighting { azimuth, altitude, .. } => {
                let mut n = 0;
                let az = if azimuth.is_finite() { normalize_azimuth(*azimuth) } else { DEFAULT_LIGHT_AZIMUTH };
                if az != *azimuth {
                    *azimuth = az;
                    n += 1;
                }
                let alt = if altitude.is_finite() { altitude.clamp(0.0, 90.0) } else { DEFAULT_LIGHT_ALTITUDE };
                if alt != *altitude {
                    *altitude = alt;
                    n += 1;
                }
                n
            }
        }
    }
}

/// One moment of a sun study.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunFrame {
    pub date: SunDate,
    /// Minutes after local midnight.
    pub minutes: f64,
}

/// A view's sun: how it is placed, the frame of a study shown, and what is drawn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SunSettings {
    #[serde(default)]
    pub mode: SunMode,
    /// The frame of a study shown (clamped to the frames when read).
    #[serde(default)]
    pub frame: u32,
    /// Draw the sun path diagram.
    #[serde(default = "yes")]
    pub show_path: bool,
    /// Cast shadows (3D views).
    #[serde(default)]
    pub shadows: bool,
    /// The level whose plane receives the shadows (`None`: no ground plane).
    #[serde(default)]
    pub ground: Option<LevelId>,
}

impl Default for SunSettings {
    /// A still sun on June 21, 2026 at 10:00, its path shown, no shadows.
    fn default() -> Self {
        SunSettings { mode: SunMode::default(), frame: 0, show_path: true, shadows: false, ground: None }
    }
}

/// The time range of one day of a study: `from..=to`, or sunrise to
/// sunset (the whole day under a polar day, nothing under a polar night),
/// kept within the day.
fn day_range(site: &SunSite, date: SunDate, from: u16, to: u16, sunrise_to_sunset: bool) -> Option<(f64, f64)> {
    let last = f64::from(MAX_MINUTE);
    if sunrise_to_sunset {
        match site.daylight(date) {
            Daylight::Normal { sunrise, sunset } => {
                let (a, b) = (sunrise.clamp(0.0, last), sunset.clamp(0.0, last));
                (a < b).then_some((a, b))
            }
            Daylight::PolarDay => Some((0.0, last)),
            Daylight::PolarNight => None,
        }
    } else {
        let (a, b) = (from.min(to).min(MAX_MINUTE), from.max(to).min(MAX_MINUTE));
        Some((f64::from(a), f64::from(b)))
    }
}

/// Frames of one day: from `start` every `step` minutes, then `end`.
fn push_day(out: &mut Vec<SunFrame>, date: SunDate, (start, end): (f64, f64), step: f64) {
    let mut t = start;
    while t < end - 1e-6 && out.len() < MAX_FRAMES {
        out.push(SunFrame { date, minutes: t });
        t += step;
    }
    if out.len() < MAX_FRAMES {
        out.push(SunFrame { date, minutes: end });
    }
}

impl SunSettings {
    /// The frames of the study at `site`, in time order, at most
    /// [`MAX_FRAMES`]: one for a still sun; a single day from `from` to
    /// `to` (or sunrise to sunset) every interval, the end included even
    /// off the step (a polar night falls back to one frame at 12:00);
    /// several days as described at [`SunMode::MultiDay`] (dates stepped
    /// from `from_date` up to `to_date`, which is not added when off the
    /// step; no frames at all fall back to 12:00 on `from_date`); none for
    /// lighting.
    pub fn frames(&self, site: &SunSite) -> Vec<SunFrame> {
        let mut out = Vec::new();
        match &self.mode {
            SunMode::Still { date, time } => {
                out.push(SunFrame { date: date.clamped(), minutes: f64::from((*time).min(MAX_MINUTE)) });
            }
            SunMode::SingleDay { date, from, to, sunrise_to_sunset, interval } => {
                let date = date.clamped();
                let step = interval.minutes().unwrap_or(60.0);
                if let Some(range) = day_range(site, date, *from, *to, *sunrise_to_sunset) {
                    push_day(&mut out, date, range, step);
                }
                if out.is_empty() {
                    out.push(SunFrame { date, minutes: 720.0 });
                }
            }
            SunMode::MultiDay { from_date, to_date, from, to, sunrise_to_sunset, interval } => {
                let (first, last) = (from_date.clamped(), to_date.clamped());
                let (first, last) = (first.min(last), first.max(last));
                match interval.minutes() {
                    Some(step) => {
                        let mut day = Some(first);
                        let mut walked = 0;
                        while let Some(d) = day
                            && d <= last
                            && walked < MAX_DAYS
                            && out.len() < MAX_FRAMES
                        {
                            if let Some(range) = day_range(site, d, *from, *to, *sunrise_to_sunset) {
                                push_day(&mut out, d, range, step);
                            }
                            day = d.next_day();
                            walked += 1;
                        }
                    }
                    None => {
                        let at = if *sunrise_to_sunset { 720.0 } else { f64::from((*from).min(MAX_MINUTE)) };
                        for k in 0..MAX_FRAMES as i64 {
                            let d = match interval {
                                SunInterval::Week => first.add_weeks(k),
                                SunInterval::Month => first.add_months(k),
                                _ => first.add_days(k),
                            };
                            let Some(d) = d.filter(|d| *d <= last) else { break };
                            out.push(SunFrame { date: d, minutes: at });
                        }
                    }
                }
                if out.is_empty() {
                    out.push(SunFrame { date: first, minutes: 720.0 });
                }
            }
            SunMode::Lighting { .. } => {}
        }
        out.truncate(MAX_FRAMES);
        out
    }

    /// The frame shown: [`Self::frame`] clamped to the frames (`None` for lighting).
    pub fn current_frame(&self, site: &SunSite) -> Option<SunFrame> {
        let frames = self.frames(site);
        let i = usize::try_from(self.frame).unwrap_or(usize::MAX).min(frames.len().saturating_sub(1));
        frames.get(i).copied()
    }

    /// Whether every value is in range (see [`SunMode::repair`]).
    pub fn is_valid(&self) -> bool {
        self.mode.is_valid()
    }

    /// Bring the mode's values into range; returns how many it fixed. (A
    /// missing ground level is the project's to check.)
    pub fn repair(&mut self) -> usize {
        self.mode.repair()
    }

    /// Where the sun stands for these settings at `site`. Dated modes take
    /// the current frame's solar position. Lighting takes its azimuth from
    /// true north, or, `relative_to_view`, clockwise (seen from above) from
    /// `view_forward`, the plan direction the viewer looks along (0: the
    /// sun ahead of the viewer, 135: behind and to the right), which is
    /// project +Y when `None` or not a direction.
    pub fn state(&self, site: &SunSite, view_forward: Option<DVec2>) -> SunState {
        match &self.mode {
            SunMode::Lighting { azimuth, altitude, relative_to_view } => {
                let az = if azimuth.is_finite() { normalize_azimuth(*azimuth) } else { DEFAULT_LIGHT_AZIMUTH };
                let alt = if altitude.is_finite() { altitude.clamp(0.0, 90.0) } else { DEFAULT_LIGHT_ALTITUDE };
                let azimuth = if *relative_to_view {
                    let f = view_forward.and_then(|f| f.try_normalize()).unwrap_or(DVec2::Y);
                    // Turn the forward direction clockwise by the azimuth.
                    let (s, c) = az.to_radians().sin_cos();
                    let d = DVec2::new(f.x * c + f.y * s, -f.x * s + f.y * c);
                    azimuth_of(site.north_dir(), d)
                } else {
                    az
                };
                SunState::new(site, azimuth, alt, None)
            }
            _ => {
                let frame = self.current_frame(site);
                let pos = frame.map(|f| site.position(f.date, f.minutes)).unwrap_or_default();
                SunState::new(site, pos.azimuth, pos.altitude, frame)
            }
        }
    }
}

/// Where a view's sun stands, resolved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunState {
    /// Degrees clockwise from true north, in `[0, 360)`.
    pub azimuth: f64,
    /// Degrees above the horizon.
    pub altitude: f64,
    /// Unit vector from the model towards the sun, project coordinates.
    pub direction: DVec3,
    /// The study frame shown (`None` for lighting).
    pub frame: Option<SunFrame>,
    /// The sun is above the horizon (it lights and casts shadows).
    pub above_horizon: bool,
}

impl SunState {
    fn new(site: &SunSite, azimuth: f64, altitude: f64, frame: Option<SunFrame>) -> SunState {
        SunState {
            azimuth,
            altitude,
            direction: sun_direction(site.north_dir(), azimuth, altitude),
            frame,
            above_horizon: altitude > 0.0,
        }
    }
}

/// The plan direction a view looks along, for a sun placed relative to
/// it: a 3D view's camera heading, an elevation's or section's look
/// direction; `None` for plans and other views.
pub fn view_forward(view: &View) -> Option<DVec2> {
    match &view.kind {
        ViewKind::ThreeD { camera, .. } => {
            let yaw = camera.yaw_deg.to_radians();
            yaw.is_finite().then(|| DVec2::new(yaw.sin(), yaw.cos()))
        }
        ViewKind::Elevation { direction } => Some(direction.look()),
        ViewKind::Section { start, end, .. } => {
            let d = (*end - *start).try_normalize()?;
            // Sections look to the right of their line.
            Some(DVec2::new(d.y, -d.x))
        }
        _ => None,
    }
}

// ---------------------------------------------------------------- presets

/// A named sun setting the views can take.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SunPreset {
    pub name: String,
    #[serde(default)]
    pub mode: SunMode,
}

/// A preset name: trimmed, cut to [`MAX_PRESET_NAME`] characters; `None` when empty.
pub fn clean_preset_name(name: &str) -> Option<String> {
    let name: String = name.trim().chars().take(MAX_PRESET_NAME).collect();
    let name = name.trim_end().to_string();
    (!name.is_empty()).then_some(name)
}

/// Clean names, drop presets with no name or a name already taken, past
/// [`MAX_PRESETS`], and repair their modes; returns how many it fixed.
pub fn repair_presets(presets: &mut Vec<SunPreset>) -> usize {
    let mut fixes = 0;
    let mut seen = std::collections::BTreeSet::new();
    let before = presets.len();
    presets.retain_mut(|p| {
        let Some(name) = clean_preset_name(&p.name) else { return false };
        if name != p.name {
            p.name = name;
            fixes += 1;
        }
        seen.insert(p.name.clone())
    });
    if presets.len() > MAX_PRESETS {
        presets.truncate(MAX_PRESETS);
    }
    fixes += before - presets.len();
    for p in presets.iter_mut() {
        fixes += p.mode.repair();
    }
    fixes
}

impl Project {
    /// The site as the sun needs it: the project's location, else the
    /// equator at Greenwich in UTC with north up (`known: false`).
    pub fn sun_site(&self) -> SunSite {
        match &self.info.site {
            Some(s) => SunSite {
                latitude: s.latitude,
                longitude: s.longitude,
                utc_offset: s.offset_hours(),
                north: s.north(),
                known: true,
            },
            None => SunSite::default(),
        }
    }

    /// Where `view`'s sun stands (`None` when the view has none).
    pub fn sun_state(&self, view: &View) -> Option<SunState> {
        let sun = view.sun.as_ref()?;
        Some(sun.state(&self.sun_site(), view_forward(view)))
    }

    /// The preset called `name`.
    pub fn sun_preset(&self, name: &str) -> Option<&SunPreset> {
        self.sun_presets.iter().find(|p| p.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, Level, ViewId};

    fn date(y: i32, m: u8, d: u8) -> SunDate {
        SunDate::new(y, m, d).unwrap()
    }

    fn near(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    /// Angle between two azimuths, degrees.
    fn az_gap(a: f64, b: f64) -> f64 {
        let d = (a - b).rem_euclid(360.0);
        d.min(360.0 - d)
    }

    #[test]
    fn dates() {
        assert_eq!(SunDate::parse(" 2026-03-20 "), Some(date(2026, 3, 20)));
        assert_eq!(date(2026, 3, 20).iso(), "2026-03-20");
        assert_eq!(SunDate::parse("2026-02-29"), None);
        assert_eq!(SunDate::new(2026, 13, 1), None);
        assert_eq!(SunDate::new(0, 1, 1), None);
        assert_eq!(date(2026, 3, 1).day_of_year(), 60);
        assert_eq!(date(2024, 12, 31).day_of_year(), 366);
        assert_eq!(date(2026, 1, 1).day_of_year(), 1);
        assert_eq!(SunDate::from_days(0), Some(date(1970, 1, 1)));
        assert_eq!(date(2000, 3, 1).to_days() - date(2000, 2, 28).to_days(), 2);
        for d in [date(1, 1, 1), date(1600, 2, 29), date(2026, 6, 21), date(9999, 12, 31)] {
            assert_eq!(SunDate::from_days(d.to_days()), Some(d));
        }
        assert_eq!(date(2026, 12, 31).next_day(), Some(date(2027, 1, 1)));
        assert_eq!(date(9999, 12, 31).next_day(), None);
        assert_eq!(date(2026, 1, 1).add_days(-1), Some(date(2025, 12, 31)));
        assert_eq!(date(2026, 1, 1).add_weeks(2), Some(date(2026, 1, 15)));
        assert_eq!(date(2026, 1, 31).add_months(1), Some(date(2026, 2, 28)));
        assert_eq!(date(2024, 1, 31).add_months(1), Some(date(2024, 2, 29)));
        assert_eq!(date(2026, 11, 30).add_months(3), Some(date(2027, 2, 28)));
        assert_eq!(date(2026, 3, 15).add_months(-3), Some(date(2025, 12, 15)));
        assert_eq!(date(9999, 12, 1).add_months(1), None);
        assert_eq!(date(2026, 1, 1).add_days(i64::MAX), None);
        assert_eq!(SunDate { year: 2026, month: 2, day: 31 }.clamped(), date(2026, 2, 28));
        assert_eq!(SunDate { year: -5, month: 0, day: 0 }.clamped(), date(1, 1, 1));
        assert!(date(2026, 1, 2) > date(2025, 12, 31));
    }

    #[test]
    fn equinox_at_the_equator() {
        let d = date(2026, 3, 20);
        let noon = solar_noon(0.0, 0.0, d);
        // The equation of time is about -7.5 minutes in late March.
        assert!(near(noon, 727.5, 2.0), "{noon}");
        let p = solar_position(0.0, 0.0, 0.0, d, noon);
        assert!(p.altitude > 89.0, "{p:?}");
        let Daylight::Normal { sunrise, sunset } = daylight(0.0, 0.0, 0.0, d) else { panic!() };
        assert!(near(sunset - sunrise, 12.0 * 60.0 + 7.0, 3.0), "{}", sunset - sunrise);
        let rise = solar_position(0.0, 0.0, 0.0, d, sunrise);
        let set = solar_position(0.0, 0.0, 0.0, d, sunset);
        assert!(az_gap(rise.azimuth, 90.0) < 0.5 && az_gap(set.azimuth, 270.0) < 0.5, "{rise:?} {set:?}");
        // The center is at the refracted horizon then.
        assert!(near(rise.altitude, HORIZON_ALTITUDE, 0.05) && near(set.altitude, HORIZON_ALTITUDE, 0.05));
        // Morning sun in the east, afternoon sun in the west.
        assert!(az_gap(solar_position(0.0, 0.0, 0.0, d, 540.0).azimuth, 90.0) < 1.0);
        assert!(az_gap(solar_position(0.0, 0.0, 0.0, d, 900.0).azimuth, 270.0) < 1.0);
    }

    #[test]
    fn tropic_and_southern_sites() {
        let june = date(2026, 6, 21);
        let noon = solar_noon(0.0, 0.0, june);
        assert!(solar_position(23.44, 0.0, 0.0, june, noon).altitude > 89.5);
        // A central Brazilian site, UTC-3: the June sun stands to the north at noon.
        let (lat, lon, tz) = (-15.8, -47.9, -3.0);
        let noon = solar_noon(lon, tz, june);
        assert!(near(noon, 12.0 * 60.0 + 13.0, 3.0), "{noon}");
        let p = solar_position(lat, lon, tz, june, noon);
        assert!(near(p.altitude, 90.0 - (15.8 + 23.44), 0.5), "{p:?}");
        assert!(az_gap(p.azimuth, 0.0) < 1.0, "{p:?}");
        assert!((0.0..360.0).contains(&p.azimuth));
        // A northern site at noon has it to the south.
        let p = solar_position(40.0, 0.0, 0.0, june, solar_noon(0.0, 0.0, june));
        assert!(az_gap(p.azimuth, 180.0) < 1.0 && near(p.altitude, 90.0 - 40.0 + 23.44, 0.5), "{p:?}");
    }

    #[test]
    fn polar_and_long_days() {
        let (june, december) = (date(2026, 6, 21), date(2026, 12, 21));
        assert_eq!(daylight(70.0, 20.0, 1.0, june), Daylight::PolarDay);
        assert_eq!(daylight(70.0, 20.0, 1.0, december), Daylight::PolarNight);
        assert_eq!(daylight(-70.0, 20.0, 1.0, june), Daylight::PolarNight);
        let length = |d| match daylight(51.5, 0.0, 0.0, d) {
            Daylight::Normal { sunrise, sunset } => sunset - sunrise,
            other => panic!("{other:?}"),
        };
        assert!(length(june) > 16.0 * 60.0, "{}", length(june));
        assert!(length(december) < 8.0 * 60.0, "{}", length(december));
        // The poles themselves.
        assert_eq!(daylight(90.0, 0.0, 0.0, june), Daylight::PolarDay);
        assert_eq!(daylight(-90.0, 0.0, 0.0, june), Daylight::PolarNight);
    }

    #[test]
    fn bad_numbers_give_numbers() {
        let d = SunDate { year: 2026, month: 40, day: 99 };
        for (lat, lon, tz, t) in [
            (f64::NAN, f64::INFINITY, f64::NAN, f64::NAN),
            (1e9, -1e9, 1e9, 1e12),
            (90.0, 0.0, 0.0, 720.0),
            (-90.0, 180.0, -12.0, 0.0),
        ] {
            let p = solar_position(lat, lon, tz, d, t);
            assert!(p.azimuth.is_finite() && p.altitude.is_finite(), "{p:?}");
            assert!((0.0..360.0).contains(&p.azimuth) && (-90.0..=90.0).contains(&p.altitude));
            if let Daylight::Normal { sunrise, sunset } = daylight(lat, lon, tz, d) {
                assert!(sunrise.is_finite() && sunset.is_finite());
            }
            assert!(solar_noon(lon, tz, d).is_finite());
        }
        let v = sun_direction(DVec2::ZERO, f64::NAN, f64::INFINITY);
        assert!(v.is_finite() && near(v.length(), 1.0, 1e-9));
    }

    #[test]
    fn direction_follows_true_north() {
        let site = SunSite::default();
        let east = sun_direction(site.north, 90.0, 0.0);
        assert!(east.abs_diff_eq(DVec3::X, 1e-12), "{east}");
        let turned = |v: DVec3| DVec3::new(-v.y, v.x, v.z);
        // True north 90 degrees counter-clockwise from +Y: everything turns with it.
        let north = DVec2::new(-1.0, 0.0);
        for (az, alt) in [(0.0, 10.0), (90.0, 0.0), (135.0, 35.0), (300.0, 72.0), (359.0, 89.0)] {
            let a = sun_direction(DVec2::Y, az, alt);
            let b = sun_direction(north, az, alt);
            assert!(b.abs_diff_eq(turned(a), 1e-12), "{a} {b}");
            assert!(near(a.length(), 1.0, 1e-12) && near(a.z, f64::to_radians(alt).sin(), 1e-12));
        }
        // Seen from the project, the state's direction agrees with its azimuth.
        let site = SunSite { north, ..SunSite::default() };
        let s = SunSettings::default().state(&site, None);
        let plan = DVec2::new(s.direction.x, s.direction.y);
        assert!(near(azimuth_of(north, plan), s.azimuth, 1e-6));
        assert!(near(s.direction.z, s.altitude.to_radians().sin(), 1e-12) && s.above_horizon);
    }

    #[test]
    fn lighting_relative_to_view_or_north() {
        let site = SunSite::default();
        let mut sun = SunSettings {
            mode: SunMode::Lighting { azimuth: 135.0, altitude: 35.0, relative_to_view: true },
            ..SunSettings::default()
        };
        let (c, s35, h) = (35f64.to_radians().cos(), 35f64.to_radians().sin(), 0.5f64.sqrt());
        // Looking along +Y: behind (-Y) and to the right (+X).
        let s = sun.state(&site, Some(DVec2::Y));
        assert!(s.direction.abs_diff_eq(DVec3::new(c * h, -c * h, s35), 1e-9), "{s:?}");
        assert!(near(s.azimuth, 135.0, 1e-9) && s.frame.is_none());
        // Looking east (+X): behind is west, the right is south.
        let s = sun.state(&site, Some(DVec2::new(2.0, 0.0)));
        assert!(near(s.azimuth, 225.0, 1e-9), "{s:?}");
        // No direction: as if looking along +Y.
        assert_eq!(sun.state(&site, None), sun.state(&site, Some(DVec2::ZERO)));
        assert!(near(sun.state(&site, None).azimuth, 135.0, 1e-9));
        // From true north, the view does not matter.
        sun.mode = SunMode::Lighting { azimuth: 90.0, altitude: 0.0, relative_to_view: false };
        let s = sun.state(&site, Some(DVec2::new(1.0, 1.0)));
        assert!(s.direction.abs_diff_eq(DVec3::X, 1e-9) && !s.above_horizon);
    }

    fn frames(mode: SunMode, site: &SunSite) -> Vec<SunFrame> {
        SunSettings { mode, ..SunSettings::default() }.frames(site)
    }

    fn multi(from_date: SunDate, to_date: SunDate, interval: SunInterval) -> SunMode {
        SunMode::MultiDay { from_date, to_date, from: 360, to: 480, sunrise_to_sunset: false, interval }
    }

    #[test]
    fn frames_of_each_mode() {
        let site = SunSite::default();
        let d = date(2026, 6, 21);
        assert_eq!(frames(SunMode::default(), &site), vec![SunFrame { date: d, minutes: 600.0 }]);
        let day = |from, to, interval| SunMode::SingleDay { date: d, from, to, sunrise_to_sunset: false, interval };
        assert_eq!(frames(day(360, 1080, SunInterval::Hour), &site).len(), 13);
        let off = frames(day(360, 1090, SunInterval::Hour), &site);
        assert_eq!(off.len(), 14);
        assert_eq!(off.last().map(|f| f.minutes), Some(1090.0));
        assert_eq!(frames(day(360, 420, SunInterval::Minutes15), &site).len(), 5);
        assert_eq!(frames(day(600, 600, SunInterval::Minutes45), &site).len(), 1);
        // Sunrise to sunset.
        let mode = SunMode::SingleDay { date: d, from: 0, to: 0, sunrise_to_sunset: true, interval: SunInterval::Hour };
        let f = frames(mode.clone(), &site);
        let Daylight::Normal { sunrise, sunset } = site.daylight(d) else { panic!() };
        assert_eq!(f.first().map(|f| f.minutes), Some(sunrise));
        assert_eq!(f.last().map(|f| f.minutes), Some(sunset));
        assert_eq!(f.len(), 14);
        // Polar night: one frame at noon; polar day: the whole day.
        let arctic = SunSite { latitude: 80.0, ..site };
        let night = SunMode::SingleDay {
            date: date(2026, 12, 21),
            from: 0,
            to: 0,
            sunrise_to_sunset: true,
            interval: SunInterval::Hour,
        };
        assert_eq!(frames(night, &arctic), vec![SunFrame { date: date(2026, 12, 21), minutes: 720.0 }]);
        let f = frames(mode, &arctic);
        assert_eq!(f.len(), 25);
        assert_eq!((f.first().map(|f| f.minutes), f.last().map(|f| f.minutes)), (Some(0.0), Some(1439.0)));
        // Several days.
        let year = frames(multi(date(2026, 1, 1), date(2026, 12, 31), SunInterval::Month), &site);
        assert_eq!(year.len(), 12);
        assert!(year.iter().all(|f| f.minutes == 360.0 && f.date.day == 1));
        let weeks = frames(multi(date(2026, 1, 1), date(2026, 1, 31), SunInterval::Week), &site);
        assert_eq!(weeks.iter().map(|f| f.date.day).collect::<Vec<_>>(), [1, 8, 15, 22, 29]);
        assert_eq!(frames(multi(date(2026, 1, 1), date(2026, 1, 31), SunInterval::Day), &site).len(), 31);
        assert_eq!(frames(multi(date(2026, 1, 1), date(2026, 1, 3), SunInterval::Hour), &site).len(), 9);
        // Months stay on the 31st where they can.
        let ends = frames(multi(date(2026, 1, 31), date(2026, 4, 30), SunInterval::Month), &site);
        let ends: Vec<SunDate> = ends.iter().map(|f| f.date).collect();
        assert_eq!(ends, [date(2026, 1, 31), date(2026, 2, 28), date(2026, 3, 31), date(2026, 4, 30)]);
        // The end date is not added off the step.
        assert_eq!(frames(multi(date(2026, 1, 31), date(2026, 4, 29), SunInterval::Month), &site).len(), 3);
        // Noon with sunrise_to_sunset; dates given backwards still run forwards.
        let noon = SunMode::MultiDay {
            from_date: date(2026, 3, 1),
            to_date: date(2026, 1, 1),
            from: 360,
            to: 480,
            sunrise_to_sunset: true,
            interval: SunInterval::Month,
        };
        let f = frames(noon, &site);
        assert_eq!(f.len(), 3);
        assert!(f.iter().all(|f| f.minutes == 720.0));
        // Polar nights only: one frame at noon of the first day.
        let dark = SunMode::MultiDay {
            from_date: date(2026, 12, 1),
            to_date: date(2026, 12, 31),
            from: 0,
            to: 0,
            sunrise_to_sunset: true,
            interval: SunInterval::Hour,
        };
        assert_eq!(frames(dark, &arctic), vec![SunFrame { date: date(2026, 12, 1), minutes: 720.0 }]);
        // Lighting has none.
        assert!(frames(SunMode::from_name("lighting").unwrap(), &site).is_empty());
    }

    #[test]
    fn frames_are_bounded_and_clamped() {
        let site = SunSite::default();
        let all = |interval, sunrise_to_sunset| SunMode::MultiDay {
            from_date: date(1, 1, 1),
            to_date: date(9999, 12, 31),
            from: 0,
            to: 1439,
            sunrise_to_sunset,
            interval,
        };
        assert_eq!(frames(all(SunInterval::Minutes15, false), &site).len(), MAX_FRAMES);
        assert_eq!(frames(all(SunInterval::Day, false), &site).len(), MAX_FRAMES);
        // Polar nights over centuries walk a bounded number of days.
        let arctic = SunSite { latitude: 89.0, ..site };
        assert!(frames(all(SunInterval::Hour, true), &arctic).len() <= MAX_FRAMES);
        let mut sun = SunSettings {
            mode: SunMode::SingleDay {
                date: date(2026, 6, 21),
                from: 360,
                to: 1080,
                sunrise_to_sunset: false,
                interval: SunInterval::Hour,
            },
            frame: 999,
            ..SunSettings::default()
        };
        assert_eq!(sun.current_frame(&site).map(|f| f.minutes), Some(1080.0));
        sun.frame = 2;
        assert_eq!(sun.current_frame(&site).map(|f| f.minutes), Some(480.0));
        let s = sun.state(&site, None);
        assert_eq!(s.frame.map(|f| f.minutes), Some(480.0));
        assert_eq!(s.azimuth, site.position(date(2026, 6, 21), 480.0).azimuth);
        sun.frame = u32::MAX;
        assert_eq!(sun.current_frame(&site).map(|f| f.minutes), Some(1080.0));
        sun.mode = SunMode::from_name("lighting").unwrap();
        assert_eq!(sun.current_frame(&site), None);
    }

    #[test]
    fn day_path_covers_the_day() {
        let site = SunSite { latitude: -23.5, longitude: -46.6, utc_offset: -3.0, ..SunSite::default() };
        let d = date(2026, 6, 21);
        let path = day_path(&site, d, 60.0);
        assert_eq!(path.len(), 25);
        assert_eq!((path[0].0, path[24].0), (0.0, 1440.0));
        assert!(path.iter().any(|(_, p)| p.altitude > 0.0) && path.iter().any(|(_, p)| p.altitude < 0.0));
        assert_eq!(day_path(&site, d, 0.0).len(), 97);
        assert_eq!(day_path(&site, d, 0.001).len(), 1441);
        assert_eq!(day_path(&site, d, 7.0).last().map(|p| p.0), Some(1440.0));
    }

    #[test]
    fn names_and_keys() {
        for i in SunInterval::ALL {
            assert_eq!(SunInterval::from_name(i.name()), Some(i));
            assert!(i.key().starts_with("sun-interval-"));
        }
        assert_eq!(SunInterval::from_name("Minutes-15"), Some(SunInterval::Minutes15));
        assert!(SunInterval::SUB_DAY.iter().all(|i| i.is_sub_day()));
        assert!(!SunInterval::Week.is_sub_day());
        for name in SunMode::NAMES {
            let mode = SunMode::from_name(name).unwrap();
            assert_eq!(mode.name(), name);
            assert!(mode.key().starts_with("sun-mode-") && mode.is_valid());
        }
        assert_eq!(SunMode::from_name("dusk"), None);
    }

    #[test]
    fn serde_defaults_and_round_trips() {
        let kind = ViewKind::ThreeD { camera: Default::default(), analytical: false, rebar: false };
        let view = View::new(ViewId(7), "3D", 100, kind);
        let text = ron::to_string(&view).unwrap();
        assert!(!text.contains("sun"), "{text}");
        let back: View = ron::from_str(&text).unwrap();
        assert_eq!(back.sun, None);
        for name in SunMode::NAMES {
            let mut v = view.clone();
            v.sun = Some(SunSettings {
                mode: SunMode::from_name(name).unwrap(),
                frame: 3,
                show_path: false,
                shadows: true,
                ground: Some(LevelId(4)),
            });
            let back: View = ron::from_str(&ron::to_string(&v).unwrap()).unwrap();
            assert_eq!(back, v);
        }
        // Partial settings fill in defaults.
        let s: SunSettings = ron::from_str("(mode: Lighting(azimuth: 10.0))").unwrap();
        assert_eq!(s.mode, SunMode::Lighting { azimuth: 10.0, altitude: 35.0, relative_to_view: true });
        assert!(s.show_path && !s.shadows && s.ground.is_none() && s.frame == 0);
        let s: SunSettings = ron::from_str("()").unwrap();
        assert_eq!(s, SunSettings::default());
        let s: SunMode = ron::from_str("SingleDay(date: (year: 2026, month: 3, day: 20))").unwrap();
        assert_eq!(s.interval(), Some(SunInterval::Hour));
        // Sites written before the offset existed.
        let site: crate::site::SiteLocation = ron::from_str("(latitude: -15.8, longitude: -47.9)").unwrap();
        assert_eq!(site.utc_offset, None);
        assert_eq!(site.offset_hours(), -3.0);
        assert!(!ron::to_string(&site).unwrap().contains("utc_offset"));
        let mut site = crate::site::SiteLocation { utc_offset: Some(5.5), ..site };
        assert_eq!(site.offset_hours(), 5.5);
        let back: crate::site::SiteLocation = ron::from_str(&ron::to_string(&site).unwrap()).unwrap();
        assert_eq!(back, site);
        site.utc_offset = Some(f64::NAN);
        assert!(!site.is_valid());
        assert_eq!(site.repair(), (true, 1));
        assert_eq!(site.utc_offset, None);
    }

    #[test]
    fn project_site_and_view_state() {
        let mut p = Project::from_template();
        let site = p.sun_site();
        assert!(!site.known && site.north == DVec2::Y);
        let mut location = crate::site::SiteLocation::at(-15.8, -47.9);
        location.true_north = 90.0;
        p.info.site = Some(location);
        let site = p.sun_site();
        assert!(site.known && site.utc_offset == -3.0 && site.north.abs_diff_eq(DVec2::new(-1.0, 0.0), 1e-12));
        let light = SunMode::Lighting { azimuth: 0.0, altitude: 0.0, relative_to_view: true };
        let kind = ViewKind::Section { start: DVec2::ZERO, end: DVec2::X, depth: 10.0, segments: None };
        let mut view = View::new(ViewId(1), "S", 100, kind);
        assert_eq!(p.sun_state(&view), None);
        view.sun = Some(SunSettings { mode: light, ..SunSettings::default() });
        // A section along +X looks to its right, -Y: the sun ahead of the viewer.
        let s = p.sun_state(&view).unwrap();
        assert!(s.direction.abs_diff_eq(DVec3::NEG_Y, 1e-9), "{s:?}");
        view.kind = ViewKind::Elevation { direction: crate::Direction::East };
        assert!(p.sun_state(&view).unwrap().direction.abs_diff_eq(DVec3::NEG_X, 1e-9));
        view.kind = ViewKind::ThreeD {
            camera: crate::CameraState { yaw_deg: 90.0, pitch_deg: 90.0, ..Default::default() },
            analytical: false,
            rebar: false,
        };
        assert!(p.sun_state(&view).unwrap().direction.abs_diff_eq(DVec3::X, 1e-9));
        view.kind = ViewKind::floor_plan(p.levels[0].id);
        assert!(p.sun_state(&view).unwrap().direction.abs_diff_eq(DVec3::Y, 1e-9));
    }

    #[test]
    fn repair_fixes_sun_settings_and_presets() {
        let mut p = Project::from_template();
        let find = |p: &Project, f: fn(&ViewKind) -> bool| p.views.values().find(|v| f(&v.kind)).unwrap().id;
        let plan = find(&p, |k| matches!(k, ViewKind::FloorPlan { .. }));
        let three_d = find(&p, |k| matches!(k, ViewKind::ThreeD { .. }));
        let schedule = find(&p, |k| matches!(k, ViewKind::Schedule { .. }));
        let level = p.levels[0].id;
        assert_eq!(p.repairs_needed(), 0);
        p.views.get_mut(&plan).unwrap().sun = Some(SunSettings {
            mode: SunMode::MultiDay {
                from_date: SunDate { year: 2026, month: 13, day: 1 },
                to_date: date(2026, 1, 1),
                from: 2000,
                to: 10,
                sunrise_to_sunset: false,
                interval: SunInterval::Week,
            },
            ground: Some(LevelId(987_654)),
            ..SunSettings::default()
        });
        p.views.get_mut(&three_d).unwrap().sun = Some(SunSettings {
            mode: SunMode::Lighting { azimuth: f64::NAN, altitude: 120.0, relative_to_view: false },
            ground: Some(level),
            ..SunSettings::default()
        });
        p.views.get_mut(&schedule).unwrap().sun = Some(SunSettings::default());
        let single = SunMode::SingleDay {
            date: date(2026, 6, 21),
            from: 10,
            to: 20,
            sunrise_to_sunset: false,
            interval: SunInterval::Month,
        };
        p.sun_presets = vec![
            SunPreset { name: "  Winter ".into(), mode: SunMode::default() },
            SunPreset { name: "Winter".into(), mode: SunMode::default() },
            SunPreset { name: "   ".into(), mode: SunMode::default() },
            SunPreset { name: "Day".into(), mode: single },
        ];
        p.sun_presets.extend((0..MAX_PRESETS).map(|i| SunPreset { name: format!("P{i}"), mode: SunMode::default() }));
        assert!(p.repairs_needed() > 0);
        assert!(p.repair() > 0);
        assert_eq!(p.repairs_needed(), 0);
        let sun = p.views[&plan].sun.as_ref().unwrap();
        assert_eq!(sun.ground, None);
        let SunMode::MultiDay { from_date, to_date, from, to, .. } = sun.mode else { panic!() };
        assert_eq!((from_date, to_date, from, to), (date(2026, 1, 1), date(2026, 12, 1), 10, MAX_MINUTE));
        let sun = p.views[&three_d].sun.as_ref().unwrap();
        assert_eq!(sun.mode, SunMode::Lighting { azimuth: 135.0, altitude: 90.0, relative_to_view: false });
        assert_eq!(sun.ground, Some(level));
        assert_eq!(p.views[&schedule].sun, None);
        assert_eq!(p.sun_presets.len(), MAX_PRESETS);
        assert_eq!(p.sun_presets[0].name, "Winter");
        assert_eq!(p.sun_presets[1].name, "Day");
        assert_eq!(p.sun_presets[1].mode.interval(), Some(SunInterval::Hour));
        assert!(p.sun_preset("Day").is_some());
    }

    #[test]
    fn presets_and_ground_undo_with_their_edits() {
        let mut doc = Document::new(Project::from_template());
        doc.edit("preset", |e| {
            e.sun_presets_mut().push(SunPreset { name: "Noon".into(), mode: SunMode::default() });
        });
        assert_eq!(doc.project.sun_presets.len(), 1);
        assert!(doc.undo());
        assert!(doc.project.sun_presets.is_empty());
        assert!(doc.redo());
        assert_eq!(doc.project.sun_presets[0].name, "Noon");
        // Deleting a level drops it as a ground plane in the same step.
        let view = doc.project.views.values().find(|v| v.kind.takes_sun()).unwrap().id;
        let level = doc.edit("level", |e| {
            let id = LevelId(e.alloc_id());
            e.levels_mut().push(Level { id, name: "Sun".into(), elevation: 50.0 });
            if let Some(v) = e.view_mut(view) {
                v.sun = Some(SunSettings { ground: Some(id), ..SunSettings::default() });
            }
            id
        });
        doc.edit("delete level", |e| e.levels_mut().retain(|l| l.id != level));
        assert_eq!(doc.project.views[&view].sun.as_ref().unwrap().ground, None);
        assert!(doc.undo());
        assert_eq!(doc.project.views[&view].sun.as_ref().unwrap().ground, Some(level));
    }
}
