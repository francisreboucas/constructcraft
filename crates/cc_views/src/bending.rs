//! Bending schedules: a reinforcement schedule with one row per bar mark
//! (mark, diameter, grade, a sketch of the shape with its dimensions, the
//! length of one bar, the count, the total length and the weight; rows of
//! bars longer than the stock length noted) and a summary per diameter and
//! grade under it (total length and weight, with the waste added when the
//! schedule gives one).
//!
//! Every total is the exact sum of the bars it counts: a row adds its bars'
//! lengths and weights, the totals and the summary add the rows, so the
//! summary per diameter equals the sum of the individual bars.
//!
//! The shape of a mark is written as a code (CSV, the app's table):
//! `straight 500`, `L 30-470`, `U 30-470-30`, `Z …`, `bent …`,
//! `stirrup 14x44`, `ring R=50 L=314`, `closed …`, `arc R=220 L=345`, a
//! spiral as `spiral R=15 p=10 L=5546` (its radius, rise per turn and exact
//! length; sketched as a few turns of a zigzag), legs
//! of arcs as `(R=… L=…)`, and a loop closed by hooks as the loop's code with
//! each hook's angle and straight end (`stirrup 14x44 hook 135° 5 hook 135°
//! 5`), a tie as its length with its hooks (`tie 20 hook 135° 5 hook 180°
//! 5`); lengths as [`format::bar_length`] writes them.
//! Sheets draw a sketch instead ([`BarSketch`]): the bar's legs laid flat in
//! order, to proportion (very short straight legs drawn longer), normalized
//! into the cell, each leg labelled with its length.

use crate::schedule::{Cell, Row, Table};
use crate::{Anchor, Drawing2D, LineStyle, format};
use cc_core::{BendingOptions, Category, Element, ElementId, ElementKind, Project, ScheduleDef};
use cc_geom::ModelGeometry;
use cc_geom::bar_marks;
use cc_geom::bar_shape::{LegSummary, ShapeSummary};
use glam::DVec2;
use std::collections::BTreeMap;

/// Height of a row with a sketch, paper millimeters.
pub const SKETCH_ROW: f64 = 16.0;
/// Narrowest column of sketches, paper millimeters.
pub const SKETCH_WIDTH: f64 = 46.0;
/// Text height of the sketches' dimensions, paper millimeters.
pub const SKETCH_TEXT: f64 = 1.8;
/// A straight leg shorter than this share of the longest is drawn this long.
const SHORTEST_LEG: f64 = 0.15;
/// The pin a tie's hooks are drawn bent around, a share of the longest leg.
const TIE_PIN: f64 = 0.04;
/// At most this many legs of a shape are drawn.
const MAX_SKETCH_LEGS: usize = 256;

/// A bar's shape drawn for a schedule: one polyline per leg in the
/// sketch's own frame (meters, schematic) and the dimensions.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BarSketch {
    pub legs: Vec<Vec<DVec2>>,
    pub labels: Vec<SketchLabel>,
}

/// A dimension of a sketch: its text, the point of the leg it stands by and
/// the unit direction it stands off to.
#[derive(Debug, Clone, PartialEq)]
pub struct SketchLabel {
    pub text: String,
    pub at: DVec2,
    pub away: DVec2,
}

fn arc_text(l: &LegSummary) -> String {
    format!("R={} L={}", format::bar_length(l.radius.unwrap_or(0.0)), format::bar_length(l.length))
}

/// A coiled leg's radius, rise per turn and length.
fn spiral_text(l: &LegSummary) -> String {
    format!(
        "R={} p={} L={}",
        format::bar_length(l.radius.unwrap_or(0.0)),
        format::bar_length(l.pitch),
        format::bar_length(l.length)
    )
}

/// Turns of a coil drawn in a sketch, at most.
const SKETCH_TURNS: f64 = 6.0;

/// Each hook's angle and straight end: `hook 135° 5 hook 180° 5`.
fn hook_codes(hooks: &[LegSummary; 2]) -> String {
    hooks
        .iter()
        .map(|h| format!("hook {}° {}", format::trimmed(h.bend.abs().to_degrees(), 0), format::bar_length(h.length)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The code of a shape (see the module's documentation).
pub fn shape_code(shape: &ShapeSummary) -> String {
    // A loop closed by hooks: the loop's code, then each hook's angle and straight end.
    if let Some((closed, hooks)) = shape.hooked_loop() {
        return format!("{} {}", shape_code(&closed), hook_codes(&hooks));
    }
    // A tie: its bar's code (a straight one as `tie`), then each hook's angle and straight end.
    if let Some((bar, hooks)) = shape.hooked_ends() {
        let code = match bar.legs.as_slice() {
            [one] if one.radius.is_none() => format!("tie {}", format::bar_length(one.length)),
            _ => shape_code(&bar),
        };
        return format!("{code} {}", hook_codes(&hooks));
    }
    // The shorter end leg first.
    let (first, last) = (shape.legs.first().map_or(0.0, |l| l.length), shape.legs.last().map_or(0.0, |l| l.length));
    let s = if !shape.closed && first > last + 1e-9 { shape.reversed() } else { shape.clone() };
    let legs: Vec<&LegSummary> = s.legs.iter().collect();
    let straight = legs.iter().all(|l| l.radius.is_none());
    let lengths = |legs: &[&LegSummary]| {
        legs.iter()
            .map(|l| if l.radius.is_some() { format!("({})", arc_text(l)) } else { format::bar_length(l.length) })
            .collect::<Vec<_>>()
            .join("-")
    };
    let right_angle = |l: &LegSummary| (l.bend.abs() - std::f64::consts::FRAC_PI_2).abs() < 0.02;
    match (s.closed, legs.as_slice()) {
        (true, [ring]) if ring.radius.is_some() => format!("ring {}", arc_text(ring)),
        (true, [a, b, _, _]) if straight && legs.iter().all(|l| right_angle(l)) => {
            let (lo, hi) = if a.length <= b.length { (a.length, b.length) } else { (b.length, a.length) };
            format!("stirrup {}x{}", format::bar_length(lo), format::bar_length(hi))
        }
        (true, legs) => format!("closed {}", lengths(legs)),
        (false, []) => String::new(),
        (false, [one]) if one.radius.is_some() && one.pitch > 0.0 => format!("spiral {}", spiral_text(one)),
        (false, [one]) if one.radius.is_some() => format!("arc {}", arc_text(one)),
        (false, [one]) => format!("straight {}", format::bar_length(one.length)),
        (false, legs @ [_, _]) if straight => format!("L {}", lengths(legs)),
        (false, legs @ [a, b, _]) if straight => {
            let same_way = a.bend * b.bend > 0.0;
            format!("{} {}", if same_way { "U" } else { "Z" }, lengths(legs))
        }
        (false, legs) => format!("bent {}", lengths(legs)),
    }
}

/// The sketch of a shape (see [`BarSketch`]): its legs laid flat one after
/// another, each bend turning the way the bar turns, the longest straight
/// leg along x and the rest above it, short straight legs of open bars
/// drawn longer, each leg labelled with its length (arcs also with their
/// radius). A loop closed by hooks is drawn closed, its hooks bent into it
/// at its closing corner (drawn longer when short, and not labelled).
pub fn sketch_of(shape: &ShapeSummary) -> BarSketch {
    let longest = shape.legs.iter().filter(|l| l.radius.is_none()).map(|l| l.length).fold(0.0, f64::max);
    let hooked = shape.hooked_loop().is_some();
    // A tie's hooks are written in its code too.
    let tie = shape.hooked_ends().is_some();
    let least = if shape.closed { 0.0 } else { SHORTEST_LEG * longest };
    let mut at = DVec2::ZERO;
    let mut heading = 0.0_f64;
    let mut legs: Vec<Vec<DVec2>> = Vec::new();
    // A tie's hooks are bent around a small pin, so that a 180° hook stands
    // off the bar: the bend drawn with the hook leg, before or after it.
    let pin = TIE_PIN * longest;
    let mut bend_before: Vec<DVec2> = Vec::new();
    let all = &shape.legs[..shape.legs.len().min(MAX_SKETCH_LEGS)];
    for (i, leg) in all.iter().enumerate() {
        let dir = DVec2::from_angle(heading);
        // The legs of a hooked loop keep their lengths, so that it closes.
        let least = if hooked && !leg.hook { 0.0 } else { least };
        let points = match leg.radius.filter(|r| r.is_finite() && *r > 0.0 && leg.sweep != 0.0) {
            // A coil seen from the side: a zigzag across its diameter, a few turns of it.
            Some(radius) if leg.pitch > 0.0 => {
                let turns = (leg.sweep.abs() / std::f64::consts::TAU).ceil().clamp(1.0, SKETCH_TURNS) as usize;
                let step = dir * (radius * 0.75);
                let mut points = vec![at];
                for k in 0..2 * turns {
                    let side = if k % 2 == 0 { 1.0 } else { -1.0 };
                    points.push(at + step * (2 * k + 1) as f64 + dir.perp() * (radius * side));
                }
                points.push(at + step * (4 * turns) as f64);
                points
            }
            Some(radius) => {
                // Around the center on the side the arc turns to.
                let center = at + dir.perp() * (radius * leg.sweep.signum());
                let n = (leg.sweep.abs() / 10f64.to_radians()).ceil();
                let n = if n.is_finite() { (n as usize).clamp(2, 36) } else { 2 };
                let from = at - center;
                heading += leg.sweep;
                (0..=n).map(|i| center + DVec2::from_angle(leg.sweep * i as f64 / n as f64).rotate(from)).collect()
            }
            None => {
                let drawn = if leg.length > 1e-9 && leg.length < least { least } else { leg.length };
                vec![at, at + dir * drawn]
            }
        };
        let mut points = if bend_before.is_empty() {
            points
        } else {
            std::mem::take(&mut bend_before).into_iter().chain(points.into_iter().skip(1)).collect()
        };
        at = points.last().copied().unwrap_or(at);
        let next_hook = all.get(i + 1).is_some_and(|l| l.hook);
        if tie && (leg.hook || next_hook) && leg.bend != 0.0 && pin > 0.0 {
            let dir = DVec2::from_angle(heading);
            let center = at + dir.perp() * (pin * leg.bend.signum());
            let from = at - center;
            let n = ((leg.bend.abs() / 15f64.to_radians()).ceil() as usize).clamp(2, 12);
            let arc: Vec<DVec2> =
                (0..=n).map(|k| center + DVec2::from_angle(leg.bend * k as f64 / n as f64).rotate(from)).collect();
            at = arc.last().copied().unwrap_or(at);
            if leg.hook {
                points.extend(arc.into_iter().skip(1));
            } else {
                bend_before = arc;
            }
        }
        heading += leg.bend;
        legs.push(points);
    }
    // The longest straight leg along x.
    let base = shape
        .legs
        .iter()
        .enumerate()
        .filter(|(i, l)| l.radius.is_none() && !l.hook && *i < legs.len())
        .max_by(|a, b| a.1.length.total_cmp(&b.1.length))
        .map(|(i, _)| i);
    let ends = |legs: &[Vec<DVec2>], i: usize| legs.get(i).and_then(|ps| ps.first().copied().zip(ps.last().copied()));
    if let Some((a, b)) = base.and_then(|i| ends(&legs, i))
        && a.distance(b) > 1e-9
    {
        let turn = DVec2::from_angle(-(b - a).to_angle());
        legs.iter_mut().flatten().for_each(|p| *p = turn.rotate(*p));
    }
    let centroid_of = |legs: &[Vec<DVec2>]| {
        let count = legs.iter().map(Vec::len).sum::<usize>().max(1) as f64;
        legs.iter().flatten().fold(DVec2::ZERO, |a, p| a + *p) / count
    };
    // The rest of the bar above its longest leg.
    if let Some((a, _)) = base.and_then(|i| ends(&legs, i))
        && centroid_of(&legs).y < a.y - 1e-9
    {
        legs.iter_mut().flatten().for_each(|p| p.y = -p.y);
    }
    let centroid = centroid_of(&legs);
    let mut labels = Vec::new();
    for (leg, pts) in shape.legs.iter().zip(&legs) {
        // The hooks of a stirrup or a tie are written in its code, not dimensioned as legs.
        if (hooked || tie) && leg.hook {
            continue;
        }
        let (Some(&a), Some(&b)) = (pts.first(), pts.last()) else { continue };
        let (at, dir) = match leg.radius {
            Some(_) => {
                let mid = pts.len() / 2;
                let p = pts.get(mid).copied().unwrap_or(a);
                let q = pts.get(mid + 1).or(pts.get(mid.saturating_sub(1))).copied().unwrap_or(b);
                (p, q - p)
            }
            None => ((a + b) / 2.0, b - a),
        };
        let mut away = dir.perp().normalize_or(DVec2::Y);
        if away.dot(at - centroid) < -1e-9 || (away.dot(at - centroid).abs() <= 1e-9 && away.y < 0.0) {
            away = -away;
        }
        let text = match leg.radius {
            Some(_) if leg.pitch > 0.0 => spiral_text(leg),
            Some(r) => format!("{} R={}", format::bar_length(leg.length), format::bar_length(r)),
            None => format::bar_length(leg.length),
        };
        labels.push(SketchLabel { text, at, away });
    }
    BarSketch { legs, labels }
}

/// Draw `sketch` into the cell from `lo` to `hi` (paper millimeters, y up)
/// of a drawing at `k` meters per paper millimeter.
pub fn draw_sketch(d: &mut Drawing2D, sketch: &BarSketch, (lo, hi): (DVec2, DVec2), k: f64) {
    let t = SKETCH_TEXT;
    // Room around the bar for its dimensions (a gap under those below it).
    let (margin_x, margin_y) = (2.4 * t, 1.9 * t);
    let (inner_lo, inner_hi) = (lo + DVec2::new(margin_x, margin_y), hi - DVec2::new(margin_x, margin_y));
    let room = (inner_hi - inner_lo).max(DVec2::splat(1.0));
    let mut b_lo = DVec2::splat(f64::INFINITY);
    let mut b_hi = DVec2::splat(f64::NEG_INFINITY);
    for p in sketch.legs.iter().flatten() {
        b_lo = b_lo.min(*p);
        b_hi = b_hi.max(*p);
    }
    if !(b_lo.is_finite() && b_hi.is_finite()) {
        return;
    }
    let size = b_hi - b_lo;
    let scale = match (size.x > 1e-9, size.y > 1e-9) {
        (true, true) => (room.x / size.x).min(room.y / size.y),
        (true, false) => room.x / size.x,
        (false, true) => room.y / size.y,
        (false, false) => 1.0,
    };
    let middle = (lo + hi) / 2.0;
    let center = (b_lo + b_hi) / 2.0;
    let paper = |p: DVec2| middle + (p - center) * scale;
    for leg in &sketch.legs {
        let pts: Vec<DVec2> = leg.iter().map(|p| paper(*p) * k).collect();
        d.polyline(&pts, false, LineStyle::Projection, None);
    }
    for label in &sketch.labels {
        let at = paper(label.at) + label.away * (0.5 * t);
        let (pos, anchor) = if label.away.y.abs() >= label.away.x.abs() {
            let y = if label.away.y >= 0.0 { at.y } else { at.y - t };
            (DVec2::new(at.x, y), Anchor::CenterBottom)
        } else if label.away.x > 0.0 {
            (DVec2::new(at.x, at.y - t / 2.0), Anchor::LeftBottom)
        } else {
            (DVec2::new(at.x, at.y - t / 2.0), Anchor::RightBottom)
        };
        d.text_at(pos * k, label.text.clone(), t, anchor, 0.0);
    }
}

/// A rebar diameter as schedules write it (`Ø 12,5 mm`, `Ø 1/2"`).
pub fn diameter_text(diameter: f64) -> String {
    format!("Ø {}", format::bar_size(diameter))
}

/// One row of a bending schedule: the bars of one mark (in one partition
/// when grouped), with exact sums.
#[derive(Debug, Clone, PartialEq)]
pub struct BendingRow {
    /// The partition heading the row when grouped (else empty).
    pub partition: String,
    /// Index of the mark in the resolved [`bar_marks::BarMarks`].
    pub mark: usize,
    pub count: usize,
    /// The bars' lengths added up, meters.
    pub length: f64,
    /// Their weight, kilograms.
    pub weight: f64,
    /// The longest bar, meters.
    pub longest: f64,
    /// The first set with bars of the row.
    pub first: ElementId,
}

/// The total of one diameter and grade: the rows of it added up.
#[derive(Debug, Clone, PartialEq)]
pub struct DiameterTotal {
    pub diameter: f64,
    pub grade: String,
    pub length: f64,
    pub weight: f64,
}

/// A bending schedule's numbers: the marks, the rows in order and the
/// totals per diameter and grade (diameters ascending).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bending {
    pub marks: bar_marks::BarMarks,
    pub rows: Vec<BendingRow>,
    pub totals: Vec<DiameterTotal>,
}

/// The rows and totals of a bending schedule of the reinforcement sets
/// `keep` accepts (those a view's phase and options show) and `options`
/// lets through. Marks are resolved over the whole model, so they read the
/// same in every schedule. Each row adds its bars in set and bar order; the
/// totals add the rows in order.
pub fn bending(
    project: &Project,
    geom: &ModelGeometry,
    options: &BendingOptions,
    keep: &mut dyn FnMut(&Element) -> bool,
) -> Bending {
    let marks = bar_marks::resolve(project, &geom.rebar);
    // Per row (grouped without a partition last, partition, mark): the row.
    let mut rows: BTreeMap<(bool, String, usize), BendingRow> = BTreeMap::new();
    for e in project.elements_of(Category::Rebar) {
        let partition = match &e.kind {
            ElementKind::Rebar(r) => r.partition.as_str(),
            ElementKind::ShapedRebar(r) => r.partition.as_str(),
            _ => continue,
        };
        if !options.shows(partition) || !keep(e) {
            continue;
        }
        let (Some(set), Some(indices)) = (geom.rebar.get(&e.id), marks.bars.get(&e.id)) else { continue };
        let group = if options.group_by_partition { partition.trim().to_string() } else { String::new() };
        let mass = set.mass_per_meter();
        for (bar, m) in set.bars.iter().zip(indices) {
            let key = (group.is_empty() && options.group_by_partition, group.clone(), *m);
            let row = rows.entry(key).or_insert_with(|| BendingRow {
                partition: group.clone(),
                mark: *m,
                count: 0,
                length: 0.0,
                weight: 0.0,
                longest: 0.0,
                first: e.id,
            });
            let length = bar.length();
            row.count += 1;
            row.length += length;
            row.weight += length * mass;
            row.longest = row.longest.max(length);
        }
    }
    let rows: Vec<BendingRow> = rows.into_values().collect();
    let mut totals: BTreeMap<(i64, String), DiameterTotal> = BTreeMap::new();
    for row in &rows {
        let Some(mark) = marks.marks.get(row.mark) else { continue };
        let key = ((mark.diameter * 1e6).round() as i64, mark.grade.clone());
        let t = totals.entry(key).or_insert_with(|| DiameterTotal {
            diameter: mark.diameter,
            grade: mark.grade.clone(),
            length: 0.0,
            weight: 0.0,
        });
        t.length += row.length;
        t.weight += row.weight;
    }
    Bending { marks, rows, totals: totals.into_values().collect() }
}

fn weight_text(w: f64) -> String {
    format!("{} kg", format::number(w, 2))
}

/// The bending schedule as a table (see [`bending`]): a row per mark (under
/// a heading per partition when grouped), the totals, and the summary per
/// diameter under it.
pub fn table(
    project: &Project,
    geom: &ModelGeometry,
    def: &ScheduleDef,
    options: &BendingOptions,
    keep: &mut dyn FnMut(&Element) -> bool,
) -> Table {
    let b = bending(project, geom, options, keep);
    let stock = project.bar_marking.stock_length;
    let over = b.rows.iter().any(|r| r.longest > stock + 1e-9);
    let mut headers = vec![
        "col-mark",
        "col-diameter",
        "col-grade",
        "col-shape",
        if format::imperial() { "col-bar-length" } else { "col-unit-length-cm" },
        "col-bar-count",
        "col-length",
        "col-weight",
    ];
    if over {
        headers.push("col-note");
    }
    let columns = headers.len();
    let mut out_rows: Vec<Row> = Vec::new();
    let (mut count, mut length, mut weight) = (0usize, 0.0, 0.0);
    let mut group: Option<&str> = None;
    for row in &b.rows {
        let Some(mark) = b.marks.marks.get(row.mark) else { continue };
        if options.group_by_partition && group != Some(row.partition.as_str()) {
            group = Some(row.partition.as_str());
            let mut cells = vec![Cell::Text(String::new()); columns];
            if let Some(c) = cells.first_mut() {
                *c = if row.partition.is_empty() {
                    Cell::Key("bending-no-partition")
                } else {
                    Cell::Text(row.partition.clone())
                };
            }
            out_rows.push(Row { heading: true, ..Row::new(row.first, cells) });
        }
        let sketch = Some((3, sketch_of(&mark.shape)));
        let unit = if row.count == 0 { 0.0 } else { row.length / row.count as f64 };
        let mut cells = vec![
            Cell::Text(mark.mark.clone()),
            Cell::Text(diameter_text(mark.diameter)),
            Cell::Text(mark.grade.clone()),
            Cell::Text(shape_code(&mark.shape)),
            Cell::Text(format::bar_length(unit)),
            Cell::Text(row.count.to_string()),
            Cell::Text(format::meters(row.length)),
            Cell::Text(weight_text(row.weight)),
        ];
        if over {
            cells.push(if row.longest > stock + 1e-9 {
                Cell::Key("bending-over-stock")
            } else {
                Cell::Text(String::new())
            });
        }
        out_rows.push(Row { sketch, ..Row::new(row.first, cells) });
        (count, length, weight) = (count + row.count, length + row.length, weight + row.weight);
    }
    let totals = if def.hide_totals {
        Vec::new()
    } else {
        let mut t = vec![String::new(); columns];
        let marks_shown = b.rows.len();
        for (i, v) in [
            (0, marks_shown.to_string()),
            (5, count.to_string()),
            (6, format::meters(length)),
            (7, weight_text(weight)),
        ] {
            if let Some(c) = t.get_mut(i) {
                *c = v;
            }
        }
        t
    };
    Table {
        headers,
        rows: out_rows,
        totals,
        summary: Some(Box::new(summary_table(&b.totals, options.waste_percent))),
        caption: None,
        ..Default::default()
    }
}

/// The summary per diameter and grade: total length and weight (with the
/// waste, when given), and the totals.
fn summary_table(rows: &[DiameterTotal], waste: Option<f64>) -> Table {
    let mut headers = vec!["col-diameter", "col-grade", "col-length", "col-weight"];
    if waste.is_some() {
        headers.extend(["col-waste-percent", "col-weight-with-waste"]);
    }
    let with_waste = |w: f64| waste.map(|p| w * (1.0 + p / 100.0));
    let (mut length, mut weight) = (0.0, 0.0);
    let mut out = Vec::new();
    for t in rows {
        let mut cells = vec![
            Cell::Text(diameter_text(t.diameter)),
            Cell::Text(t.grade.clone()),
            Cell::Text(format::meters(t.length)),
            Cell::Text(weight_text(t.weight)),
        ];
        if let (Some(p), Some(ww)) = (waste, with_waste(t.weight)) {
            cells.push(Cell::Text(format::trimmed(p, 1)));
            cells.push(Cell::Text(weight_text(ww)));
        }
        out.push(Row::new(ElementId(0), cells));
        (length, weight) = (length + t.length, weight + t.weight);
    }
    let mut totals = vec![out.len().to_string(), String::new(), format::meters(length), weight_text(weight)];
    if let (Some(p), Some(ww)) = (waste, with_waste(weight)) {
        totals.push(format::trimmed(p, 1));
        totals.push(weight_text(ww));
    }
    Table { headers, rows: out, totals, caption: Some("bending-summary"), ..Default::default() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_geom::bar_shape::summary;
    use cc_geom::rebar::Bar;
    use glam::{DVec2, DVec3};

    fn code(points: &[[f64; 3]], closed: bool) -> String {
        format::set_units(&cc_core::Units::default());
        shape_code(&summary(&Bar::new(points.iter().map(|p| DVec3::from_array(*p)).collect(), closed)))
    }

    #[test]
    fn shapes_have_codes() {
        assert_eq!(code(&[[0.0, 0.0, 0.0], [5.0, 0.0, 0.0]], false), "straight 500");
        // The shorter end leg first, whichever end the bar was drawn from.
        assert_eq!(code(&[[0.0, 0.0, 0.3], [0.0, 0.0, 0.0], [4.7, 0.0, 0.0]], false), "L 30-470");
        assert_eq!(code(&[[4.7, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.3]], false), "L 30-470");
        let u = [[0.0, 0.0, 0.3], [0.0, 0.0, 0.0], [4.7, 0.0, 0.0], [4.7, 0.0, 0.3]];
        assert_eq!(code(&u, false), "U 30-470-30");
        let z = [[0.0, 0.0, 0.3], [0.0, 0.0, 0.0], [4.7, 0.0, 0.0], [4.7, 0.0, -0.3]];
        assert_eq!(code(&z, false), "Z 30-470-30");
        let stirrup = [[0.0, 0.0, 0.0], [0.14, 0.0, 0.0], [0.14, 0.0, 0.44], [0.0, 0.0, 0.44]];
        assert_eq!(code(&stirrup, true), "stirrup 14x44");
        assert_eq!(code(&[], false), "");
    }

    /// A 5 mm stirrup closed by two hooks: 135 degrees, straight ends of 10
    /// diameters (5 cm), or 90 degrees.
    fn hooked(angle_deg: f64) -> ShapeSummary {
        let points = [[0.0, 0.0, 0.0], [0.14, 0.0, 0.0], [0.14, 0.0, 0.44], [0.0, 0.0, 0.44]];
        let hooks = cc_geom::rebar::LoopHooks { angle: angle_deg.to_radians(), radius: 0.01, tail: 0.05 };
        let bar = Bar { hooks: Some(hooks), ..Bar::new(points.iter().map(|p| DVec3::from_array(*p)).collect(), true) };
        summary(&bar)
    }

    #[test]
    fn hooked_stirrups_have_codes_and_sketches() {
        format::set_units(&cc_core::Units::default());
        assert_eq!(shape_code(&hooked(135.0)), "stirrup 14x44 hook 135° 5 hook 135° 5");
        assert_eq!(shape_code(&hooked(90.0)), "stirrup 14x44 hook 90° 5 hook 90° 5");
        // The canonical reading the marks keep reads the same.
        assert_eq!(shape_code(&cc_geom::bar_marks::canonical(&hooked(135.0))), "stirrup 14x44 hook 135° 5 hook 135° 5");
        // The sketch: the loop closed, two hooks at its closing corner, only the sides labelled.
        let s = sketch_of(&hooked(135.0));
        assert_eq!(s.legs.len(), 6);
        let texts: Vec<&str> = s.labels.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["14", "44", "14", "44"]);
        let corner = s.legs[1][0];
        assert!(
            s.legs[4][1].distance(corner) < 1e-9
                && s.legs[0][1].distance(corner) < 1e-9
                && s.legs[5][0].distance(corner) < 1e-9
        );
        // Each hook points into the loop.
        let (lo, hi) = s.legs[1..5]
            .iter()
            .flatten()
            .fold((DVec2::INFINITY, DVec2::NEG_INFINITY), |(a, b), p| (a.min(*p), b.max(*p)));
        let inside = |p: DVec2| p.x > lo.x - 1e-9 && p.x < hi.x + 1e-9 && p.y > lo.y - 1e-9 && p.y < hi.y + 1e-9;
        assert!(inside(s.legs[0][0]) && inside(s.legs[5][1]), "{:?}", s.legs);
    }

    /// A 5 mm tie 20 cm between the bars it holds, hooked by 135° and 180°
    /// (an S when `opposite`).
    fn tie(opposite: bool) -> ShapeSummary {
        use std::f64::consts::PI;
        let ends = cc_geom::rebar::EndHooks::of_tie(0.75 * PI, PI, 0.005, opposite);
        let points = vec![DVec3::ZERO, DVec3::new(0.0, 0.2, 0.0)];
        summary(&Bar { true_length: Some(0.2 + ends.length()), end_hooks: Some(ends), ..Bar::new(points, false) })
    }

    #[test]
    fn ties_have_codes_and_sketches() {
        format::set_units(&cc_core::Units::default());
        // Not `straight 32`, the length of the bar with its hooks.
        assert_eq!(shape_code(&tie(false)), "tie 20 hook 135° 5 hook 180° 5");
        assert_eq!(shape_code(&tie(true)), "tie 20 hook 135° 5 hook 180° 5");
        assert_eq!(shape_code(&cc_geom::bar_marks::canonical(&tie(true))).split(' ').next(), Some("tie"));
        // The sketch: the bar and its two hooks, only the bar labelled.
        for opposite in [false, true] {
            let s = sketch_of(&tie(opposite));
            assert_eq!(s.legs.len(), 3);
            let texts: Vec<&str> = s.labels.iter().map(|l| l.text.as_str()).collect();
            assert_eq!(texts, ["20"]);
            // A C has both hooks on one side of the bar, an S one on each.
            let bar_y = s.legs[1][0].y;
            let (a, b) = (s.legs[0][0].y - bar_y, s.legs[2][1].y - bar_y);
            assert!(a.abs() > 1e-9 && b.abs() > 1e-9);
            assert_eq!(a * b < 0.0, opposite, "{:?}", s.legs);
        }
    }

    #[test]
    fn sketches_label_each_leg_and_lie_flat() {
        format::set_units(&cc_core::Units::default());
        let shape = |points: &[[f64; 3]], closed: bool| {
            summary(&Bar::new(points.iter().map(|p| DVec3::from_array(*p)).collect(), closed))
        };
        // A vertical L in a wall: drawn flat, the long leg along x, the short one up.
        let s = sketch_of(&shape(&[[1.0, 2.0, 0.3], [1.0, 2.0, 0.0], [1.0, 6.7, 0.0]], false));
        assert_eq!(s.legs.len(), 2);
        let texts: Vec<&str> = s.labels.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["30", "470"]);
        let (short, long) = (&s.legs[0], &s.legs[1]);
        assert!((long[0].y - long[1].y).abs() < 1e-9, "{long:?}");
        assert!(short[0].y > short[1].y && short.iter().all(|p| p.y >= long[0].y - 1e-9), "{short:?}");
        // The short leg is drawn longer than to scale.
        assert!(short[0].distance(short[1]) > 0.3 + 1e-6);
        // Drawn into a cell: lines and the two texts inside it.
        let mut d = Drawing2D::default();
        draw_sketch(&mut d, &s, (DVec2::new(0.0, -16.0), DVec2::new(46.0, 0.0)), 1.0);
        let (lo, hi) = d.bounds().unwrap();
        assert!(lo.x >= -1e-9 && hi.x <= 46.0 + 1e-9 && lo.y >= -16.0 - 1e-9 && hi.y <= 1e-9, "{lo} {hi}");
        let n = d.prims.iter().filter(|p| matches!(p, crate::Prim::Text { .. })).count();
        assert_eq!(n, 2);
        // A stirrup closes on itself: its last corner is back at its first.
        let stirrup =
            sketch_of(&shape(&[[0.0, 0.0, 0.0], [0.14, 0.0, 0.0], [0.14, 0.0, 0.44], [0.0, 0.0, 0.44]], true));
        assert_eq!(stirrup.legs.len(), 4);
        assert!(stirrup.legs[3][1].distance(stirrup.legs[0][0]) < 1e-9, "{:?}", stirrup.legs);
        // A ring is a closed loop labelled with its length and radius.
        let ring: Vec<[f64; 3]> = (0..120)
            .map(|i| (DVec2::from_angle(std::f64::consts::TAU * i as f64 / 120.0) * 0.5).extend(0.0).to_array())
            .collect();
        let s = sketch_of(&shape(&ring, true));
        assert_eq!(s.labels.len(), 1);
        assert!(s.labels[0].text.ends_with("R=50"), "{}", s.labels[0].text);
        let (first, last) = (s.legs[0][0], *s.legs[0].last().unwrap());
        assert!(first.distance(last) < 1e-9);
        // Nothing to draw: nothing drawn.
        let mut d = Drawing2D::default();
        draw_sketch(&mut d, &sketch_of(&ShapeSummary::default()), (DVec2::ZERO, DVec2::ONE), 1.0);
        assert!(d.prims.is_empty());
    }
}
