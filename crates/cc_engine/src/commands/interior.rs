//! Interior elevations: a marker in a space closed by walls and a view of
//! each of its sides (see `cc_core::interior` and `cc_views::interior`).

use super::{add_element, edit};
use crate::error::{EngineError, Result};
use crate::refs::{self, Point, Ref, finite, point};
use crate::registry::{Command, CommandKind};
use crate::session::Session;
use crate::t;
use cc_core::interior::{InteriorMarker, Side, next_number};
use cc_core::{
    Category, EditCtx, ElementId, ElementKind, ElementType, ElementTypeId, Project, TypeData, View, ViewId, ViewKind,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Default scale of an interior elevation (1:50).
pub const DEFAULT_SCALE: u32 = 50;

/// The interior marker type a project gets with its first marker.
fn new_marker_type(id: ElementTypeId) -> ElementType {
    ElementType {
        id,
        name: t!("tpl-type-interior-marker"),
        data: TypeData::InteriorMarker {},
        info: Default::default(),
        uid: Default::default(),
    }
}

/// The sides named in `codes` (`north`, `east`, `south`, `west`), each once.
fn sides_of(codes: &[String]) -> Result<Vec<Side>> {
    let mut sides = Vec::new();
    for code in codes {
        let side = Side::from_code(code)
            .ok_or_else(|| EngineError::invalid(format!("unknown side '{code}' (north, east, south or west)")))?;
        if !sides.contains(&side) {
            sides.push(side);
        }
    }
    Ok(sides)
}

/// The name of view number `number` of a marker: the room's name and the
/// number, or a generic name and the number with no room.
fn view_name(p: &Project, room: Option<ElementId>, number: u32) -> String {
    let room = room.and_then(|r| match p.element(r).map(|e| &e.kind) {
        Some(ElementKind::Room(r)) if !r.name.trim().is_empty() => Some(r.name.trim().to_string()),
        _ => None,
    });
    match room {
        Some(name) => format!("{name} {number}"),
        None => format!("{} {number}", t!("default-interior-elevation-name")),
    }
}

/// Add the views of `sides` to marker `marker` (those it has are kept),
/// numbered after the numbers its views hold; returns the new views.
fn add_sides(e: &mut EditCtx, marker: ElementId, sides: &[Side]) -> Vec<ViewId> {
    let p = e.project();
    let Some(ElementKind::InteriorMarker(m)) = p.element(marker).map(|el| el.kind.clone()) else {
        return Vec::new();
    };
    let have = p.marker_sides(marker);
    let mut taken: Vec<u32> = have.iter().map(|(_, n, _)| *n).collect();
    let mut made = Vec::new();
    // Clockwise from north, so numbers follow the sides around the marker.
    for side in Side::ALL.into_iter().filter(|s| sides.contains(s)) {
        if have.iter().any(|(s, ..)| *s == side) {
            continue;
        }
        let number = next_number(&taken);
        taken.push(number);
        let name = view_name(e.project(), m.room, number);
        let kind = ViewKind::InteriorElevation {
            marker,
            side,
            number,
            origin: m.position,
            look: m.look(side),
            auto_extents: true,
        };
        let id = ViewId(e.alloc_id());
        made.push(e.add_view(View::new(id, name, DEFAULT_SCALE, kind)));
    }
    made
}

/// The marker's views as JSON: side, number, view id and name, clockwise from north.
fn sides_json(p: &Project, marker: ElementId) -> Value {
    let mut sides = p.marker_sides(marker);
    sides.sort_by_key(|(side, ..)| *side);
    json!(
        sides
            .iter()
            .map(|(side, number, view)| json!({
                "side": side.code(),
                "number": number,
                "view": view.0,
                "name": p.view(*view).map(|v| v.name.clone()).unwrap_or_default(),
            }))
            .collect::<Vec<_>>()
    )
}

/// Place an interior elevation marker in a space closed by walls (a room's,
/// or any other) and make a view of each of the sides picked: each looks
/// from the marker at the wall on that side, through the space up to the
/// far wall's outer face, cropped to the space across and from the level
/// to the ceiling or what covers the space above (else the room's height,
/// else the level above). The views follow the walls, the marker and the
/// room as they change. Views are named after the room and numbered
/// clockwise from north ("Kitchen 1", "Kitchen 2"…), at 1:50.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct InteriorElevationsCreate {
    /// Room id: the marker stands at its point, on its level, and the views
    /// take its name. Or give `position`.
    #[serde(default)]
    pub room: Option<u64>,
    /// Where the marker stands in plan, `[x, y]` (with no `room`).
    #[serde(default)]
    pub position: Option<Point>,
    /// Level id or name, with `position` (default: the lowest level).
    #[serde(default)]
    pub level: Option<Ref>,
    /// The sides to show: `north`, `east`, `south`, `west` (default all four).
    #[serde(default)]
    pub sides: Option<Vec<String>>,
    /// Turn of the sides counter-clockwise, degrees (default 0: facing
    /// north, east, south and west of the plan), to face slanted walls.
    #[serde(default)]
    pub rotation: Option<f64>,
}

impl Command for InteriorElevationsCreate {
    const NAME: &'static str = "interior_elevations_create";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let (position, level, room) = match (self.room, self.position) {
            (Some(_), Some(_)) => return Err(EngineError::invalid("give room or position, not both")),
            (Some(id), None) => {
                let room = refs::element(p, id)?;
                let el = p.element(room).ok_or_else(|| EngineError::not_found("room"))?;
                let ElementKind::Room(r) = &el.kind else {
                    return Err(EngineError::invalid(format!("element {id} is not a room")));
                };
                (r.position, el.level, Some(room))
            }
            (None, Some(at)) => (point(at)?, refs::level_or_first(p, self.level.as_ref())?, None),
            (None, None) => return Err(EngineError::invalid("give a room or a position")),
        };
        let sides = match &self.sides {
            Some(codes) => sides_of(codes)?,
            None => Side::ALL.to_vec(),
        };
        if sides.is_empty() {
            return Err(EngineError::invalid("give at least one side"));
        }
        let rotation_deg = finite(self.rotation.unwrap_or(0.0), "rotation")?.rem_euclid(360.0);
        let marker = InteriorMarker { position, room, rotation_deg };
        if !marker.is_valid() {
            return Err(EngineError::invalid("the position is out of bounds"));
        }
        let project = s.project().clone();
        let phase = s.doc.creation_phase;
        if s.geometry().room_at(&project, level, position, phase).is_none() {
            return Err(EngineError::rejected("no walls close a space around the marker"));
        }
        let ty = s.project().default_type(Category::InteriorMarker);
        let id = edit(s, t!("tx-create-interior-elevations"), |e| {
            let type_id = match ty {
                Some(t) => t,
                None => {
                    let id = ElementTypeId(e.alloc_id());
                    e.add_type(new_marker_type(id))
                }
            };
            let id = add_element(e, level, type_id, ElementKind::InteriorMarker(marker));
            add_sides(e, id, &sides);
            Ok(id)
        })?;
        let p = s.project();
        Ok(json!({ "marker": id.0, "views": sides_json(p, id) }))
    }
}

/// Set the sides an interior elevation marker shows: views are made for
/// the sides added (numbered with the lowest numbers free) and deleted,
/// with their viewports on sheets, for the sides left out. One undo step.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InteriorElevationSidesSet {
    /// Marker id.
    pub marker: u64,
    /// The sides to show: `north`, `east`, `south`, `west` (at least one).
    pub sides: Vec<String>,
}

impl Command for InteriorElevationSidesSet {
    const NAME: &'static str = "interior_elevation_sides_set";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let marker = refs::element(p, self.marker)?;
        if !matches!(p.element(marker).map(|e| &e.kind), Some(ElementKind::InteriorMarker(_))) {
            return Err(EngineError::invalid(format!("element {} is not an interior elevation marker", self.marker)));
        }
        let sides = sides_of(&self.sides)?;
        if sides.is_empty() {
            return Err(EngineError::invalid("give at least one side (delete the marker to remove them all)"));
        }
        let gone: Vec<ViewId> =
            p.marker_sides(marker).into_iter().filter(|(side, ..)| !sides.contains(side)).map(|(.., v)| v).collect();
        edit(s, t!("tx-interior-elevation-sides"), |e| {
            for view in gone {
                e.remove_view(view);
            }
            add_sides(e, marker, &sides);
            Ok(())
        })?;
        let p = s.project();
        Ok(json!({ "marker": marker.0, "views": sides_json(p, marker) }))
    }
}

/// The derived data of a marker for `element_get`: its views by side.
pub(crate) fn marker_derived(p: &Project, marker: ElementId) -> Value {
    json!({ "views": sides_json(p, marker) })
}
