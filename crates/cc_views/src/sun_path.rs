//! The sun path diagram of a view with a sun ([`cc_core::View::sun`] with
//! `show_path`): a compass ring turned to true north on the ground, the
//! sky dome's path of the sun over the day shown (and, over several days,
//! over the first and last), its whole hours, sunrise, sunset and the date,
//! and the sun where it stands.
//!
//! [`diagram`] builds it in world coordinates, view-independent and purely
//! geometric, so it can be drawn over any projection (2D views here, the
//! live 3D view by the app); [`draw`] lays it over a view's drawing as an
//! overlay ([`Drawing2D::overlay`]): no element, not picked or snapped to,
//! not cropped, but fitted.

use crate::{Anchor, Drawing2D, LineStyle, Pen, Prim};
use cc_core::sun::{self, Daylight, SunDate, SunPosition, SunSite};
use cc_core::{LevelId, LinePattern, Project, View};
use cc_geom::ModelGeometry;
use glam::{DVec2, DVec3};

/// Smallest and largest radius of the diagram, meters.
pub const MIN_RADIUS: f64 = 10.0;
pub const MAX_RADIUS: f64 = 2000.0;
/// The radius as a share of the plan diagonal of the model's extents.
const RADIUS_FACTOR: f64 = 0.75;
/// Chords of each circle of the compass ring.
const RING_SEGMENTS: usize = 96;
/// Minutes between the points of a day's path.
const PATH_STEP: f64 = 10.0;
/// Most points a diagram holds (lines, dots and labels together).
pub const MAX_POINTS: usize = 4096;
/// Most dots of study frames on the day shown.
const MAX_FRAME_DOTS: usize = 200;

/// Radii, as multiples of the diagram's radius.
const OUTER_RING: f64 = 1.06;
const TICK_END: f64 = 1.10;
const CARDINAL_TICK_END: f64 = 1.14;
const AXIS_END: f64 = 1.14;
const CARDINAL_LABEL: f64 = 1.20;
const SUN_RADIUS: f64 = 0.045;

/// What a line of the diagram is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SunLineKind {
    /// A circle of the compass ring (the inner at the radius, then the outer).
    Ring,
    /// A tick of the compass every 15 degrees.
    Tick,
    /// The north-south and east-west axes.
    Axis,
    /// The sun's path over the day shown.
    Path,
    /// The path over the first or last day of a study over several days.
    PathRange,
}

/// A polyline of the diagram, world coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct SunLine {
    pub pts: Vec<DVec3>,
    pub closed: bool,
    pub kind: SunLineKind,
}

/// What a label says, to be written in the reader's language.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SunText {
    /// A translation key (the cardinal points).
    Key(&'static str),
    /// A time of day, minutes after local midnight.
    Time(f64),
    /// A date.
    Date(SunDate),
}

/// What a label marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SunLabelKind {
    Cardinal,
    Sunrise,
    Sunset,
    Date,
    /// The time of the sun shown.
    Now,
}

/// A label of the diagram at `pos`, pushed away from it along `out` (a
/// unit direction in world coordinates, or zero).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunLabel {
    pub pos: DVec3,
    pub text: SunText,
    pub kind: SunLabelKind,
    pub out: DVec3,
}

/// A sun path diagram in world coordinates (see the module's documentation).
#[derive(Debug, Clone, PartialEq)]
pub struct SunDiagram {
    /// The center of the compass, on the ground plane.
    pub center: DVec3,
    /// The radius of the compass ring and of the sky dome.
    pub radius: f64,
    pub lines: Vec<SunLine>,
    /// Whole hours on the day's path, and the frames of a study on that day.
    pub dots: Vec<DVec3>,
    pub labels: Vec<SunLabel>,
    /// The sun's place on the dome (`None` below the horizon).
    pub sun: Option<DVec3>,
    /// The radius of the sun's disc, meters.
    pub sun_radius: f64,
}

impl SunDiagram {
    /// How many points it holds (lines, dots, labels and the sun).
    pub fn point_count(&self) -> usize {
        self.lines.iter().map(|l| l.pts.len()).sum::<usize>()
            + self.dots.len()
            + self.labels.len()
            + usize::from(self.sun.is_some())
    }
}

/// The sun path diagram of `view`: `None` unless it takes a sun, has one and
/// shows its path. Centered on the plan center of the model's extents in
/// `geom` (the geometry the view shows; the origin for an empty model), on
/// the ground level's plane (else the bottom of the extents), with a radius
/// of three quarters of the extents' plan diagonal within
/// [`MIN_RADIUS`]..=[`MAX_RADIUS`]. Every point is finite, and there are at
/// most [`MAX_POINTS`].
pub fn diagram(project: &Project, geom: &ModelGeometry, view: &View) -> Option<SunDiagram> {
    let settings = view.sun.as_ref()?;
    if !settings.show_path || !view.kind.takes_sun() {
        return None;
    }
    let state = project.sun_state(view)?;
    let site = project.sun_site();
    let (center, radius) = placement(project, geom, settings.ground);
    let mut b = Builder::new(center, radius, site.north_dir());
    b.compass();
    if settings.mode.is_dated()
        && let Some(frame) = state.frame
    {
        let date = frame.date.clamped();
        b.day(&site, date);
        if let cc_core::sun::SunMode::MultiDay { from_date, to_date, .. } = &settings.mode {
            let (first, last) = (from_date.clamped(), to_date.clamped());
            let mut days = vec![first.min(last), first.max(last)];
            days.dedup();
            for d in days.into_iter().filter(|d| *d != date) {
                b.range(&site, d);
            }
        }
        // The frames of a study on the day shown.
        let frames = settings.frames(&site);
        for f in
            frames.iter().filter(|f| f.date == date && f.minutes.rem_euclid(60.0).abs() > 1e-6).take(MAX_FRAME_DOTS)
        {
            let p = site.position(date, f.minutes);
            if p.altitude > 0.0 {
                let dot = b.on_dome(p);
                b.out.dots.push(dot);
            }
        }
    }
    if state.above_horizon && state.direction.is_finite() {
        let sun = center + state.direction * radius;
        b.out.sun = Some(sun);
        if let Some(frame) = state.frame {
            let out = plan_dir(state.direction).unwrap_or(b.east);
            b.out.labels.push(SunLabel { pos: sun, text: SunText::Time(frame.minutes), kind: SunLabelKind::Now, out });
        }
    }
    Some(b.finish())
}

/// The diagram's center and radius (see [`diagram`]).
fn placement(project: &Project, geom: &ModelGeometry, ground: Option<LevelId>) -> (DVec3, f64) {
    let bounds = geom.bounds().filter(|(lo, hi)| lo.is_finite() && hi.is_finite());
    let (plan, diagonal, bottom) = match bounds {
        Some((lo, hi)) => (lo.truncate() * 0.5 + hi.truncate() * 0.5, (hi - lo).truncate().length(), lo.z),
        None => (DVec2::ZERO, 0.0, 0.0),
    };
    let plan = if plan.is_finite() { plan } else { DVec2::ZERO };
    let z = ground.and_then(|l| project.level(l)).map(|l| l.elevation).filter(|z| z.is_finite()).unwrap_or(bottom);
    let z = if z.is_finite() { z } else { 0.0 };
    let radius = RADIUS_FACTOR * diagonal;
    let radius = if radius.is_finite() { radius.clamp(MIN_RADIUS, MAX_RADIUS) } else { MIN_RADIUS };
    (DVec3::new(plan.x, plan.y, z), radius)
}

/// The plan part of `d`, as a unit horizontal vector (`None` when vertical
/// or not a number).
fn plan_dir(d: DVec3) -> Option<DVec3> {
    d.truncate().try_normalize().map(|p| p.extend(0.0))
}

/// The sun's path over a day: the runs above the horizon (each starting and
/// ending on the horizon where it crosses it), and where it rises and sets.
struct DayPath {
    runs: Vec<Vec<(f64, SunPosition)>>,
    rises: Vec<(f64, SunPosition)>,
    sets: Vec<(f64, SunPosition)>,
    /// Whole hours above the horizon.
    hours: Vec<SunPosition>,
}

fn day_path(site: &SunSite, date: SunDate) -> DayPath {
    let samples = sun::day_path(site, date, PATH_STEP);
    let mut out = DayPath { runs: Vec::new(), rises: Vec::new(), sets: Vec::new(), hours: Vec::new() };
    let mut run: Vec<(f64, SunPosition)> = Vec::new();
    let mut prev: Option<(f64, SunPosition)> = None;
    for &(t, p) in &samples {
        let up = p.altitude > 0.0;
        if let Some((t0, p0)) = prev
            && (p0.altitude > 0.0) != up
        {
            // The crossing, between the samples: on the horizon itself.
            let (a0, a1) = (p0.altitude, p.altitude);
            let f = if (a0 - a1).abs() > 1e-12 { (a0 / (a0 - a1)).clamp(0.0, 1.0) } else { 0.5 };
            let tc = t0 + (t - t0) * f;
            let pc = SunPosition { azimuth: site.position(date, tc).azimuth, altitude: 0.0 };
            if up {
                out.rises.push((tc, pc));
                run = vec![(tc, pc)];
            } else {
                out.sets.push((tc, pc));
                run.push((tc, pc));
                out.runs.push(std::mem::take(&mut run));
            }
        }
        if up {
            run.push((t, p));
            if t < 1440.0 && (t / 60.0 - (t / 60.0).round()).abs() < 1e-9 {
                out.hours.push(p);
            }
        }
        prev = Some((t, p));
    }
    if run.len() >= 2 {
        out.runs.push(run);
    }
    out
}

/// Accumulates a diagram.
struct Builder {
    center: DVec3,
    radius: f64,
    north: DVec3,
    east: DVec3,
    out: SunDiagram,
}

impl Builder {
    fn new(center: DVec3, radius: f64, north: DVec2) -> Self {
        let north = north.try_normalize().unwrap_or(DVec2::Y);
        Builder {
            center,
            radius,
            north: north.extend(0.0),
            east: DVec3::new(north.y, -north.x, 0.0),
            out: SunDiagram {
                center,
                radius,
                lines: Vec::new(),
                dots: Vec::new(),
                labels: Vec::new(),
                sun: None,
                sun_radius: SUN_RADIUS * radius,
            },
        }
    }

    /// The horizontal unit vector at `azimuth` degrees clockwise from north.
    fn bearing(&self, azimuth: f64) -> DVec3 {
        let (s, c) = azimuth.to_radians().sin_cos();
        self.north * c + self.east * s
    }

    /// The point on the ground `k` radii from the center at `azimuth`.
    fn ground(&self, azimuth: f64, k: f64) -> DVec3 {
        self.center + self.bearing(azimuth) * (k * self.radius)
    }

    /// Where the sun at `p` stands on the dome.
    fn on_dome(&self, p: SunPosition) -> DVec3 {
        let north = DVec2::new(self.north.x, self.north.y);
        self.center + sun::sun_direction(north, p.azimuth, p.altitude) * self.radius
    }

    /// The compass ring, ticks, axes and cardinal points.
    fn compass(&mut self) {
        for k in [1.0, OUTER_RING] {
            let pts = (0..RING_SEGMENTS).map(|i| self.ground(i as f64 * 360.0 / RING_SEGMENTS as f64, k)).collect();
            self.out.lines.push(SunLine { pts, closed: true, kind: SunLineKind::Ring });
        }
        for i in 0..24 {
            let az = f64::from(i) * 15.0;
            let end = if i % 6 == 0 { CARDINAL_TICK_END } else { TICK_END };
            let pts = vec![self.ground(az, 1.0), self.ground(az, end)];
            self.out.lines.push(SunLine { pts, closed: false, kind: SunLineKind::Tick });
        }
        for az in [0.0, 90.0] {
            let pts = vec![self.ground(az + 180.0, AXIS_END), self.ground(az, AXIS_END)];
            self.out.lines.push(SunLine { pts, closed: false, kind: SunLineKind::Axis });
        }
        for (az, key) in [(0.0, "sun-north"), (90.0, "sun-east"), (180.0, "sun-south"), (270.0, "sun-west")] {
            self.out.labels.push(SunLabel {
                pos: self.ground(az, CARDINAL_LABEL),
                text: SunText::Key(key),
                kind: SunLabelKind::Cardinal,
                out: self.bearing(az),
            });
        }
    }

    /// The day shown: its path, whole hours, sunrise, sunset and date.
    fn day(&mut self, site: &SunSite, date: SunDate) {
        let path = day_path(site, date);
        for run in &path.runs {
            let pts = run.iter().map(|(_, p)| self.on_dome(*p)).collect();
            self.out.lines.push(SunLine { pts, closed: false, kind: SunLineKind::Path });
        }
        for p in &path.hours {
            let dot = self.on_dome(*p);
            self.out.dots.push(dot);
        }
        let (sunrise, sunset) = match site.daylight(date) {
            Daylight::Normal { sunrise, sunset } => (Some(sunrise), Some(sunset)),
            Daylight::PolarDay | Daylight::PolarNight => (None, None),
        };
        for (crossings, time, kind) in
            [(&path.rises, sunrise, SunLabelKind::Sunrise), (&path.sets, sunset, SunLabelKind::Sunset)]
        {
            for (t, p) in crossings {
                let pos = self.on_dome(*p);
                let out = plan_dir(pos - self.center).unwrap_or(self.north);
                let minutes = time.filter(|m| m.is_finite()).unwrap_or(*t);
                self.out.labels.push(SunLabel { pos, text: SunText::Time(minutes), kind, out });
            }
        }
        // The date at the sunrise, or at the lowest point of a polar day on its eastern side.
        let at = match path.rises.first() {
            Some((_, p)) => Some(*p),
            None => {
                let points = || path.runs.iter().flatten().map(|(_, p)| *p);
                let lowest = |a: &SunPosition, b: &SunPosition| a.altitude.total_cmp(&b.altitude);
                let east = points().filter(|p| p.azimuth > 0.0 && p.azimuth < 180.0).min_by(lowest);
                east.or_else(|| points().min_by(lowest))
            }
        };
        if let Some(p) = at {
            let pos = self.on_dome(p);
            let out = plan_dir(pos - self.center).unwrap_or(self.east);
            self.out.labels.push(SunLabel { pos, text: SunText::Date(date), kind: SunLabelKind::Date, out });
        }
    }

    /// The path of another day of a study, alone.
    fn range(&mut self, site: &SunSite, date: SunDate) {
        for run in day_path(site, date).runs {
            let pts = run.iter().map(|(_, p)| self.on_dome(*p)).collect();
            self.out.lines.push(SunLine { pts, closed: false, kind: SunLineKind::PathRange });
        }
    }

    /// The diagram, its points finite and within [`MAX_POINTS`].
    fn finish(mut self) -> SunDiagram {
        let d = &mut self.out;
        d.lines.retain(|l| l.pts.iter().all(|p| p.is_finite()));
        d.dots.retain(|p| p.is_finite());
        d.labels.retain(|l| l.pos.is_finite() && l.out.is_finite());
        d.sun = d.sun.filter(|p| p.is_finite());
        // Bounded: drop the least important first (dots, then range paths).
        while d.point_count() > MAX_POINTS && d.dots.pop().is_some() {}
        while d.point_count() > MAX_POINTS {
            match d.lines.iter().rposition(|l| l.kind == SunLineKind::PathRange) {
                Some(i) => {
                    d.lines.remove(i);
                }
                None => break,
            }
        }
        self.out
    }
}

// ---------------------------------------------------------------- drawing

/// Lines of the compass: neutral grey-blue.
const GRID_COLOR: [u8; 3] = [90, 110, 150];
const GRID_WEIGHT: f64 = 0.13;
/// The band between the two circles of the ring.
const BAND_COLOR: [u8; 4] = [120, 150, 220, 60];
/// The sun's path: amber.
const PATH_COLOR: [u8; 3] = [240, 170, 0];
const PATH_WEIGHT: f64 = 0.25;
const RANGE_WEIGHT: f64 = 0.13;
/// Radius of an hour's dot on paper, mm.
const DOT_RADIUS_MM: f64 = 0.6;
/// The sun's disc and its outline.
const SUN_FILL: [u8; 4] = [255, 225, 90, 230];
const SUN_OUTLINE: [u8; 3] = [200, 140, 0];
const SUN_OUTLINE_WEIGHT: f64 = 0.18;
/// Smallest radius of the sun's disc on paper, mm.
const MIN_SUN_RADIUS_MM: f64 = 2.5;
/// Chords of a disc.
const DISC_SEGMENTS: usize = 20;
/// Text heights on paper, mm.
const CARDINAL_SIZE: f64 = 3.5;
const TIME_SIZE: f64 = 2.0;
const DATE_SIZE: f64 = 2.5;
/// Gap between a label and its point on paper, mm.
const LABEL_GAP_MM: f64 = 1.5;
/// The translation key of the date's pattern (`{ $day }`, `{ $month }`, `{ $year }`).
const DATE_PATTERN_KEY: &str = "sun-date-pattern";

/// A time of day as `HH:MM` (minutes after midnight, any day).
pub fn time_text(minutes: f64) -> String {
    let m = if minutes.is_finite() { minutes.round().rem_euclid(1440.0) } else { 0.0 };
    let m = m as u32 % 1440;
    format!("{:02}:{:02}", m / 60, m % 60)
}

/// `date` in the reader's language: the pattern `label` gives for
/// [`DATE_PATTERN_KEY`], else ISO `YYYY-MM-DD`.
pub fn date_text(date: SunDate, label: &dyn Fn(&str) -> String) -> String {
    let pattern = label(DATE_PATTERN_KEY);
    if !pattern.contains("{ $year }") {
        return date.iso();
    }
    pattern
        .replace("{ $year }", &format!("{:04}", date.year))
        .replace("{ $month }", &format!("{:02}", date.month))
        .replace("{ $day }", &format!("{:02}", date.day))
}

/// The text of a label in the reader's language.
pub fn label_text(text: SunText, label: &dyn Fn(&str) -> String) -> String {
    match text {
        SunText::Key(key) => label(key),
        SunText::Time(m) => time_text(m),
        SunText::Date(d) => date_text(d, label),
    }
}

/// Lay `view`'s sun path diagram over `d` (nothing when it has none), last
/// and counted in [`Drawing2D::overlay`]: plans in plan coordinates, the
/// other views through their frame ([`crate::projected::view_frame`]),
/// with no hidden lines removed and no element.
pub fn draw(d: &mut Drawing2D, project: &Project, geom: &ModelGeometry, view: &View, label: &dyn Fn(&str) -> String) {
    let Some(diagram) = diagram(project, geom, view) else { return };
    let frame = crate::projected::view_frame(view);
    let to2d = |p: DVec3| match &frame {
        Some(f) => f.project(p),
        None => p.truncate(),
    };
    let mm = f64::from(view.scale.max(1)) / 1000.0;
    let start = d.prims.len();
    let pen = |color: [u8; 3], weight: f64, pattern: Option<LinePattern>| Pen {
        color: Some(color),
        weight: Some(weight),
        pattern,
        halftone: false,
    };
    let line = |d: &mut Drawing2D, pts: &[DVec2], closed: bool, pen: Pen| {
        let n = pts.len();
        let segments = if closed && n > 2 { n } else { n.saturating_sub(1) };
        for i in 0..segments {
            if let (Some(a), Some(b)) = (pts.get(i), pts.get((i + 1) % n)) {
                d.line_pen(*a, *b, LineStyle::Annotation, Some(pen), None);
            }
        }
    };
    // The band between the circles of the ring, when it is seen at all.
    let mut rings: Vec<Vec<DVec2>> = diagram
        .lines
        .iter()
        .filter(|l| l.kind == SunLineKind::Ring)
        .map(|l| l.pts.iter().map(|p| to2d(*p)).collect())
        .collect();
    if let [inner, outer] = &mut rings[..] {
        let area = cc_geom::polygon::area(outer).abs();
        let tiny = 1e-6 * (diagram.radius * diagram.radius);
        if area > tiny {
            match cc_geom::polygon::keyhole(outer, std::slice::from_ref(inner)) {
                Some(ring) => d.fill(ring, BAND_COLOR, None),
                None => {
                    // One quad per chord.
                    let n = inner.len().min(outer.len());
                    for i in 0..n {
                        let j = (i + 1) % n;
                        d.fill(vec![inner[i], inner[j], outer[j], outer[i]], BAND_COLOR, None);
                    }
                }
            }
        }
    }
    let order = [SunLineKind::Ring, SunLineKind::Tick, SunLineKind::Axis, SunLineKind::PathRange, SunLineKind::Path];
    for kind in order {
        let pen = match kind {
            SunLineKind::Ring | SunLineKind::Tick | SunLineKind::Axis => pen(GRID_COLOR, GRID_WEIGHT, None),
            SunLineKind::PathRange => pen(PATH_COLOR, RANGE_WEIGHT, Some(LinePattern::Dash)),
            SunLineKind::Path => pen(PATH_COLOR, PATH_WEIGHT, None),
        };
        for l in diagram.lines.iter().filter(|l| l.kind == kind) {
            let pts: Vec<DVec2> = l.pts.iter().map(|p| to2d(*p)).collect();
            line(d, &pts, l.closed, pen);
        }
    }
    let disc = |c: DVec2, r: f64| -> Vec<DVec2> {
        (0..DISC_SEGMENTS)
            .map(|i| c + DVec2::from_angle(i as f64 * std::f64::consts::TAU / DISC_SEGMENTS as f64) * r)
            .collect()
    };
    let dot_color = [PATH_COLOR[0], PATH_COLOR[1], PATH_COLOR[2], 255];
    for p in &diagram.dots {
        d.fill(disc(to2d(*p), DOT_RADIUS_MM * mm), dot_color, None);
    }
    let sun_r = diagram.sun_radius.max(MIN_SUN_RADIUS_MM * mm);
    if let Some(s) = diagram.sun {
        let outline = disc(to2d(s), sun_r);
        d.fill(outline.clone(), SUN_FILL, None);
        line(d, &outline, true, pen(SUN_OUTLINE, SUN_OUTLINE_WEIGHT, None));
    }
    for l in &diagram.labels {
        let text = label_text(l.text, label);
        if text.trim().is_empty() {
            continue;
        }
        let size = match l.kind {
            SunLabelKind::Cardinal => CARDINAL_SIZE,
            SunLabelKind::Date => DATE_SIZE,
            SunLabelKind::Sunrise | SunLabelKind::Sunset | SunLabelKind::Now => TIME_SIZE,
        };
        let at = to2d(l.pos);
        let out = (to2d(l.pos + l.out) - at).try_normalize().unwrap_or(DVec2::ZERO);
        // Clear of the point by half the text's box along `out`, in paper mm.
        let width = text.chars().map(|c| if c >= '\u{1100}' { 1.0 } else { 0.6 }).sum::<f64>() * size;
        let half = out.x.abs() * width * 0.5 + out.y.abs() * size * 0.5;
        let clear = match l.kind {
            SunLabelKind::Cardinal => 0.0,
            SunLabelKind::Now => sun_r / mm + LABEL_GAP_MM,
            _ => LABEL_GAP_MM,
        };
        let mut pos = at + out * ((clear + half) * mm);
        if l.kind == SunLabelKind::Date {
            // Under the sunrise's time.
            pos -= DVec2::new(0.0, (TIME_SIZE + DATE_SIZE) * 0.6 * mm);
        }
        if pos.is_finite() {
            d.text_at(pos, text, size, Anchor::Center, 0.0);
        }
    }
    // Keep only finite primitives; count them as the overlay.
    let finite = |p: &Prim| match p {
        Prim::Line { a, b, .. } => a.is_finite() && b.is_finite(),
        Prim::Fill { pts, .. } => pts.iter().all(|p| p.is_finite()),
        Prim::Text { pos, .. } => pos.is_finite(),
        Prim::Image { origin, u, v, .. } => origin.is_finite() && u.is_finite() && v.is_finite(),
    };
    let mut added: Vec<Prim> = d.prims.split_off(start.min(d.prims.len()));
    added.retain(finite);
    d.overlay = added.len();
    d.prims.extend(added);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Look, generate};
    use cc_core::site::{SiteAddress, SiteLocation};
    use cc_core::sun::{SunMode, SunSettings};
    use cc_core::{ViewCategory, ViewKind};

    fn sample() -> (Project, ModelGeometry) {
        let project = cc_core::from_ron(include_str!("../../../examples/casa_simples.ccad")).unwrap();
        let geom = ModelGeometry::build(&project);
        (project, geom)
    }

    fn site(project: &mut Project, latitude: f64, longitude: f64, offset: f64, true_north: f64) {
        project.info.site = Some(SiteLocation {
            latitude,
            longitude,
            elevation: 0.0,
            true_north,
            place: String::new(),
            address: SiteAddress::default(),
            utc_offset: Some(offset),
        });
    }

    fn still(year: i32, month: u8, day: u8, time: u16) -> SunSettings {
        SunSettings {
            mode: SunMode::Still { date: SunDate::new(year, month, day).unwrap(), time },
            ..SunSettings::default()
        }
    }

    fn view(project: &Project, category: ViewCategory) -> View {
        project.views_of(category).next().unwrap().clone()
    }

    fn lines(d: &SunDiagram, kind: SunLineKind) -> Vec<&SunLine> {
        d.lines.iter().filter(|l| l.kind == kind).collect()
    }

    fn label(d: &SunDiagram, kind: SunLabelKind) -> Vec<&SunLabel> {
        d.labels.iter().filter(|l| l.kind == kind).collect()
    }

    fn all_points(d: &SunDiagram) -> Vec<DVec3> {
        let mut pts: Vec<DVec3> = d.lines.iter().flat_map(|l| l.pts.iter().copied()).collect();
        pts.extend(d.dots.iter().copied());
        pts.extend(d.labels.iter().map(|l| l.pos));
        pts.extend(d.sun);
        pts
    }

    fn prim_finite(p: &Prim) -> bool {
        match p {
            Prim::Line { a, b, .. } => a.is_finite() && b.is_finite(),
            Prim::Fill { pts, .. } => pts.iter().all(|p| p.is_finite()),
            Prim::Text { pos, .. } => pos.is_finite(),
            Prim::Image { origin, u, v, .. } => origin.is_finite() && u.is_finite() && v.is_finite(),
        }
    }

    fn texts(prims: &[Prim]) -> Vec<&str> {
        prims
            .iter()
            .filter_map(|p| match p {
                Prim::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Views with no sun draw as before (the default light, no overlay), and
    /// a sun whose path is hidden adds nothing to a plan.
    #[test]
    fn views_without_a_sun_are_unchanged() {
        let (project, geom) = sample();
        for category in [ViewCategory::FloorPlan, ViewCategory::Elevation, ViewCategory::ThreeD] {
            let v = view(&project, category);
            assert!(v.sun.is_none());
            assert_eq!(Look::of(&project, &v).light, None);
            let d = generate(&project, &geom, &v).unwrap();
            assert_eq!(d.overlay, 0);
            assert_eq!(d.own_prims().len(), d.prims.len() - d.underlay);
            // The generators with the default light give the same drawing.
            let look = Look { light: None, ..Look::of(&project, &v) };
            let direct = match &v.kind {
                ViewKind::Elevation { direction } => Some(crate::elevation(&project, &geom, *direction, look)),
                ViewKind::ThreeD { camera, .. } => Some(crate::axonometric(&project, &geom, camera, look)),
                _ => None,
            };
            if let Some(direct) = direct {
                assert!(d.prims.starts_with(&direct.prims), "{category:?}");
            }
        }
        let mut plan = view(&project, ViewCategory::FloorPlan);
        let before = generate(&project, &geom, &plan).unwrap();
        plan.sun = Some(SunSettings { show_path: false, ..SunSettings::default() });
        assert!(diagram(&project, &geom, &plan).is_none());
        let after = generate(&project, &geom, &plan).unwrap();
        assert_eq!(after.overlay, 0);
        assert_eq!(after.prims, before.prims, "plans in hidden line are not shaded: nothing changes");
    }

    /// Central Brazil on June 21 at 10:00: the sun stands to the north-east,
    /// it rises in the east and sets in the west; true north turned 90
    /// degrees turns the whole diagram with it.
    #[test]
    fn a_plan_shows_the_sun_to_the_north_east_in_june_in_brazil() {
        let (mut project, geom) = sample();
        site(&mut project, -15.8, -47.9, -3.0, 0.0);
        let mut plan = view(&project, ViewCategory::FloorPlan);
        plan.sun = Some(still(2026, 6, 21, 600));
        let a = diagram(&project, &geom, &plan).unwrap();
        let c = a.center;
        let sun = a.sun.unwrap();
        assert!(sun.x > c.x + 1e-3 && sun.y > c.y + 1e-3, "{sun} around {c}");
        assert!(sun.z > c.z);
        let (rise, set) = (label(&a, SunLabelKind::Sunrise), label(&a, SunLabelKind::Sunset));
        assert_eq!((rise.len(), set.len()), (1, 1));
        assert!(rise[0].pos.x > c.x + 0.5 * a.radius, "{:?}", rise[0]);
        assert!(set[0].pos.x < c.x - 0.5 * a.radius, "{:?}", set[0]);
        assert!(matches!(rise[0].text, SunText::Time(m) if (300.0..480.0).contains(&m)), "{:?}", rise[0]);
        assert_eq!(label(&a, SunLabelKind::Date).len(), 1);
        assert_eq!(label(&a, SunLabelKind::Now).len(), 1);
        assert_eq!(label(&a, SunLabelKind::Cardinal).len(), 4);
        // Hours from 07:00 to 17:00 are above the horizon.
        assert!((10..=12).contains(&a.dots.len()), "{}", a.dots.len());
        assert!((a.sun_radius - 0.045 * a.radius).abs() < 1e-12);
        assert!(a.point_count() <= MAX_POINTS);

        // True north to the left of the plan: everything turns 90 degrees counter-clockwise.
        site(&mut project, -15.8, -47.9, -3.0, 90.0);
        let b = diagram(&project, &geom, &plan).unwrap();
        let turn = |p: DVec3| {
            let r = p - c;
            c + DVec3::new(-r.y, r.x, r.z)
        };
        let (pa, pb) = (all_points(&a), all_points(&b));
        assert_eq!(pa.len(), pb.len());
        for (p, q) in pa.iter().zip(&pb) {
            assert!(turn(*p).distance(*q) < 1e-6 * a.radius, "{p} -> {q}");
        }

        // Drawn: the sun's disc and the labels, none belonging to an element.
        let d = generate(&project, &geom, &plan).unwrap();
        assert!(d.overlay > 100);
        let overlay = d.overlay_prims();
        assert!(overlay.iter().all(|p| p.element().is_none() && prim_finite(p)));
        assert!(overlay.iter().any(|p| matches!(p, Prim::Fill { color, .. } if *color == SUN_FILL)));
        // Keys stay keys without a translation; times are HH:MM; the date falls back to ISO.
        let texts = texts(overlay);
        for key in ["sun-north", "sun-east", "sun-south", "sun-west", "10:00", "2026-06-21"] {
            assert!(texts.contains(&key), "{key} in {texts:?}");
        }
    }

    /// Texts in the reader's language: keys translated, the date by its pattern.
    #[test]
    fn labels_are_written_in_the_readers_language() {
        let tr = |key: &str| match key {
            "sun-east" => "L".to_string(),
            DATE_PATTERN_KEY => "{ $day }/{ $month }/{ $year }".to_string(),
            other => other.to_string(),
        };
        let date = SunDate::new(2026, 3, 5).unwrap();
        assert_eq!(label_text(SunText::Key("sun-east"), &tr), "L");
        assert_eq!(label_text(SunText::Date(date), &tr), "05/03/2026");
        assert_eq!(label_text(SunText::Date(date), &|k: &str| k.to_string()), "2026-03-05");
        assert_eq!(time_text(367.4), "06:07");
        assert_eq!(time_text(1440.2), "00:00");
        assert_eq!(time_text(-30.0), "23:30");
        assert_eq!(time_text(f64::NAN), "00:00");
    }

    /// At the equator on the equinox the sun passes overhead, and its path
    /// starts and ends on the compass ring.
    #[test]
    fn the_equinox_path_at_the_equator_crosses_the_zenith() {
        let (mut project, geom) = sample();
        site(&mut project, 0.0, 0.0, 0.0, 0.0);
        let mut plan = view(&project, ViewCategory::FloorPlan);
        plan.sun = Some(still(2026, 3, 20, 720));
        let d = diagram(&project, &geom, &plan).unwrap();
        let paths = lines(&d, SunLineKind::Path);
        assert_eq!(paths.len(), 1);
        let path = &paths[0].pts;
        let c = d.center;
        let nearest = path.iter().map(|p| p.truncate().distance(c.truncate())).fold(f64::INFINITY, f64::min);
        assert!(nearest < 0.06 * d.radius, "{nearest} of {}", d.radius);
        for end in [path[0], path[path.len() - 1]] {
            assert!((end.distance(c) - d.radius).abs() < 1e-9 * d.radius, "{end}");
            assert!((end.z - c.z).abs() < 1e-9, "on the ground: {end}");
        }
        // Every point of the path on the dome.
        assert!(path.iter().all(|p| (p.distance(c) - d.radius).abs() < 1e-9 * d.radius && p.z >= c.z - 1e-9));
        // Rising in the east, setting in the west.
        assert!(path[0].x > c.x && path[path.len() - 1].x < c.x);
        // The ring: two circles in the ground plane, ticks and axes.
        let rings = lines(&d, SunLineKind::Ring);
        assert_eq!(rings.len(), 2);
        assert!(rings[0].pts.len() >= 72 && rings.iter().all(|r| r.closed));
        assert!(rings[1].pts.iter().all(|p| (p.distance(c) - 1.06 * d.radius).abs() < 1e-9 * d.radius));
        assert_eq!(lines(&d, SunLineKind::Tick).len(), 24);
        assert_eq!(lines(&d, SunLineKind::Axis).len(), 2);
        assert!(d.radius >= MIN_RADIUS && d.radius <= MAX_RADIUS);
    }

    /// In an elevation the path's highest point stands above the ring, seen edge on.
    #[test]
    fn an_elevation_shows_the_path_above_the_ring() {
        let (mut project, geom) = sample();
        site(&mut project, -15.8, -47.9, -3.0, 0.0);
        let mut elevation = view(&project, ViewCategory::Elevation);
        elevation.sun = Some(still(2026, 6, 21, 600));
        let d = diagram(&project, &geom, &elevation).unwrap();
        let frame = crate::view_frame(&elevation).unwrap();
        let top = lines(&d, SunLineKind::Path)[0].pts.iter().map(|p| frame.project(*p).y).fold(f64::MIN, f64::max);
        let ring = lines(&d, SunLineKind::Ring)[1].pts.iter().map(|p| frame.project(*p).y).fold(f64::MIN, f64::max);
        assert!(top > ring + 0.3 * d.radius, "{top} over {ring}");
        let drawing = generate(&project, &geom, &elevation).unwrap();
        assert!(drawing.overlay > 0);
        assert!(drawing.prims.iter().all(prim_finite));
        // Elevations fit everything drawn, the diagram included.
        let (lo, hi) = drawing.fit_bounds().unwrap();
        assert!(hi.y >= top - 1e-9 && lo.y <= frame.project(d.center).y);
        // A 3D view draws it too.
        let mut three_d = view(&project, ViewCategory::ThreeD);
        three_d.sun = elevation.sun.clone();
        let drawing = generate(&project, &geom, &three_d).unwrap();
        assert!(drawing.overlay > 0 && drawing.prims.iter().all(prim_finite));
    }

    /// Lighting draws no path; a polar night no path and no sun, the ring still.
    #[test]
    fn lighting_and_polar_nights() {
        let (mut project, geom) = sample();
        let mut plan = view(&project, ViewCategory::FloorPlan);
        plan.sun = Some(SunSettings { mode: SunMode::from_name("lighting").unwrap(), ..SunSettings::default() });
        let d = diagram(&project, &geom, &plan).unwrap();
        assert!(lines(&d, SunLineKind::Path).is_empty() && d.dots.is_empty());
        assert!(d.sun.is_some());
        assert!(d.labels.iter().all(|l| l.kind == SunLabelKind::Cardinal));

        site(&mut project, 80.0, 15.0, 1.0, 0.0);
        plan.sun = Some(still(2026, 12, 21, 720));
        let d = diagram(&project, &geom, &plan).unwrap();
        assert!(lines(&d, SunLineKind::Path).is_empty());
        assert!(d.sun.is_none() && d.dots.is_empty());
        assert_eq!(lines(&d, SunLineKind::Ring).len(), 2);
        assert!(label(&d, SunLabelKind::Sunrise).is_empty() && label(&d, SunLabelKind::Date).is_empty());
        // A polar day: the whole path above the horizon, no sunrise, a date still.
        plan.sun = Some(still(2026, 6, 21, 720));
        let d = diagram(&project, &geom, &plan).unwrap();
        assert_eq!(lines(&d, SunLineKind::Path).len(), 1);
        assert!(label(&d, SunLabelKind::Sunrise).is_empty() && label(&d, SunLabelKind::Sunset).is_empty());
        assert_eq!(label(&d, SunLabelKind::Date).len(), 1);
        assert_eq!(d.dots.len(), 24);
    }

    /// A study over several days adds the first and last days' paths.
    #[test]
    fn several_days_show_their_first_and_last_paths() {
        let (mut project, geom) = sample();
        site(&mut project, -23.5, -46.6, -3.0, 0.0);
        let mut plan = view(&project, ViewCategory::FloorPlan);
        let multi = SunMode::from_name("multi_day").unwrap();
        plan.sun = Some(SunSettings { mode: multi, frame: 5, ..SunSettings::default() });
        let d = diagram(&project, &geom, &plan).unwrap();
        assert_eq!(lines(&d, SunLineKind::PathRange).len(), 2);
        assert_eq!(lines(&d, SunLineKind::Path).len(), 1);
        // The first frame is on the first day: only the last day's path is added.
        if let Some(s) = plan.sun.as_mut() {
            s.frame = 0;
        }
        let d = diagram(&project, &geom, &plan).unwrap();
        assert_eq!(lines(&d, SunLineKind::PathRange).len(), 1);
    }

    /// Settings and a site made of bad numbers draw a finite diagram.
    #[test]
    fn bad_numbers_draw_finite_diagrams() {
        let (mut project, geom) = sample();
        site(&mut project, f64::NAN, f64::INFINITY, f64::NAN, f64::NAN);
        let modes = [
            SunMode::Lighting { azimuth: f64::NAN, altitude: f64::INFINITY, relative_to_view: true },
            SunMode::Lighting { azimuth: f64::NEG_INFINITY, altitude: f64::NAN, relative_to_view: false },
            SunMode::Still { date: SunDate { year: -4, month: 40, day: 99 }, time: u16::MAX },
            SunMode::SingleDay {
                date: SunDate { year: 99_999, month: 0, day: 0 },
                from: u16::MAX,
                to: 3,
                sunrise_to_sunset: true,
                interval: cc_core::sun::SunInterval::Month,
            },
            SunMode::MultiDay {
                from_date: SunDate { year: 9999, month: 12, day: 31 },
                to_date: SunDate { year: 1, month: 1, day: 1 },
                from: 2000,
                to: 0,
                sunrise_to_sunset: false,
                interval: cc_core::sun::SunInterval::Week,
            },
        ];
        for category in [ViewCategory::FloorPlan, ViewCategory::Elevation, ViewCategory::ThreeD] {
            let mut v = view(&project, category);
            v.scale = 0;
            if let ViewKind::ThreeD { camera, .. } = &mut v.kind {
                camera.yaw_deg = f64::NAN;
            }
            for mode in &modes {
                let ground = Some(cc_core::LevelId(9999));
                v.sun = Some(SunSettings { mode: mode.clone(), frame: u32::MAX, ground, ..SunSettings::default() });
                let d = diagram(&project, &geom, &v).unwrap();
                assert!(d.center.is_finite() && d.radius.is_finite() && d.sun_radius.is_finite());
                assert!(all_points(&d).iter().all(|p| p.is_finite()), "{mode:?}");
                assert!(d.labels.iter().all(|l| l.out.is_finite()));
                assert!(d.point_count() <= MAX_POINTS);
                let mut drawing = Drawing2D::default();
                draw(&mut drawing, &project, &geom, &v, &|k: &str| k.to_string());
                assert!(drawing.prims.iter().all(prim_finite), "{category:?} {mode:?}");
            }
        }
        // An empty model: around the origin, at the smallest radius.
        let empty = Project::from_template();
        let geom = ModelGeometry::build(&empty);
        let mut plan = view(&empty, ViewCategory::FloorPlan);
        plan.sun = Some(SunSettings::default());
        let d = diagram(&empty, &geom, &plan).unwrap();
        assert_eq!((d.center, d.radius), (DVec3::ZERO, MIN_RADIUS));
    }

    /// The diagram is not picked or snapped to (it is no element and not
    /// among the view's own primitives), not cropped, and fitted.
    #[test]
    fn the_diagram_is_not_picked_nor_cropped_but_fitted() {
        let (mut project, geom) = sample();
        site(&mut project, -15.8, -47.9, -3.0, 0.0);
        let mut plan = view(&project, ViewCategory::FloorPlan);
        plan.sun = Some(still(2026, 6, 21, 600));
        let a = diagram(&project, &geom, &plan).unwrap();
        let (c, r) = (a.center.truncate(), a.radius);
        let d = generate(&project, &geom, &plan).unwrap();
        // In the ring's band, on its north tick and on the east axis, nothing is picked.
        for p in [c + DVec2::new(0.0, 1.03 * r), c + DVec2::new(0.0, 1.08 * r), c + DVec2::new(1.12 * r, 0.0)] {
            assert_eq!(d.hit_test(p, 0.2), None, "{p}");
            assert_eq!(d.hit_test_line(p, 0.2), None, "{p}");
        }
        let without = {
            let mut v = plan.clone();
            v.sun = None;
            generate(&project, &geom, &v).unwrap()
        };
        let (lo, hi) = (c - DVec2::splat(2.0 * r), c + DVec2::splat(2.0 * r));
        assert_eq!(d.elements_in(lo, hi, true), without.elements_in(lo, hi, true));
        assert_eq!(d.own_prims(), without.own_prims(), "snapping sees only the view's own primitives");
        assert_eq!(d.overlay_prims().len(), d.overlay);
        // Zoom to fit a plan takes the diagram in (plans fit their elements and the overlay).
        let (flo, fhi) = d.fit_bounds().unwrap();
        assert!(fhi.x >= c.x + 1.14 * r - 1e-9 && flo.x <= c.x - 1.14 * r + 1e-9, "{flo} {fhi}");
        assert!(fhi.y >= c.y + 1.14 * r - 1e-9 && flo.y <= c.y - 1.14 * r + 1e-9, "{flo} {fhi}");
        // A crop around the house leaves the diagram whole.
        plan.crop = Some(cc_core::callout::CropBox::new(c - DVec2::splat(2.0), c + DVec2::splat(2.0)));
        let cropped = generate(&project, &geom, &plan).unwrap();
        assert_eq!(cropped.overlay_prims(), d.overlay_prims());
        let far = cropped
            .overlay_prims()
            .iter()
            .filter_map(|p| match p {
                Prim::Line { a, b, .. } => Some(a.distance(c).max(b.distance(c))),
                _ => None,
            })
            .fold(0.0, f64::max);
        assert!(far >= 1.14 * r - 1e-6, "{far}");
        // Exports take it: SVG lines in the compass's color, DXF on the annotation layer.
        let svg = crate::export::to_svg(&d, DVec2::new(420.0, 297.0));
        assert!(svg.contains("rgb(90,110,150)"));
        let layer = |p: &Prim| crate::dxf::default_layer(p).to_string();
        let dxf = crate::dxf::to_dxf(&d, crate::dxf::DxfScale::model(plan.scale), &layer);
        assert!(String::from_utf8_lossy(&dxf).contains("ANNOTATION"));
    }
}
