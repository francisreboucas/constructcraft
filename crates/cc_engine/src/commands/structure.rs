//! Structural layout from grids: columns at grid intersections and beams
//! between the columns along grid lines or in a given order.

use super::create::top;
use super::{add_element, edit, ids_json};
use crate::defaults::default_top;
use crate::error::{EngineError, Result};
use crate::refs::{self, Ref, finite, positive};
use crate::registry::{Command, CommandKind};
use crate::session::Session;
use crate::t;
use cc_core::{Beam, Category, Column, ElementId, ElementKind, LevelId, Project};
use glam::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Points closer than this are the same place (1 cm).
pub(crate) const SAME: f64 = 0.01;

/// The grids named by `ids` with their lines, or every grid when none are given.
pub(crate) fn grids_with_ids(p: &Project, ids: &[u64]) -> Result<Vec<(ElementId, (DVec2, DVec2))>> {
    let line = |id: ElementId| match p.element(id).map(|e| &e.kind) {
        Some(ElementKind::Grid(g)) => Ok((id, (g.start, g.end))),
        _ => Err(EngineError::invalid(format!("element {} is not a grid", id.0))),
    };
    if ids.is_empty() {
        return Ok(p
            .elements
            .values()
            .filter_map(|e| match &e.kind {
                ElementKind::Grid(g) => Some((e.id, (g.start, g.end))),
                _ => None,
            })
            .collect());
    }
    ids.iter().map(|&id| refs::element(p, id).and_then(line)).collect()
}

/// The grid lines named by `ids`, or every grid when none are given.
fn grid_lines(p: &Project, ids: &[u64]) -> Result<Vec<(DVec2, DVec2)>> {
    Ok(grids_with_ids(p, ids)?.into_iter().map(|(_, l)| l).collect())
}

/// Where segments `a` and `b` cross (ends included), if they are not parallel.
pub(crate) fn crossing((a0, a1): (DVec2, DVec2), (b0, b1): (DVec2, DVec2)) -> Option<DVec2> {
    let (u, v) = (a1 - a0, b1 - b0);
    let den = u.perp_dot(v);
    if den.abs() < 1e-9 * u.length() * v.length() {
        return None;
    }
    let t = (b0 - a0).perp_dot(v) / den;
    let s = (b0 - a0).perp_dot(u) / den;
    let eps = |len: f64| 1e-6 / len.max(1e-9);
    let inside = |x: f64, len: f64| (-eps(len)..=1.0 + eps(len)).contains(&x);
    (inside(t, u.length()) && inside(s, v.length())).then(|| a0 + u * t)
}

/// Every intersection of the lines, each place once.
pub(crate) fn intersections(lines: &[(DVec2, DVec2)]) -> Vec<DVec2> {
    let mut points: Vec<DVec2> = Vec::new();
    for (i, a) in lines.iter().enumerate() {
        for b in &lines[i + 1..] {
            if let Some(p) = crossing(*a, *b)
                && p.is_finite()
                && !points.iter().any(|q| q.distance(p) < SAME)
            {
                points.push(p);
            }
        }
    }
    points
}

/// Columns of `level` with their centers.
pub(crate) fn columns_on(p: &Project, level: LevelId) -> Vec<(ElementId, DVec2)> {
    p.elements
        .values()
        .filter(|e| e.level == level)
        .filter_map(|e| match &e.kind {
            ElementKind::Column(c) => Some((e.id, c.position)),
            _ => None,
        })
        .collect()
}

/// Whether a beam of `level` already runs between `a` and `b` (either way).
pub(crate) fn beam_exists(p: &Project, level: LevelId, a: DVec2, b: DVec2) -> bool {
    p.elements.values().filter(|e| e.level == level).any(|e| match &e.kind {
        ElementKind::Beam(x) => {
            (x.start.distance(a) < SAME && x.end.distance(b) < SAME)
                || (x.start.distance(b) < SAME && x.end.distance(a) < SAME)
        }
        _ => false,
    })
}

/// The location elevation of new beams: `top_offset`, else the level's height.
fn beam_top(p: &Project, level: LevelId, top_offset: Option<f64>) -> Result<f64> {
    finite(top_offset.unwrap_or_else(|| p.level_height(level, p.defaults().story)), "top_offset")
}

/// Create beams along `spans` (skipping those already modeled) in one undo step.
fn create_beams(
    s: &mut Session,
    level: LevelId,
    type_ref: Option<&Ref>,
    top_offset: f64,
    spans: Vec<(DVec2, DVec2)>,
) -> Result<Value> {
    let type_id = refs::type_of(s.project(), Category::Beam, type_ref)?;
    let mut skipped = 0;
    let mut todo = Vec::new();
    for (a, b) in spans {
        let repeated = todo.iter().any(|&(x, y): &(DVec2, DVec2)| {
            (x.distance(a) < SAME && y.distance(b) < SAME) || (x.distance(b) < SAME && y.distance(a) < SAME)
        });
        if a.distance(b) < SAME || repeated || beam_exists(s.project(), level, a, b) {
            skipped += 1;
        } else {
            todo.push((a, b));
        }
    }
    if todo.is_empty() {
        return Ok(json!({"created": [], "skipped": skipped}));
    }
    let created = edit(s, t!("tx-create-beam"), |e| {
        Ok(todo
            .iter()
            .map(|&(a, b)| add_element(e, level, type_id, ElementKind::Beam(Beam::new(a, b, top_offset))))
            .collect::<Vec<_>>())
    })?;
    Ok(json!({"created": ids_json(&created), "skipped": skipped}))
}

/// Place a column at every intersection of grid lines on a level, skipping
/// places that already have a column. Returns the new ids (`created`) and how
/// many intersections were skipped.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ColumnsAtGrids {
    /// Grid ids (default: every grid).
    #[serde(default)]
    pub grids: Vec<u64>,
    /// Level id or name (default: the lowest level).
    #[serde(default)]
    pub level: Option<Ref>,
    /// Column type id or name (default: the category's default type).
    #[serde(default, rename = "type")]
    pub type_ref: Option<Ref>,
    /// Rotation in plan, degrees.
    #[serde(default)]
    pub rotation: Option<f64>,
    /// Unconnected height (default: the level's height).
    #[serde(default)]
    pub height: Option<f64>,
    /// Level the tops are attached to (default: the level above).
    #[serde(default)]
    pub top_level: Option<Ref>,
    /// Offset of the tops from `top_level`, meters.
    #[serde(default)]
    pub top_offset: Option<f64>,
    /// Load-bearing (default true).
    #[serde(default)]
    pub structural: Option<bool>,
}

impl Command for ColumnsAtGrids {
    const NAME: &'static str = "columns_at_grids";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let level = refs::level_or_first(p, self.level.as_ref())?;
        let type_id = refs::type_of(p, Category::Column, self.type_ref.as_ref())?;
        let lines = grid_lines(p, &self.grids)?;
        if lines.len() < 2 {
            return Err(EngineError::invalid(t!("msg-grids-two")));
        }
        let column = |position| -> Result<Column> {
            Ok(Column {
                position,
                rotation_deg: finite(self.rotation.unwrap_or(0.0), "rotation")?,
                base_offset: 0.0,
                height: positive(self.height.unwrap_or_else(|| p.level_height(level, p.defaults().story)), "height")?,
                top: top(p, self.top_level.as_ref(), self.top_offset)?.unwrap_or_else(|| default_top(p, level)),
                structural: self.structural.unwrap_or(true),
                base_support: Default::default(),
            })
        };
        let existing = columns_on(p, level);
        let points = intersections(&lines);
        let (taken, free): (Vec<DVec2>, Vec<DVec2>) =
            points.into_iter().partition(|q| existing.iter().any(|(_, c)| c.distance(*q) < SAME));
        let columns: Vec<Column> = free.into_iter().map(column).collect::<Result<_>>()?;
        if columns.is_empty() {
            return Ok(json!({"created": [], "skipped": taken.len()}));
        }
        let created = edit(s, t!("tx-create-column"), |e| {
            Ok(columns.into_iter().map(|c| add_element(e, level, type_id, ElementKind::Column(c))).collect::<Vec<_>>())
        })?;
        Ok(json!({"created": ids_json(&created), "skipped": taken.len()}))
    }
}

/// Place beams along grid lines: on each grid, a beam between each pair of
/// neighboring columns of the level whose centers lie on it. Spans that
/// already have a beam are skipped.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct BeamsOnGrids {
    /// Grid ids (default: every grid).
    #[serde(default)]
    pub grids: Vec<u64>,
    /// Level id or name (default: the lowest level).
    #[serde(default)]
    pub level: Option<Ref>,
    /// Beam type id or name (default: the category's default type).
    #[serde(default, rename = "type")]
    pub type_ref: Option<Ref>,
    /// Top of the beams above the level, meters (default: the level height).
    #[serde(default)]
    pub top_offset: Option<f64>,
}

impl Command for BeamsOnGrids {
    const NAME: &'static str = "beams_on_grids";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let level = refs::level_or_first(p, self.level.as_ref())?;
        let top_offset = beam_top(p, level, self.top_offset)?;
        let columns = columns_on(p, level);
        let mut spans = Vec::new();
        for (a, b) in grid_lines(p, &self.grids)? {
            let Some(u) = (b - a).try_normalize() else { continue };
            let len = a.distance(b);
            let mut on: Vec<(f64, DVec2)> = columns
                .iter()
                .map(|&(_, c)| ((c - a).dot(u), c))
                .filter(|&(t, c)| (c - a).perp_dot(u).abs() < SAME && t > -SAME && t < len + SAME)
                .collect();
            on.sort_by(|x, y| x.0.total_cmp(&y.0));
            on.dedup_by(|x, y| x.1.distance(y.1) < SAME);
            spans.extend(on.windows(2).map(|w| (w[0].1, w[1].1)));
        }
        create_beams(s, level, self.type_ref.as_ref(), top_offset, spans)
    }
}

/// Place beams from column to column in the order given (a chain), on the
/// level of the first column. Spans that already have a beam are skipped.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct BeamsBetweenColumns {
    /// Column ids, at least two, in the order to connect.
    pub columns: Vec<u64>,
    /// Beam type id or name (default: the category's default type).
    #[serde(default, rename = "type")]
    pub type_ref: Option<Ref>,
    /// Top of the beams above the level, meters (default: the level height).
    #[serde(default)]
    pub top_offset: Option<f64>,
}

impl Command for BeamsBetweenColumns {
    const NAME: &'static str = "beams_between_columns";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let mut centers = Vec::new();
        let mut level = None;
        for &id in &self.columns {
            let e = p.element(refs::element(p, id)?);
            match e.map(|e| (e.level, &e.kind)) {
                Some((l, ElementKind::Column(c))) => {
                    level.get_or_insert(l);
                    centers.push(c.position);
                }
                _ => return Err(EngineError::invalid(format!("element {id} is not a column"))),
            }
        }
        let (Some(level), true) = (level, centers.len() >= 2) else {
            return Err(EngineError::invalid(t!("msg-columns-two")));
        };
        let top_offset = beam_top(p, level, self.top_offset)?;
        let spans = centers.windows(2).map(|w| (w[0], w[1])).collect();
        create_beams(s, level, self.type_ref.as_ref(), top_offset, spans)
    }
}

/// Join each slab with the structure that carries it: beams, structural
/// columns and structural walls whose bodies overlap it (default: every
/// floor, or every floor of `level`). Beams and columns cut the slab, the
/// slab cuts walls, and the shared volume counts once. Pairs already joined
/// are left as they are.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct SlabJoinSupports {
    /// Floor ids (default: every floor, or those of `level`).
    #[serde(default)]
    pub floors: Vec<u64>,
    /// Level id or name, when no floors are given.
    #[serde(default)]
    pub level: Option<Ref>,
}

impl Command for SlabJoinSupports {
    const NAME: &'static str = "slab_join_supports";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let floors: Vec<ElementId> = if self.floors.is_empty() {
            let level = self.level.as_ref().map(|r| refs::level(p, r)).transpose()?;
            p.elements
                .values()
                .filter(|e| e.category() == Category::Floor && level.is_none_or(|l| e.level == l))
                .map(|e| e.id)
                .collect()
        } else {
            let ids = refs::elements(p, &self.floors)?;
            if let Some(id) = ids.iter().find(|id| p.element(**id).is_none_or(|e| e.category() != Category::Floor)) {
                return Err(EngineError::invalid(format!("element {} is not a floor", id.0)));
            }
            ids
        };
        let supports: Vec<ElementId> = p
            .elements
            .values()
            .filter(|e| match &e.kind {
                ElementKind::Beam(_) => true,
                ElementKind::Column(c) => c.structural,
                ElementKind::Wall(w) => w.structural,
                _ => false,
            })
            .map(|e| e.id)
            .collect();
        let pairs: Vec<(ElementId, ElementId)> = floors
            .iter()
            .flat_map(|f| supports.iter().map(move |x| (*f, *x)))
            .filter(|(f, x)| p.join_of(*f, *x).is_none())
            .collect();
        let geom = s.geometry();
        let meeting: Vec<(ElementId, ElementId)> =
            pairs.into_iter().filter(|(f, x)| geom.shared_volume(*f, *x) > 1e-9).collect();
        if meeting.is_empty() {
            return Ok(json!({"joined": []}));
        }
        let p = s.project();
        let priority = |id| p.element(id).map_or(0, cc_core::join_priority);
        let ordered: Vec<(ElementId, ElementId)> =
            meeting.into_iter().map(|(f, x)| if priority(x) > priority(f) { (x, f) } else { (f, x) }).collect();
        edit(s, t!("tx-join-geometry"), |e| {
            for &(cutting, cut) in &ordered {
                e.set_joined(cutting, cut, true);
            }
            Ok(())
        })?;
        Ok(json!({"joined": ordered.iter().map(|(a, b)| json!({"cutting": a.0, "cut": b.0})).collect::<Vec<_>>()}))
    }
}

/// Number the marks of beams, columns and pad footings (default: all three)
/// in reading order: horizontal beams top to bottom then left to right,
/// the other beams left to right then top to bottom, columns and footings
/// top to bottom then left to right. Marks are a prefix per category (`V`,
/// `P`, `S` in Portuguese; `B`, `C`, `F` in English) and a number.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct StructureNumber {
    /// Categories to number: `beam`, `column` and/or `foundation` (default: all three).
    #[serde(default)]
    pub categories: Vec<String>,
    /// Level id or name (default: every level, numbered as one sequence per category).
    #[serde(default)]
    pub level: Option<Ref>,
}

impl Command for StructureNumber {
    const NAME: &'static str = "structure_number";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let level = self.level.as_ref().map(|r| refs::level(p, r)).transpose()?;
        let categories: Vec<Category> = if self.categories.is_empty() {
            vec![Category::Beam, Category::Column, Category::Foundation]
        } else {
            self.categories.iter().map(|c| refs::category(c)).collect::<Result<_>>()?
        };
        if let Some(c) =
            categories.iter().find(|c| !matches!(c, Category::Beam | Category::Column | Category::Foundation))
        {
            return Err(EngineError::invalid(format!("{} cannot be numbered here", refs::category_name(*c))));
        }
        // Rows round to 10 cm so elements on one line share it.
        let row = |y: f64| -(y * 10.0).round();
        let mut marks: Vec<(ElementId, String)> = Vec::new();
        for category in categories {
            let mut keyed: Vec<(i64, i64, f64, ElementId)> = p
                .elements
                .values()
                .filter(|e| e.category() == category && level.is_none_or(|l| e.level == l))
                .filter_map(|e| {
                    let (horizontal, at) = match &e.kind {
                        ElementKind::Beam(b) => {
                            let d = b.end - b.start;
                            (d.x.abs() >= d.y.abs(), (b.start + b.end) * 0.5)
                        }
                        ElementKind::Column(c) => (true, c.position),
                        ElementKind::Foundation(f) => match f.shape {
                            cc_core::FoundationShape::Pad { position, .. } => (true, position),
                            _ => return None,
                        },
                        _ => return None,
                    };
                    let key =
                        if horizontal { (0, row(at.y) as i64, at.x) } else { (1, (at.x * 10.0).round() as i64, -at.y) };
                    Some((key.0, key.1, key.2, e.id))
                })
                .collect();
            keyed.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.total_cmp(&b.2)).then(a.3.cmp(&b.3)));
            let prefix = match category {
                Category::Beam => t!("mark-prefix-beam"),
                Category::Column => t!("mark-prefix-column"),
                _ => t!("mark-prefix-footing"),
            };
            marks.extend(keyed.iter().enumerate().map(|(i, k)| (k.3, format!("{prefix}{}", i + 1))));
        }
        if marks.is_empty() {
            return Ok(json!({"numbered": 0}));
        }
        edit(s, t!("tx-structure-number"), |e| {
            for (id, mark) in &marks {
                if let Some(x) = e.element_mut(*id) {
                    x.mark = mark.clone();
                }
            }
            Ok(())
        })?;
        Ok(
            json!({"numbered": marks.len(), "marks": marks.iter().map(|(id, m)| json!({"id": id.0, "mark": m})).collect::<Vec<_>>()}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_cross_only_within_their_extents() {
        let h = (DVec2::new(0.0, 0.0), DVec2::new(10.0, 0.0));
        assert_eq!(crossing(h, (DVec2::new(4.0, -1.0), DVec2::new(4.0, 1.0))), Some(DVec2::new(4.0, 0.0)));
        // Touching at an end counts; falling short does not.
        assert!(crossing(h, (DVec2::new(10.0, 0.0), DVec2::new(10.0, 5.0))).is_some());
        assert!(crossing(h, (DVec2::new(11.0, -1.0), DVec2::new(11.0, 1.0))).is_none());
        assert!(crossing(h, (DVec2::new(0.0, 1.0), DVec2::new(10.0, 1.0))).is_none());
        // Three lines through one point give it once.
        let v = (DVec2::new(5.0, -5.0), DVec2::new(5.0, 5.0));
        let d = (DVec2::new(0.0, -5.0), DVec2::new(10.0, 5.0));
        assert_eq!(intersections(&[h, v, d]), vec![DVec2::new(5.0, 0.0)]);
    }
}
