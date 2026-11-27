//! Parametric steel buildings: a host element that holds the parameters of
//! a steel shed (span, eave height, roof slope and shape, bays, the types of
//! its members, purlins and girts) and owns ordinary columns and beams made
//! from them. Every edit keeps the members in step (see
//! [`crate::EditCtx`]): a changed building rewrites its members in place,
//! keeping their ids while their roles exist; a member the user edited or
//! deleted is detached, and the building leaves its role alone from then on.
//!
//! Local coordinates: x across the span (from the first column line), y
//! along the building (frame `k` at `k * bay_spacing`), mapped to plan
//! through `origin` and `rotation_deg`.

use crate::member_host::{MemberHost, RoleKey, fix_type, reset, within};
use crate::{
    Beam, Category, Column, ElementId, ElementKind, ElementType, ElementTypeId, PlanTransform, Project, TypeData,
    WallTop,
};
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

/// Most members one building makes; the layout stops there.
pub const MAX_MEMBERS: usize = 4000;
/// Most purlin lines on a roof plane and girt rows on a side.
const MAX_LINES: usize = 999;
/// The width and depth of a type whose section cannot be read, meters.
pub const MISSING_SECTION: f64 = 0.2;

pub const SPAN: RangeInclusive<f64> = 2.0..=100.0;
pub const EAVE_HEIGHT: RangeInclusive<f64> = 1.0..=50.0;
pub const SLOPE: RangeInclusive<f64> = 0.0..=100.0;
pub const BAYS: RangeInclusive<u32> = 1..=100;
pub const BAY_SPACING: RangeInclusive<f64> = 1.0..=30.0;
/// Purlin and girt spacing.
pub const SPACING: RangeInclusive<f64> = 0.3..=10.0;

pub const DEFAULT_SPAN: f64 = 20.0;
pub const DEFAULT_EAVE_HEIGHT: f64 = 6.0;
pub const DEFAULT_SLOPE: f64 = 10.0;
pub const DEFAULT_BAYS: u32 = 5;
pub const DEFAULT_BAY_SPACING: f64 = 6.0;
pub const DEFAULT_SECONDARY_SPACING: f64 = 1.5;

/// The roof of the frames: two rafters meeting at mid-span, or one rising
/// toward `x = span`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FrameRoof {
    #[default]
    Gable,
    Mono,
}

impl FrameRoof {
    pub const ALL: [FrameRoof; 2] = [FrameRoof::Gable, FrameRoof::Mono];

    pub fn code(self) -> &'static str {
        match self {
            FrameRoof::Gable => "gable",
            FrameRoof::Mono => "mono",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.code().eq_ignore_ascii_case(code.trim()))
    }
}

/// What a member is in its building.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RoleKind {
    Column,
    Rafter,
    Purlin,
    Girt,
}

impl RoleKind {
    pub const ALL: [RoleKind; 4] = [RoleKind::Column, RoleKind::Rafter, RoleKind::Purlin, RoleKind::Girt];
}

/// A member's place: its kind, its frame (columns, rafters) or bay
/// (purlins, girts), and its index there (girts: `side * 1000 + row`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MemberRole {
    pub kind: RoleKind,
    pub frame: u16,
    pub index: u16,
}

impl MemberRole {
    /// Whether an element of `kind` can fill this role: a column for
    /// columns, a beam for the rest.
    pub fn fits(self, kind: &ElementKind) -> bool {
        match self.kind {
            RoleKind::Column => matches!(kind, ElementKind::Column(_)),
            RoleKind::Rafter | RoleKind::Purlin | RoleKind::Girt => matches!(kind, ElementKind::Beam(_)),
        }
    }

    /// Its key for the shared member-host code (same order as the role).
    pub fn key(self) -> RoleKey {
        RoleKey::new(self.kind as u8, self.frame, self.index)
    }

    pub fn from_key(key: RoleKey) -> Option<Self> {
        let (kind, frame, index) = key.parts();
        Some(Self { kind: *RoleKind::ALL.get(usize::from(kind))?, frame, index })
    }
}

/// Purlins or girts: their beam type and the distance between lines
/// (purlins: along the rafter's plan run; girts: up the wall).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Secondary {
    pub type_id: ElementTypeId,
    pub spacing: f64,
}

/// A member the building manages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameMember {
    pub role: MemberRole,
    pub id: ElementId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SteelBuilding {
    /// Plan point of the first frame's left column.
    pub origin: DVec2,
    /// Turn of the local x axis (across the span), degrees counter-clockwise.
    #[serde(default)]
    pub rotation_deg: f64,
    /// Column axis to column axis, meters.
    pub span: f64,
    /// Column top above the level (the rafters' underside at the column axis), meters.
    pub eave_height: f64,
    /// Roof slope, percent.
    pub slope_percent: f64,
    #[serde(default)]
    pub roof: FrameRoof,
    /// Bays between frames (frames = bays + 1).
    pub bays: u32,
    /// Frame to frame, meters.
    pub bay_spacing: f64,
    pub column_type: ElementTypeId,
    pub rafter_type: ElementTypeId,
    #[serde(default)]
    pub purlins: Option<Secondary>,
    #[serde(default)]
    pub girts: Option<Secondary>,
    /// The members it manages, by role.
    #[serde(default)]
    pub members: Vec<FrameMember>,
    /// Roles the user took over (edited or deleted members): not made again.
    #[serde(default)]
    pub detached: Vec<MemberRole>,
}

/// The width (across) and depth (up) of a column or beam type's section
/// box, [`MISSING_SECTION`] each when it cannot be read.
pub fn section_box(project: &Project, type_id: ElementTypeId) -> (f64, f64) {
    let size = project.element_type(type_id).and_then(|t| project.type_section(&t.data)).map(|s| {
        let (lo, hi) = s.outline.bbox();
        (hi.x - lo.x, hi.y - lo.y)
    });
    match size {
        Some((w, d)) if w.is_finite() && d.is_finite() && w > 0.0 && d > 0.0 => (w, d),
        _ => (MISSING_SECTION, MISSING_SECTION),
    }
}

/// The type a project gets with its first steel building.
pub fn host_type(id: ElementTypeId, name: String) -> ElementType {
    ElementType { id, name, data: TypeData::SteelBuilding {}, info: Default::default(), uid: Default::default() }
}

/// Members collected by a layout, stopping at [`MAX_MEMBERS`].
struct Members(Vec<(MemberRole, ElementTypeId, ElementKind)>);

impl Members {
    fn full(&self) -> bool {
        self.0.len() >= MAX_MEMBERS
    }

    fn push(&mut self, kind: RoleKind, frame: u32, index: usize, type_id: ElementTypeId, element: ElementKind) {
        let (Ok(frame), Ok(index)) = (u16::try_from(frame), u16::try_from(index)) else { return };
        if !self.full() {
            self.0.push((MemberRole { kind, frame, index }, type_id, element));
        }
    }
}

impl SteelBuilding {
    /// A building with the default parameters at the origin; `secondary`
    /// gives purlins and girts of that beam type.
    pub fn new(column_type: ElementTypeId, rafter_type: ElementTypeId, secondary: Option<ElementTypeId>) -> Self {
        let line = secondary.map(|type_id| Secondary { type_id, spacing: DEFAULT_SECONDARY_SPACING });
        Self {
            origin: DVec2::ZERO,
            rotation_deg: 0.0,
            span: DEFAULT_SPAN,
            eave_height: DEFAULT_EAVE_HEIGHT,
            slope_percent: DEFAULT_SLOPE,
            roof: FrameRoof::Gable,
            bays: DEFAULT_BAYS,
            bay_spacing: DEFAULT_BAY_SPACING,
            column_type,
            rafter_type,
            purlins: line,
            girts: line,
            members: Vec::new(),
            detached: Vec::new(),
        }
    }

    /// Whether every number is in its range (the first one that is not, named).
    pub fn check(&self) -> Result<(), String> {
        if !(self.origin.is_finite() && self.rotation_deg.is_finite()) {
            return Err("origin and rotation must be finite numbers".into());
        }
        within("span", self.span, &SPAN)?;
        within("eave_height", self.eave_height, &EAVE_HEIGHT)?;
        within("slope_percent", self.slope_percent, &SLOPE)?;
        within("bay_spacing", self.bay_spacing, &BAY_SPACING)?;
        if !BAYS.contains(&self.bays) {
            return Err(format!("bays must be between {} and {}", BAYS.start(), BAYS.end()));
        }
        for (name, line) in [("purlin spacing", &self.purlins), ("girt spacing", &self.girts)] {
            if let Some(s) = line {
                within(name, s.spacing, &SPACING)?;
            }
        }
        Ok(())
    }

    /// Whether the column type is a column type and the others beam types.
    pub fn check_types(&self, project: &Project) -> Result<(), String> {
        let is = |t: ElementTypeId, c: Category| project.element_type(t).is_some_and(|x| x.category() == c);
        if !is(self.column_type, Category::Column) {
            return Err(format!("type {} is not a column type", self.column_type.0));
        }
        let beams = [Some(self.rafter_type), self.purlins.map(|s| s.type_id), self.girts.map(|s| s.type_id)];
        match beams.into_iter().flatten().find(|t| !is(*t, Category::Beam)) {
            Some(t) => Err(format!("type {} is not a beam type", t.0)),
            None => Ok(()),
        }
    }

    /// Put every out-of-range number back to its default; returns how many.
    pub fn reset_out_of_range(&mut self) -> usize {
        let mut n = reset(&mut self.span, &SPAN, DEFAULT_SPAN)
            + reset(&mut self.eave_height, &EAVE_HEIGHT, DEFAULT_EAVE_HEIGHT)
            + reset(&mut self.slope_percent, &SLOPE, DEFAULT_SLOPE)
            + reset(&mut self.bay_spacing, &BAY_SPACING, DEFAULT_BAY_SPACING);
        if !BAYS.contains(&self.bays) {
            self.bays = DEFAULT_BAYS;
            n += 1;
        }
        for s in [&mut self.purlins, &mut self.girts].into_iter().flatten() {
            n += reset(&mut s.spacing, &SPACING, DEFAULT_SECONDARY_SPACING);
        }
        if !self.origin.is_finite() {
            self.origin = DVec2::ZERO;
            n += 1;
        }
        if !self.rotation_deg.is_finite() {
            self.rotation_deg = 0.0;
            n += 1;
        }
        n
    }

    /// The member types it uses.
    pub fn type_ids(&self) -> impl Iterator<Item = ElementTypeId> + '_ {
        [Some(self.column_type), Some(self.rafter_type), self.purlins.map(|s| s.type_id), self.girts.map(|s| s.type_id)]
            .into_iter()
            .flatten()
    }

    /// Frame line to frame line, meters.
    pub fn length(&self) -> f64 {
        f64::from(self.bays) * self.bay_spacing
    }

    fn axes(&self) -> (DVec2, DVec2) {
        let x = DVec2::from_angle(self.rotation_deg.to_radians());
        (x, x.perp())
    }

    /// A local point in plan.
    pub fn to_plan(&self, local: DVec2) -> DVec2 {
        let (x, y) = self.axes();
        self.origin + x * local.x + y * local.y
    }

    /// The column lines' rectangle in plan, counter-clockwise from the origin.
    pub fn footprint(&self) -> [DVec2; 4] {
        let (w, l) = (self.span, self.length());
        [DVec2::ZERO, DVec2::new(w, 0.0), DVec2::new(w, l), DVec2::new(0.0, l)].map(|p| self.to_plan(p))
    }

    /// Move, turn or mirror it. A mirror flips the local frame's handedness,
    /// so the far gable's corner becomes the origin (the frames run the
    /// other way and the footprint stays where the mirror puts it).
    pub fn transform(&mut self, t: &PlanTransform) {
        let corner = if t.mirror { DVec2::new(0.0, self.length()) } else { DVec2::ZERO };
        self.origin = t.apply(self.to_plan(corner));
        self.rotation_deg = t.apply_angle_deg(self.rotation_deg);
    }

    fn slope(&self) -> f64 {
        self.slope_percent / 100.0
    }

    /// The rafters' top above the level at local `x` (the location line).
    fn rafter_top(&self, rafter_depth: f64, x: f64) -> f64 {
        let run = match self.roof {
            FrameRoof::Gable => x.min(self.span - x),
            FrameRoof::Mono => x,
        };
        self.eave_height + rafter_depth + self.slope() * run.clamp(0.0, self.span)
    }

    /// The local x of the purlin lines: on each roof plane from the column
    /// face every `spacing`, the last at the ridge (gable: half a purlin
    /// short of mid-span, mirrored on the other half) or at the far column
    /// face (mono); a line closer than a purlin's width to the last is left out.
    fn purlin_lines(&self, half_column: f64, width: f64, spacing: f64) -> Vec<f64> {
        let plane = |from: f64, to: f64| {
            let mut xs = Vec::new();
            let mut x = from;
            while x < to - width - 1e-9 && xs.len() < MAX_LINES {
                xs.push(x);
                x += spacing;
            }
            xs.push(to.max(from));
            xs
        };
        match self.roof {
            FrameRoof::Gable => {
                let left = plane(half_column, self.span / 2.0 - width / 2.0);
                let right: Vec<f64> = left.iter().map(|x| self.span - x).collect();
                left.into_iter().chain(right).collect()
            }
            FrameRoof::Mono => plane(half_column, self.span - half_column),
        }
    }

    /// The girt rows' tops above the level: every `spacing` from `spacing`
    /// up, below the eave.
    fn girt_rows(&self, spacing: f64) -> Vec<f64> {
        (1..=MAX_LINES).map(|r| r as f64 * spacing).take_while(|z| *z < self.eave_height - 1e-9).collect()
    }

    /// Its members: role, type and element, in a fixed order (columns,
    /// rafters, purlins, girts), at most [`MAX_MEMBERS`]. Empty when a
    /// number is out of range.
    pub fn layout(&self, project: &Project) -> Vec<(MemberRole, ElementTypeId, ElementKind)> {
        let mut out = Members(Vec::new());
        if self.check().is_err() {
            return out.0;
        }
        let (_, column_depth) = section_box(project, self.column_type);
        let (_, rafter_depth) = section_box(project, self.rafter_type);
        let half_column = column_depth / 2.0;
        let plan = |x: f64, y: f64| self.to_plan(DVec2::new(x, y));
        let rafters = match self.roof {
            FrameRoof::Gable => vec![(0.0, self.span / 2.0), (self.span, self.span / 2.0)],
            FrameRoof::Mono => vec![(0.0, self.span)],
        };
        for k in 0..=self.bays {
            let y = f64::from(k) * self.bay_spacing;
            for (i, x) in [0.0, self.span].into_iter().enumerate() {
                // Turned a quarter so the section's depth runs across the span,
                // up to the rafters' top so they frame into it.
                let column = Column {
                    position: plan(x, y),
                    rotation_deg: self.rotation_deg + 90.0,
                    base_offset: 0.0,
                    height: self.eave_height + rafter_depth,
                    top: WallTop::Unconnected,
                    structural: true,
                    base_support: Default::default(),
                };
                out.push(RoleKind::Column, k, i, self.column_type, ElementKind::Column(column));
            }
            for (i, &(a, b)) in rafters.iter().enumerate() {
                let mut beam = Beam::new(plan(a, y), plan(b, y), self.rafter_top(rafter_depth, a));
                let end = self.rafter_top(rafter_depth, b);
                beam.end_offset = ((end - beam.top_offset).abs() > 1e-9).then_some(end);
                out.push(RoleKind::Rafter, k, i, self.rafter_type, ElementKind::Beam(beam));
            }
        }
        if let Some(s) = self.purlins {
            let (width, depth) = section_box(project, s.type_id);
            let lines = self.purlin_lines(half_column, width, s.spacing);
            for k in 0..self.bays {
                let (y0, y1) = (f64::from(k) * self.bay_spacing, f64::from(k + 1) * self.bay_spacing);
                for (i, &x) in lines.iter().enumerate() {
                    if out.full() {
                        break;
                    }
                    // It bears on the rafters: its bottom on their top.
                    let beam = Beam::new(plan(x, y0), plan(x, y1), self.rafter_top(rafter_depth, x) + depth);
                    out.push(RoleKind::Purlin, k, i, s.type_id, ElementKind::Beam(beam));
                }
            }
        }
        if let Some(s) = self.girts {
            let (width, _) = section_box(project, s.type_id);
            let off = half_column + width / 2.0;
            let rows = self.girt_rows(s.spacing);
            for k in 0..self.bays {
                let (y0, y1) = (f64::from(k) * self.bay_spacing, f64::from(k + 1) * self.bay_spacing);
                for (side, x) in [(0, -off), (1, self.span + off)] {
                    for (r, &z) in rows.iter().enumerate() {
                        if out.full() {
                            break;
                        }
                        let beam = Beam::new(plan(x, y0), plan(x, y1), z);
                        out.push(RoleKind::Girt, k, side * 1000 + r, s.type_id, ElementKind::Beam(beam));
                    }
                }
            }
        }
        out.0
    }
}

impl MemberHost for SteelBuilding {
    fn member_types(&self) -> Vec<ElementTypeId> {
        self.type_ids().collect()
    }

    fn member_layout(&self, project: &Project) -> Vec<(RoleKey, ElementTypeId, ElementKind)> {
        self.layout(project).into_iter().map(|(r, t, k)| (r.key(), t, k)).collect()
    }

    fn member_list(&self) -> Vec<(RoleKey, ElementId)> {
        self.members.iter().map(|m| (m.role.key(), m.id)).collect()
    }

    fn set_member_list(&mut self, members: &[(RoleKey, ElementId)]) {
        self.members =
            members.iter().filter_map(|&(k, id)| Some(FrameMember { role: MemberRole::from_key(k)?, id })).collect();
    }

    fn detached_roles(&self) -> Vec<RoleKey> {
        self.detached.iter().map(|r| r.key()).collect()
    }

    fn set_detached_roles(&mut self, roles: &[RoleKey]) {
        self.detached = roles.iter().filter_map(|k| MemberRole::from_key(*k)).collect();
    }

    fn role_fits(&self, role: RoleKey, kind: &ElementKind) -> bool {
        MemberRole::from_key(role).is_some_and(|r| r.fits(kind))
    }

    fn check_parameters(&self, project: &Project) -> Result<(), String> {
        self.check().and_then(|()| self.check_types(project))
    }

    fn repair_parameters(&mut self, project: &Project) -> Option<usize> {
        let mut n = self.reset_out_of_range();
        n += fix_type(project, &mut self.column_type, Category::Column)?;
        n += fix_type(project, &mut self.rafter_type, Category::Beam)?;
        for s in [&mut self.purlins, &mut self.girts].into_iter().flatten() {
            n += fix_type(project, &mut s.type_id, Category::Beam)?;
        }
        Some(n)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{ColumnSection, Document, Element, LevelId};
    use std::collections::BTreeSet;

    fn close(a: DVec2, b: DVec2) -> bool {
        a.distance(b) < 1e-9
    }

    fn add_type(e: &mut crate::EditCtx, data: TypeData) -> ElementTypeId {
        let id = ElementTypeId(e.alloc_id());
        e.add_type(ElementType {
            id,
            name: format!("T{}", id.0),
            data,
            info: Default::default(),
            uid: Default::default(),
        })
    }

    pub(super) fn element(id: ElementId, level: LevelId, type_id: ElementTypeId, kind: ElementKind) -> Element {
        Element {
            id,
            level,
            type_id,
            mark: String::new(),
            comments: String::new(),
            uid: Default::default(),
            kind,
            phase_created: Default::default(),
            phase_demolished: None,
            design_option: None,
            paints: Vec::new(),
            plugin_data: Default::default(),
        }
    }

    /// A document with a default gable building on the first level: columns
    /// of a 0.2 x 0.3 type, rafters, purlins and girts of a 0.1 x 0.2 beam type.
    pub(crate) fn site() -> (Document, ElementId) {
        let mut doc = Document::new(Project::from_template());
        let host = doc.edit("building", |e| {
            let level = e.project().levels[0].id;
            let material = e.project().materials[0].name.clone();
            let column = add_type(
                e,
                TypeData::Column {
                    section: ColumnSection::Rectangular { width: 0.2, depth: 0.3 },
                    material: material.clone(),
                    profile: None,
                    profile_rotation_deg: 0.0,
                    profile_mirror: false,
                },
            );
            let beam = add_type(
                e,
                TypeData::Beam {
                    width: 0.1,
                    height: 0.2,
                    material,
                    shape: Default::default(),
                    profile: None,
                    profile_rotation_deg: 0.0,
                    profile_mirror: false,
                },
            );
            let ty = add_type(e, TypeData::SteelBuilding {});
            let id = ElementId(e.alloc_id());
            let b = SteelBuilding::new(column, beam, Some(beam));
            e.add_element(element(id, level, ty, ElementKind::SteelBuilding(Box::new(b))))
        });
        (doc, host)
    }

    pub(crate) fn building(doc: &Document, host: ElementId) -> SteelBuilding {
        match doc.project.element(host).map(|e| &e.kind) {
            Some(ElementKind::SteelBuilding(b)) => (**b).clone(),
            _ => panic!("not a steel building"),
        }
    }

    fn count(layout: &[(MemberRole, ElementTypeId, ElementKind)], kind: RoleKind) -> usize {
        layout.iter().filter(|(r, _, _)| r.kind == kind).count()
    }

    fn find(
        layout: &[(MemberRole, ElementTypeId, ElementKind)],
        kind: RoleKind,
        frame: u16,
        index: u16,
    ) -> ElementKind {
        let role = MemberRole { kind, frame, index };
        layout.iter().find(|(r, _, _)| *r == role).map(|(_, _, k)| k.clone()).expect("role")
    }

    #[test]
    fn layout_counts_members() {
        let (doc, host) = site();
        let mut b = building(&doc, host);
        let p = &doc.project;
        let kinds = |l: &[(MemberRole, ElementTypeId, ElementKind)]| {
            [
                count(l, RoleKind::Column),
                count(l, RoleKind::Rafter),
                count(l, RoleKind::Purlin),
                count(l, RoleKind::Girt),
            ]
        };
        // Gable, 5 bays: 6 frames of 2 columns and 2 rafters; per bay 16 purlin
        // lines (7 every 1.5 m from the column face plus the ridge, each half)
        // and 3 girt rows (1.5, 3, 4.5 m) on each side.
        assert_eq!(kinds(&b.layout(p)), [12, 12, 80, 30]);
        // Mono: 1 rafter per frame, 15 purlin lines (14 plus the far column face).
        b.roof = FrameRoof::Mono;
        assert_eq!(kinds(&b.layout(p)), [12, 6, 75, 30]);
        (b.purlins, b.girts) = (None, None);
        assert_eq!(b.layout(p).len(), 18);
        b.roof = FrameRoof::Gable;
        assert_eq!(b.layout(p).len(), 24);
    }

    #[test]
    fn rafters_rise_from_the_columns() {
        let (doc, host) = site();
        let l = building(&doc, host).layout(&doc.project);
        // Rafter depth 0.2: the top starts 0.2 over the 6 m eave and rises
        // 10 % over half the 20 m span.
        let ElementKind::Beam(left) = find(&l, RoleKind::Rafter, 0, 0) else { panic!("a beam") };
        assert!(close(left.start, DVec2::ZERO) && close(left.end, DVec2::new(10.0, 0.0)));
        assert!((left.top_offset - 6.2).abs() < 1e-9);
        assert!((left.end_offset.unwrap_or(0.0) - 7.2).abs() < 1e-9);
        let ElementKind::Beam(right) = find(&l, RoleKind::Rafter, 1, 1) else { panic!("a beam") };
        assert!(close(right.start, DVec2::new(20.0, 6.0)) && close(right.end, DVec2::new(10.0, 6.0)));
        // Columns rise to the rafters' top, turned so their depth runs across the span.
        let ElementKind::Column(c) = find(&l, RoleKind::Column, 0, 1) else { panic!("a column") };
        assert!(close(c.position, DVec2::new(20.0, 0.0)));
        assert!((c.height - 6.2).abs() < 1e-9 && (c.rotation_deg - 90.0).abs() < 1e-9);
        // The first purlin starts at the column face (0.15 m in) and bears on the rafter.
        let ElementKind::Beam(p0) = find(&l, RoleKind::Purlin, 0, 0) else { panic!("a beam") };
        assert!(close(p0.start, DVec2::new(0.15, 0.0)) && close(p0.end, DVec2::new(0.15, 6.0)));
        assert!((p0.top_offset - (6.2 + 0.015 + 0.2)).abs() < 1e-9);
        // The ridge purlins sit half a purlin off mid-span.
        let ElementKind::Beam(ridge) = find(&l, RoleKind::Purlin, 0, 7) else { panic!("a beam") };
        assert!((ridge.start.x - 9.95).abs() < 1e-9);
        // Girts: outside the column face, rows every 1.5 m below the eave.
        let ElementKind::Beam(g) = find(&l, RoleKind::Girt, 0, 1002) else { panic!("a beam") };
        assert!((g.start.x - 20.2).abs() < 1e-9 && (g.top_offset - 4.5).abs() < 1e-9);
    }

    #[test]
    fn origin_and_rotation_place_the_frames() {
        let (doc, host) = site();
        let mut b = building(&doc, host);
        (b.origin, b.rotation_deg) = (DVec2::new(5.0, 5.0), 90.0);
        let l = b.layout(&doc.project);
        let at = |frame, index| match find(&l, RoleKind::Column, frame, index) {
            ElementKind::Column(c) => c.position,
            _ => panic!("a column"),
        };
        assert!(close(at(0, 1), DVec2::new(5.0, 25.0)), "the span runs along +y");
        assert!(close(at(1, 0), DVec2::new(-1.0, 5.0)), "the frames along -x");
        assert!(close(b.footprint()[2], DVec2::new(-25.0, 25.0)));
    }

    #[test]
    fn a_mirror_keeps_the_footprint() {
        let (doc, host) = site();
        let mut b = building(&doc, host);
        b.rotation_deg = 30.0;
        let t = PlanTransform::mirror_across(DVec2::new(3.0, 0.0), DVec2::new(0.0, 1.0));
        let want: Vec<DVec2> = b.footprint().iter().map(|p| t.apply(*p)).collect();
        b.transform(&t);
        for p in b.footprint() {
            assert!(want.iter().any(|q| close(*q, p)), "{p}");
        }
    }

    #[test]
    fn layouts_stop_at_the_limit_and_need_valid_numbers() {
        let (doc, host) = site();
        let mut b = building(&doc, host);
        (b.span, b.bays, b.eave_height) = (100.0, 100, 50.0);
        for s in [&mut b.purlins, &mut b.girts].into_iter().flatten() {
            s.spacing = 0.3;
        }
        assert_eq!(b.layout(&doc.project).len(), MAX_MEMBERS);
        b.span = 150.0;
        assert!(b.check().is_err());
        assert!(b.layout(&doc.project).is_empty());
        assert_eq!(b.reset_out_of_range(), 1);
        assert_eq!(b.span, DEFAULT_SPAN);
        b.column_type = b.rafter_type;
        assert!(b.check_types(&doc.project).is_err());
    }

    #[test]
    fn roof_codes_round_trip() {
        for r in FrameRoof::ALL {
            assert_eq!(FrameRoof::from_code(r.code()), Some(r));
        }
        assert_eq!(FrameRoof::from_code(" MONO "), Some(FrameRoof::Mono));
        assert_eq!(FrameRoof::from_code("dome"), None);
    }

    fn role(kind: RoleKind, frame: u16, index: u16) -> MemberRole {
        MemberRole { kind, frame, index }
    }

    fn member(b: &SteelBuilding, kind: RoleKind, frame: u16, index: u16) -> ElementId {
        b.members.iter().find(|m| m.role == role(kind, frame, index)).map(|m| m.id).expect("member")
    }

    fn set(doc: &mut Document, host: ElementId, f: impl FnOnce(&mut SteelBuilding)) {
        doc.edit("set", |e| {
            if let Some(ElementKind::SteelBuilding(b)) = e.element_mut(host).map(|x| &mut x.kind) {
                f(b);
            }
        });
    }

    fn column_at(doc: &Document, id: ElementId) -> DVec2 {
        match doc.project.element(id).map(|e| &e.kind) {
            Some(ElementKind::Column(c)) => c.position,
            _ => panic!("not a column"),
        }
    }

    fn columns(b: &SteelBuilding) -> usize {
        b.members.iter().filter(|m| m.role.kind == RoleKind::Column).count()
    }

    #[test]
    fn a_new_building_gets_its_members() {
        let (doc, host) = site();
        let b = building(&doc, host);
        assert_eq!(b.members.len(), 134);
        let level = doc.project.element(host).map(|e| e.level);
        for m in &b.members {
            let e = doc.project.element(m.id).expect("the member exists");
            assert!(m.role.fits(&e.kind));
            assert_eq!(Some(e.level), level);
        }
        assert_eq!(doc.project.clone().repair(), 0, "a consistent model");
    }

    #[test]
    fn a_new_span_moves_the_members_and_keeps_their_ids() {
        let (mut doc, host) = site();
        let before = building(&doc, host);
        let start = doc.project.elements.clone();
        let right = member(&before, RoleKind::Column, 0, 1);
        set(&mut doc, host, |b| b.span = 24.0);
        let after = building(&doc, host);
        assert_eq!(member(&after, RoleKind::Column, 0, 1), right);
        assert!(close(column_at(&doc, right), DVec2::new(24.0, 0.0)));
        let frames = |b: &SteelBuilding| -> Vec<ElementId> {
            b.members
                .iter()
                .filter(|m| matches!(m.role.kind, RoleKind::Column | RoleKind::Rafter))
                .map(|m| m.id)
                .collect()
        };
        assert_eq!(frames(&after), frames(&before));
        // Undo puts back the members as they were, in the same step.
        let edited = doc.project.elements.clone();
        assert!(doc.undo());
        assert!(close(column_at(&doc, right), DVec2::new(20.0, 0.0)));
        assert_eq!(building(&doc, host), before);
        assert_eq!(doc.project.elements, start);
        // And redo brings back exactly what the edit made.
        assert!(doc.redo());
        assert_eq!(doc.project.elements, edited);
    }

    #[test]
    fn moving_the_building_moves_its_members() {
        let (mut doc, host) = site();
        let right = member(&building(&doc, host), RoleKind::Column, 0, 1);
        doc.edit("move", |e| e.transform_elements(&[host], &PlanTransform::translate(DVec2::new(2.0, 3.0))));
        assert!(close(column_at(&doc, right), DVec2::new(22.0, 3.0)));
    }

    #[test]
    fn moving_the_building_with_its_members_detaches_none() {
        let (mut doc, host) = site();
        let mut ids = vec![host];
        ids.extend(building(&doc, host).members.iter().map(|m| m.id));
        doc.edit("move", |e| e.transform_elements(&ids, &PlanTransform::translate(DVec2::new(2.0, 3.0))));
        let b = building(&doc, host);
        assert!(b.detached.is_empty());
        assert_eq!(b.members.len(), 134);
        assert!(close(column_at(&doc, member(&b, RoleKind::Column, 0, 1)), DVec2::new(22.0, 3.0)));
    }

    #[test]
    fn bays_add_and_remove_frames() {
        let (mut doc, host) = site();
        let last = member(&building(&doc, host), RoleKind::Column, 5, 0);
        set(&mut doc, host, |b| b.bays = 3);
        assert_eq!(columns(&building(&doc, host)), 8);
        assert!(doc.project.element(last).is_none(), "the last frame's column is deleted");
        set(&mut doc, host, |b| b.bays = 6);
        let b = building(&doc, host);
        assert_eq!(columns(&b), 14);
        assert!(b.members.iter().all(|m| doc.project.element(m.id).is_some()));
    }

    #[test]
    fn a_member_moved_by_hand_is_left_alone() {
        let (mut doc, host) = site();
        let col = member(&building(&doc, host), RoleKind::Column, 0, 0);
        doc.edit("move", |e| e.transform_elements(&[col], &PlanTransform::translate(DVec2::new(1.0, 0.0))));
        let b = building(&doc, host);
        assert!(b.detached.contains(&role(RoleKind::Column, 0, 0)));
        assert!(b.members.iter().all(|m| m.id != col));
        set(&mut doc, host, |b| b.span = 24.0);
        assert!(close(column_at(&doc, col), DVec2::new(1.0, 0.0)), "left where the user put it");
        assert_eq!(columns(&building(&doc, host)), 11, "and not made again");
        // A new mark changes no shape: the member stays managed.
        let other = member(&building(&doc, host), RoleKind::Column, 1, 0);
        doc.edit("mark", |e| {
            if let Some(x) = e.element_mut(other) {
                x.mark = "C1".into();
            }
        });
        assert!(building(&doc, host).members.iter().any(|m| m.id == other));
    }

    #[test]
    fn a_role_detached_by_hand_keeps_its_element() {
        let (mut doc, host) = site();
        let col = member(&building(&doc, host), RoleKind::Column, 1, 1);
        set(&mut doc, host, |b| b.detached.push(role(RoleKind::Column, 1, 1)));
        assert!(doc.project.element(col).is_some(), "the element is the user's now");
        assert!(building(&doc, host).members.iter().all(|m| m.id != col));
        // Cleared again: the role gets a new member, the old element stays.
        set(&mut doc, host, |b| b.detached.clear());
        let b = building(&doc, host);
        assert_eq!(columns(&b), 12);
        assert_ne!(member(&b, RoleKind::Column, 1, 1), col);
        assert!(doc.project.element(col).is_some());
    }

    #[test]
    fn a_deleted_member_is_not_made_again() {
        let (mut doc, host) = site();
        let col = member(&building(&doc, host), RoleKind::Column, 2, 1);
        doc.edit("delete", |e| e.remove_element(col));
        set(&mut doc, host, |b| b.eave_height = 7.0);
        let b = building(&doc, host);
        assert!(b.detached.contains(&role(RoleKind::Column, 2, 1)));
        assert!(b.members.iter().all(|m| m.role != role(RoleKind::Column, 2, 1)));
        assert_eq!(columns(&b), 11);
    }

    #[test]
    fn deleting_the_building_deletes_its_members() {
        let (mut doc, host) = site();
        let ids: Vec<ElementId> = building(&doc, host).members.iter().map(|m| m.id).collect();
        doc.edit("delete", |e| e.remove_element(host));
        assert!(ids.iter().all(|id| doc.project.element(*id).is_none()));
        assert!(doc.undo());
        assert!(ids.iter().all(|id| doc.project.element(*id).is_some()));
    }

    #[test]
    fn a_copy_gets_its_own_members() {
        let (mut doc, host) = site();
        let original = building(&doc, host);
        let ids: std::collections::BTreeSet<ElementId> = original.members.iter().map(|m| m.id).collect();
        // Copied alone, and copied with its members selected too: one new
        // building with members of its own, no loose duplicates.
        for with_members in [false, true] {
            let mut selection = vec![host];
            if with_members {
                selection.extend(ids.iter().copied());
            }
            let before = doc.project.elements.len();
            let copies =
                doc.edit("copy", |e| e.copy_elements(&selection, &[PlanTransform::translate(DVec2::new(50.0, 0.0))]));
            assert_eq!(copies.len(), 1, "only the building is copied");
            let copy = building(&doc, copies[0]);
            assert_eq!(copy.members.len(), original.members.len());
            assert!(copy.members.iter().all(|m| !ids.contains(&m.id)));
            assert_eq!(doc.project.elements.len(), before + 1 + original.members.len());
            assert_eq!(building(&doc, host), original, "the original keeps its own");
            assert!(doc.undo());
        }
    }

    #[test]
    fn a_paste_after_a_cut_gets_new_members() {
        let (mut doc, host) = site();
        let clip = doc.project.copy_set(&[host]);
        assert_eq!(clip.len(), 1);
        doc.edit("cut", |e| e.remove_element(host));
        let pasted = doc.edit("paste", |e| e.paste_elements(&clip, &[PlanTransform::translate(DVec2::ZERO)], None));
        assert_eq!(pasted.len(), 1);
        let b = building(&doc, pasted[0]);
        assert_eq!(b.members.len(), 134);
        assert!(b.members.iter().all(|m| doc.project.element(m.id).is_some()));
    }

    #[test]
    fn a_paste_never_takes_over_a_detached_member() {
        let (mut doc, host) = site();
        // Copy, then move a column by hand: it is the user's now.
        let clip = doc.project.copy_set(&[host]);
        let col = member(&building(&doc, host), RoleKind::Column, 0, 0);
        doc.edit("move", |e| e.transform_elements(&[col], &PlanTransform::translate(DVec2::new(1.0, 0.0))));
        assert!(building(&doc, host).detached.contains(&role(RoleKind::Column, 0, 0)));
        let at = column_at(&doc, col);
        let pasted =
            doc.edit("paste", |e| e.paste_elements(&clip, &[PlanTransform::translate(DVec2::new(50.0, 0.0))], None));
        assert_eq!(pasted.len(), 1);
        assert!(close(column_at(&doc, col), at), "the user's column stays where it was");
        let b = building(&doc, pasted[0]);
        assert_eq!(b.members.len(), 134, "the paste gets all its own members");
        assert!(b.members.iter().all(|m| m.id != col));
        let original: BTreeSet<ElementId> = building(&doc, host).members.iter().map(|m| m.id).collect();
        assert!(b.members.iter().all(|m| !original.contains(&m.id)));
        assert_eq!(doc.project.clone().repair(), 0);
    }

    #[test]
    fn members_follow_the_host_phase() {
        let (mut doc, host) = site();
        let last = doc.project.phases.last().map(|p| p.id).expect("the template has phases");
        let ids: Vec<ElementId> = building(&doc, host).members.iter().map(|m| m.id).collect();
        let start = doc.project.elements.clone();
        doc.edit("demolish", |e| {
            if let Some(x) = e.element_mut(host) {
                x.phase_demolished = Some(last);
            }
        });
        assert!(ids.iter().all(|id| doc.project.element(*id).is_some_and(|e| e.phase_demolished == Some(last))));
        let b = building(&doc, host);
        assert!(b.detached.is_empty(), "following the host detaches nothing");
        assert_eq!(b.members.len(), 134);
        assert!(doc.undo());
        assert_eq!(doc.project.elements, start);
    }

    #[test]
    fn members_follow_the_host_design_option() {
        let (mut doc, host) = site();
        let (a, b) = doc.edit("options", |e| {
            let set = crate::OptionSetId(e.alloc_id());
            let (a, b) = (crate::OptionId(e.alloc_id()), crate::OptionId(e.alloc_id()));
            let options =
                vec![crate::DesignOption { id: a, name: "A".into() }, crate::DesignOption { id: b, name: "B".into() }];
            e.option_sets_mut().push(crate::OptionSet { id: set, name: "Set".into(), options, primary: a });
            (a, b)
        });
        let ids: Vec<ElementId> = building(&doc, host).members.iter().map(|m| m.id).collect();
        let start = doc.project.elements.clone();
        for option in [Some(b), Some(a), None] {
            doc.edit("option", |e| {
                if let Some(x) = e.element_mut(host) {
                    x.design_option = option;
                }
            });
            assert!(ids.iter().all(|id| doc.project.element(*id).is_some_and(|e| e.design_option == option)));
            assert!(building(&doc, host).detached.is_empty());
        }
        assert!(doc.undo() && doc.undo() && doc.undo());
        assert_eq!(doc.project.elements, start);
    }

    #[test]
    fn a_changed_type_regenerates() {
        let (mut doc, host) = site();
        let b = building(&doc, host);
        let rafter = member(&b, RoleKind::Rafter, 0, 0);
        doc.edit("type", |e| {
            if let Some(TypeData::Beam { height, .. }) = e.type_mut(b.rafter_type).map(|t| &mut t.data) {
                *height = 0.3;
            }
        });
        let Some(ElementKind::Beam(beam)) = doc.project.element(rafter).map(|e| &e.kind) else { panic!("a beam") };
        assert!((beam.top_offset - 6.3).abs() < 1e-9);
    }

    #[test]
    fn repair_drops_dangling_members_and_resets_bad_values() {
        let (doc, host) = site();
        let b = building(&doc, host);
        let mut p = doc.project.clone();
        p.elements.remove(&member(&b, RoleKind::Girt, 0, 0));
        if let Some(ElementKind::SteelBuilding(x)) = p.elements.get_mut(&host).map(|e| &mut e.kind) {
            x.span = f64::NAN;
            x.detached = vec![role(RoleKind::Purlin, 3, 3), role(RoleKind::Purlin, 3, 3)];
            x.column_type = x.rafter_type;
        }
        assert_eq!(p.repair(), 4);
        let Some(ElementKind::SteelBuilding(x)) = p.element(host).map(|e| &e.kind) else { panic!("kept") };
        assert_eq!(x.span, DEFAULT_SPAN);
        assert_eq!(x.members.len(), b.members.len() - 1);
        assert_eq!(x.detached.len(), 1);
        assert_eq!(p.element_type(x.column_type).map(|t| t.category()), Some(Category::Column));
        assert_eq!(p.repair(), 0);
    }

    #[test]
    fn repair_resets_a_bad_placement_and_keeps_the_building() {
        let (doc, host) = site();
        let b = building(&doc, host);
        let mut p = doc.project.clone();
        if let Some(ElementKind::SteelBuilding(x)) = p.elements.get_mut(&host).map(|e| &mut e.kind) {
            x.origin = DVec2::new(f64::NAN, 0.0);
            x.rotation_deg = f64::INFINITY;
        }
        assert_eq!(p.repair(), 2);
        let Some(ElementKind::SteelBuilding(x)) = p.element(host).map(|e| &e.kind) else { panic!("kept") };
        assert_eq!((x.origin, x.rotation_deg), (DVec2::ZERO, 0.0));
        assert_eq!(x.members, b.members, "the members stay its own");
    }
}
