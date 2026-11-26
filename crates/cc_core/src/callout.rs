//! Crop regions and callouts.
//!
//! A view of the model drawn in 2D (a floor or ceiling plan, a section or
//! an elevation) may have a `crop`: a rectangle in its own drawing
//! coordinates (meters on the view plane) outside which nothing of the
//! model is drawn. A *callout* is a mark in a parent view standing for a
//! detail view of a region of it: the detail view keeps the parent's kind
//! (so it stays live with the model) and its crop is the callout's
//! rectangle, or the callout points to a drafting view and keeps its own
//! rectangle. The callout lives on the detail view ([`crate::View::callout`]),
//! so deleting that view removes its mark, and moving the mark is editing
//! the detail view's crop.

use crate::{Project, View, ViewId, ViewKind};
use glam::DVec2;
use serde::{Deserialize, Serialize};

/// Smallest side of a crop rectangle, meters.
pub const MIN_CROP: f64 = 0.01;
/// Largest coordinate of a crop rectangle, meters from the view's origin.
pub const MAX_CROP: f64 = 100_000.0;
/// How many parent links [`Project::sync_callouts`] follows (details of details).
pub const MAX_CHAIN: usize = 16;

/// A rectangle in a view's drawing coordinates, `min` below and left of `max`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CropBox {
    pub min: DVec2,
    pub max: DVec2,
}

impl CropBox {
    /// The rectangle with corners `a` and `b` (any two opposite corners).
    pub fn new(a: DVec2, b: DVec2) -> Self {
        CropBox { min: a.min(b), max: a.max(b) }
    }

    /// Finite, ordered, at least [`MIN_CROP`] on each side and within [`MAX_CROP`].
    pub fn is_valid(&self) -> bool {
        self.min.is_finite()
            && self.max.is_finite()
            && (self.max - self.min).cmpge(DVec2::splat(MIN_CROP)).all()
            && self.min.abs().max(self.max.abs()).max_element() <= MAX_CROP
    }

    pub fn contains(&self, p: DVec2) -> bool {
        p.cmpge(self.min).all() && p.cmple(self.max).all()
    }

    pub fn size(&self) -> DVec2 {
        self.max - self.min
    }

    pub fn center(&self) -> DVec2 {
        (self.min + self.max) * 0.5
    }

    /// The corners, counter-clockwise from `min`.
    pub fn corners(&self) -> [DVec2; 4] {
        [self.min, DVec2::new(self.max.x, self.min.y), self.max, DVec2::new(self.min.x, self.max.y)]
    }

    pub fn translated(&self, d: DVec2) -> Self {
        CropBox { min: self.min + d, max: self.max + d }
    }
}

/// A callout: the mark in `parent` standing for the view that holds it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Callout {
    /// The view the mark is drawn in.
    pub parent: ViewId,
    /// The mark's rectangle in the parent, for a callout pointing to a
    /// drafting view (which has no crop). A detail view of the model has
    /// none: its crop is the rectangle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<CropBox>,
}

impl ViewKind {
    /// Whether views of this kind can be cropped and hold callouts: floor
    /// and ceiling plans, sections and elevations.
    pub fn croppable(&self) -> bool {
        matches!(
            self,
            ViewKind::FloorPlan { .. }
                | ViewKind::CeilingPlan { .. }
                | ViewKind::Section { .. }
                | ViewKind::Elevation { .. }
                | ViewKind::InteriorElevation { .. }
        )
    }
}

impl View {
    /// The rectangle of the view's callout in its parent: the crop of a
    /// detail view, the callout's own rectangle for a drafting view.
    pub fn callout_rect(&self) -> Option<CropBox> {
        let c = self.callout?;
        if self.is_drafting() { c.rect } else { self.crop }
    }

    /// Move or resize the view's callout (the crop of a detail view);
    /// `false` when it has none.
    pub fn set_callout_rect(&mut self, rect: CropBox) -> bool {
        let drafting = self.is_drafting();
        let Some(c) = &mut self.callout else { return false };
        if drafting {
            c.rect = Some(rect);
        } else {
            self.crop = Some(rect);
        }
        true
    }

    /// A detail view of the model made from a callout (not a drafting view).
    pub fn is_detail(&self) -> bool {
        self.callout.is_some() && !self.is_drafting()
    }
}

impl Project {
    /// The views whose callouts are drawn in `parent`, in id order.
    pub fn callouts_in(&self, parent: ViewId) -> impl Iterator<Item = &View> {
        self.views.values().filter(move |v| v.callout.is_some_and(|c| c.parent == parent) && v.id != parent)
    }

    /// The parent view of the view's callout, if it has one.
    pub fn callout_parent(&self, view: ViewId) -> Option<&View> {
        self.view(self.view(view)?.callout?.parent)
    }

    /// The kind a detail view takes from its parent: a section's or
    /// elevation's whole kind (its drawing coordinates are the parent's),
    /// a plan's level (cut height, underlay and the rest stay its own).
    /// `None` when nothing changes.
    pub fn detail_kind(&self, view: &View) -> Option<ViewKind> {
        if !view.is_detail() {
            return None;
        }
        let parent = self.view(view.callout?.parent)?;
        let mut kind = view.kind.clone();
        match (&mut kind, &parent.kind) {
            (ViewKind::Section { .. }, ViewKind::Section { .. })
            | (ViewKind::Elevation { .. }, ViewKind::Elevation { .. }) => kind = parent.kind.clone(),
            // A detail of an interior elevation keeps its own crop.
            (ViewKind::InteriorElevation { .. }, ViewKind::InteriorElevation { .. }) => {
                kind = parent.kind.clone();
                if let ViewKind::InteriorElevation { auto_extents, .. } = &mut kind {
                    *auto_extents = false;
                }
            }
            (ViewKind::FloorPlan { level, .. }, ViewKind::FloorPlan { level: l, .. })
            | (ViewKind::CeilingPlan { level, .. }, ViewKind::CeilingPlan { level: l, .. }) => *level = *l,
            _ => return None,
        }
        (kind != view.kind).then_some(kind)
    }

    /// Repair crops and callouts: drop invalid crops and crops of views
    /// that cannot have one, and callouts whose parent is missing, is the
    /// view itself or cannot hold them, of another kind than a detail view,
    /// or with no valid rectangle. Returns how many were fixed.
    pub fn repair_callouts(&mut self) -> usize {
        let kinds: std::collections::BTreeMap<ViewId, ViewKind> =
            self.views.values().map(|v| (v.id, v.kind.clone())).collect();
        let mut repairs = 0;
        for view in self.views.values_mut() {
            if view.crop.is_some_and(|c| !c.is_valid() || !view.kind.croppable()) {
                view.crop = None;
                repairs += 1;
            }
            let Some(c) = view.callout else { continue };
            let parent = kinds.get(&c.parent).filter(|k| k.croppable() && c.parent != view.id);
            let ok = match parent {
                None => false,
                Some(_) if view.is_drafting() => c.rect.is_some_and(|r| r.is_valid()),
                Some(k) => {
                    std::mem::discriminant(k) == std::mem::discriminant(&view.kind)
                        && c.rect.is_none()
                        && view.crop.is_some()
                }
            };
            if !ok {
                view.callout = None;
                repairs += 1;
            }
        }
        repairs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LevelId;

    #[test]
    fn crop_boxes_are_ordered_and_bounded() {
        let b = CropBox::new(DVec2::new(3.0, 1.0), DVec2::new(1.0, 2.0));
        assert_eq!(b.min, DVec2::new(1.0, 1.0));
        assert_eq!(b.max, DVec2::new(3.0, 2.0));
        assert!(b.is_valid());
        assert!(b.contains(DVec2::new(2.0, 1.5)));
        assert!(!b.contains(DVec2::new(0.0, 1.5)));
        assert!(!CropBox::new(DVec2::ZERO, DVec2::new(1.0, 0.0)).is_valid());
        assert!(!CropBox::new(DVec2::ZERO, DVec2::new(f64::NAN, 1.0)).is_valid());
        assert!(!CropBox::new(DVec2::ZERO, DVec2::splat(1e9)).is_valid());
    }

    #[test]
    fn repair_drops_dangling_callouts() {
        let mut p = Project::empty();
        let level = LevelId(1);
        let plan = View::new(ViewId(10), "P", 50, ViewKind::floor_plan(level));
        let mut detail = View::new(ViewId(11), "D", 10, ViewKind::floor_plan(level));
        detail.crop = Some(CropBox::new(DVec2::ZERO, DVec2::ONE));
        detail.callout = Some(Callout { parent: ViewId(10), rect: None });
        let mut drafting = View::new(ViewId(12), "R", 10, ViewKind::Drafting {});
        drafting.callout = Some(Callout { parent: ViewId(99), rect: Some(CropBox::new(DVec2::ZERO, DVec2::ONE)) });
        for v in [plan, detail, drafting] {
            p.views.insert(v.id, v);
        }
        assert_eq!(p.repair_callouts(), 1, "the missing parent");
        assert!(p.views[&ViewId(11)].callout.is_some());
        assert!(p.views[&ViewId(12)].callout.is_none());
        // A detail view with no crop loses its callout too.
        if let Some(v) = p.views.get_mut(&ViewId(11)) {
            v.crop = None;
        }
        assert_eq!(p.repair_callouts(), 1);
        assert!(p.views[&ViewId(11)].callout.is_none());
    }
}
