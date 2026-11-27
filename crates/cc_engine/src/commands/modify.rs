//! Modifying elements: delete, move, copy/array, rotate, mirror, pin, flip,
//! joins, trim/extend, split, offset, align, end points.

use super::{edit, ids_json};
use crate::error::{EngineError, Result};
use crate::refs::{self, Point, finite, point};
use crate::registry::{Command, CommandKind};
use crate::session::Session;
use crate::t;
use cc_core::{ElementId, ElementKind, PlanTransform, Project};
use glam::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Refuse to edit a pinned element.
pub(crate) fn not_pinned(p: &Project, id: ElementId) -> Result<()> {
    if p.is_pinned(id) { Err(EngineError::rejected(t!("msg-pinned-skipped", "count" => 1))) } else { Ok(()) }
}

/// Split `ids` into free and pinned elements (pinned ones are not changed).
fn unpinned(p: &Project, ids: Vec<ElementId>) -> (Vec<ElementId>, Vec<ElementId>) {
    ids.into_iter().partition(|id| !p.is_pinned(*id))
}

fn transform(s: &mut Session, ids: &[u64], t: PlanTransform, tx: String) -> Result<Value> {
    let ids = refs::elements(s.project(), ids)?;
    let (free, pinned) = unpinned(s.project(), ids);
    let changed = edit(s, tx, |e| Ok(e.transform_elements(&free, &t)))?;
    Ok(json!({ "changed": ids_json(&changed), "pinned_skipped": ids_json(&pinned) }))
}

fn copy(s: &mut Session, ids: &[u64], transforms: &[PlanTransform], tx: String) -> Result<Value> {
    let ids = refs::elements(s.project(), ids)?;
    let created = edit(s, tx, |e| Ok(e.copy_elements(&ids, transforms)))?;
    Ok(json!({ "created": ids_json(&created) }))
}

/// Delete elements (doors and windows of deleted walls go too, piles with their caps; pinned
/// elements stay, and so do the piles of a cap that is not deleted).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementsDelete {
    /// Element ids.
    pub ids: Vec<u64>,
}

impl Command for ElementsDelete {
    const NAME: &'static str = "elements_delete";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let ids = refs::elements(s.project(), &self.ids)?;
        // A cap's piles go only with their cap (it would put them back).
        let owned = super::piles::owned_piles(s.project(), &ids);
        let ids: Vec<_> = ids.into_iter().filter(|id| !owned.contains(id)).collect();
        let (free, pinned) = unpinned(s.project(), ids);
        edit(s, t!("tx-delete"), |e| {
            for id in &free {
                e.remove_element(*id);
            }
            Ok(())
        })?;
        Ok(json!({
            "deleted": ids_json(&free),
            "pinned_skipped": ids_json(&pinned),
            "cap_piles_skipped": ids_json(&owned),
        }))
    }
}

/// Move elements by `delta` (walls take their doors and windows; doors and
/// windows alone slide along their wall).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementsMove {
    /// Element ids.
    pub ids: Vec<u64>,
    /// Displacement `[dx, dy]` in meters.
    pub delta: Point,
}

impl Command for ElementsMove {
    const NAME: &'static str = "elements_move";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        transform(s, &self.ids, PlanTransform::translate(point(self.delta)?), t!("tx-move"))
    }
}

/// Copy elements by `delta`; with `count` > 1, a linear array of that many
/// copies spaced by `delta`.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementsCopy {
    /// Element ids.
    pub ids: Vec<u64>,
    /// Displacement `[dx, dy]` of the first copy, meters (the next ones repeat it).
    pub delta: Point,
    /// Number of copies (default 1, at most 1000).
    #[serde(default)]
    pub count: Option<u32>,
}

impl Command for ElementsCopy {
    const NAME: &'static str = "elements_copy";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let delta = point(self.delta)?;
        let count = self.count.unwrap_or(1).clamp(1, 1000);
        let transforms: Vec<PlanTransform> =
            (1..=count).map(|k| PlanTransform::translate(delta * f64::from(k))).collect();
        let tx = if count > 1 { t!("tx-array") } else { t!("tx-copy") };
        copy(s, &self.ids, &transforms, tx)
    }
}

/// Center of the elements' extents in plan.
fn center_of(s: &mut Session, ids: &[ElementId]) -> Option<DVec2> {
    let mut pts: Vec<DVec2> = Vec::new();
    let geom = s.geometry().clone();
    let p = s.project();
    for id in ids {
        for solid in geom.solids_of(*id) {
            if let Some((lo, hi)) = solid.bounds() {
                pts.extend([lo.truncate(), hi.truncate()]);
            }
        }
        let Some(e) = p.element(*id) else { continue };
        if let Some((a, b)) = cc_core::wall_ops::line_of(e) {
            pts.extend([a, b]);
        }
        if let ElementKind::Room(r) = &e.kind {
            pts.push(r.position);
        }
    }
    let (lo, hi) = pts
        .iter()
        .fold(None, |acc: Option<(DVec2, DVec2)>, q| Some(acc.map_or((*q, *q), |(lo, hi)| (lo.min(*q), hi.max(*q)))))?;
    Some((lo + hi) * 0.5)
}

/// Rotate elements by `angle` degrees (counter-clockwise) about `center`
/// (default: the center of their extents); with `copy`, rotated copies.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementsRotate {
    /// Element ids.
    pub ids: Vec<u64>,
    /// Angle in degrees, counter-clockwise.
    pub angle: f64,
    /// Center of rotation (default: the center of the elements' extents).
    #[serde(default)]
    pub center: Option<Point>,
    /// Rotate copies, keeping the originals.
    #[serde(default)]
    pub copy: bool,
}

impl Command for ElementsRotate {
    const NAME: &'static str = "elements_rotate";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let ids = refs::elements(s.project(), &self.ids)?;
        let center = match self.center {
            Some(c) => point(c)?,
            None => center_of(s, &ids).ok_or_else(|| EngineError::invalid("give a center"))?,
        };
        let t = PlanTransform::rotate_about(center, finite(self.angle, "angle")?.to_radians());
        if self.copy { copy(s, &self.ids, &[t], t!("tx-rotate")) } else { transform(s, &self.ids, t, t!("tx-rotate")) }
    }
}

/// Mirror elements across an axis: two points, or the line of a wall, grid,
/// beam or railing (`axis_element`). Copies by default (`copy: false` moves them).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementsMirror {
    /// Element ids.
    pub ids: Vec<u64>,
    /// Two points of the axis.
    #[serde(default)]
    pub axis: Option<[Point; 2]>,
    /// Id of an element whose line is the axis.
    #[serde(default)]
    pub axis_element: Option<u64>,
    /// Keep the originals and mirror copies (default true).
    #[serde(default)]
    pub copy: Option<bool>,
}

impl Command for ElementsMirror {
    const NAME: &'static str = "elements_mirror";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let (a, b) = match (self.axis, self.axis_element) {
            (Some([a, b]), _) => (point(a)?, point(b)?),
            (None, Some(id)) => {
                let e = p.element(refs::element(p, id)?).ok_or_else(|| EngineError::not_found("element"))?;
                match &e.kind {
                    ElementKind::Wall(w) => p.wall_axis(e, w),
                    _ => {
                        cc_core::wall_ops::line_of(e).ok_or_else(|| EngineError::invalid("the element has no line"))?
                    }
                }
            }
            (None, None) => return Err(EngineError::invalid("give an axis or an axis_element")),
        };
        if a.distance(b) < 1e-9 {
            return Err(EngineError::invalid("the axis has no length"));
        }
        let t = PlanTransform::mirror_across(a, b - a);
        if self.copy.unwrap_or(true) {
            copy(s, &self.ids, &[t], t!("tx-mirror"))
        } else {
            transform(s, &self.ids, t, t!("tx-mirror"))
        }
    }
}

/// Pin elements (they cannot be moved or deleted) or unpin them.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementsPin {
    /// Element ids.
    pub ids: Vec<u64>,
    /// `true` to pin, `false` to unpin.
    pub pinned: bool,
}

impl Command for ElementsPin {
    const NAME: &'static str = "elements_pin";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let ids = refs::elements(s.project(), &self.ids)?;
        let tx = if self.pinned { t!("tx-pin") } else { t!("tx-unpin") };
        let changed = edit(s, tx, |e| Ok(e.set_pinned(&ids, self.pinned)))?;
        Ok(json!({ "changed": changed }))
    }
}

/// Flip a door or window to the other side of its wall (`facing`) and/or to the other jamb (`hand`).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementFlip {
    /// Id of the door or window.
    pub id: u64,
    /// Flip to the other side of the wall.
    #[serde(default)]
    pub facing: bool,
    /// Flip the hinge to the other jamb.
    #[serde(default)]
    pub hand: bool,
}

impl Command for ElementFlip {
    const NAME: &'static str = "element_flip";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let id = refs::element(s.project(), self.id)?;
        not_pinned(s.project(), id)?;
        edit(s, t!("tx-modify"), |e| {
            let h = e
                .element_mut(id)
                .and_then(|x| x.as_hosted_mut())
                .ok_or_else(|| EngineError::invalid("not a door or window"))?;
            h.flip_facing ^= self.facing;
            h.flip_hand ^= self.hand;
            Ok(())
        })?;
        Ok(super::created(s, id))
    }
}

/// Flip walls: swap their exterior and interior sides, their location line
/// staying where it is (the Space key on selected walls). The wall is
/// walked the other way: its layers mirror about the location line (a wall
/// located by a finish face keeps that face on the line, its body goes to
/// the other side); arcs keep their circle and splines their curve; its
/// doors, windows and niches stay in place facing the same way; its
/// profile and curtain grid stay in place along it (a curtain grid its type
/// lays out from one end becomes the wall's own lines); its reinforcement
/// stays on the faces looking the same way, its vertical bars at the same
/// stations; dimensions keep their points. One
/// undo step; pinned walls are left alone. A wall that cannot be flipped
/// (an arc or spline too tight for its body on the other side) fails the
/// command and nothing changes.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WallFlip {
    /// Ids of the walls.
    pub walls: Vec<u64>,
}

/// A wall that cannot be flipped, as a command error.
fn flip_error(id: ElementId, e: cc_core::wall_flip::FlipError) -> EngineError {
    match e {
        cc_core::wall_flip::FlipError::NotWall => EngineError::invalid(format!("element {} is not a wall", id.0)),
        e => EngineError::rejected(format!("wall {} cannot be flipped: {e}", id.0)),
    }
}

/// What a flip keeps in place that only the walls' derived geometry knows
/// (see `cc_core::wall_flip`): the offsets that keep vertical bar sets at
/// their stations, and the own grid lines that keep a curtain wall's type
/// grid where it is when its layout is not the same counted from the other
/// end (a fixed distance justified at the start or the end).
#[derive(Default)]
struct FlipKeep {
    offsets: Vec<(ElementId, f64)>,
    lines: Vec<(ElementId, Vec<f64>)>,
}

/// What flipping `walls` must keep in place, from the geometry before the flip.
fn flip_keep(s: &mut Session, walls: &[ElementId]) -> FlipKeep {
    let (p, geom) = s.project_and_geometry();
    let mut keep = FlipKeep::default();
    for id in walls {
        let (Some(e), Some(g)) = (p.element(*id), geom.walls.get(id)) else { continue };
        let Some(w) = e.as_wall() else { continue };
        for (bar, r) in p.rebar_of(*id) {
            if let Some(offset) = cc_geom::flipped_vertical_offset(g, r) {
                keep.offsets.push((bar.id, offset));
            }
        }
        // A curtain wall laid out by its type's vertical grid.
        let system = p.element_type(e.type_id).and_then(|t| t.curtain());
        let (Some(system), Some(curtain), None) = (system, g.curtain.as_ref(), &w.curtain.vertical) else { continue };
        let (Some(&lo), Some(&hi)) = (curtain.layout.u.first(), curtain.layout.u.last()) else { continue };
        let along = p.flipped_along(e, w);
        let mut kept: Vec<f64> =
            curtain.layout.interior(cc_core::curtain::GridAxis::Vertical).iter().map(|u| along(*u)).collect();
        kept.sort_by(f64::total_cmp);
        let laid = system.vertical.lines(along(hi), along(lo));
        let same = laid.len() == kept.len() && laid.iter().zip(&kept).all(|(a, b)| (a - b).abs() < 1e-9);
        if !same {
            keep.lines.push((*id, kept));
        }
    }
    keep
}

/// Flip `walls` in this edit, keeping in place what `keep` says.
fn flip_walls(e: &mut cc_core::EditCtx, walls: &[ElementId], keep: &FlipKeep) -> Result<Vec<ElementId>> {
    let changed = e.flip_walls(walls).map_err(|(id, err)| flip_error(id, err))?;
    for (id, value) in &keep.offsets {
        if let Some(ElementKind::Rebar(r)) = e.element_mut(*id).map(|x| &mut x.kind)
            && let cc_core::RebarLayout::WallVertical { offset, .. } = &mut r.layout
        {
            *offset = *value;
        }
    }
    for (id, lines) in &keep.lines {
        if let Some(ElementKind::Wall(w)) = e.element_mut(*id).map(|x| &mut x.kind) {
            w.curtain.vertical = Some(lines.clone());
        }
    }
    Ok(changed)
}

impl Command for WallFlip {
    const NAME: &'static str = "wall_flip";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let ids = refs::elements(s.project(), &self.walls)?;
        for id in &ids {
            s.project().check_wall_flip(*id).map_err(|e| flip_error(*id, e))?;
        }
        let (free, pinned) = unpinned(s.project(), ids);
        let changed = if free.is_empty() {
            Vec::new()
        } else {
            let keep = flip_keep(s, &free);
            edit(s, t!("tx-wall-flip"), |e| flip_walls(e, &free, &keep))?
        };
        Ok(json!({ "changed": ids_json(&changed), "walls": ids_json(&free), "pinned_skipped": ids_json(&pinned) }))
    }
}

/// Allow or disallow joining at one end of a wall or beam. A wall end joins
/// other walls; a beam end stops at the face of a structural column it lies
/// in, or meets other beams at the same depth (mitered corners, butt joints).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WallJoin {
    /// Wall or beam id.
    pub id: u64,
    /// `start` or `end`.
    pub end: String,
    /// `true` to allow joining at that end, `false` to keep it free.
    pub joined: bool,
}

fn at_start(end: &str) -> Result<bool> {
    match end {
        "start" => Ok(true),
        "end" => Ok(false),
        other => Err(EngineError::invalid(format!("end must be 'start' or 'end', not '{other}'"))),
    }
}

impl Command for WallJoin {
    const NAME: &'static str = "wall_join";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let id = refs::element(s.project(), self.id)?;
        let start = at_start(&self.end)?;
        let name = match s.project().element(id).map(|x| &x.kind) {
            Some(ElementKind::Beam(_)) => t!("tx-beam-join"),
            _ => t!("tx-wall-join"),
        };
        edit(s, name, |e| match e.element_mut(id).map(|x| &mut x.kind) {
            Some(ElementKind::Wall(w)) => {
                *(if start { &mut w.join_start } else { &mut w.join_end }) = self.joined;
                Ok(())
            }
            Some(ElementKind::Beam(b)) => {
                *(if start { &mut b.join_start } else { &mut b.join_end }) = self.joined;
                Ok(())
            }
            _ => Err(EngineError::invalid("not a wall or beam")),
        })?;
        Ok(super::created(s, id))
    }
}

/// Join the geometry of two elements that touch or overlap (walls, floors,
/// ceilings, roofs, columns, beams, stairs, railings, components). Where they
/// overlap, one cuts the other: structural columns and beams cut walls,
/// floors, ceilings and roofs; floors, ceilings and roofs cut walls; walls
/// cut architectural columns; between equals the first cuts the second. The
/// shared volume is counted once, and plans and sections draw two elements
/// cut in the same material as one piece.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeometryJoin {
    /// The first element.
    pub first: u64,
    /// The second element.
    pub second: u64,
}

/// The two elements of a join, checked: distinct, with a body.
fn join_pair(p: &Project, first: u64, second: u64) -> Result<(ElementId, ElementId)> {
    let (a, b) = (refs::element(p, first)?, refs::element(p, second)?);
    if a == b {
        return Err(EngineError::invalid(t!("msg-join-same")));
    }
    for id in [a, b] {
        if !p.element(id).is_some_and(|e| cc_core::can_join_geometry(e.category())) {
            return Err(EngineError::invalid(t!("msg-join-category")));
        }
    }
    Ok((a, b))
}

/// Whether two elements' bodies touch or overlap.
fn bodies_meet(geom: &cc_geom::ModelGeometry, a: ElementId, b: ElementId) -> bool {
    let bounds =
        |id| geom.solids_of(id).filter_map(cc_geom::Solid::bounds).reduce(|(lo, hi), (l, h)| (lo.min(l), hi.max(h)));
    let (Some((alo, ahi)), Some((blo, bhi))) = (bounds(a), bounds(b)) else { return false };
    let eps = glam::DVec3::splat(1e-6);
    alo.cmple(bhi + eps).all() && blo.cmple(ahi + eps).all()
}

impl Command for GeometryJoin {
    const NAME: &'static str = "geometry_join";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let (a, b) = join_pair(s.project(), self.first, self.second)?;
        if !bodies_meet(s.geometry(), a, b) {
            return Err(EngineError::invalid(t!("msg-join-apart")));
        }
        let p = s.project();
        let priority = |id| p.element(id).map_or(0, cc_core::join_priority);
        let (cutting, cut) = if priority(b) > priority(a) { (b, a) } else { (a, b) };
        edit(s, t!("tx-join-geometry"), |e| {
            e.set_joined(cutting, cut, true);
            Ok(())
        })?;
        Ok(json!({"cutting": cutting.0, "cut": cut.0}))
    }
}

/// Unjoin the geometry of two joined elements: each keeps its whole body again.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeometryUnjoin {
    /// The first element.
    pub first: u64,
    /// The second element.
    pub second: u64,
}

impl Command for GeometryUnjoin {
    const NAME: &'static str = "geometry_unjoin";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let (a, b) = join_pair(s.project(), self.first, self.second)?;
        let Some((cutting, cut)) = s.project().join_of(a, b) else {
            return Err(EngineError::not_found(t!("msg-join-none")));
        };
        edit(s, t!("tx-unjoin-geometry"), |e| {
            e.set_joined(cutting, cut, false);
            Ok(())
        })?;
        Ok(json!({"unjoined": [a.0, b.0]}))
    }
}

/// Group elements: selecting one of them selects them all, so they move,
/// copy and rotate together. Elements already in a group leave it.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GroupCreate {
    /// The elements (at least two).
    pub ids: Vec<u64>,
    /// Group name (default: "Group N").
    #[serde(default)]
    pub name: Option<String>,
}

impl Command for GroupCreate {
    const NAME: &'static str = "group_create";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let members: std::collections::BTreeSet<ElementId> = refs::elements(p, &self.ids)?.into_iter().collect();
        if members.len() < 2 {
            return Err(EngineError::invalid(t!("msg-group-two")));
        }
        let name = match self.name.map(|n| n.trim().to_string()) {
            Some(n) if !n.is_empty() => n,
            Some(_) => return Err(EngineError::invalid(t!("msg-group-name"))),
            None => (1..)
                .map(|n| t!("group-default-name", "n" => n))
                .find(|n| !p.groups.values().any(|g| &g.name == n))
                .unwrap_or_default(),
        };
        let mut id = cc_core::GroupId(0);
        edit(s, t!("tx-group"), |e| {
            id = e.add_group(name.clone(), members.clone());
            Ok(())
        })?;
        Ok(json!({"id": id.0, "name": name, "members": members.iter().map(|m| m.0).collect::<Vec<_>>()}))
    }
}

/// Ungroup: the group goes, its elements stay.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GroupUngroup {
    /// Group id (see `groups_list`, or `group` in `element_get`).
    pub group: u64,
}

impl Command for GroupUngroup {
    const NAME: &'static str = "group_ungroup";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let id = cc_core::GroupId(self.group);
        let Some(group) = s.project().groups.get(&id).cloned() else {
            return Err(EngineError::not_found(t!("msg-group-missing")));
        };
        edit(s, t!("tx-ungroup"), |e| {
            e.remove_group(id);
            Ok(())
        })?;
        Ok(json!({"ungrouped": group.members.iter().map(|m| m.0).collect::<Vec<_>>()}))
    }
}

/// Trim or extend two linear elements (walls, beams, grids) to meet at a
/// corner; each pick point marks the part of its element to keep. Arc walls
/// meet lines and other arcs where their circles cross them. Two detail
/// lines meet where their lines, circles or ellipses cross.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrimCorner {
    /// First element id.
    pub a: u64,
    /// A point on the part of `a` to keep.
    pub pick_a: Point,
    /// Second element id.
    pub b: u64,
    /// A point on the part of `b` to keep.
    pub pick_b: Point,
}

impl Command for TrimCorner {
    const NAME: &'static str = "trim_corner";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let (a, b) = (refs::element(s.project(), self.a)?, refs::element(s.project(), self.b)?);
        not_pinned(s.project(), a)?;
        not_pinned(s.project(), b)?;
        let (pa, pb) = (point(self.pick_a)?, point(self.pick_b)?);
        edit(s, t!("tx-trim"), |e| Ok(e.trim_corner(a, pa, b, pb)?))?;
        Ok(json!({ "changed": [a.0, b.0] }))
    }
}

/// Trim or extend a linear element to the line of a reference element (the
/// circle of an arc wall); the pick point marks the part to keep. Arc walls
/// trim and extend along their circle. A detail line trims or extends to
/// another detail line (also a circle or an ellipse) or to the line of a
/// wall, beam or grid.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrimExtend {
    /// Element to trim or extend.
    pub id: u64,
    /// A point on the part to keep.
    pub pick: Point,
    /// Id of the element whose line is the boundary.
    pub reference: u64,
}

fn path(s: &Session, id: u64) -> Result<cc_core::wall_ops::Path> {
    let p = s.project();
    let e = p.element(refs::element(p, id)?).ok_or_else(|| EngineError::not_found("element"))?;
    // A spline wall is no boundary to trim to yet.
    if cc_core::wall_ops::is_spline(e) {
        return Err(cc_core::wall_ops::EditError::Spline.into());
    }
    cc_core::wall_ops::path_of(e).ok_or_else(|| EngineError::invalid(format!("element {id} has no line")))
}

impl Command for TrimExtend {
    const NAME: &'static str = "trim_extend";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let id = refs::element(s.project(), self.id)?;
        not_pinned(s.project(), id)?;
        let pick = point(self.pick)?;
        // A detail line trims to any curve: another detail line's (a circle,
        // an ellipse) or the line of a wall, beam or grid.
        if matches!(s.project().element(id).map(|e| &e.kind), Some(ElementKind::DetailLine(_))) {
            let p = s.project();
            let boundary = p
                .element(refs::element(p, self.reference)?)
                .and_then(cc_core::wall_ops::curve_of)
                .ok_or_else(|| EngineError::invalid(format!("element {} has no line", self.reference)))?;
            edit(s, t!("tx-trim"), |e| Ok(e.trim_detail(id, pick, &boundary)?))?;
            return Ok(super::created(s, id));
        }
        let reference = path(s, self.reference)?;
        edit(s, t!("tx-trim"), |e| Ok(e.trim_to(id, pick, reference)?))?;
        Ok(super::created(s, id))
    }
}

/// Split a linear element (wall, beam, grid) or a detail line at a point;
/// returns the new element. A detail circle or whole ellipse splits into
/// halves, from the point and from the one opposite.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementSplit {
    /// Element id.
    pub id: u64,
    /// Where to split (projected on the element's line).
    pub point: Point,
}

impl Command for ElementSplit {
    const NAME: &'static str = "element_split";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let id = refs::element(s.project(), self.id)?;
        not_pinned(s.project(), id)?;
        let at = point(self.point)?;
        let new = edit(s, t!("tx-split"), |e| Ok(e.split(id, at)?))?;
        Ok(json!({ "original": id.0, "created": new.0 }))
    }
}

/// Copy a linear element parallel at `distance` (positive: to the left of
/// its direction). Detail lines too: an arc's or a circle's copy is
/// concentric (positive: towards the center of a circle); ellipses have no
/// offset.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementOffset {
    /// Element id.
    pub id: u64,
    /// Offset distance, meters: positive to the left of start→end, negative to the right.
    pub distance: f64,
}

impl Command for ElementOffset {
    const NAME: &'static str = "element_offset";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let id = refs::element(s.project(), self.id)?;
        let d = finite(self.distance, "distance")?;
        let new = edit(s, t!("tx-offset"), |e| Ok(e.offset_copy(id, d)?))?;
        Ok(super::created(s, new))
    }
}

/// Align an element onto a line of a reference element, in plan: the edge
/// picked on the element (a wall's face, a column's side) moves onto the
/// line picked on the reference (a face or side, or the location line of a
/// grid or of an element picked without a point). When the two are not
/// parallel the element first turns about the picked point of its edge. An
/// arc wall moves onto the circle of a concentric arc wall (its radius
/// changes). Alignment is in plan, so the elements may be on different
/// levels (a column above onto the face of the column below). With `view`
/// an elevation or a section, the picks are points of that view and the
/// lines those drawn there: by vertical lines (a wall's face cut, a beam's
/// side) the element moves across the view until its line lies on the
/// reference's; by horizontal lines (a beam's underside, a wall's top) a
/// wall's or column's top or bottom, whichever is picked, rises or drops
/// onto the reference's line, and other elements move up or down whole.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementAlign {
    /// Element to move.
    pub id: u64,
    /// Element whose line it moves onto.
    pub reference: u64,
    /// Plan point `[x, y]` near the edge of `id` that moves onto the reference line (the edge of
    /// its plan outline nearest to it). Without it a linear element moves its location line, and
    /// other elements their insertion point, onto the line.
    #[serde(default)]
    pub pick: Option<Point>,
    /// Plan point `[x, y]` near the line of `reference` to align to (the edge of its plan outline
    /// nearest to it, such as a wall face). Without it the reference's location line is used,
    /// which a column or component does not have.
    #[serde(default)]
    pub reference_pick: Option<Point>,
    /// The view the picks were made in. An elevation or a section takes the picks as points of
    /// that view (both needed); a plan, or none, as plan points.
    #[serde(default)]
    pub view: Option<refs::Ref>,
}

/// The line of element `id` that Align uses: the edge of its plan outline
/// nearest to `pick` (a wall face, a column side), else its location line
/// (grids have only that). An arc wall gives its arc.
pub fn align_line(
    project: &cc_core::Project,
    geom: &cc_geom::ModelGeometry,
    id: ElementId,
    pick: Option<DVec2>,
) -> Option<cc_core::wall_ops::Path> {
    use cc_core::wall_ops::{Path, path_of};
    let path = project.element(id).and_then(path_of);
    if let Some(Path::Arc(_)) = path {
        return path;
    }
    pick.and_then(|at| geom.nearest_plan_edge(id, at)).map(|(a, b)| Path::Line(a, b)).or(path)
}

/// The turn (radians, within ±90°) that makes direction `from` parallel to `to`.
fn turn_to_parallel(from: DVec2, to: DVec2) -> f64 {
    let angle = from.perp_dot(to).atan2(from.dot(to));
    let half = std::f64::consts::FRAC_PI_2;
    if angle > half {
        angle - std::f64::consts::PI
    } else if angle <= -half {
        angle + std::f64::consts::PI
    } else {
        angle
    }
}

impl Command for ElementAlign {
    const NAME: &'static str = "element_align";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        use cc_core::wall_ops::{EditError, Path};
        let id = refs::element(s.project(), self.id)?;
        not_pinned(s.project(), id)?;
        let reference_id = refs::element(s.project(), self.reference)?;
        let pick = self.pick.map(point).transpose()?;
        let reference_pick = self.reference_pick.map(point).transpose()?;
        // Spline walls are neither aligned nor aligned to.
        if [id, reference_id].iter().any(|x| s.project().element(*x).is_some_and(cc_core::wall_ops::is_spline)) {
            return Err(EditError::Spline.into());
        }
        if let Some(view) = &self.view {
            let view = refs::view(s.project(), view)?;
            let level_view = |v: &&cc_core::View| {
                use cc_core::ViewKind::*;
                matches!(v.kind, Elevation { .. } | Section { .. } | InteriorElevation { .. })
            };
            if let Some(frame) = s.project().view(view).filter(level_view).and_then(cc_views::view_frame) {
                let (Some(pick), Some(reference_pick)) = (pick, reference_pick) else {
                    return Err(EngineError::invalid(
                        "aligning in an elevation or a section needs pick and reference_pick",
                    ));
                };
                return align_in_view(s, view, &frame, (id, pick), (reference_id, reference_pick));
            }
        }
        s.geometry();
        let (project, geom) = (&s.doc.project, s.geom());
        let reference = align_line(project, geom, reference_id, reference_pick).ok_or_else(|| {
            EngineError::invalid(format!(
                "element {} has no line: give reference_pick near the edge to align to",
                self.reference
            ))
        })?;
        let moved = align_line(project, geom, id, pick);
        // Arc walls onto concentric arcs.
        if let (Some(Path::Arc(_)), Path::Arc(_)) = (&moved, &reference) {
            edit(s, t!("tx-align"), |e| Ok(e.align_to(id, reference)?))?;
            return Ok(super::created(s, id));
        }
        let Path::Line(r0, r1) = reference else { return Err(EditError::Curved.into()) };
        let v = (r1 - r0).normalize_or_zero();
        if v == DVec2::ZERO {
            return Err(EditError::Degenerate.into());
        }
        // The point of the element that lands on the reference line, and the
        // direction of its edge (none for a lone insertion point).
        let (on, dir) = match moved {
            Some(Path::Line(a, b)) => {
                let u = (b - a).normalize_or_zero();
                if u == DVec2::ZERO {
                    return Err(EditError::Degenerate.into());
                }
                let at = pick.map_or(a, |p| a + u * (p - a).dot(u));
                (at, Some(u))
            }
            Some(Path::Arc(_)) => return Err(EditError::Curved.into()),
            None => (insertion_point(s, id)?, None),
        };
        let turn = dir.map_or(0.0, |u| turn_to_parallel(u, v));
        let n = v.perp();
        let shift = n * (r0 - on).dot(n);
        let mut t = PlanTransform::translate(shift);
        if turn.abs() > 1e-9 {
            t = PlanTransform::rotate_about(on, turn).then(t);
        }
        edit(s, t!("tx-align"), |e| {
            e.transform_elements(&[id], &t);
            Ok(())
        })?;
        Ok(super::created(s, id))
    }
}

/// The line of element `id` drawn in a view nearest to `at` (view
/// coordinates): its cut and projected linework, not hatching or annotation.
pub fn view_align_line(drawing: &cc_views::Drawing2D, id: ElementId, at: DVec2) -> Option<(DVec2, DVec2)> {
    use cc_views::{LineStyle, Prim};
    let ids = std::collections::BTreeSet::from([id]);
    let mut best: Option<(f64, (DVec2, DVec2))> = None;
    for i in drawing.prims_of(&ids) {
        let Some(Prim::Line { a, b, style, .. }) = drawing.prims.get(i) else { continue };
        if matches!(style, LineStyle::Hatch | LineStyle::Annotation | LineStyle::Faint) || a.distance(*b) < 1e-9 {
            continue;
        }
        let d = cc_geom::polygon::closest_on_segment(*a, *b, at).0.distance(at);
        if d.is_finite() && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, (*a, *b)));
        }
    }
    best.map(|(_, line)| line)
}

/// Whether a line of a view runs up and down it (within a hundredth of a degree).
fn upright(a: DVec2, b: DVec2) -> bool {
    let d = b - a;
    d.y.abs() > 1e-12 && (d.x / d.y).abs() < 2e-4
}

/// Whether a line of a view runs across it (within a hundredth of a degree).
fn level(a: DVec2, b: DVec2) -> bool {
    upright(DVec2::new(a.y, a.x), DVec2::new(b.y, b.x))
}

/// The thinnest an element is left by aligning its top or bottom, meters.
const MIN_ALIGNED_HEIGHT: f64 = 0.01;

/// `kind` with its top (or bottom) raised by `dz`: walls and columns
/// stretch (their other end stays), other elements move up or down whole.
/// `None` for what has no height to change this way (a wall with a profile,
/// the top of a wall attached to a roof).
fn raised(kind: &ElementKind, top: bool, dz: f64) -> Option<ElementKind> {
    use cc_core::WallTop;
    let stretch = |base_offset: &mut f64, height: &mut f64, wall_top: &mut WallTop| {
        if top {
            match wall_top {
                WallTop::Unconnected => *height += dz,
                WallTop::Level { offset, .. } => *offset += dz,
            }
        } else {
            *base_offset += dz;
            if *wall_top == WallTop::Unconnected {
                *height -= dz;
            }
        }
    };
    let mut kind = kind.clone();
    match &mut kind {
        ElementKind::Wall(w) => {
            if w.profile.is_some() || (top && w.attach_top.is_some()) {
                return None;
            }
            stretch(&mut w.base_offset, &mut w.height, &mut w.top);
        }
        ElementKind::Column(c) => stretch(&mut c.base_offset, &mut c.height, &mut c.top),
        ElementKind::Beam(b) => {
            b.top_offset += dz;
            if let Some(end) = &mut b.end_offset {
                *end += dz;
            }
        }
        ElementKind::Floor(f) => f.height_offset += dz,
        ElementKind::Ceiling(c) => c.height_offset += dz,
        ElementKind::Roof(r) => r.base_offset += dz,
        ElementKind::Foundation(f) => f.top_offset += dz,
        ElementKind::Component(c) => c.base_offset += dz,
        _ => return None,
    }
    Some(kind)
}

/// Align in an elevation or a section: the line of `id` nearest to its pick
/// goes onto the line of the reference nearest to its pick. Vertical lines
/// move the element in plan along the view's horizontal axis; horizontal
/// ones raise or lower it ([`raised`]: the top or the bottom picked, of a
/// wall or a column).
fn align_in_view(
    s: &mut Session,
    view: cc_core::ViewId,
    frame: &cc_views::Frame,
    (id, pick): (ElementId, DVec2),
    (reference, reference_pick): (ElementId, DVec2),
) -> Result<Value> {
    let drawing = s.drawing(view).ok_or_else(|| EngineError::not_found("view"))?;
    let line = |e: ElementId, at: DVec2| {
        view_align_line(&drawing, e, at)
            .ok_or_else(|| EngineError::invalid(format!("element {} is not drawn in the view", e.0)))
    };
    let (m0, m1) = line(id, pick)?;
    let (r0, r1) = line(reference, reference_pick)?;
    if level(m0, m1) && level(r0, r1) {
        return align_height(s, frame, id, (m0, m1), (r0, r1));
    }
    if !upright(m0, m1) || !upright(r0, r1) {
        return Err(EngineError::rejected(t!("msg-align-view-vertical")));
    }
    let right = frame.right.truncate();
    // Only views looking level (elevations, sections), whose horizontal axis lies in plan.
    if right.length() < 1e-9 || frame.right.z.abs() > 1e-9 || frame.forward.z.abs() > 1e-9 {
        return Err(EngineError::rejected(t!("msg-align-view-vertical")));
    }
    let dx = (r0.x + r1.x - m0.x - m1.x) * 0.5;
    if dx.abs() < 1e-12 {
        return Ok(super::created(s, id));
    }
    let t = PlanTransform::translate(right * (dx / right.length_squared()));
    edit(s, t!("tx-align"), |e| {
        e.transform_elements(&[id], &t);
        Ok(())
    })?;
    Ok(super::created(s, id))
}

/// Align the horizontal line `moved` of `id` onto the horizontal line
/// `onto`, both of a view through `frame`: the element's top or bottom,
/// whichever the line is nearer to, rises or drops by the difference.
fn align_height(
    s: &mut Session,
    frame: &cc_views::Frame,
    id: ElementId,
    moved: (DVec2, DVec2),
    onto: (DVec2, DVec2),
) -> Result<Value> {
    let z = |(a, b): (DVec2, DVec2)| frame.unproject((a + b) * 0.5, 0.0).z;
    let dz = z(onto) - z(moved);
    if !dz.is_finite() || frame.up.z < 1.0 - 1e-9 {
        return Err(EngineError::rejected(t!("msg-align-view-vertical")));
    }
    if dz.abs() < 1e-12 {
        return Ok(super::created(s, id));
    }
    let bounds =
        s.geometry().solids_of(id).filter_map(|solid| solid.bounds()).reduce(|(a, b), (c, d)| (a.min(c), b.max(d)));
    let (lo, hi) = bounds.ok_or_else(|| EngineError::invalid(format!("element {} has no body to align by", id.0)))?;
    let at = z(moved);
    let top = (at - hi.z).abs() <= (at - lo.z).abs();
    let element = s.project().element(id).ok_or_else(|| EngineError::not_found("element"))?;
    let kind = raised(&element.kind, top, dz).ok_or_else(|| EngineError::rejected(t!("msg-align-view-height")))?;
    // Walls and columns stretch: they must keep some height.
    let stretches = matches!(kind, ElementKind::Wall(_) | ElementKind::Column(_));
    let (bottom, top_z) = if top { (lo.z, hi.z + dz) } else { (lo.z + dz, hi.z) };
    if stretches && top_z - bottom < MIN_ALIGNED_HEIGHT {
        return Err(EngineError::rejected(t!("msg-align-view-too-low")));
    }
    edit(s, t!("tx-align"), |e| {
        let x = e.element_mut(id).ok_or_else(|| EngineError::not_found("element"))?;
        x.kind = kind;
        Ok(())
    })?;
    Ok(super::created(s, id))
}

/// Where an element without a location line stands in plan: a column's or
/// component's position, else the center of its outline.
fn insertion_point(s: &mut Session, id: ElementId) -> Result<DVec2> {
    match s.project().element(id).map(|e| &e.kind) {
        Some(ElementKind::Column(c)) => return Ok(c.position),
        Some(ElementKind::Component(c)) => return Ok(c.position),
        _ => {}
    }
    let solids = s.geometry().solids_of(id).filter_map(|solid| solid.bounds());
    let bounds = solids.reduce(|(a, b), (c, d)| (a.min(c), b.max(d)));
    let (lo, hi) =
        bounds.ok_or_else(|| EngineError::invalid(format!("element {} has no outline to align by", id.0)))?;
    Ok(DVec2::new(lo.x + hi.x, lo.y + hi.y) * 0.5)
}

/// Move one end of a wall, beam or grid (joined wall ends follow).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementMoveEnd {
    /// Element id.
    pub id: u64,
    /// `start` or `end`.
    pub end: String,
    /// New position of that end.
    pub point: Point,
}

impl Command for ElementMoveEnd {
    const NAME: &'static str = "element_move_end";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let id = refs::element(s.project(), self.id)?;
        not_pinned(s.project(), id)?;
        let start = at_start(&self.end)?;
        let to = point(self.point)?;
        edit(s, t!("tx-modify"), |e| Ok(e.move_endpoint(id, start, to)?))?;
        Ok(super::created(s, id))
    }
}

/// Give grids the extents of a reference grid, so their bubbles line up:
/// each grid parallel to the reference (within 1°) has its ends moved along
/// its own line onto the lines through the reference's ends perpendicular
/// to it (each end to the reference end on its side). Grids that are not
/// parallel and pinned grids are left alone.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GridsAlignEnds {
    /// Id of the reference grid.
    pub reference: u64,
    /// Ids of the grids to align (default: every other grid parallel to the reference).
    #[serde(default)]
    pub grids: Vec<u64>,
    /// Which ends move: `both` (default), `start` (the bubble end) or `end`.
    #[serde(default)]
    pub ends: Option<String>,
}

impl Command for GridsAlignEnds {
    const NAME: &'static str = "grids_align_ends";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let grid_of = |id: ElementId| match p.element(id).map(|e| &e.kind) {
            Some(ElementKind::Grid(g)) => Ok(g),
            _ => Err(EngineError::invalid(format!("element {} is not a grid", id.0))),
        };
        let reference_id = refs::element(p, self.reference)?;
        let reference = grid_of(reference_id)?.clone();
        let (start, end) = match self.ends.as_deref().unwrap_or("both") {
            "both" => (true, true),
            "start" => (true, false),
            "end" => (false, true),
            other => return Err(EngineError::invalid(format!("ends must be 'both', 'start' or 'end', not '{other}'"))),
        };
        let ids: Vec<ElementId> = if self.grids.is_empty() {
            super::create::grids(p).map(|(id, _)| id).collect()
        } else {
            let ids = refs::elements(p, &self.grids)?;
            for id in &ids {
                grid_of(*id)?;
            }
            ids
        };
        let ids: Vec<ElementId> = ids.into_iter().filter(|id| *id != reference_id).collect();
        let (free, pinned) = unpinned(p, ids);
        let mut lines: Vec<(ElementId, (DVec2, DVec2))> = Vec::new();
        let mut not_parallel: Vec<ElementId> = Vec::new();
        for id in free {
            let Ok(g) = grid_of(id) else { continue };
            match cc_core::grid::aligned_ends(g, &reference, start, end) {
                Some((a, b)) if g.start.distance(a) > 1e-9 || g.end.distance(b) > 1e-9 => lines.push((id, (a, b))),
                Some(_) => {}
                None => not_parallel.push(id),
            }
        }
        // Explicit ids that are not parallel are reported; by default they are just other grids.
        if self.grids.is_empty() {
            not_parallel.clear();
        }
        let changed: Vec<ElementId> = lines.iter().map(|(id, _)| *id).collect();
        if !lines.is_empty() {
            edit(s, t!("tx-align-grid-ends"), |e| {
                for (id, (a, b)) in &lines {
                    if let Some(ElementKind::Grid(g)) = e.element_mut(*id).map(|el| &mut el.kind) {
                        (g.start, g.end) = (*a, *b);
                    }
                }
                Ok(())
            })?;
        }
        Ok(json!({
            "changed": ids_json(&changed),
            "pinned_skipped": ids_json(&pinned),
            "not_parallel": ids_json(&not_parallel),
        }))
    }
}

/// Turn elements a quarter: components and columns rotate 90° about their
/// insertion point; doors and windows flip to the other side of their wall;
/// walls swap their exterior and interior sides as `wall_flip` does, their
/// doors, windows and niches going with them, selected or not (the Space
/// key on a selection), in the same undo step. Others, pinned elements and
/// walls that cannot be flipped (`not_flipped`) are left alone.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementsTurn {
    /// Element ids.
    pub ids: Vec<u64>,
}

impl Command for ElementsTurn {
    const NAME: &'static str = "elements_turn";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let ids = refs::elements(s.project(), &self.ids)?;
        let (free, pinned) = unpinned(s.project(), ids);
        let (walls, free): (Vec<ElementId>, Vec<ElementId>) =
            free.into_iter().partition(|id| s.project().element(*id).is_some_and(|e| e.as_wall().is_some()));
        let (walls, not_flipped): (Vec<ElementId>, Vec<ElementId>) =
            walls.into_iter().partition(|id| s.project().check_wall_flip(*id).is_ok());
        // Doors, windows and niches of a flipped wall go with it.
        let with_wall = |id: &ElementId| {
            s.project().element(*id).and_then(|e| e.as_hosted()).is_some_and(|h| walls.contains(&h.host))
        };
        let turnable: Vec<ElementId> = free
            .into_iter()
            .filter(|id| {
                matches!(
                    s.project().element(*id).map(|e| &e.kind),
                    Some(
                        ElementKind::Component(_)
                            | ElementKind::Column(_)
                            | ElementKind::Door(_)
                            | ElementKind::Window(_)
                            | ElementKind::Niche(_)
                    )
                ) && !with_wall(id)
            })
            .collect();
        if turnable.is_empty() && walls.is_empty() {
            return Ok(
                json!({ "changed": [], "pinned_skipped": ids_json(&pinned), "not_flipped": ids_json(&not_flipped) }),
            );
        }
        let name = if turnable.is_empty() { t!("tx-wall-flip") } else { t!("tx-rotate") };
        let keep = flip_keep(s, &walls);
        edit(s, name, |e| {
            flip_walls(e, &walls, &keep)?;
            for id in &turnable {
                let Some(el) = e.element_mut(*id) else { continue };
                match &mut el.kind {
                    ElementKind::Component(c) => c.rotation_deg = (c.rotation_deg + 90.0).rem_euclid(360.0),
                    ElementKind::Column(c) => c.rotation_deg = (c.rotation_deg + 90.0).rem_euclid(360.0),
                    ElementKind::Door(h) | ElementKind::Window(h) | ElementKind::Niche(h) => {
                        h.flip_facing = !h.flip_facing
                    }
                    _ => {}
                }
            }
            Ok(())
        })?;
        let changed: Vec<ElementId> = walls.into_iter().chain(turnable).collect();
        Ok(json!({
            "changed": ids_json(&changed),
            "pinned_skipped": ids_json(&pinned),
            "not_flipped": ids_json(&not_flipped),
        }))
    }
}

/// The elements as a portable set for `elements_paste`: doors and windows of
/// the given walls come along. The set keeps type, level and view ids, so it
/// pastes into this project or one sharing them.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementsExport {
    /// Element ids.
    pub ids: Vec<u64>,
}

impl Command for ElementsExport {
    const NAME: &'static str = "elements_export";
    const KIND: CommandKind = CommandKind::Query;
    fn run(self, s: &mut Session) -> Result<Value> {
        let ids = refs::elements(s.project(), &self.ids)?;
        let set = s.project().copy_set(&ids);
        Ok(json!({ "elements": serde_json::to_value(set).map_err(|e| EngineError::invalid(e.to_string()))? }))
    }
}

/// Paste a set from `elements_export` moved by `delta`, on `level` when
/// given (else on their own levels). Elements whose type, level, view or
/// wall does not exist here are skipped. Returns the new ids.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementsPaste {
    /// The `elements` of an `elements_export` result.
    pub elements: Value,
    /// Displacement `[dx, dy]`, meters (default: none).
    #[serde(default)]
    pub delta: Option<Point>,
    /// Level id or name to paste on (default: each element's own level).
    #[serde(default)]
    pub level: Option<crate::refs::Ref>,
    /// View id or name to paste the elements drawn in a view into (detail
    /// lines, filled and masking regions, text notes, dimensions, level
    /// marks, detail components and imported drawings), at the same drawing
    /// coordinates and on its level (default: each one's own view). Every
    /// such element must be one the view takes. Elements of the model ignore it.
    #[serde(default)]
    pub view: Option<crate::refs::Ref>,
}

impl Command for ElementsPaste {
    const NAME: &'static str = "elements_paste";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let mut set: Vec<cc_core::Element> = serde_json::from_value(self.elements)
            .map_err(|e| EngineError::invalid(format!("invalid elements: {e}")))?;
        let t = PlanTransform::translate(self.delta.map(point).transpose()?.unwrap_or(DVec2::ZERO));
        let level = self.level.as_ref().map(|l| refs::level(s.project(), l)).transpose()?;
        if let Some(view) = &self.view {
            let view = refs::view(s.project(), view)?;
            retarget(s.project(), &mut set, view)?;
        }
        let created = edit(s, t!("tx-paste"), |e| Ok(e.paste_elements(&set, &[t], level)))?;
        Ok(json!({ "created": ids_json(&created) }))
    }
}

/// Move the view-owned elements of a copied set (see
/// [`cc_core::Element::owner_view`]) into `view`, onto its level (or the
/// lowest). Refused unless the view takes every one of them, as the
/// commands that create them would. A dimension moved between views that
/// do not share a drawing frame (two plans) stops measuring elements and
/// stays where it is drawn.
pub(crate) fn retarget(p: &Project, set: &mut [cc_core::Element], view: cc_core::ViewId) -> Result<()> {
    let target = p.view(view).ok_or_else(|| EngineError::not_found("view"))?;
    let level =
        target.level().or_else(|| p.levels.first().map(|l| l.id)).ok_or_else(|| EngineError::rejected("no levels"))?;
    for e in set.iter_mut() {
        let Some(from) = e.owner_view() else { continue };
        if !view_takes(target, &e.kind) {
            let category = refs::category_name(e.category());
            return Err(EngineError::invalid(format!("the view '{}' does not take {category} elements", target.name)));
        }
        let same_frame = from == view || (target.is_plan() && p.view(from).is_some_and(|v| v.is_plan()));
        if let ElementKind::Dimension(d) = &mut e.kind
            && !same_frame
        {
            for r in d.all_refs_mut() {
                *r = None;
            }
        }
        e.set_owner_view(view);
        e.level = level;
    }
    Ok(())
}

/// Whether `view` takes a view-owned element of this kind, as the command
/// that creates it would.
fn view_takes(view: &cc_core::View, e: &ElementKind) -> bool {
    use cc_core::ViewKind as V;
    match e {
        ElementKind::Text(_) => !matches!(view.kind, V::Schedule { .. } | V::Sheet { .. }),
        ElementKind::Dimension(_) => view.takes_dimensions(),
        ElementKind::BarTag(_) => super::bar_marks::takes_tags(view),
        ElementKind::LevelMark(_) => {
            matches!(view.kind, V::FloorPlan { .. } | V::CeilingPlan { .. } | V::Elevation { .. } | V::Section { .. })
        }
        _ => cc_views::hosts_cad(&view.kind),
    }
}

/// Attach the tops of walls to a roof, ceiling or floor (`top`), or take
/// them off it (`detach`). Under the element a wall rises or drops to its
/// underside, following its slope (a gable wall ends in a triangle);
/// elsewhere it keeps its own top. The walls follow the element when it
/// changes.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WallAttach {
    /// Wall ids.
    pub ids: Vec<u64>,
    /// The roof, ceiling or floor to attach the tops to.
    #[serde(default)]
    pub top: Option<u64>,
    /// Take the tops off what they are attached to.
    #[serde(default)]
    pub detach: bool,
}

impl Command for WallAttach {
    const NAME: &'static str = "wall_attach";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let target = match (self.top, self.detach) {
            (Some(_), true) => return Err(EngineError::invalid("give top or detach, not both")),
            (None, false) => {
                return Err(EngineError::invalid("give the roof, ceiling or floor to attach to, or detach"));
            }
            (Some(t), false) => {
                let t = refs::element(p, t)?;
                match p.element(t).map(|e| &e.kind) {
                    Some(ElementKind::Roof(_) | ElementKind::Ceiling(_) | ElementKind::Floor(_)) => Some(t),
                    _ => return Err(EngineError::invalid(format!("element {} is not a roof, ceiling or floor", t.0))),
                }
            }
            (None, true) => None,
        };
        if self.ids.is_empty() {
            return Err(EngineError::invalid("no walls given"));
        }
        let mut walls = Vec::with_capacity(self.ids.len());
        for id in &self.ids {
            let id = refs::element(p, *id)?;
            match p.element(id).map(|e| &e.kind) {
                Some(ElementKind::Wall(w)) if w.spline.is_some() && target.is_some() => {
                    return Err(EngineError::invalid(format!(
                        "wall {} is a spline wall: its top is not attached to a roof, ceiling or floor yet",
                        id.0
                    )));
                }
                Some(ElementKind::Wall(w)) if w.profile.is_some() && target.is_some() => {
                    return Err(EngineError::invalid(format!(
                        "wall {} has an elevation profile, which sets its top: remove it (wall_profile_remove) to attach the top",
                        id.0
                    )));
                }
                Some(ElementKind::Wall(_)) => {}
                _ => return Err(EngineError::invalid(format!("element {} is not a wall", id.0))),
            }
            walls.push(id);
        }
        edit(s, t!("tx-wall-attach"), |e| {
            for id in &walls {
                if let Some(ElementKind::Wall(w)) = e.element_mut(*id).map(|x| &mut x.kind) {
                    w.attach_top = target;
                }
            }
            Ok(())
        })?;
        Ok(json!({ "ids": walls.iter().map(|id| id.0).collect::<Vec<_>>(), "top": target.map(|t| t.0) }))
    }
}
