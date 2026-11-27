//! A multi-storey grid frame: on every storey of a range of levels, columns
//! at the grid intersections (tops attached to the level above) and beams
//! along the grids on the upper level, plus optional secondary beams, in one
//! undo step.

use super::structure::{SAME, beam_exists, columns_on, crossing, grids_with_ids, intersections};
use super::structure_material::{material_arg, structural_type};
use super::{add_element, edit, ids_json};
use crate::error::{EngineError, Result};
use crate::refs::{self, Ref};
use crate::registry::{Command, CommandKind};
use crate::session::Session;
use crate::t;
use cc_core::{Beam, Category, Column, ElementId, ElementKind, ElementTypeId, LevelId, StructuralMaterial, WallTop};
use glam::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Most members one run creates.
pub const MAX_MEMBERS: usize = 2000;

/// Secondary beams in each bay, parallel to a grid.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct SecondaryArg {
    /// The grid the secondary beams run parallel to (one of `grids`).
    pub parallel_to: u64,
    /// The largest distance between beams, meters (at least 0.3).
    pub spacing: f64,
}

/// Frame a range of levels on the grids: on every storey (each pair of
/// consecutive levels from `base_level` to `top_level`), a column at each
/// grid intersection of the lower level with its top attached to the upper
/// one, and beams on the upper level along each grid between neighboring
/// columns. Columns and beams already modeled are skipped. With `secondary`,
/// each bay between two grids parallel to `parallel_to` and two crossing
/// grids gets beams parallel to it, at most `spacing` apart. One undo step;
/// returns the new `columns`, `beams` and `secondary` beams and how many
/// were `skipped` (and bays `skipped_bays`).
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct GridFrameCreate {
    /// Grid ids (default: every grid).
    #[serde(default)]
    pub grids: Vec<u64>,
    /// The lowest level (default: the lowest level of the project).
    #[serde(default)]
    pub base_level: Option<Ref>,
    /// The highest level (default: the level above the base).
    #[serde(default)]
    pub top_level: Option<Ref>,
    /// Column type id or name (default: the first of `material`, made when missing).
    #[serde(default)]
    pub column_type: Option<Ref>,
    /// Beam type id or name (default: the first of `material`, made when missing).
    #[serde(default)]
    pub beam_type: Option<Ref>,
    /// `concrete`, `steel` (default) or `timber`: the default types' material.
    #[serde(default)]
    pub material: Option<String>,
    /// Secondary beams in each bay.
    #[serde(default)]
    pub secondary: Option<SecondaryArg>,
}

/// What a run will create, worked out before any edit.
#[derive(Default)]
struct Layout {
    columns: Vec<(LevelId, Column)>,
    beams: Vec<(LevelId, DVec2, DVec2)>,
    secondary: Vec<(LevelId, DVec2, DVec2)>,
    skipped: usize,
    skipped_bays: usize,
}

impl Layout {
    fn total(&self) -> usize {
        self.columns.len() + self.beams.len() + self.secondary.len()
    }

    /// More than one run may create: the layout stops there, the run is refused.
    fn over(&self) -> bool {
        self.total() > MAX_MEMBERS
    }
}

impl Command for GridFrameCreate {
    const NAME: &'static str = "grid_frame_create";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let material = match &self.material {
            Some(m) => material_arg(m)?,
            None => StructuralMaterial::Steel,
        };
        // Everything that can be refused is checked before any edit.
        let p = s.project();
        let storeys = storeys(p, self.base_level.as_ref(), self.top_level.as_ref())?;
        let grids = grids_with_ids(p, &self.grids)?;
        if grids.len() < 2 {
            return Err(EngineError::invalid(t!("msg-grids-two")));
        }
        let lines: Vec<(DVec2, DVec2)> = grids.iter().map(|(_, l)| *l).collect();
        let points = intersections(&lines);
        if points.is_empty() {
            return Err(EngineError::invalid("the grids do not cross"));
        }
        if let Some(sec) = &self.secondary {
            check_secondary(&grids, sec)?;
        }
        let layout = layout(p, &storeys, &grids, &points, self.secondary.as_ref());
        if layout.over() {
            return Err(EngineError::invalid(format!(
                "more than {MAX_MEMBERS} members in one run; frame fewer levels or grids, or space the secondary beams wider"
            )));
        }
        let column_given =
            self.column_type.as_ref().map(|r| refs::type_of(p, Category::Column, Some(r))).transpose()?;
        let beam_given = self.beam_type.as_ref().map(|r| refs::type_of(p, Category::Beam, Some(r))).transpose()?;
        let depth = s.doc.undo_depth();
        let result = (|| -> Result<Value> {
            let column_type = match column_given {
                Some(t) => t,
                None => structural_type(s, Category::Column, material)?.0,
            };
            let beam_type = match beam_given {
                Some(t) => t,
                None => structural_type(s, Category::Beam, material)?.0,
            };
            create(s, layout, column_type, beam_type)
        })();
        match result {
            Ok(v) => {
                s.doc.squash_since(depth, t!("tx-grid-frame"));
                Ok(v)
            }
            Err(e) => {
                s.undo_to(depth);
                Err(e)
            }
        }
    }
}

/// The storeys from `base` to `top`: consecutive levels by elevation.
fn storeys(p: &cc_core::Project, base: Option<&Ref>, top: Option<&Ref>) -> Result<Vec<(LevelId, LevelId)>> {
    let mut levels: Vec<(f64, LevelId)> = p.levels.iter().map(|l| (l.elevation, l.id)).collect();
    levels.sort_by(|a, b| a.0.total_cmp(&b.0));
    let index = |id: LevelId| levels.iter().position(|(_, l)| *l == id);
    let base = match base {
        Some(r) => refs::level(p, r)?,
        None => levels.first().map(|l| l.1).ok_or_else(|| EngineError::rejected("the project has no levels"))?,
    };
    let b = index(base).ok_or_else(|| EngineError::not_found("base level"))?;
    let t = match top {
        Some(r) => index(refs::level(p, r)?).ok_or_else(|| EngineError::not_found("top level"))?,
        None => b + 1,
    };
    if t <= b || t >= levels.len() {
        return Err(EngineError::invalid("the top level must be above the base level"));
    }
    Ok(levels[b..=t].windows(2).map(|w| (w[0].1, w[1].1)).collect())
}

fn check_secondary(grids: &[(ElementId, (DVec2, DVec2))], sec: &SecondaryArg) -> Result<()> {
    if !(sec.spacing.is_finite() && sec.spacing >= 0.3) {
        return Err(EngineError::invalid("secondary spacing must be at least 0.3 m"));
    }
    if !grids.iter().any(|(id, _)| id.0 == sec.parallel_to) {
        return Err(EngineError::invalid(format!("grid {} is not one of the grids", sec.parallel_to)));
    }
    Ok(())
}

/// Columns and main beams of every storey, skipping what exists.
fn layout(
    p: &cc_core::Project,
    storeys: &[(LevelId, LevelId)],
    grids: &[(ElementId, (DVec2, DVec2))],
    points: &[DVec2],
    secondary: Option<&SecondaryArg>,
) -> Layout {
    let mut out = Layout::default();
    for &(lower, upper) in storeys {
        if out.over() {
            break;
        }
        let existing = columns_on(p, lower);
        let mut on_level: Vec<DVec2> = existing.iter().map(|(_, c)| *c).collect();
        for &q in points {
            if out.over() {
                break;
            }
            if on_level.iter().any(|c| c.distance(q) < SAME) {
                out.skipped += 1;
                continue;
            }
            on_level.push(q);
            out.columns.push((
                lower,
                Column {
                    position: q,
                    rotation_deg: 0.0,
                    base_offset: 0.0,
                    height: (p.level_elevation(upper) - p.level_elevation(lower)).max(0.1),
                    top: WallTop::Level { level: upper, offset: 0.0 },
                    structural: true,
                    base_support: Default::default(),
                },
            ));
        }
        for (_, (a, b)) in grids {
            if out.over() {
                break;
            }
            for (x, y) in spans_along(*a, *b, &on_level) {
                let planned = out.beams.iter().any(|&(l, s, e)| l == upper && same_span((s, e), (x, y)));
                if planned || beam_exists(p, upper, x, y) {
                    out.skipped += 1;
                } else {
                    out.beams.push((upper, x, y));
                }
            }
        }
        if let Some(sec) = secondary {
            secondary_beams(p, upper, grids, sec, &mut out);
        }
    }
    out
}

fn same_span((a, b): (DVec2, DVec2), (x, y): (DVec2, DVec2)) -> bool {
    (a.distance(x) < SAME && b.distance(y) < SAME) || (a.distance(y) < SAME && b.distance(x) < SAME)
}

/// Spans between neighboring points of `on` that lie on the line `a`-`b`.
fn spans_along(a: DVec2, b: DVec2, on: &[DVec2]) -> Vec<(DVec2, DVec2)> {
    let Some(u) = (b - a).try_normalize() else { return Vec::new() };
    let len = a.distance(b);
    let mut along: Vec<(f64, DVec2)> = on
        .iter()
        .map(|&c| ((c - a).dot(u), c))
        .filter(|&(t, c)| (c - a).perp_dot(u).abs() < SAME && t > -SAME && t < len + SAME)
        .collect();
    along.sort_by(|x, y| x.0.total_cmp(&y.0));
    along.dedup_by(|x, y| x.1.distance(y.1) < SAME);
    along.windows(2).map(|w| (w[0].1, w[1].1)).filter(|(x, y)| x.distance(*y) >= SAME).collect()
}

/// Grids within this angle are parallel.
const PARALLEL_DEG: f64 = 1.0;

fn parallel(u: DVec2, v: DVec2) -> bool {
    u.perp_dot(v).abs() <= PARALLEL_DEG.to_radians().sin()
}

/// Secondary beams of `upper`: in each bay between two neighboring grids
/// parallel to `parallel_to` and two neighboring crossing grids, beams
/// parallel to it, evenly spaced at most `spacing` apart, from one crossing
/// grid to the other. Bays whose crossing grids are not parallel to each
/// other, or that lack a corner, are skipped.
fn secondary_beams(
    p: &cc_core::Project,
    upper: LevelId,
    grids: &[(ElementId, (DVec2, DVec2))],
    sec: &SecondaryArg,
    out: &mut Layout,
) {
    let Some(&(_, (a0, a1))) = grids.iter().find(|(id, _)| id.0 == sec.parallel_to) else { return };
    let Some(d) = (a1 - a0).try_normalize() else { return };
    let n = d.perp();
    let dir = |(s, e): (DVec2, DVec2)| (e - s).try_normalize();
    // Parallel grids by their offset across `d`; crossing grids by where they meet the reference line.
    let mut parallels: Vec<(f64, (DVec2, DVec2))> = Vec::new();
    let mut crossings: Vec<(f64, (DVec2, DVec2))> = Vec::new();
    for &(_, line) in grids {
        let Some(u) = dir(line) else { continue };
        if parallel(u, d) {
            parallels.push(((line.0 - a0).dot(n), line));
        } else {
            let un = u.dot(n);
            if un.abs() < 1e-9 {
                continue;
            }
            let along = (line.0 - a0).dot(d) - (line.0 - a0).dot(n) * u.dot(d) / un;
            if along.is_finite() {
                crossings.push((along, line));
            }
        }
    }
    parallels.sort_by(|x, y| x.0.total_cmp(&y.0));
    crossings.sort_by(|x, y| x.0.total_cmp(&y.0));
    for pw in parallels.windows(2) {
        let ((o0, p0), (o1, p1)) = (pw[0], pw[1]);
        let width = (o1 - o0).abs();
        if width < SAME {
            continue;
        }
        // Beams per bay, bounded so a tiny spacing cannot run away.
        let count = (((width / sec.spacing) - 1e-9).ceil().clamp(1.0, MAX_MEMBERS as f64) as usize).saturating_sub(1);
        for cw in crossings.windows(2) {
            if out.over() {
                return;
            }
            let (c0, c1) = (cw[0].1, cw[1].1);
            let corners = (crossing(p0, c0), crossing(p1, c0), crossing(p0, c1), crossing(p1, c1));
            let (Some(q00), Some(q10), Some(q01), Some(q11)) = corners else {
                out.skipped_bays += 1;
                continue;
            };
            let (Some(u0), Some(u1)) = (dir(c0), dir(c1)) else { continue };
            if !parallel(u0, u1) {
                out.skipped_bays += 1;
                continue;
            }
            for k in 1..=count {
                if out.over() {
                    return;
                }
                let f = k as f64 / (count + 1) as f64;
                let (x, y) = (q00.lerp(q10, f), q01.lerp(q11, f));
                let planned = out.secondary.iter().any(|&(l, s, e)| l == upper && same_span((s, e), (x, y)));
                if planned || beam_exists(p, upper, x, y) {
                    out.skipped += 1;
                } else {
                    out.secondary.push((upper, x, y));
                }
            }
        }
    }
}

fn create(s: &mut Session, layout: Layout, column_type: ElementTypeId, beam_type: ElementTypeId) -> Result<Value> {
    let Layout { columns, beams, secondary, skipped, skipped_bays } = layout;
    if columns.is_empty() && beams.is_empty() && secondary.is_empty() {
        return Ok(
            json!({"columns": [], "beams": [], "secondary": [], "skipped": skipped, "skipped_bays": skipped_bays}),
        );
    }
    let (c, b, sec) = edit(s, t!("tx-grid-frame"), |e| {
        let c: Vec<ElementId> =
            columns.into_iter().map(|(l, col)| add_element(e, l, column_type, ElementKind::Column(col))).collect();
        let mut beam =
            |(l, x, y): (LevelId, DVec2, DVec2)| add_element(e, l, beam_type, ElementKind::Beam(Beam::new(x, y, 0.0)));
        let b: Vec<ElementId> = beams.into_iter().map(&mut beam).collect();
        let sec: Vec<ElementId> = secondary.into_iter().map(&mut beam).collect();
        Ok((c, b, sec))
    })?;
    Ok(json!({
        "columns": ids_json(&c),
        "beams": ids_json(&b),
        "secondary": ids_json(&sec),
        "skipped": skipped,
        "skipped_bays": skipped_bays,
    }))
}
