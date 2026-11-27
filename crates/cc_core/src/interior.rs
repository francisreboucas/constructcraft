//! Interior elevations: the walls of a space seen from inside it. An
//! [`InteriorMarker`] stands in a plan at a point of a space closed by walls
//! (a room's, or any other); each of its interior elevation views
//! ([`crate::ViewKind::InteriorElevation`]) shows one [`Face`]: one of four
//! sides at right angles, turned with the marker and seen from it
//! ([`FaceMode::Directions`]), or one wall of the space, seen square on
//! from a set distance in front of it ([`FaceMode::Walls`]).
//!
//! A view stores only its marker and its face; where it looks from and in
//! which direction (`origin`, `look`) are worked out from the marker and
//! the wall by every edit ([`Project::interior_frame`], see
//! [`crate::EditCtx`]), so the view's projection needs no geometry. How deep it looks and how it is cropped come from the space's
//! boundary, worked out when the view is drawn (`cc_views::interior`).

use crate::{ElementId, ElementKind, Project, ViewId, ViewKind};
use glam::DVec2;
use serde::{Deserialize, Serialize};

/// Farthest a marker stands from the origin, meters.
pub const MAX_POSITION: f64 = 1e6;
/// How far in front of its wall a wall's view is cut by default, meters.
pub const DEFAULT_DISTANCE: f64 = 1.0;
/// Shortest stretch of a wall that gets a view by default, meters.
pub const DEFAULT_MIN_LENGTH: f64 = 0.3;
/// Largest distance and minimum length a marker takes, meters.
pub const MAX_DISTANCE: f64 = 100.0;

/// What a marker's views show.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum FaceMode {
    /// Up to four sides at right angles, seen from the marker.
    #[default]
    Directions,
    /// One view per wall of the space with at least `min_length` of it
    /// along the space, each cut `distance` in front of its wall (never
    /// behind the marker).
    Walls { distance: f64, min_length: f64 },
}

impl FaceMode {
    /// The walls mode with its default distance and minimum length.
    pub fn walls() -> Self {
        FaceMode::Walls { distance: DEFAULT_DISTANCE, min_length: DEFAULT_MIN_LENGTH }
    }

    pub fn is_directions(&self) -> bool {
        matches!(self, FaceMode::Directions)
    }

    /// Finite values within [`MAX_DISTANCE`].
    pub fn is_valid(&self) -> bool {
        match *self {
            FaceMode::Directions => true,
            FaceMode::Walls { distance, min_length } => {
                (0.0..=MAX_DISTANCE).contains(&distance) && (0.0..=MAX_DISTANCE).contains(&min_length)
            }
        }
    }
}

/// What a view of a marker shows: a side of the marker, or a wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum Face {
    Side(Side),
    Wall(ElementId),
}

/// Files of format 36 wrote the side alone (`side: North`); those read as
/// that side's face.
impl<'de> Deserialize<'de> for Face {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        enum Written {
            Side(Side),
            Wall(ElementId),
            North,
            East,
            South,
            West,
        }
        Ok(match Written::deserialize(deserializer)? {
            Written::Side(side) => Face::Side(side),
            Written::Wall(wall) => Face::Wall(wall),
            Written::North => Face::Side(Side::North),
            Written::East => Face::Side(Side::East),
            Written::South => Face::Side(Side::South),
            Written::West => Face::Side(Side::West),
        })
    }
}

/// A marker of interior elevations in a plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteriorMarker {
    /// Where it stands in plan; the views look from here.
    pub position: DVec2,
    /// The room it was placed in: it names the views and gives their
    /// height. `None` in a space with no room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room: Option<ElementId>,
    /// Turn of the four sides counter-clockwise, degrees (0: the sides
    /// face north, east, south and west of the plan).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation_deg: f64,
    /// Four directions, or a view per wall.
    #[serde(default, skip_serializing_if = "FaceMode::is_directions")]
    pub mode: FaceMode,
}

fn is_zero(x: &f64) -> bool {
    *x == 0.0
}

impl InteriorMarker {
    /// Finite and within [`MAX_POSITION`].
    pub fn is_valid(&self) -> bool {
        self.position.is_finite()
            && self.position.abs().max_element() <= MAX_POSITION
            && self.rotation_deg.is_finite()
            && self.mode.is_valid()
    }

    /// Whether `face` is of the kind the marker's mode shows.
    pub fn takes(&self, face: Face) -> bool {
        matches!((self.mode, face), (FaceMode::Directions, Face::Side(_)) | (FaceMode::Walls { .. }, Face::Wall(_)))
    }

    /// The unit direction a view of `side` looks along.
    pub fn look(&self, side: Side) -> DVec2 {
        side.look(self.rotation_deg)
    }
}

/// A side of a marker: the wall its view shows, seen from the marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Side {
    North,
    East,
    South,
    West,
}

impl Side {
    /// Clockwise from north, the order views are numbered in.
    pub const ALL: [Side; 4] = [Side::North, Side::East, Side::South, Side::West];

    /// The name commands use: `north`, `east`, `south`, `west`.
    pub fn code(self) -> &'static str {
        match self {
            Side::North => "north",
            Side::East => "east",
            Side::South => "south",
            Side::West => "west",
        }
    }

    /// The side named `code` (any case).
    pub fn from_code(code: &str) -> Option<Side> {
        Side::ALL.into_iter().find(|s| s.code().eq_ignore_ascii_case(code.trim()))
    }

    /// The unit direction a view of this side looks along, the four sides
    /// turned `rotation_deg` counter-clockwise.
    pub fn look(self, rotation_deg: f64) -> DVec2 {
        let base = match self {
            Side::North => DVec2::Y,
            Side::East => DVec2::X,
            Side::South => DVec2::NEG_Y,
            Side::West => DVec2::NEG_X,
        };
        DVec2::from_angle(rotation_deg.to_radians()).rotate(base)
    }
}

impl Project {
    /// The interior elevations that go with element `id`: those of the
    /// marker it is, and those showing the wall it is (detail views of them
    /// included), in id order.
    pub fn interior_views_of(&self, id: ElementId) -> Vec<ViewId> {
        self.views
            .values()
            .filter(|v| match v.kind {
                ViewKind::InteriorElevation { marker, face, .. } => marker == id || face == Face::Wall(id),
                _ => false,
            })
            .map(|v| v.id)
            .collect()
    }

    /// The interior elevation markers placed in room `room`.
    pub fn markers_in_room(&self, room: ElementId) -> impl Iterator<Item = ElementId> + '_ {
        self.elements.values().filter_map(move |e| match &e.kind {
            ElementKind::InteriorMarker(m) if m.room == Some(room) => Some(e.id),
            _ => None,
        })
    }

    /// Where a view of `face` of `marker` looks from, and the unit
    /// direction it looks along. A side: from the marker, along the side. A
    /// wall: square on to the face it shows (`left`: the face to the left of
    /// the wall's line from start to end, else the right one; the chord's,
    /// when curved), from the marker's foot on a plane the mode's `distance`
    /// in front of that face. The plane passes through the marker instead
    /// when it stands in front of the wall but nearer than that. `None`
    /// when the wall is gone or has no length.
    pub fn interior_frame(&self, marker: &InteriorMarker, face: Face, left: bool) -> Option<(DVec2, DVec2)> {
        let wall = match face {
            Face::Side(side) => return Some((marker.position, marker.look(side))),
            Face::Wall(id) => id,
        };
        let e = self.element(wall)?;
        let w = e.as_wall()?;
        let dir = (w.end - w.start).try_normalize()?;
        // Towards the viewer: the side of the chord the face shown is on.
        let out = if left { dir.perp() } else { -dir.perp() };
        let (face_left, face_right) = self.wall_faces(e.type_id, w.location_line);
        // Distances towards the viewer from the chord: the marker, the face
        // at the chord, and how far the wall's middle bows.
        let at = (marker.position - w.start).dot(out);
        let face = if left { face_left } else { -face_right };
        let bow = (w.at(w.length() * 0.5, 0.0) - w.start).dot(out);
        // The face's nearest and farthest points, towards the viewer.
        let (front, deep) = (face + bow.max(0.0), face + bow.min(0.0));
        let distance = match marker.mode {
            FaceMode::Walls { distance, .. } => distance,
            FaceMode::Directions => DEFAULT_DISTANCE,
        }
        .clamp(0.0, MAX_DISTANCE);
        // Where the cut plane stands: `distance` in front of the face, or
        // through a marker that stands in front of the wall nearer than that.
        let plane = if at > deep && at < front + distance { at } else { front + distance };
        let origin = marker.position + out * (plane - at);
        let look = -out;
        (origin.is_finite() && look.is_finite()).then_some((origin, look))
    }

    /// Whether `point`, on a face of wall `wall`, is on its left face (the
    /// one to the left of its line from start to end) rather than its right.
    pub fn wall_face_is_left(&self, wall: ElementId, point: DVec2) -> Option<bool> {
        let e = self.element(wall)?;
        let w = e.as_wall()?;
        let (_, across) = w.project(point);
        let (face_left, face_right) = self.wall_faces(e.type_id, w.location_line);
        across.is_finite().then_some((across - face_left).abs() <= (across - face_right).abs())
    }

    /// Repair interior elevations: drop views whose marker is gone (or is
    /// not a marker) or whose face is not of the marker's mode, set their
    /// origin and look from their marker, give them a number, reset bad
    /// modes and clear markers' rooms that are gone. Returns how many were
    /// fixed.
    pub(crate) fn repair_interior(&mut self) -> usize {
        let mut repairs = 0;
        for e in self.elements.values_mut() {
            if let ElementKind::InteriorMarker(m) = &mut e.kind
                && !m.mode.is_valid()
            {
                m.mode = FaceMode::walls();
                repairs += 1;
            }
        }
        let markers: std::collections::BTreeMap<ElementId, InteriorMarker> = self
            .elements
            .values()
            .filter_map(|e| match &e.kind {
                ElementKind::InteriorMarker(m) => Some((e.id, m.clone())),
                _ => None,
            })
            .collect();
        let before = self.views.len();
        let walls: std::collections::BTreeSet<ElementId> =
            self.elements.values().filter(|e| e.as_wall().is_some()).map(|e| e.id).collect();
        self.views.retain(|_, v| match &v.kind {
            ViewKind::InteriorElevation { marker, face, .. } => {
                let wall_ok = match face {
                    Face::Wall(w) => walls.contains(w),
                    Face::Side(_) => true,
                };
                wall_ok && markers.get(marker).is_some_and(|m| m.takes(*face))
            }
            _ => true,
        });
        repairs += before - self.views.len();
        // Sheets drop viewports of the views that went.
        let ids: std::collections::BTreeSet<ViewId> = self.views.keys().copied().collect();
        for v in self.views.values_mut() {
            if let ViewKind::Sheet { viewports, .. } = &mut v.kind {
                let n = viewports.len();
                viewports.retain(|vp| ids.contains(&vp.view));
                repairs += n - viewports.len();
            }
        }
        let frames: Vec<(ViewId, Option<(DVec2, DVec2)>)> = self
            .views
            .values()
            .filter_map(|v| match &v.kind {
                ViewKind::InteriorElevation { marker, face, left, .. } => {
                    Some((v.id, markers.get(marker).and_then(|m| self.interior_frame(m, *face, *left))))
                }
                _ => None,
            })
            .collect();
        for (id, frame) in frames {
            let Some(ViewKind::InteriorElevation { number, origin, look, .. }) =
                self.views.get_mut(&id).map(|v| &mut v.kind)
            else {
                continue;
            };
            // A wall that is gone leaves the view where it looked, finite at least.
            let want = frame.unwrap_or(if origin.is_finite() && look.is_finite() {
                (*origin, *look)
            } else {
                (DVec2::ZERO, DVec2::Y)
            });
            if (*origin, *look) != want {
                (*origin, *look) = want;
                repairs += 1;
            }
            if *number == 0 {
                *number = 1;
                repairs += 1;
            }
        }
        let rooms: std::collections::BTreeSet<ElementId> =
            self.elements.values().filter(|e| matches!(e.kind, ElementKind::Room(_))).map(|e| e.id).collect();
        for e in self.elements.values_mut() {
            if let ElementKind::InteriorMarker(m) = &mut e.kind
                && m.room.is_some_and(|r| !rooms.contains(&r))
            {
                m.room = None;
                repairs += 1;
            }
        }
        repairs
    }

    /// The face and number of each view of marker `marker` that is not a
    /// detail view.
    pub fn marker_faces(&self, marker: ElementId) -> Vec<(Face, u32, ViewId)> {
        self.views
            .values()
            .filter(|v| !v.is_detail())
            .filter_map(|v| match v.kind {
                ViewKind::InteriorElevation { marker: m, face, number, .. } if m == marker => {
                    Some((face, number, v.id))
                }
                _ => None,
            })
            .collect()
    }
}

/// The number the next view of a marker takes: the lowest not used by
/// `taken`, from 1.
pub fn next_number(taken: &[u32]) -> u32 {
    (1..).find(|n| !taken.contains(n)).unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sides_look_at_right_angles_and_turn_together() {
        assert!((Side::North.look(0.0) - DVec2::Y).length() < 1e-12);
        assert!((Side::East.look(0.0) - DVec2::X).length() < 1e-12);
        let turned = Side::North.look(90.0);
        assert!((turned - DVec2::NEG_X).length() < 1e-12, "{turned}");
        for s in Side::ALL {
            assert_eq!(Side::from_code(s.code()), Some(s));
        }
        assert_eq!(next_number(&[1, 2, 4]), 3);
        assert_eq!(next_number(&[]), 1);
    }
}
