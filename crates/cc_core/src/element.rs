use crate::{Category, ElementId, ElementTypeId, LevelId, Uid, ViewId};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// A model element instance. Coordinates are in meters; X east, Y north, Z up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: ElementId,
    pub level: LevelId,
    pub type_id: ElementTypeId,
    /// "Mark" instance parameter, shown in schedules and tags.
    #[serde(default)]
    pub mark: String,
    #[serde(default)]
    pub comments: String,
    pub kind: ElementKind,
    /// Persistent unique id (IFC GlobalId and similar).
    #[serde(default)]
    pub uid: Uid,
    /// The phase the element is built in (see [`crate::phase`]).
    #[serde(default)]
    pub phase_created: crate::PhaseId,
    /// The phase the element is demolished in (not before it is built).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_demolished: Option<crate::PhaseId>,
    /// The design option the element belongs to (`None`: the main model,
    /// shown with every option; see [`crate::design_option`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design_option: Option<crate::OptionId>,
    /// Faces painted with a material of their own (see [`crate::paint`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paints: Vec<crate::paint::FacePaint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ElementKind {
    Wall(Wall),
    Door(Hosted),
    Window(Hosted),
    Floor(Floor),
    Ceiling(Ceiling),
    Roof(Roof),
    Room(Room),
    Grid(Grid),
    Dimension(Box<Dimension>),
    Text(Box<TextNote>),
    Column(Column),
    Beam(Beam),
    Stair(Stair),
    Ramp(Ramp),
    Railing(Railing),
    Component(Component),
    Foundation(Foundation),
    Rebar(Rebar),
    /// Reinforcing bars bent to shapes of their own, placed by their center
    /// lines (bars read from IFC files).
    ShapedRebar(ShapedRebar),
    Pipe(crate::MepSegment),
    Conduit(crate::MepSegment),
    /// A cable tray run (a rectangular channel carrying circuits).
    CableTray(crate::MepSegment),
    /// A valve on a pipe.
    Valve(crate::Valve),
    /// A conductor of a lightning protection system (roof, down or earth conductor).
    LightningConductor(crate::lightning::LightningConductor),
    /// A DWG or DXF drawing placed in a 2D view (an underlay to trace over).
    CadImport(Box<crate::cad::CadImport>),
    /// A conceptual mass (massing studies).
    Mass(crate::mass::Mass),
    /// A profile swept along a path (baseboards, moldings, fascias, gutters).
    Sweep(crate::profile_sweep::Sweep),
    /// A vertical opening through floors, ceilings and roofs.
    Shaft(crate::shaft::Shaft),
    /// A parametric steel building: it owns the columns and beams of its frames.
    SteelBuilding(Box<crate::steel_building::SteelBuilding>),
    /// A parametric truss: it owns the beams and columns of its chords and web.
    Truss(Box<crate::truss::Truss>),
    /// A parametric bracing: it owns the braces of one bay of a frame.
    Bracing(Box<crate::bracing::Bracing>),
    /// Light steel or wood framing of a wall: it owns the studs and the beams of its tracks, headers and sills.
    WallFraming(Box<crate::wall_framing::WallFraming>),
    /// A steel or timber joint at one end of a column or beam (placed by its member).
    Connection(crate::connection::Connection),
    /// A terrain surface from measured points inside a boundary.
    Topography(crate::topography::Topography),
    /// A building pad graded into its terrain.
    Pad(crate::pad::Pad),
    /// A 2D line, arc, circle or ellipse drawn in one view.
    DetailLine(crate::detail::DetailLine),
    /// A 2D area of one view filled as its type says.
    FilledRegion(crate::region::Region),
    /// A 2D area of one view that hides the model's linework under it.
    MaskingRegion(crate::region::Region),
    /// The elevation of a point, written in one view.
    LevelMark(Box<crate::level_mark::LevelMark>),
    /// The count, mark, diameter, spacing and length of a reinforcement
    /// set's bars, written in one view.
    BarTag(Box<crate::bar_tag::BarTag>),
    /// A pile: free, or one of a pile cap's (see [`crate::pile`]).
    Pile(crate::pile::Pile),
    /// A 2D component placed in one view (construction details).
    DetailComponent(Box<crate::detail_component::DetailComponent>),
}

impl Element {
    pub fn category(&self) -> Category {
        match self.kind {
            ElementKind::Wall(_) => Category::Wall,
            ElementKind::Door(_) => Category::Door,
            ElementKind::Window(_) => Category::Window,
            ElementKind::Floor(_) => Category::Floor,
            ElementKind::Ceiling(_) => Category::Ceiling,
            ElementKind::Roof(_) => Category::Roof,
            ElementKind::Room(_) => Category::Room,
            ElementKind::Grid(_) => Category::Grid,
            ElementKind::Dimension(_) => Category::Dimension,
            ElementKind::Text(_) => Category::Text,
            ElementKind::Column(_) => Category::Column,
            ElementKind::Beam(_) => Category::Beam,
            ElementKind::Stair(_) => Category::Stair,
            ElementKind::Ramp(_) => Category::Ramp,
            ElementKind::Railing(_) => Category::Railing,
            ElementKind::Component(_) => Category::Component,
            ElementKind::Foundation(_) => Category::Foundation,
            ElementKind::Rebar(_) | ElementKind::ShapedRebar(_) => Category::Rebar,
            ElementKind::Pipe(_) => Category::Pipe,
            ElementKind::Conduit(_) => Category::Conduit,
            ElementKind::CableTray(_) => Category::CableTray,
            ElementKind::Valve(_) => Category::Valve,
            ElementKind::LightningConductor(_) => Category::LightningConductor,
            ElementKind::CadImport(_) => Category::CadImport,
            ElementKind::Mass(_) => Category::Mass,
            ElementKind::Sweep(_) => Category::Sweep,
            ElementKind::Shaft(_) => Category::Shaft,
            ElementKind::SteelBuilding(_) => Category::SteelBuilding,
            ElementKind::Truss(_) => Category::Truss,
            ElementKind::Bracing(_) => Category::Bracing,
            ElementKind::WallFraming(_) => Category::WallFraming,
            ElementKind::Connection(_) => Category::Connection,
            ElementKind::Topography(_) => Category::Topography,
            ElementKind::Pad(_) => Category::Pad,
            ElementKind::DetailLine(_) => Category::DetailLine,
            ElementKind::FilledRegion(_) => Category::FilledRegion,
            ElementKind::MaskingRegion(_) => Category::MaskingRegion,
            ElementKind::LevelMark(_) => Category::LevelMark,
            ElementKind::BarTag(_) => Category::BarTag,
            ElementKind::Pile(_) => Category::Pile,
            ElementKind::DetailComponent(_) => Category::DetailComponent,
        }
    }

    /// The partition of a reinforcement set or of shaped bars.
    pub fn rebar_partition(&self) -> Option<&str> {
        match &self.kind {
            ElementKind::Rebar(r) => Some(r.partition.as_str()),
            ElementKind::ShapedRebar(r) => Some(r.partition.as_str()),
            _ => None,
        }
    }

    /// The mark given by hand to a reinforcement set's or shaped bars' bars.
    pub fn rebar_bar_mark(&self) -> Option<&str> {
        match &self.kind {
            ElementKind::Rebar(r) => r.bar_mark.as_deref(),
            ElementKind::ShapedRebar(r) => r.bar_mark.as_deref(),
            _ => None,
        }
        .map(str::trim)
        .filter(|m| !m.is_empty())
    }

    /// The view that owns the element (dimensions, text notes, detail lines
    /// and imported drawings live in one view and go with it).
    pub fn owner_view(&self) -> Option<ViewId> {
        match &self.kind {
            ElementKind::Dimension(d) => Some(d.view),
            ElementKind::Text(t) => Some(t.view),
            ElementKind::CadImport(c) => Some(c.view),
            ElementKind::DetailLine(l) => Some(l.view),
            ElementKind::FilledRegion(r) | ElementKind::MaskingRegion(r) => Some(r.view),
            ElementKind::LevelMark(m) => Some(m.view),
            ElementKind::BarTag(t) => Some(t.view),
            ElementKind::DetailComponent(c) => Some(c.view),
            _ => None,
        }
    }

    /// Give a view-owned element (see [`Self::owner_view`]) another view.
    /// Returns false, changing nothing, for elements of the model.
    pub fn set_owner_view(&mut self, view: ViewId) -> bool {
        let owner = match &mut self.kind {
            ElementKind::Dimension(d) => &mut d.view,
            ElementKind::Text(t) => &mut t.view,
            ElementKind::CadImport(c) => &mut c.view,
            ElementKind::DetailLine(l) => &mut l.view,
            ElementKind::FilledRegion(r) | ElementKind::MaskingRegion(r) => &mut r.view,
            ElementKind::LevelMark(m) => &mut m.view,
            ElementKind::BarTag(t) => &mut t.view,
            ElementKind::DetailComponent(c) => &mut c.view,
            _ => return false,
        };
        *owner = view;
        true
    }

    /// The run of a pipe, conduit, cable tray or lightning conductor.
    pub fn as_segment(&self) -> Option<&crate::MepSegment> {
        match &self.kind {
            ElementKind::Pipe(s) | ElementKind::Conduit(s) | ElementKind::CableTray(s) => Some(s),
            ElementKind::LightningConductor(c) => Some(&c.run),
            _ => None,
        }
    }

    pub fn as_segment_mut(&mut self) -> Option<&mut crate::MepSegment> {
        match &mut self.kind {
            ElementKind::Pipe(s) | ElementKind::Conduit(s) | ElementKind::CableTray(s) => Some(s),
            ElementKind::LightningConductor(c) => Some(&mut c.run),
            _ => None,
        }
    }

    /// Systems (or circuits) the element belongs to.
    pub fn systems(&self) -> Vec<crate::SystemId> {
        match &self.kind {
            ElementKind::Pipe(s) | ElementKind::Conduit(s) | ElementKind::CableTray(s) => s.systems.clone(),
            ElementKind::LightningConductor(c) => c.run.systems.clone(),
            ElementKind::Component(c) => c.system.into_iter().collect(),
            _ => Vec::new(),
        }
    }

    pub fn as_wall(&self) -> Option<&Wall> {
        match &self.kind {
            ElementKind::Wall(w) => Some(w),
            _ => None,
        }
    }

    pub fn as_hosted(&self) -> Option<&Hosted> {
        match &self.kind {
            ElementKind::Door(h) | ElementKind::Window(h) => Some(h),
            _ => None,
        }
    }

    pub fn as_hosted_mut(&mut self) -> Option<&mut Hosted> {
        match &mut self.kind {
            ElementKind::Door(h) | ElementKind::Window(h) => Some(h),
            _ => None,
        }
    }

    /// The sketch an outline was drawn with (floors, ceilings, roofs and
    /// masses), to read or set.
    pub fn drawn_sketch_mut(&mut self) -> Option<&mut Option<crate::sketch::Sketch>> {
        match &mut self.kind {
            ElementKind::Floor(f) => Some(&mut f.sketch),
            ElementKind::Foundation(f) if matches!(f.shape, FoundationShape::Slab { .. }) => Some(&mut f.sketch),
            ElementKind::Ceiling(c) => Some(&mut c.sketch),
            ElementKind::Roof(r) => Some(&mut r.sketch),
            ElementKind::Mass(m) => Some(&mut m.sketch),
            ElementKind::Shaft(s) => Some(&mut s.sketch),
            ElementKind::Topography(t) => Some(&mut t.sketch),
            ElementKind::Pad(p) => Some(&mut p.sketch),
            ElementKind::FilledRegion(r) | ElementKind::MaskingRegion(r) => Some(&mut r.sketch),
            _ => None,
        }
    }

    /// The sketch an outline was drawn with, if it has one.
    pub fn drawn_sketch(&self) -> Option<&crate::sketch::Sketch> {
        match &self.kind {
            ElementKind::Floor(f) => f.sketch.as_ref(),
            ElementKind::Foundation(f) => f.sketch.as_ref(),
            ElementKind::Ceiling(c) => c.sketch.as_ref(),
            ElementKind::Roof(r) => r.sketch.as_ref(),
            ElementKind::Mass(m) => m.sketch.as_ref(),
            ElementKind::Shaft(s) => s.sketch.as_ref(),
            ElementKind::Topography(t) => t.sketch.as_ref(),
            ElementKind::Pad(p) => p.sketch.as_ref(),
            ElementKind::FilledRegion(r) | ElementKind::MaskingRegion(r) => r.sketch.as_ref(),
            _ => None,
        }
    }

    /// The outline and openings a sketch makes for this element: floors and
    /// roofs have openings, ceilings and masses an outline only.
    pub fn sketched_profile(&self) -> Option<(&[DVec2], &[Vec<DVec2>])> {
        match &self.kind {
            ElementKind::Floor(f) => Some((&f.boundary, &f.openings)),
            ElementKind::Foundation(f) => match &f.shape {
                FoundationShape::Slab { boundary } => Some((boundary, &[])),
                _ => None,
            },
            ElementKind::Ceiling(c) => Some((&c.boundary, &[])),
            ElementKind::Roof(r) => Some((&r.footprint, &r.holes)),
            ElementKind::Mass(m) => Some((&m.outline, &[])),
            ElementKind::Shaft(s) => Some((&s.outline, &[])),
            ElementKind::Topography(t) => Some((&t.boundary, &[])),
            ElementKind::Pad(p) => Some((&p.outline, &[])),
            ElementKind::FilledRegion(r) | ElementKind::MaskingRegion(r) => Some((&r.outline, &r.openings)),
            _ => None,
        }
    }

    /// Drop a sketch that no longer makes the outline (edited without it), or
    /// is invalid; returns whether it did.
    pub fn drop_stale_sketch(&mut self) -> bool {
        let stale = match (self.drawn_sketch(), self.sketched_profile()) {
            (Some(s), Some((outline, openings))) => !s.is_valid() || !s.makes(outline, openings),
            _ => false,
        };
        if stale && let Some(s) = self.drawn_sketch_mut() {
            *s = None;
        }
        stale
    }

    /// Sketch boundary for floors, ceilings and roofs.
    pub fn sketch(&self) -> Option<&[DVec2]> {
        match &self.kind {
            ElementKind::Floor(f) => Some(&f.boundary),
            ElementKind::Ceiling(c) => Some(&c.boundary),
            ElementKind::Roof(r) => Some(&r.footprint),
            _ => None,
        }
    }

    /// Translate the element in plan. Hosted elements move with their host.
    pub fn translate(&mut self, delta: DVec2) {
        let t = crate::transform::PlanTransform::translate(delta);
        if let Some(Some(sketch)) = self.drawn_sketch_mut() {
            sketch.transform(&t);
        }
        self.paints.iter_mut().for_each(|p| p.translate(delta));
        match &mut self.kind {
            ElementKind::Wall(w) => {
                w.start += delta;
                w.end += delta;
            }
            ElementKind::Floor(f) => {
                f.boundary.iter_mut().chain(f.openings.iter_mut().flatten()).for_each(|p| *p += delta)
            }
            ElementKind::Ceiling(c) => c.boundary.iter_mut().for_each(|p| *p += delta),
            ElementKind::Roof(r) => {
                r.footprint.iter_mut().chain(r.holes.iter_mut().flatten()).for_each(|p| *p += delta)
            }
            ElementKind::Room(r) => r.position += delta,
            ElementKind::Grid(g) => {
                g.start += delta;
                g.end += delta;
            }
            ElementKind::Dimension(d) => {
                d.a += delta;
                d.b += delta;
                if let DimensionKind::Angular { vertex, .. } = &mut d.kind {
                    *vertex += delta;
                }
            }
            ElementKind::LevelMark(m) => m.position += delta,
            // A cap's piles go back to their places (see `crate::pile`).
            ElementKind::Pile(p) => p.position += delta,
            ElementKind::BarTag(t) => {
                t.position += delta;
                if let Some(l) = &mut t.leader {
                    *l += delta;
                }
            }
            ElementKind::DetailComponent(c) => c.position += delta,
            ElementKind::Text(t) => {
                t.position += delta;
                for l in &mut t.leaders {
                    l.end += delta;
                    if let Some(e) = &mut l.elbow {
                        *e += delta;
                    }
                }
            }
            ElementKind::Column(c) => c.position += delta,
            ElementKind::Beam(b) => {
                b.start += delta;
                b.end += delta;
            }
            ElementKind::Stair(s) => s.start += delta,
            ElementKind::Ramp(r) => r.start += delta,
            ElementKind::Railing(r) => r.path.iter_mut().for_each(|p| *p += delta),
            ElementKind::Sweep(s) => s.path.iter_mut().for_each(|p| *p += delta),
            ElementKind::Shaft(s) => s.outline.iter_mut().for_each(|p| *p += delta),
            ElementKind::SteelBuilding(b) => b.origin += delta,
            ElementKind::Truss(t) => {
                t.start += delta;
                t.end += delta;
            }
            ElementKind::Bracing(b) => {
                b.start += delta;
                b.end += delta;
            }
            // Placed by its wall.
            ElementKind::WallFraming(_) => {}
            ElementKind::Topography(t) => {
                t.boundary.iter_mut().for_each(|p| *p += delta);
                t.points.iter_mut().for_each(|p| *p += delta.extend(0.0));
            }
            ElementKind::Pad(p) => p.outline.iter_mut().for_each(|q| *q += delta),
            ElementKind::Component(c) => c.position += delta,
            ElementKind::Pipe(s) | ElementKind::Conduit(s) | ElementKind::CableTray(s) => {
                s.start += delta;
                s.end += delta;
            }
            ElementKind::LightningConductor(c) => {
                c.run.start += delta;
                c.run.end += delta;
            }
            ElementKind::Foundation(f) => f.shape.points_mut().into_iter().for_each(|p| *p += delta),
            ElementKind::CadImport(c) => c.position += delta,
            ElementKind::DetailLine(l) => l.curve.transform(&t),
            ElementKind::FilledRegion(r) | ElementKind::MaskingRegion(r) => r.points_mut().for_each(|p| *p += delta),
            ElementKind::Mass(m) => m.outline.iter_mut().chain(m.top.iter_mut().flatten()).for_each(|p| *p += delta),
            ElementKind::ShapedRebar(r) => r.points_mut().for_each(|p| *p += delta.extend(0.0)),
            // Placed by their hosts.
            ElementKind::Door(_)
            | ElementKind::Window(_)
            | ElementKind::Rebar(_)
            | ElementKind::Valve(_)
            | ElementKind::Connection(_) => {}
        }
    }
}

/// How the top of a wall is defined.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum WallTop {
    /// `height` above the base.
    #[default]
    Unconnected,
    /// At a level's elevation plus `offset`; follows the level when it moves.
    Level { level: LevelId, offset: f64 },
}

/// Which line of the wall the drawn `start`-`end` line follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LocationLine {
    #[default]
    Centerline,
    CoreCenter,
    FinishExterior,
    FinishInterior,
    CoreExterior,
    CoreInterior,
}

impl LocationLine {
    pub const ALL: [LocationLine; 6] = [
        LocationLine::Centerline,
        LocationLine::CoreCenter,
        LocationLine::FinishExterior,
        LocationLine::FinishInterior,
        LocationLine::CoreExterior,
        LocationLine::CoreInterior,
    ];

    pub fn key(self) -> &'static str {
        match self {
            LocationLine::Centerline => "loc-centerline",
            LocationLine::CoreCenter => "loc-core-center",
            LocationLine::FinishExterior => "loc-finish-exterior",
            LocationLine::FinishInterior => "loc-finish-interior",
            LocationLine::CoreExterior => "loc-core-exterior",
            LocationLine::CoreInterior => "loc-core-interior",
        }
    }
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Wall {
    /// Location line start point (see `location_line`).
    pub start: DVec2,
    pub end: DVec2,
    /// Offset of the wall base from its level elevation.
    pub base_offset: f64,
    /// Unconnected height (used when `top` is [`WallTop::Unconnected`]).
    pub height: f64,
    #[serde(default)]
    pub top: WallTop,
    #[serde(default)]
    pub location_line: LocationLine,
    /// The wall bounds rooms.
    #[serde(default = "yes")]
    pub room_bounding: bool,
    /// The wall is load-bearing.
    #[serde(default)]
    pub structural: bool,
    /// The start (end) may join other walls; when off it stays square.
    #[serde(default = "yes")]
    pub join_start: bool,
    #[serde(default = "yes")]
    pub join_end: bool,
    /// An arc wall: `tan(sweep / 4)` of its location line, positive when it
    /// turns counter-clockwise from `start` to `end` (see [`crate::arc`]);
    /// 0 for a straight wall.
    #[serde(default)]
    pub bulge: f64,
    /// The roof, ceiling or floor the top is attached to: under it the wall
    /// rises or drops to its underside, following its slope; elsewhere it
    /// keeps its own top.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attach_top: Option<ElementId>,
    /// Adjustments to the grid of a curtain wall type (ignored by other types).
    #[serde(default, skip_serializing_if = "crate::curtain::CurtainEdits::is_empty")]
    pub curtain: crate::curtain::CurtainEdits,
}

impl Default for Wall {
    fn default() -> Self {
        Self {
            start: DVec2::ZERO,
            end: DVec2::X,
            base_offset: 0.0,
            height: 3.0,
            top: WallTop::Unconnected,
            location_line: LocationLine::Centerline,
            room_bounding: true,
            structural: false,
            join_start: true,
            join_end: true,
            bulge: 0.0,
            attach_top: None,
            curtain: Default::default(),
        }
    }
}

impl Wall {
    /// The arc of the location line (`None` for a straight wall).
    pub fn arc(&self) -> Option<crate::arc::Arc> {
        crate::arc::Arc::from_bulge(self.start, self.end, self.bulge)
    }

    /// Length of the location line (along the arc for arc walls).
    pub fn length(&self) -> f64 {
        match self.arc() {
            Some(a) => a.length(),
            None => self.start.distance(self.end),
        }
    }

    /// Unit direction from start to end (X axis if degenerate): the chord of an arc wall.
    pub fn direction(&self) -> DVec2 {
        (self.end - self.start).try_normalize().unwrap_or(DVec2::X)
    }

    /// Left-hand normal of the chord.
    pub fn normal(&self) -> DVec2 {
        self.direction().perp()
    }

    /// Direction of the location line `distance` along it.
    pub fn tangent_at(&self, distance: f64) -> DVec2 {
        match self.arc() {
            Some(a) => a.tangent(distance),
            None => self.direction(),
        }
    }

    /// Left-hand normal of the location line `distance` along it.
    pub fn normal_at(&self, distance: f64) -> DVec2 {
        self.tangent_at(distance).perp()
    }

    pub fn point_at(&self, distance: f64) -> DVec2 {
        self.at(distance, 0.0)
    }

    /// The point `along` the location line and `across` to its left.
    pub fn at(&self, along: f64, across: f64) -> DVec2 {
        match self.arc() {
            Some(a) => a.at(along, across),
            None => self.start + self.direction() * along + self.normal() * across,
        }
    }

    /// Distance along the wall of the projection of `p`, and the signed
    /// perpendicular distance (positive on the left).
    pub fn project(&self, p: DVec2) -> (f64, f64) {
        if let Some(a) = self.arc() {
            return a.locate(p);
        }
        let d = p - self.start;
        (d.dot(self.direction()), d.dot(self.normal()))
    }

    /// The location line moved `distance` to its left (concentric for arc
    /// walls, which keep their bulge).
    pub fn offset_line(&self, distance: f64) -> (DVec2, DVec2) {
        (self.at(0.0, distance), self.at(self.length(), distance))
    }
}

/// A door or window hosted by a wall.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hosted {
    pub host: ElementId,
    /// Distance from the host wall start to the opening center, along the wall.
    pub offset: f64,
    /// Sill height above the wall base (0 for doors).
    pub sill_height: f64,
    /// Swing/facing to the wall's right side instead of the left.
    #[serde(default)]
    pub flip_facing: bool,
    /// Hinge at the opposite jamb.
    #[serde(default)]
    pub flip_hand: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Floor {
    /// Boundary sketch (closed polygon, either winding).
    pub boundary: Vec<DVec2>,
    /// Offset of the top face from the level elevation.
    pub height_offset: f64,
    /// Openings through the slab (shafts, stairwells): closed outlines
    /// inside the boundary, apart from each other.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub openings: Vec<Vec<DVec2>>,
    /// Openings cut by shafts, kept in step with them by every edit (see
    /// [`crate::shaft`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cuts: Vec<Vec<DVec2>>,
    /// The curves the outline was drawn with (see [`crate::sketch`]); dropped
    /// when the outline is edited without them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sketch: Option<crate::sketch::Sketch>,
    /// A structural slab: one that takes reinforcement.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub structural: bool,
}

impl Floor {
    /// A floor over `boundary` with no openings.
    pub fn new(boundary: Vec<DVec2>, height_offset: f64) -> Self {
        Self { boundary, height_offset, openings: Vec::new(), cuts: Vec::new(), sketch: None, structural: false }
    }

    /// Its openings and the shafts' cuts.
    pub fn all_openings(&self) -> impl Iterator<Item = &Vec<DVec2>> {
        self.openings.iter().chain(&self.cuts)
    }

    /// Plan area: the boundary less its openings, exact along the arcs,
    /// circles and ellipses of its sketch (the boundary's points are chords).
    pub fn area(&self) -> f64 {
        if let Some(drawn) = self.sketch.as_ref().filter(|s| s.is_curved()).and_then(|s| s.area().ok()) {
            // The sketch draws the boundary and its openings (it is dropped when
            // they change without it); shafts cut the rest.
            return (drawn - self.cuts.iter().map(|o| polygon_area(o)).sum::<f64>()).max(0.0);
        }
        (polygon_area(&self.boundary) - self.all_openings().map(|o| polygon_area(o)).sum::<f64>()).max(0.0)
    }
}

/// Area of a simple polygon of either winding.
fn polygon_area(p: &[DVec2]) -> f64 {
    let n = p.len();
    (0..n).map(|i| p[i].perp_dot(p[(i + 1) % n])).sum::<f64>().abs() * 0.5
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ceiling {
    pub boundary: Vec<DVec2>,
    /// Height of the ceiling's underside above the level.
    pub height_offset: f64,
    /// Openings cut by shafts, kept in step with them by every edit (see
    /// [`crate::shaft`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cuts: Vec<Vec<DVec2>>,
    /// The curves the outline was drawn with (see [`crate::sketch`]); dropped
    /// when the outline is edited without them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sketch: Option<crate::sketch::Sketch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RoofShape {
    Flat,
    /// Single slope rising towards `direction_deg` (0 = +X), slope as rise/run.
    Shed {
        slope: f64,
        direction_deg: f64,
    },
    /// Two slopes meeting at a ridge along `ridge_deg`; the eaves parallel to
    /// the ridge rise, the others become gable ends.
    Gable {
        slope: f64,
        ridge_deg: f64,
    },
    /// Every eave rises with the same slope (hips and, on long plans, a ridge).
    Hip {
        slope: f64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Roof {
    pub footprint: Vec<DVec2>,
    /// Offset of the roof base above the level.
    pub base_offset: f64,
    pub overhang: f64,
    pub shape: RoofShape,
    /// Openings through the roof (courtyards), closed outlines inside the
    /// footprint; pitched roofs slope down to their eaves too.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holes: Vec<Vec<DVec2>>,
    /// Slope of each footprint edge (edge `i` from point `i` to `i + 1`) that
    /// differs from the shape's, for gable and hip roofs: `Some(0.0)` makes
    /// the edge a gable end, `Some(s)` an eave rising at `s`; missing or
    /// `None` entries follow the shape.
    /// Openings cut by shafts, kept in step with them by every edit (see
    /// [`crate::shaft`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cuts: Vec<Vec<DVec2>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edge_slopes: Vec<Option<f64>>,
    /// The curves the outline was drawn with (see [`crate::sketch`]); dropped
    /// when the outline is edited without them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sketch: Option<crate::sketch::Sketch>,
}

impl Roof {
    /// A roof over `footprint` with no openings.
    pub fn new(footprint: Vec<DVec2>, base_offset: f64, overhang: f64, shape: RoofShape) -> Self {
        Self {
            footprint,
            base_offset,
            overhang,
            shape,
            holes: Vec::new(),
            cuts: Vec::new(),
            edge_slopes: Vec::new(),
            sketch: None,
        }
    }

    /// The slope set on footprint edge `i`, if any.
    pub fn edge_slope(&self, i: usize) -> Option<f64> {
        self.edge_slopes.get(i).copied().flatten()
    }

    /// Plan area: the footprint less its openings.
    pub fn area(&self) -> f64 {
        (polygon_area(&self.footprint) - self.holes.iter().chain(&self.cuts).map(|h| polygon_area(h)).sum::<f64>())
            .max(0.0)
    }
}

/// A room: the space enclosed by room-bounding walls around `position`.
/// Its boundary is derived from the walls (see `cc_geom`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Room {
    /// A point inside the room; also where its tag is drawn.
    pub position: DVec2,
    pub number: String,
    pub name: String,
    /// Height of the room above its level (upper limit).
    pub height: f64,
}

/// A named grid line (datum), shown in every plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub start: DVec2,
    pub end: DVec2,
    pub name: String,
}

/// A text note in one view, in that view's drawing coordinates (like
/// dimensions), optionally with leaders pointing at what it describes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextNote {
    pub view: ViewId,
    /// Insertion point: the top left corner of the text, before rotation.
    pub position: DVec2,
    /// The text; line breaks start new lines.
    pub text: String,
    /// Wrapping width in paper millimeters for multiline text; `None` for
    /// single-line text (only explicit line breaks break it).
    #[serde(default)]
    pub width: Option<f64>,
    /// Rotation in degrees counter-clockwise.
    #[serde(default)]
    pub rotation_deg: f64,
    #[serde(default)]
    pub leaders: Vec<Leader>,
}

/// A leader line from a text to what it points at: one segment, or two
/// when it has an elbow (text → elbow → end).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Leader {
    /// The point it points at, where the symbol is drawn.
    pub end: DVec2,
    /// Bend point splitting the leader in two segments.
    #[serde(default)]
    pub elbow: Option<DVec2>,
    #[serde(default)]
    pub symbol: LeaderSymbol,
}

/// The mark at the end of a leader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LeaderSymbol {
    /// Filled arrowhead.
    #[default]
    Arrow,
    /// Open arrowhead (two strokes).
    OpenArrow,
    /// Outlined arrowhead.
    Triangle,
    /// Small filled circle.
    Dot,
    Circle,
    Square,
    FilledSquare,
    /// Slanted stroke (architectural tick).
    Tick,
    None,
}

impl LeaderSymbol {
    pub const ALL: [LeaderSymbol; 9] = [
        LeaderSymbol::Arrow,
        LeaderSymbol::OpenArrow,
        LeaderSymbol::Triangle,
        LeaderSymbol::Dot,
        LeaderSymbol::Circle,
        LeaderSymbol::Square,
        LeaderSymbol::FilledSquare,
        LeaderSymbol::Tick,
        LeaderSymbol::None,
    ];

    /// Translation key of the symbol's name.
    pub fn key(self) -> &'static str {
        match self {
            LeaderSymbol::Arrow => "leader-arrow",
            LeaderSymbol::OpenArrow => "leader-open-arrow",
            LeaderSymbol::Triangle => "leader-triangle",
            LeaderSymbol::Dot => "leader-dot",
            LeaderSymbol::Circle => "leader-circle",
            LeaderSymbol::Square => "leader-square",
            LeaderSymbol::FilledSquare => "leader-filled-square",
            LeaderSymbol::Tick => "leader-tick",
            LeaderSymbol::None => "leader-none",
        }
    }
}

/// A dimension in one view, in that view's drawing coordinates. What `a`,
/// `b` and `offset` mean depends on the [`DimensionKind`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dimension {
    pub view: ViewId,
    pub a: DVec2,
    pub b: DVec2,
    /// Linear kinds: distance of the dimension line from `a`, positive on
    /// the left of the measured direction. Angular: radius of the arc.
    pub offset: f64,
    #[serde(default)]
    pub kind: DimensionKind,
    /// Elements the ends `a` and `b` follow (associative dimension).
    #[serde(default)]
    pub refs: [Option<crate::DimRef>; 2],
}

/// How a dimension measures.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum DimensionKind {
    /// Distance from `a` to `b`, along `a`-`b`.
    #[default]
    Aligned,
    /// Distance from `a` to `b` measured along `dir` (a unit vector,
    /// horizontal or vertical when placed).
    Linear { dir: DVec2 },
    /// Angle at `vertex` between the rays through `a` and through `b`.
    /// `rays` hold a second point of each measured line (the vertex, in the
    /// frame of that line's element), so the vertex follows the elements.
    Angular {
        vertex: DVec2,
        #[serde(default)]
        rays: [Option<crate::DimRef>; 2],
    },
    /// Radius of the arc centered at `a` through `b`.
    Radial,
    /// Diameter of the circle centered at `a` through `b`.
    Diameter,
    /// Distance from the origin `a` to `b` measured along `dir` (a unit
    /// vector), drawn as a running (ordinate) dimension: the dimensions of
    /// a set share the origin and the line, and each shows its running
    /// total from the origin.
    Ordinate { dir: DVec2 },
}

impl Dimension {
    pub fn length(&self) -> f64 {
        self.a.distance(self.b)
    }

    /// Direction the dimension measures along (linear kinds), if defined.
    pub fn direction(&self) -> Option<DVec2> {
        match self.kind {
            DimensionKind::Linear { dir } | DimensionKind::Ordinate { dir } => dir.try_normalize(),
            DimensionKind::Aligned => (self.b - self.a).try_normalize(),
            _ => None,
        }
    }

    /// Every element reference: the ends and, for angles, the rays.
    pub fn all_refs_mut(&mut self) -> impl Iterator<Item = &mut Option<crate::DimRef>> {
        let rays: &mut [Option<crate::DimRef>] = match &mut self.kind {
            DimensionKind::Angular { rays, .. } => rays,
            _ => &mut [],
        };
        self.refs.iter_mut().chain(rays.iter_mut())
    }

    /// Whether any end or ray follows an element.
    pub fn is_associated(&self) -> bool {
        let rays = match &self.kind {
            DimensionKind::Angular { rays, .. } => rays.iter().any(Option::is_some),
            _ => false,
        };
        rays || self.refs.iter().any(Option::is_some)
    }

    /// The measured value: meters for lengths, radians for angles.
    pub fn value(&self) -> f64 {
        match self.kind {
            DimensionKind::Aligned => self.length(),
            DimensionKind::Linear { dir } | DimensionKind::Ordinate { dir } => {
                (self.b - self.a).dot(dir.normalize_or_zero()).abs()
            }
            DimensionKind::Angular { vertex, .. } => (self.a - vertex).angle_to(self.b - vertex).abs(),
            DimensionKind::Radial => self.length(),
            DimensionKind::Diameter => 2.0 * self.length(),
        }
    }
}

/// A column placed at a point, from its base to its top constraint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub position: DVec2,
    /// Rotation of the section in plan, degrees counter-clockwise.
    pub rotation_deg: f64,
    pub base_offset: f64,
    /// Height when `top` is unconnected.
    pub height: f64,
    pub top: WallTop,
    /// A load-bearing column; an architectural one (off) is cut by the walls it is joined with.
    #[serde(default = "yes")]
    pub structural: bool,
}

/// Which line across a beam's section its `start`-`end` location line follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BeamLateral {
    /// The middle of the section.
    #[default]
    Center,
    /// The left face (looking from the start to the end).
    Left,
    /// The right face.
    Right,
}

impl BeamLateral {
    pub const ALL: [BeamLateral; 3] = [BeamLateral::Center, BeamLateral::Left, BeamLateral::Right];

    pub fn key(self) -> &'static str {
        match self {
            BeamLateral::Center => "just-center",
            BeamLateral::Left => "just-left",
            BeamLateral::Right => "just-right",
        }
    }

    /// The other side, as seen in a mirror.
    pub fn mirrored(self) -> Self {
        match self {
            BeamLateral::Center => BeamLateral::Center,
            BeamLateral::Left => BeamLateral::Right,
            BeamLateral::Right => BeamLateral::Left,
        }
    }
}

/// Which height of a beam's section sits at its location elevation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BeamVertical {
    #[default]
    Top,
    Center,
    Bottom,
}

impl BeamVertical {
    pub const ALL: [BeamVertical; 3] = [BeamVertical::Top, BeamVertical::Center, BeamVertical::Bottom];

    pub fn key(self) -> &'static str {
        match self {
            BeamVertical::Top => "just-top",
            BeamVertical::Center => "just-center",
            BeamVertical::Bottom => "just-bottom",
        }
    }
}

/// A horizontal beam between two points. Its location line is at the level
/// elevation plus `top_offset`; `vertical` picks which height of the section
/// sits there (the top by default) and `lateral` which line across it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Beam {
    pub start: DVec2,
    pub end: DVec2,
    pub top_offset: f64,
    /// Location elevation of the end above the level when it differs from
    /// the start's (a sloped beam).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_offset: Option<f64>,
    #[serde(default)]
    pub lateral: BeamLateral,
    #[serde(default)]
    pub vertical: BeamVertical,
    /// The start (end) may join structural columns and other beams; when off it stays square.
    #[serde(default = "yes")]
    pub join_start: bool,
    #[serde(default = "yes")]
    pub join_end: bool,
    /// Bulge of an arc beam (`tan(sweep / 4)`, positive when it turns
    /// counter-clockwise from `start` to `end`; see [`crate::arc`]); 0 for a
    /// straight beam. An arc beam is level: it has no `end_offset`.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub bulge: f64,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

impl Beam {
    /// Rise of the location line from the start to the end.
    pub fn rise(&self) -> f64 {
        self.end_offset.map_or(0.0, |end| end - self.top_offset)
    }

    /// Length along the slope, or along the arc of an arc beam.
    pub fn true_length(&self) -> f64 {
        match self.arc() {
            Some(arc) => arc.length(),
            None => self.length().hypot(self.rise()),
        }
    }

    /// The arc of the location line (`None` for a straight beam).
    pub fn arc(&self) -> Option<crate::arc::Arc> {
        crate::arc::Arc::from_bulge(self.start, self.end, self.bulge)
    }

    /// A beam whose ends may join.
    pub fn new(start: DVec2, end: DVec2, top_offset: f64) -> Self {
        Self {
            start,
            end,
            top_offset,
            end_offset: None,
            lateral: BeamLateral::Center,
            vertical: BeamVertical::Top,
            join_start: true,
            join_end: true,
            bulge: 0.0,
        }
    }

    /// Distance between the ends (the chord of an arc beam).
    pub fn length(&self) -> f64 {
        self.start.distance(self.end)
    }
}

/// Where a foundation is in plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FoundationShape {
    /// A pad centered at `position`, its width along the rotated X axis.
    Pad { position: DVec2, rotation_deg: f64 },
    /// A strip centered on the line `start`-`end`, or on an arc between them
    /// when `bulge` is not 0 (`tan(sweep / 4)`, positive when it turns
    /// counter-clockwise from `start` to `end`; see [`crate::arc`]).
    Strip {
        start: DVec2,
        end: DVec2,
        #[serde(default)]
        bulge: f64,
    },
    /// A slab with this boundary.
    Slab { boundary: Vec<DVec2> },
}

impl FoundationShape {
    pub fn points_mut(&mut self) -> Vec<&mut DVec2> {
        match self {
            FoundationShape::Pad { position, .. } => vec![position],
            FoundationShape::Strip { start, end, .. } => vec![start, end],
            FoundationShape::Slab { boundary } => boundary.iter_mut().collect(),
        }
    }

    pub fn points(&self) -> Vec<DVec2> {
        match self {
            FoundationShape::Pad { position, .. } => vec![*position],
            FoundationShape::Strip { start, end, .. } => vec![*start, *end],
            FoundationShape::Slab { boundary } => boundary.clone(),
        }
    }

    /// The centerline arc of an arc strip (`None` for straight strips and other shapes).
    pub fn arc(&self) -> Option<crate::arc::Arc> {
        match self {
            FoundationShape::Strip { start, end, bulge } => crate::arc::Arc::from_bulge(*start, *end, *bulge),
            _ => None,
        }
    }

    /// Length of a strip's centerline (along the arc for arc strips); 0 for other shapes.
    pub fn strip_length(&self) -> f64 {
        match self {
            FoundationShape::Strip { start, end, .. } => self.arc().map_or(start.distance(*end), |a| a.length()),
            _ => 0.0,
        }
    }
}

/// A foundation: its top is `top_offset` above the level; the type gives its
/// size and depth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Foundation {
    pub shape: FoundationShape,
    pub top_offset: f64,
    /// The curves a slab's boundary was drawn with (see [`crate::sketch`]);
    /// dropped when the boundary is edited without them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sketch: Option<crate::sketch::Sketch>,
}

impl Foundation {
    /// A foundation of `shape`, its top `top_offset` above its level.
    pub fn new(shape: FoundationShape, top_offset: f64) -> Self {
        Self { shape, top_offset, sketch: None }
    }

    /// Plan area of a slab, exact along the arcs and circles of its sketch
    /// (the boundary's points are chords); `None` for pads and strips.
    pub fn slab_area(&self) -> Option<f64> {
        let FoundationShape::Slab { boundary } = &self.shape else { return None };
        let drawn = self.sketch.as_ref().filter(|s| s.is_curved()).and_then(|s| s.area().ok());
        Some(drawn.unwrap_or_else(|| polygon_area(boundary)))
    }
}

/// Where a set of longitudinal bars lies in its host's section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BarFace {
    /// Along the top of a beam.
    Top,
    /// Along the bottom of a beam.
    #[default]
    Bottom,
    /// Around a column's section (corners first).
    Perimeter,
}

impl BarFace {
    pub fn key(self) -> &'static str {
        match self {
            BarFace::Top => "bars-top",
            BarFace::Bottom => "bars-bottom",
            BarFace::Perimeter => "bars-perimeter",
        }
    }
}

/// The face of a wall a layer of bars lies near: its exterior (left) face
/// or its interior (right) one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum WallSide {
    #[default]
    Exterior,
    Interior,
}

impl WallSide {
    pub const ALL: [WallSide; 2] = [WallSide::Exterior, WallSide::Interior];

    pub fn key(self) -> &'static str {
        match self {
            WallSide::Exterior => "wall-side-exterior",
            WallSide::Interior => "wall-side-interior",
        }
    }

    /// 1 towards the wall's left (exterior) face, -1 towards its right.
    pub fn sign(self) -> f64 {
        match self {
            WallSide::Exterior => 1.0,
            WallSide::Interior => -1.0,
        }
    }
}

/// The spacing of a wall's horizontal bars up to a height above its base;
/// the last zone of a set runs to the top of the wall.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpacingZone {
    pub up_to: f64,
    pub spacing: f64,
}

/// How an end of a wall's vertical bar is bent: `across` the wall (towards
/// the outside of its own face when positive, towards the other face when
/// negative; 0 for a straight end), then `along` (back beside the bar when
/// positive, on past its end when negative; 0 for none). An L hook has only
/// `across`; a bar turned into the slab and up again has both.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct BarEnd {
    #[serde(default)]
    pub across: f64,
    #[serde(default)]
    pub along: f64,
}

impl BarEnd {
    /// Longest leg of a bent end, meters.
    pub const MAX_LEG: f64 = 5.0;

    pub fn is_straight(&self) -> bool {
        self.across == 0.0 && self.along == 0.0
    }

    pub fn is_valid(&self) -> bool {
        let leg = |v: f64| v.is_finite() && v.abs() <= Self::MAX_LEG;
        // A leg along the bar needs one across it first.
        leg(self.across) && leg(self.along) && (self.along == 0.0 || self.across != 0.0)
    }

    /// Length of the legs.
    pub fn length(&self) -> f64 {
        self.across.abs() + self.along.abs()
    }
}

/// Limits of wall reinforcement.
pub mod wall_bars {
    /// Least spacing of bars, meters.
    pub const MIN_SPACING: f64 = 0.02;
    /// Most spacing zones of one set of horizontal bars.
    pub const MAX_ZONES: usize = 32;
    /// Ties are placed at most every this many bars.
    pub const MAX_EVERY: u32 = 100;
    /// The angles a tie's hooks may be bent by, degrees.
    pub const HOOK_ANGLES: std::ops::RangeInclusive<f64> = 90.0..=180.0;

    /// The hooks of a tie when none are given: 135 degrees at both ends.
    pub fn default_hooks() -> [f64; 2] {
        [135.0, 135.0]
    }
}

/// How a set of bars is laid out in its host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RebarLayout {
    /// `count` bars running along the host, on `face`.
    Longitudinal { face: BarFace, count: u32 },
    /// Closed stirrups (ties) around the section every `spacing` along the host.
    Stirrups { spacing: f64 },
    /// Side (skin) bars running along a beam on both side faces, between its
    /// top and bottom bars: `count` on each face, or with a count of 0 as
    /// many as keep them at most `spacing` apart over the height.
    Sides { count: u32, spacing: f64 },
    /// Cross-ties of a column every `spacing` up it: a straight bar with a
    /// hook at each end from every bar of its longitudinal set that is not
    /// at a corner, across the section to the bar opposite.
    Ties { spacing: f64 },
    /// Horizontal bars near one face of a wall (ring bars in an arc wall),
    /// spaced up the wall by `zones` from its base. `inner` puts them inside
    /// the vertical bars of that face instead of outside them.
    WallHorizontal {
        side: WallSide,
        zones: Vec<SpacingZone>,
        #[serde(default)]
        inner: bool,
    },
    /// Vertical bars near one face of a wall every `spacing` along it (from
    /// `offset` after its start: half a spacing interleaves a second set),
    /// from `from` to `to` above its base (`to` 0: the top), their ends bent
    /// as `bottom` and `top` say. `inner` as for horizontal bars.
    WallVertical {
        side: WallSide,
        spacing: f64,
        #[serde(default)]
        offset: f64,
        #[serde(default)]
        from: f64,
        #[serde(default)]
        to: f64,
        #[serde(default)]
        bottom: BarEnd,
        #[serde(default)]
        top: BarEnd,
        #[serde(default)]
        inner: bool,
    },
    /// Ties between the two faces of a wall: a hooked bar across it at
    /// every `every_horizontal`-th horizontal bar and `every_vertical`-th
    /// vertical bar of its exterior face. `hooks` are the angles its two
    /// ends are bent by, degrees (at the exterior face, then the interior).
    WallTies {
        every_horizontal: u32,
        every_vertical: u32,
        #[serde(default = "wall_bars::default_hooks")]
        hooks: [f64; 2],
    },
    /// Straight bars of a slab's top or bottom layer, `spacing` apart, in
    /// the direction `angle_deg` from the plan's X axis, each clipped to the
    /// slab's boundary and openings. `inner` puts them inside the bars of
    /// the other direction of that layer. `hook` bends the ends at the
    /// slab's edge by that length: towards the middle of the slab when
    /// positive (down for top bars), out of it when negative (into the wall
    /// above a top bar or below a bottom one); 0 for straight ends.
    SlabStraight {
        top: bool,
        angle_deg: f64,
        spacing: f64,
        #[serde(default)]
        inner: bool,
        #[serde(default)]
        hook: f64,
    },
    /// Ring bars of a slab's layer around `center` (default: the middle of
    /// the slab), `spacing` apart, clipped to the slab.
    SlabRing {
        top: bool,
        spacing: f64,
        #[serde(default)]
        center: Option<DVec2>,
        #[serde(default)]
        inner: bool,
    },
    /// `count` radial bars of a slab's layer around `center`, from the
    /// radius `start` to the slab's edge; with an `intermediate` radius, a
    /// shorter bar from there to the edge between each two.
    SlabRadial {
        top: bool,
        count: u32,
        #[serde(default)]
        center: Option<DVec2>,
        #[serde(default)]
        start: f64,
        #[serde(default)]
        intermediate: f64,
        #[serde(default)]
        inner: bool,
    },
    /// Bars across a strip footing (radial in an arc strip) every `spacing`
    /// along it. Bottom bars are straight; a top bar follows the top of the
    /// footing (up its slopes and flat between the faces of its beam) and
    /// its ends turn down to the bottom layer, closing the edges.
    StripTransverse { top: bool, spacing: f64 },
    /// Bars along a strip footing (ring bars in an arc strip) `spacing`
    /// apart across it, in its bottom layer or under its top, following its
    /// slopes; top bars leave the width of the beam to the beam's own.
    StripLongitudinal { top: bool, spacing: f64 },
    /// Trimming bars around each opening of a slab, in one layer: `count`
    /// bars `spacing` apart along each side of the opening, reaching
    /// `anchorage` past it, and with `diagonals` one bar across each corner.
    SlabTrim {
        top: bool,
        count: u32,
        spacing: f64,
        anchorage: f64,
        #[serde(default)]
        diagonals: bool,
    },
}

/// Limits of slab reinforcement.
pub mod slab_bars {
    /// Most radial bars of one set.
    pub const MAX_RADIAL: u32 = 2000;
    /// Most trimming bars along one side of an opening.
    pub const MAX_TRIM: u32 = 10;
    /// Longest anchorage, hook or start radius, meters.
    pub const MAX_LENGTH: f64 = 50.0;
}

/// The longitudinal bars on one face of a column's section: `count` of them
/// evenly spread, the two at the face's ends (the corners, shared with the
/// faces beside it) included, so at least 2; or with a count of 0 as many
/// as keep them at most `spacing` apart.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FaceBars {
    pub count: u32,
    #[serde(default)]
    pub spacing: f64,
}

impl FaceBars {
    /// Most bars on one face.
    pub const MAX_COUNT: u32 = 200;
    /// Most faces a set lists.
    pub const MAX_FACES: usize = 64;

    pub fn is_valid(&self) -> bool {
        self.count <= Self::MAX_COUNT && self.spacing.is_finite() && (self.count > 0 || self.spacing >= 0.02)
    }

    /// Bars strictly between the corners of a face `length` long.
    pub fn between(&self, length: f64) -> usize {
        if self.count > 0 {
            return self.count.min(Self::MAX_COUNT).saturating_sub(2) as usize;
        }
        if !(length.is_finite() && length > 0.0 && self.spacing.is_finite() && self.spacing > 0.0) {
            return 0;
        }
        (((length / self.spacing) - 1e-9).ceil().max(1.0) as usize - 1).min(Self::MAX_COUNT as usize)
    }
}

impl RebarLayout {
    /// Whether the bars run along the host (and so may be spliced).
    pub fn runs_along(&self) -> bool {
        !matches!(
            self,
            RebarLayout::Stirrups { .. }
                | RebarLayout::Ties { .. }
                | RebarLayout::WallTies { .. }
                | RebarLayout::SlabTrim { .. }
                | RebarLayout::StripTransverse { .. }
        )
    }

    /// Whether the set reinforces a wall.
    pub fn in_wall(&self) -> bool {
        matches!(
            self,
            RebarLayout::WallHorizontal { .. } | RebarLayout::WallVertical { .. } | RebarLayout::WallTies { .. }
        )
    }

    /// Whether the set reinforces a slab (a floor or a foundation slab).
    pub fn in_slab(&self) -> bool {
        matches!(
            self,
            RebarLayout::SlabStraight { .. }
                | RebarLayout::SlabRing { .. }
                | RebarLayout::SlabRadial { .. }
                | RebarLayout::SlabTrim { .. }
        )
    }

    /// Whether the set is one of a strip footing's own (across or along its footing slab).
    pub fn in_strip(&self) -> bool {
        matches!(self, RebarLayout::StripTransverse { .. } | RebarLayout::StripLongitudinal { .. })
    }

    /// Whether a strip footing may host the set: its own layouts, and those
    /// of a beam (for the beam on top of it, or its whole section).
    pub fn fits_strip(&self) -> bool {
        self.in_strip()
            || matches!(
                self,
                RebarLayout::Stirrups { .. }
                    | RebarLayout::Sides { .. }
                    | RebarLayout::Longitudinal { face: BarFace::Top | BarFace::Bottom, .. }
            )
    }

    /// Whether a slab's set lies in its top layer.
    pub fn slab_top(&self) -> Option<bool> {
        match self {
            RebarLayout::SlabStraight { top, .. }
            | RebarLayout::SlabRing { top, .. }
            | RebarLayout::SlabRadial { top, .. }
            | RebarLayout::SlabTrim { top, .. } => Some(*top),
            _ => None,
        }
    }

    /// The face of a wall the set lies near.
    pub fn wall_side(&self) -> Option<WallSide> {
        match self {
            RebarLayout::WallHorizontal { side, .. } | RebarLayout::WallVertical { side, .. } => Some(*side),
            _ => None,
        }
    }

    /// Whether the values are usable (finite and within their limits).
    pub fn is_valid(&self) -> bool {
        let spaced = |s: f64| s.is_finite() && s >= wall_bars::MIN_SPACING;
        match self {
            RebarLayout::Longitudinal { count, .. } => (1..=200).contains(count),
            RebarLayout::Stirrups { spacing } | RebarLayout::Ties { spacing } => spaced(*spacing),
            RebarLayout::Sides { count, spacing } => {
                *count <= 200 && spacing.is_finite() && (*count > 0 || spaced(*spacing))
            }
            RebarLayout::WallHorizontal { zones, .. } => {
                (1..=wall_bars::MAX_ZONES).contains(&zones.len())
                    && zones.iter().all(|z| spaced(z.spacing) && z.up_to.is_finite() && z.up_to >= 0.0)
                    && zones.windows(2).all(|w| w[0].up_to < w[1].up_to)
            }
            RebarLayout::WallVertical { spacing, offset, from, to, bottom, top, .. } => {
                spaced(*spacing)
                    && offset.is_finite()
                    && from.is_finite()
                    && to.is_finite()
                    && *from >= 0.0
                    && (*to == 0.0 || *to > *from)
                    && bottom.is_valid()
                    && top.is_valid()
            }
            RebarLayout::StripTransverse { spacing, .. } | RebarLayout::StripLongitudinal { spacing, .. } => {
                spaced(*spacing)
            }
            RebarLayout::SlabStraight { angle_deg, spacing, hook, .. } => {
                spaced(*spacing) && angle_deg.is_finite() && hook.is_finite() && hook.abs() <= slab_bars::MAX_LENGTH
            }
            RebarLayout::SlabRing { spacing, center, .. } => spaced(*spacing) && center.is_none_or(|c| c.is_finite()),
            RebarLayout::SlabRadial { count, center, start, intermediate, .. } => {
                let radius = |r: f64| r.is_finite() && (0.0..=slab_bars::MAX_LENGTH).contains(&r);
                (1..=slab_bars::MAX_RADIAL).contains(count)
                    && center.is_none_or(|c| c.is_finite())
                    && radius(*start)
                    && radius(*intermediate)
            }
            RebarLayout::SlabTrim { count, spacing, anchorage, .. } => {
                (1..=slab_bars::MAX_TRIM).contains(count)
                    && spaced(*spacing)
                    && anchorage.is_finite()
                    && (0.0..=slab_bars::MAX_LENGTH).contains(anchorage)
            }
            RebarLayout::WallTies { every_horizontal, every_vertical, hooks } => {
                (1..=wall_bars::MAX_EVERY).contains(every_horizontal)
                    && (1..=wall_bars::MAX_EVERY).contains(every_vertical)
                    && hooks.iter().all(|a| wall_bars::HOOK_ANGLES.contains(a))
            }
        }
    }
}

/// Lap splices: a bar longer than `max_length` (the stock length) is made of
/// pieces overlapping by `lap`, the splices of neighbouring bars staggered.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Splice {
    pub max_length: f64,
    pub lap: f64,
}

impl Splice {
    /// Shortest stock length and lap accepted, meters.
    pub const MIN_LENGTH: f64 = 1.0;
    pub const MIN_LAP: f64 = 0.05;

    /// Usable: finite, a stock of at least [`Self::MIN_LENGTH`] and a lap
    /// from [`Self::MIN_LAP`] to half the stock.
    pub fn is_valid(&self) -> bool {
        self.max_length.is_finite()
            && self.lap.is_finite()
            && self.max_length >= Self::MIN_LENGTH
            && self.lap >= Self::MIN_LAP
            && self.lap <= self.max_length * 0.5
    }

    /// The pieces of bar number `index` of a set, `length` long: `(from,
    /// to)` along it. A bar within the stock length is one piece; a longer
    /// one is cut in the fewest equal pieces, and odd bars are cut half a
    /// piece off the even ones so that neighbours never splice together.
    pub fn pieces(&self, length: f64, index: usize) -> Vec<(f64, f64)> {
        if !self.is_valid() || !length.is_finite() || length <= self.max_length + 1e-9 {
            return vec![(0.0, length)];
        }
        // `n` pieces overlapping by `lap` cover `n * pitch + lap`.
        let n = (((length - self.lap) / (self.max_length - self.lap)) - 1e-9).ceil().clamp(2.0, 10_000.0);
        let pitch = (length - self.lap) / n;
        let n = n as usize;
        let shift = if index % 2 == 1 { pitch * 0.5 } else { 0.0 };
        let mut cuts: Vec<f64> = (0..n).map(|i| shift + pitch * i as f64).collect();
        if shift > 0.0 {
            cuts.insert(0, 0.0);
        }
        (0..cuts.len()).map(|i| (cuts[i], cuts.get(i + 1).map_or(length, |next| next + self.lap))).collect()
    }
}

/// A set of reinforcing bars in a beam, column, wall or slab (`host`), `cover`
/// from its faces; the type gives the bar diameter. The bars follow their host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rebar {
    pub host: ElementId,
    pub layout: RebarLayout,
    pub cover: f64,
    /// Lap splices of the bars running along a beam, or the lifts of a
    /// column's vertical bars (none: whole bars).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub splice: Option<Splice>,
    /// A column's longitudinal bars face by face, in the order of the faces
    /// of its section (empty: `count` bars spread around it).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub faces: Vec<FaceBars>,
    /// Bars along the host (a wall's horizontal bars, those along a strip
    /// footing or a beam) run half a lap past each end where another host
    /// carries on in the same line, as the walls of a cylinder or the
    /// strips of a ring footing do: the bars of the two then lap by a
    /// whole one at the joint, instead of meeting end to end.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub lap_at_joins: bool,
    /// A column's stirrups as several closed rectangles that overlap, one
    /// around each of the rectangles a section with only square corners is
    /// made of (two in an L or a T, three in a U), instead of one stirrup
    /// following the whole outline.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub overlapping: bool,
    /// Heights above a column's base where its vertical bars are cut, rising:
    /// the bars of each lift start at one and run a lap past the next (empty:
    /// whole bars, or the equal lifts of `splice`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lifts: Vec<f64>,
    /// The part of the structure the set belongs to (e.g. "FOUNDATION",
    /// "WALL"), which schedules filter and group by; empty for none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub partition: String,
    /// A mark given by hand to all the set's bars, instead of the one the
    /// project's marking gives their shape (see [`crate::bar_marks`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_mark: Option<String>,
}

impl Rebar {
    /// Most heights a set's bars are cut at.
    pub const MAX_LIFTS: usize = 64;

    /// A set in `host` with nothing but its layout and cover.
    pub fn new(host: ElementId, layout: RebarLayout, cover: f64) -> Rebar {
        Rebar {
            host,
            layout,
            cover,
            splice: None,
            faces: Vec::new(),
            lap_at_joins: false,
            overlapping: false,
            lifts: Vec::new(),
            partition: String::new(),
            bar_mark: None,
        }
    }

    /// Whether the heights of `lifts` are usable: positive, finite and rising.
    pub fn lifts_are_valid(lifts: &[f64]) -> bool {
        lifts.len() <= Self::MAX_LIFTS
            && lifts.iter().all(|h| h.is_finite() && *h > 0.0)
            && lifts.windows(2).all(|w| w[0] < w[1])
    }

    /// Whether the values are usable (finite and within their limits).
    pub fn is_valid(&self) -> bool {
        self.layout.is_valid()
            && self.cover.is_finite()
            && self.cover >= 0.0
            && Self::lifts_are_valid(&self.lifts)
            && texts_fit(&self.partition, self.bar_mark.as_deref())
    }

    /// The pieces `(from, to)` of a vertical bar of a column `height` high
    /// cut at `lifts`: each runs from a cut (or the base) to `lap` past the
    /// next one (or to the top). Cuts at or above the top are ignored.
    pub fn lift_pieces(lifts: &[f64], height: f64, lap: f64) -> Vec<(f64, f64)> {
        let lap = if lap.is_finite() { lap.max(0.0) } else { 0.0 };
        let cuts: Vec<f64> = lifts.iter().copied().filter(|h| *h > 1e-6 && *h < height - 1e-6).collect();
        let mut out = Vec::with_capacity(cuts.len() + 1);
        let mut from = 0.0;
        for cut in cuts.into_iter().take(Self::MAX_LIFTS) {
            if cut <= from {
                continue;
            }
            out.push((from, (cut + lap).min(height)));
            from = cut;
        }
        out.push((from, height));
        out
    }
}

/// Reinforcing bars of any shape (bars bent to a schedule's shape codes,
/// read from IFC files): each bar is its center line, in plan x/y and z
/// above the element's level (meters), closed when its ends meet; the type
/// gives the diameter. Unlike [`Rebar`] they have no host and keep their
/// place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapedRebar {
    pub bars: Vec<Vec<DVec3>>,
    /// Property and quantity sets read from an IFC file (shape code, bar
    /// marks, grade), written back on export.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<crate::properties::PropertySet>,
    /// The part of the structure the bars belong to, as [`Rebar::partition`].
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub partition: String,
    /// A mark given by hand to all the bars, as [`Rebar::bar_mark`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_mark: Option<String>,
}

/// Whether a partition and a bar mark are short enough to keep.
fn texts_fit(partition: &str, bar_mark: Option<&str>) -> bool {
    let fits = |t: &str| t.chars().count() <= crate::bar_marks::MAX_MARK_TEXT;
    fits(partition) && bar_mark.is_none_or(fits)
}

impl ShapedRebar {
    /// At most this many bars in one element (the ties of a tank wall are
    /// a set of over a thousand).
    pub const MAX_BARS: usize = 20_000;
    /// At most this many points along one bar.
    pub const MAX_POINTS: usize = 2000;
    /// At most this many points in all the bars of one element.
    pub const MAX_TOTAL_POINTS: usize = 400_000;

    /// Every point of every bar.
    pub fn points_mut(&mut self) -> impl Iterator<Item = &mut DVec3> {
        self.bars.iter_mut().flatten()
    }

    /// Whether the bars are usable: one to [`Self::MAX_BARS`] of them, each
    /// with two to [`Self::MAX_POINTS`] finite points and some length, and
    /// no more than [`Self::MAX_TOTAL_POINTS`] points in all.
    pub fn is_valid(&self) -> bool {
        (1..=Self::MAX_BARS).contains(&self.bars.len())
            && self.bars.iter().map(Vec::len).sum::<usize>() <= Self::MAX_TOTAL_POINTS
            && self.bars.iter().all(|b| {
                (2..=Self::MAX_POINTS).contains(&b.len())
                    && b.iter().all(|p| p.is_finite())
                    && b.windows(2).any(|w| w[0].distance(w[1]) > 1e-6)
            })
            && texts_fit(&self.partition, self.bar_mark.as_deref())
    }
}

/// A stair starting at `start` (bottom of the first riser, center of the
/// width) going up along `direction_deg`; `layout` gives its plan shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stair {
    pub start: DVec2,
    pub direction_deg: f64,
    pub base_offset: f64,
    /// Height when `top` is unconnected.
    pub height: f64,
    pub top: WallTop,
    #[serde(default)]
    pub layout: crate::StairLayout,
}

/// A ramp starting at `start` (the middle of its foot) going up along
/// `direction_deg`; `layout` gives its plan shape and slope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ramp {
    pub start: DVec2,
    pub direction_deg: f64,
    pub base_offset: f64,
    /// Height when `top` is unconnected.
    pub height: f64,
    pub top: WallTop,
    #[serde(default)]
    pub layout: crate::RampLayout,
}

/// A railing along an open path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Railing {
    pub path: Vec<DVec2>,
    pub base_offset: f64,
    /// The stair side the railing follows: its path and rise come from the
    /// stair (`path` keeps the last one, for when the stair is gone).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stair: Option<crate::StairRailing>,
}

impl Railing {
    pub fn length(&self) -> f64 {
        self.path.windows(2).map(|w| w[0].distance(w[1])).sum()
    }
}

/// A placed parametric component (see [`crate::component`]).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Component {
    /// Insertion point in plan.
    pub position: DVec2,
    /// Rotation in plan, degrees counter-clockwise.
    pub rotation_deg: f64,
    /// Elevation of the insertion point above the level.
    pub base_offset: f64,
    /// Values of instance parameters that differ from the type.
    #[serde(default)]
    pub overrides: std::collections::BTreeMap<String, f64>,
    /// Property and quantity sets of the object (from IFC import).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<crate::properties::PropertySet>,
    /// The system or circuit it is connected to (an electrical device's circuit).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<crate::SystemId>,
    /// Values of instance text, choice and material parameters that differ from the type.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub text_overrides: std::collections::BTreeMap<String, String>,
    /// Hotspots moved away from where the type has them (component frame).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub hotspots: std::collections::BTreeMap<String, [f64; 3]>,
}
