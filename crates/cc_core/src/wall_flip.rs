//! Flipping a wall: its exterior and interior sides swap while its location
//! line stays where it is.
//!
//! A wall's exterior face is on the left of its location line, walked from
//! `start` to `end` (see [`crate::ElementType::location_offset`]). Flipping
//! walks the same line the other way: the ends swap (with their joins), an
//! arc keeps its circle (its bulge changes sign), a spline its curve (the
//! control points reversed), so the layers mirror about the location line —
//! a wall drawn on its exterior finish face keeps that face on the line and
//! its body moves to the other side of it.
//!
//! What is placed along the wall follows: an elevation profile and a
//! curtain wall's own grid lines, removed segments and panels are reversed
//! along it; doors, windows and niches keep their place on the wall (their
//! offset reversed, along the arc of the body centerline for an arc wall,
//! whose length may change) and face the same way in the world (facing and
//! hand both flip, as a half turn of their frame does). The wall's
//! reinforcement stays on the faces that look the same way in the world (a
//! set near the exterior face now lies near the interior one), and
//! dimensions measuring the wall or its openings keep measuring the same
//! points. A wall framing is laid out again from the new start, as after
//! any change to its wall (it has no setting to count its studs from the
//! end).
//!
//! Two things only the derived geometry knows are left to the caller (the
//! engine's `wall_flip` does them in the same edit): the `offset` that keeps
//! a vertical bar set at its stations (`cc_geom::flipped_vertical_offset`,
//! since the stations count from the face's end after joins), and the own
//! grid lines that keep a curtain wall's type grid in place when its layout
//! counted from the other end differs (a fixed distance justified at the
//! start or the end), set from [`Project::flipped_along`].

use crate::{EditCtx, ElementId, ElementKind, Project, RebarLayout, Wall, WallSide};
use std::collections::BTreeSet;

/// Why a wall cannot be flipped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlipError {
    /// The element is not a wall (or does not exist).
    NotWall,
    /// An arc wall whose body, moved to the other side of its location
    /// line, would pass through the arc's center.
    TooTight,
    /// A spline wall whose body, moved to the other side of its location
    /// line, makes no wall along its curve.
    Spline(crate::wall_spline::SplineError),
}

impl std::fmt::Display for FlipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlipError::NotWall => write!(f, "not a wall"),
            FlipError::TooTight => {
                write!(f, "the arc is too tight for the wall's body on the other side of its location line")
            }
            FlipError::Spline(e) => write!(f, "flipped, the spline makes no wall: {e}"),
        }
    }
}

impl WallSide {
    /// The other face.
    pub fn other(self) -> Self {
        match self {
            WallSide::Exterior => WallSide::Interior,
            WallSide::Interior => WallSide::Exterior,
        }
    }
}

impl Wall {
    /// The same location line walked from `end` to `start`: the ends and
    /// their joins swap, an arc's bulge changes sign, a spline's control
    /// points run backwards and a profile is reversed along the wall. What
    /// depends on the body centerline (a curtain wall's own grid) is left
    /// to [`EditCtx::flip_walls`].
    pub fn reversed(&self) -> Wall {
        let mut w = self.clone();
        std::mem::swap(&mut w.start, &mut w.end);
        std::mem::swap(&mut w.join_start, &mut w.join_end);
        // (A straight wall keeps a plain 0, not -0.)
        if w.bulge != 0.0 {
            w.bulge = -w.bulge;
        }
        if let Some(spline) = &mut w.spline {
            *spline = spline.reversed();
        }
        if let Some(profile) = &mut w.profile {
            // Profiles stand on straight walls, where `u` runs along the location line.
            *profile = profile.reversed(self.length());
        }
        w
    }
}

/// `u` along a wall `old` long, measured from the other end of the wall
/// now `new` long (arc walls scale it with their body centerline).
fn reverse_along(u: f64, old: f64, new: f64) -> f64 {
    let ratio = if old > 1e-9 && (new - old).abs() > 1e-12 * old { new / old } else { 1.0 };
    (old - u) * ratio
}

impl Project {
    /// Where a position `u` along the body centerline of wall `w` (of
    /// element `e`) lies once the wall is flipped: `L - u`, scaled by the
    /// new centerline length on an arc wall whose centerline moves.
    pub fn flipped_along(&self, e: &crate::Element, w: &Wall) -> impl Fn(f64) -> f64 + use<> {
        let (old, new) = if w.arc().is_some() {
            (self.wall_length(e, w), self.wall_length(e, &w.reversed()))
        } else {
            (w.length(), w.length())
        };
        move |u| reverse_along(u, old, new)
    }

    /// Whether wall `id` can be flipped (see [`crate::wall_flip`]).
    pub fn check_wall_flip(&self, id: ElementId) -> Result<(), FlipError> {
        let e = self.element(id).ok_or(FlipError::NotWall)?;
        let w = e.as_wall().ok_or(FlipError::NotWall)?;
        let flipped = w.reversed();
        if let Some(spline) = &flipped.spline {
            self.check_wall_spline(e.type_id, flipped.location_line, spline).map_err(FlipError::Spline)?;
        } else if let Some(arc) = flipped.arc() {
            let (left, right) = self.wall_faces(e.type_id, flipped.location_line);
            if arc.radius_at(left) <= 1e-6 || arc.radius_at(right) <= 1e-6 {
                return Err(FlipError::TooTight);
            }
        }
        Ok(())
    }
}

impl EditCtx<'_> {
    /// Flip walls `ids` (see [`crate::wall_flip`]) in this edit. Every wall
    /// is checked first: when one cannot be flipped nothing changes and its
    /// id comes back with the reason. Returns the elements changed (the
    /// walls, then their doors, windows, niches and bar sets).
    pub fn flip_walls(&mut self, ids: &[ElementId]) -> Result<Vec<ElementId>, (ElementId, FlipError)> {
        let walls: Vec<ElementId> = ids.iter().copied().collect::<BTreeSet<_>>().into_iter().collect();
        for id in &walls {
            self.project().check_wall_flip(*id).map_err(|e| (*id, e))?;
        }
        let mut changed = Vec::new();
        let mut hosted_all: BTreeSet<ElementId> = BTreeSet::new();
        for id in &walls {
            let p = self.project();
            let Some(e) = p.element(*id) else { continue };
            let Some(w) = e.as_wall() else { continue };
            let mut flipped = w.reversed();
            // Positions along the body centerline: its length changes on an
            // arc wall whose location line is not its centerline.
            let along = p.flipped_along(e, w);
            flipped.curtain.remap(&along);
            let hosted: Vec<ElementId> = p.hosted_by(*id).map(|h| h.id).collect();
            let bars: Vec<ElementId> = p.rebar_of(*id).map(|(b, _)| b.id).collect();
            if let Some(ElementKind::Wall(x)) = self.element_mut(*id).map(|e| &mut e.kind) {
                *x = flipped;
            }
            changed.push(*id);
            for h in hosted {
                if let Some(x) = self.element_mut(h).and_then(|e| e.as_hosted_mut()) {
                    x.offset = along(x.offset);
                    // A half turn of the opening's frame: it keeps facing the same way.
                    x.flip_facing = !x.flip_facing;
                    x.flip_hand = !x.flip_hand;
                    hosted_all.insert(h);
                    changed.push(h);
                }
            }
            for b in bars {
                if let Some(ElementKind::Rebar(r)) = self.element_mut(b).map(|e| &mut e.kind)
                    && let RebarLayout::WallHorizontal { side, .. } | RebarLayout::WallVertical { side, .. } =
                        &mut r.layout
                {
                    *side = side.other();
                    changed.push(b);
                }
            }
        }
        // Interior elevations of a flipped wall keep showing the same physical
        // face, now the other one by the wall's own left and right.
        let views: Vec<crate::ViewId> = self
            .project()
            .views
            .values()
            .filter(|v| match v.kind {
                crate::ViewKind::InteriorElevation { face: crate::interior::Face::Wall(w), .. } => walls.contains(&w),
                _ => false,
            })
            .map(|v| v.id)
            .collect();
        for view in views {
            if let Some(crate::ViewKind::InteriorElevation { left, .. }) = self.view_mut(view).map(|v| &mut v.kind) {
                *left = !*left;
            }
        }
        self.flip_dimension_refs(&walls.into_iter().collect(), &hosted_all);
        Ok(changed)
    }

    /// Point the dimension ends on flipped `walls` and on their `hosted`
    /// openings at the same points in their new frames.
    fn flip_dimension_refs(&mut self, walls: &BTreeSet<ElementId>, hosted: &BTreeSet<ElementId>) {
        use crate::dim_refs::RefAnchor;
        let updates: Vec<(ElementId, Box<crate::Dimension>)> = self
            .project()
            .elements
            .values()
            .filter_map(|e| {
                let ElementKind::Dimension(d) = &e.kind else { return None };
                let mut n = d.clone();
                let mut hit = false;
                for r in n.all_refs_mut().flatten() {
                    let Some(target) = r.element() else { continue };
                    r.anchor = match r.anchor {
                        RefAnchor::Line { from_end, along, across } if walls.contains(&target) => {
                            RefAnchor::Line { from_end: !from_end, along, across: -across }
                        }
                        RefAnchor::Hosted { along, across } if hosted.contains(&target) => {
                            RefAnchor::Hosted { along: -along, across: -across }
                        }
                        _ => continue,
                    };
                    hit = true;
                }
                hit.then_some((e.id, n))
            })
            .collect();
        for (id, n) in updates {
            if let Some(ElementKind::Dimension(d)) = self.element_mut(id).map(|e| &mut e.kind) {
                *d = n;
            }
        }
    }
}
