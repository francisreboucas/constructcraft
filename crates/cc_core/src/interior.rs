//! Interior elevations: the walls of a space seen from inside it. An
//! [`InteriorMarker`] stands in a plan at a point of a space closed by walls
//! (a room's, or any other); each of its interior elevation views
//! ([`crate::ViewKind::InteriorElevation`]) looks from the marker towards
//! one of four sides at right angles, turned with the marker.
//!
//! A view stores only its marker and its side; where it looks from and in
//! which direction (`origin`, `look`) are copied from the marker by every
//! edit (see [`crate::EditCtx`]), so the view's projection needs no
//! geometry. How deep it looks and how it is cropped come from the space's
//! boundary, worked out when the view is drawn (`cc_views::interior`).

use crate::{ElementId, ElementKind, Project, ViewId, ViewKind};
use glam::DVec2;
use serde::{Deserialize, Serialize};

/// Farthest a marker stands from the origin, meters.
pub const MAX_POSITION: f64 = 1e6;

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
}

fn is_zero(x: &f64) -> bool {
    *x == 0.0
}

impl InteriorMarker {
    /// Finite and within [`MAX_POSITION`].
    pub fn is_valid(&self) -> bool {
        self.position.is_finite() && self.position.abs().max_element() <= MAX_POSITION && self.rotation_deg.is_finite()
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
    /// The interior elevations of marker `marker` (detail views of them
    /// included), in id order.
    pub fn interior_views_of(&self, marker: ElementId) -> Vec<ViewId> {
        self.views
            .values()
            .filter(|v| matches!(v.kind, ViewKind::InteriorElevation { marker: m, .. } if m == marker))
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

    /// Repair interior elevations: drop views whose marker is gone (or is
    /// not a marker), set their origin and look from their marker, give
    /// them a number, and clear markers' rooms that are gone. Returns how
    /// many were fixed.
    pub(crate) fn repair_interior(&mut self) -> usize {
        let mut repairs = 0;
        let markers: std::collections::BTreeMap<ElementId, InteriorMarker> = self
            .elements
            .values()
            .filter_map(|e| match &e.kind {
                ElementKind::InteriorMarker(m) => Some((e.id, m.clone())),
                _ => None,
            })
            .collect();
        let before = self.views.len();
        self.views.retain(|_, v| match &v.kind {
            ViewKind::InteriorElevation { marker, .. } => markers.contains_key(marker),
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
        for v in self.views.values_mut() {
            let ViewKind::InteriorElevation { marker, side, number, origin, look, .. } = &mut v.kind else { continue };
            let Some(m) = markers.get(marker) else { continue };
            let want = (m.position, m.look(*side));
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

    /// The side and number of each view of marker `marker` that is not a
    /// detail view.
    pub fn marker_sides(&self, marker: ElementId) -> Vec<(Side, u32, ViewId)> {
        self.views
            .values()
            .filter(|v| !v.is_detail())
            .filter_map(|v| match v.kind {
                ViewKind::InteriorElevation { marker: m, side, number, .. } if m == marker => {
                    Some((side, number, v.id))
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
