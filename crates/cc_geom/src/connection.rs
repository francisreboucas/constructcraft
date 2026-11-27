//! Steel and timber connections (see [`cc_core::connection`]): the plates
//! and bolts of each, derived from its member's geometry on every build, so
//! they follow the member. Plates are oriented boxes ([`Block`]), bolts and
//! washers cylinders ([`Rod`]). A connection whose member is missing, or
//! whose member end is not framed as its kind needs, builds nothing and
//! carries a [`ConnectionWarning`].

use crate::{BeamEnd, BeamGeom, Solid};
use cc_core::connection::{Connection, ConnectionKind, ConnectionParams, MemberEnd};
use cc_core::{Category, Element, ElementId, ElementKind, Project};
use glam::{DVec2, DVec3};
use std::collections::BTreeMap;

/// End and fin plate bolts are the plate's thickness plus this (nut and head), meters.
pub const BOLT_EXTRA: f64 = 0.02;
/// Washer diameter per bolt diameter.
pub const WASHER_RATIO: f64 = 3.0;
/// A fin plate's height per section height.
pub const FIN_HEIGHT: f64 = 0.8;
/// A hanger's sides per beam height.
pub const HANGER_SIDE: f64 = 0.6;
/// Connections drawn without a metal material in the project.
pub const STEEL_COLOR: [u8; 3] = [150, 156, 162];

/// A box: its center, two square unit axes `x` and `y` (the third is
/// `x × y`) and its size along the three.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    pub center: DVec3,
    pub x: DVec3,
    pub y: DVec3,
    pub size: DVec3,
}

impl Block {
    pub fn z(&self) -> DVec3 {
        self.x.cross(self.y)
    }

    pub fn volume(&self) -> f64 {
        self.size.x * self.size.y * self.size.z
    }

    /// The six faces, wound outward.
    pub fn faces(&self) -> Vec<Vec<DVec3>> {
        let (x, y, z) = (self.x * (self.size.x / 2.0), self.y * (self.size.y / 2.0), self.z() * (self.size.z / 2.0));
        let p = |i: f64, j: f64, k: f64| self.center + x * i + y * j + z * k;
        vec![
            vec![p(-1.0, -1.0, 1.0), p(1.0, -1.0, 1.0), p(1.0, 1.0, 1.0), p(-1.0, 1.0, 1.0)],
            vec![p(-1.0, -1.0, -1.0), p(-1.0, 1.0, -1.0), p(1.0, 1.0, -1.0), p(1.0, -1.0, -1.0)],
            vec![p(1.0, -1.0, -1.0), p(1.0, 1.0, -1.0), p(1.0, 1.0, 1.0), p(1.0, -1.0, 1.0)],
            vec![p(-1.0, -1.0, -1.0), p(-1.0, -1.0, 1.0), p(-1.0, 1.0, 1.0), p(-1.0, 1.0, -1.0)],
            vec![p(-1.0, 1.0, -1.0), p(-1.0, 1.0, 1.0), p(1.0, 1.0, 1.0), p(1.0, 1.0, -1.0)],
            vec![p(-1.0, -1.0, -1.0), p(1.0, -1.0, -1.0), p(1.0, -1.0, 1.0), p(-1.0, -1.0, 1.0)],
        ]
    }
}

/// A cylinder from `a` to `b`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rod {
    pub a: DVec3,
    pub b: DVec3,
    pub diameter: f64,
}

impl Rod {
    pub fn length(&self) -> f64 {
        self.a.distance(self.b)
    }
}

/// A bolt (or anchor bolt) and its washers.
#[derive(Debug, Clone, PartialEq)]
pub struct Bolt {
    pub shank: Rod,
    pub washers: Vec<Rod>,
}

/// A plate: one box, or several for a bent plate (a hanger's base and sides).
#[derive(Debug, Clone, PartialEq)]
pub struct Plate {
    pub blocks: Vec<Block>,
}

impl Plate {
    pub fn volume(&self) -> f64 {
        self.blocks.iter().map(Block::volume).sum()
    }
}

/// Why a connection built nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionWarning {
    /// Its member is gone.
    MissingMember,
    /// Its member end does not take this kind (or the member's section fails).
    Unsuitable,
}

impl ConnectionWarning {
    pub fn code(self) -> &'static str {
        match self {
            ConnectionWarning::MissingMember => "missing_member",
            ConnectionWarning::Unsuitable => "unsuitable",
        }
    }
}

/// The parts of one connection.
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionGeom {
    pub element: ElementId,
    pub member: ElementId,
    pub kind: ConnectionKind,
    pub plates: Vec<Plate>,
    pub bolts: Vec<Bolt>,
    pub bolt_diameter: f64,
    pub warning: Option<ConnectionWarning>,
}

impl ConnectionGeom {
    pub fn plate_volume(&self) -> f64 {
        self.plates.iter().map(Plate::volume).sum()
    }

    /// Kilograms of steel in the plates.
    pub fn plate_mass(&self) -> f64 {
        self.plate_volume() * crate::rebar::STEEL_DENSITY
    }

    pub fn bolt_count(&self) -> usize {
        self.bolts.len()
    }

    /// The length of one bolt (they are alike); 0 with none.
    pub fn bolt_length(&self) -> f64 {
        self.bolts.first().map_or(0.0, |b| b.shank.length())
    }

    /// The faces of every part.
    pub fn faces(&self) -> Vec<Vec<DVec3>> {
        let blocks = self.plates.iter().flat_map(|p| &p.blocks).flat_map(Block::faces);
        let rods = self
            .bolts
            .iter()
            .flat_map(|b| std::iter::once(&b.shank).chain(&b.washers))
            .flat_map(|r| crate::mep::cylinder(r.a, r.b, r.diameter / 2.0));
        blocks.chain(rods).collect()
    }
}

/// How the end `end` of a beam is framed (`None` for a column's base).
pub fn beam_end_join(g: &BeamGeom, end: MemberEnd) -> Option<BeamEnd> {
    match end {
        MemberEnd::Start => Some(g.start),
        MemberEnd::End => Some(g.end),
        MemberEnd::Base => None,
    }
}

/// Whether a beam end framed as `joined` takes a connection of `kind`.
pub fn suits(kind: ConnectionKind, joined: BeamEnd) -> bool {
    match kind {
        ConnectionKind::EndPlate | ConnectionKind::TimberBolts => matches!(joined, BeamEnd::Column(_)),
        ConnectionKind::FinPlate => matches!(joined, BeamEnd::Column(_) | BeamEnd::Beam),
        ConnectionKind::Hanger => joined == BeamEnd::Beam,
        ConnectionKind::BasePlate => false,
    }
}

/// The material connections are drawn in: the project's first metal.
pub fn steel_material(project: &Project) -> Option<&str> {
    project.materials.iter().find(|m| m.identity.class == cc_core::MaterialClass::Metal).map(|m| m.name.as_str())
}

/// An empty solid of connection `element`, in steel.
pub fn solid(project: &Project, element: ElementId) -> Solid {
    match steel_material(project) {
        Some(m) => Solid::of_material(element, Category::Connection, project, m),
        None => Solid::new(element, Category::Connection, STEEL_COLOR),
    }
}

/// `n` positions spread evenly over `[-half, half]` (the middle for one).
fn spread(n: u32, half: f64) -> Vec<f64> {
    match n {
        0 => Vec::new(),
        1 => vec![0.0],
        _ => (0..n).map(|i| -half + 2.0 * half * f64::from(i) / f64::from(n - 1)).collect(),
    }
}

/// `n` positions `spacing` apart, centered on 0.
fn centered(n: u32, spacing: f64) -> Vec<f64> {
    (0..n).map(|i| (f64::from(i) - f64::from(n.saturating_sub(1)) / 2.0) * spacing).collect()
}

fn flat(v: DVec2) -> DVec3 {
    DVec3::new(v.x, v.y, 0.0)
}

/// A column's base: the center of its section's box at the base, the
/// section's axes in plan and the box's size.
struct ColumnFrame {
    base: DVec3,
    x: DVec3,
    y: DVec3,
    size: DVec2,
}

fn column_frame(project: &Project, member: &Element, col: &cc_core::Column) -> Option<ColumnFrame> {
    let ty = project.element_type(member.type_id)?;
    let section = project.type_section(&ty.data)?;
    let (lo, hi) = section.outline.bbox();
    let turn = if col.rotation_deg.is_finite() { col.rotation_deg.to_radians() } else { 0.0 };
    let rot = DVec2::from_angle(turn);
    let center = col.position + rot.rotate((lo + hi) * 0.5);
    let (base, _) = project.column_span(member, col);
    let frame = ColumnFrame { base: center.extend(base), x: flat(rot), y: flat(rot.perp()), size: hi - lo };
    (frame.base.is_finite() && frame.size.is_finite()).then_some(frame)
}

/// A beam's end face: the point on the axis there, the plan direction out
/// of the beam through it, the beam's left, the section's bottom and top
/// elevations at the face, the half width and how the end is framed.
struct EndFrame {
    face: DVec2,
    out: DVec2,
    side: DVec2,
    bottom: f64,
    top: f64,
    half: f64,
    /// The section's thickness across the beam at mid-height (a web's, else the width).
    web: f64,
    joined: BeamEnd,
}

/// The material width of `section` along the horizontal line at height `y`.
fn width_at(section: &cc_core::structural_profile::SectionOutline, y: f64) -> f64 {
    let mut xs: Vec<f64> = Vec::new();
    for ring in std::iter::once(&section.outer).chain(&section.holes) {
        for (i, a) in ring.iter().enumerate() {
            let Some(b) = ring.get((i + 1) % ring.len()) else { continue };
            if (a.y <= y) != (b.y <= y) {
                xs.push(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x));
            }
        }
    }
    xs.sort_by(f64::total_cmp);
    xs.as_chunks::<2>().0.iter().map(|[a, b]| b - a).sum()
}

impl EndFrame {
    fn height(&self) -> f64 {
        self.top - self.bottom
    }

    /// The face's point at mid-height.
    fn middle(&self) -> DVec3 {
        self.face.extend((self.bottom + self.top) * 0.5)
    }
}

fn end_frame(g: &BeamGeom, end: MemberEnd) -> Option<EndFrame> {
    // Along an arc beam the frame is the tangent's at the end.
    let (u, sign) = match end {
        MemberEnd::Start => ((g.start_ext.0 + g.start_ext.1) * 0.5, -1.0),
        MemberEnd::End => ((g.end_ext.0 + g.end_ext.1) * 0.5, 1.0),
        MemberEnd::Base => return None,
    };
    let dir = g.dir_at(u);
    let out = dir * sign;
    let (bottom, top) = g.z_at(u);
    let joined = beam_end_join(g, end)?;
    let (lo, hi) = g.section.bbox();
    let web = width_at(&g.section, (lo.y + hi.y) * 0.5 + 1e-7);
    let web = if web.is_finite() && web > 0.0 { web } else { 2.0 * g.half };
    Some(EndFrame { face: g.world(u, 0.0), out, side: dir.perp(), bottom, top, half: g.half, web, joined })
}

type Parts = (Vec<Plate>, Vec<Bolt>);

fn base_plate(f: &ColumnFrame, p: &ConnectionParams) -> Parts {
    let w = if p.width > 0.0 { p.width } else { f.size.x + 2.0 * p.edge };
    let l = if p.length > 0.0 { p.length } else { f.size.y + 2.0 * p.edge };
    let t = p.thickness;
    let plate = Block { center: f.base - DVec3::Z * (t / 2.0), x: f.x, y: f.y, size: DVec3::new(w, l, t) };
    let mut bolts = Vec::new();
    for y in spread(p.rows, (l / 2.0 - p.edge).max(0.0)) {
        for x in spread(p.columns, (w / 2.0 - p.edge).max(0.0)) {
            let top = f.base + f.x * x + f.y * y;
            let shank = Rod { a: top, b: top - DVec3::Z * (t + p.anchor_length), diameter: p.bolt_diameter };
            bolts.push(Bolt { shank, washers: Vec::new() });
        }
    }
    (vec![Plate { blocks: vec![plate] }], bolts)
}

fn end_plate(f: &EndFrame, p: &ConnectionParams) -> Parts {
    let (out, side) = (flat(f.out), flat(f.side));
    let w = if p.width > 0.0 { p.width } else { 2.0 * f.half + 2.0 * p.edge };
    let l = if p.length > 0.0 { p.length } else { f.height() + 2.0 * p.edge };
    let t = p.thickness;
    let center = f.middle() + out * (t / 2.0);
    let plate = Block { center, x: side, y: DVec3::Z, size: DVec3::new(w, l, t) };
    let half_bolt = (t + BOLT_EXTRA) / 2.0;
    let mut bolts = Vec::new();
    for z in spread(p.rows, (l / 2.0 - p.edge).max(0.0)) {
        for x in spread(p.columns, (w / 2.0 - p.edge).max(0.0)) {
            let at = center + side * x + DVec3::Z * z;
            let shank = Rod { a: at - out * half_bolt, b: at + out * half_bolt, diameter: p.bolt_diameter };
            bolts.push(Bolt { shank, washers: Vec::new() });
        }
    }
    (vec![Plate { blocks: vec![plate] }], bolts)
}

fn fin_plate(f: &EndFrame, p: &ConnectionParams) -> Parts {
    let (inward, side) = (-flat(f.out), flat(f.side));
    let len = if p.length > 0.0 { p.length } else { 2.0 * p.edge + f64::from(p.columns.saturating_sub(1)) * p.spacing };
    let h = if p.width > 0.0 { p.width } else { FIN_HEIGHT * f.height() };
    let t = p.thickness;
    // Beside the web, on the beam's left.
    let mid = f.middle() + side * (f.web / 2.0 + t / 2.0);
    let plate = Block { center: mid + inward * (len / 2.0), x: inward, y: DVec3::Z, size: DVec3::new(len, h, t) };
    let half_bolt = (t + BOLT_EXTRA) / 2.0;
    let mut bolts = Vec::new();
    for i in 0..p.columns {
        let u = p.edge + f64::from(i) * p.spacing;
        for z in centered(p.rows, p.spacing) {
            let at = mid + inward * u + DVec3::Z * z;
            let shank = Rod { a: at - side * half_bolt, b: at + side * half_bolt, diameter: p.bolt_diameter };
            bolts.push(Bolt { shank, washers: Vec::new() });
        }
    }
    (vec![Plate { blocks: vec![plate] }], bolts)
}

fn timber_bolts(f: &EndFrame, p: &ConnectionParams) -> Parts {
    let (inward, side) = (-flat(f.out), flat(f.side));
    let tw = p.thickness;
    let washer = WASHER_RATIO * p.bolt_diameter;
    let mut bolts = Vec::new();
    for i in 0..p.columns {
        let u = p.edge + f64::from(i) * p.spacing;
        for z in centered(p.rows, p.spacing) {
            let at = f.middle() + inward * u + DVec3::Z * z;
            let reach = f.half + tw;
            let shank = Rod { a: at - side * reach, b: at + side * reach, diameter: p.bolt_diameter };
            let washers = [1.0, -1.0]
                .map(|s| Rod { a: at + side * (s * f.half), b: at + side * (s * reach), diameter: washer })
                .to_vec();
            bolts.push(Bolt { shank, washers });
        }
    }
    (Vec::new(), bolts)
}

fn hanger(f: &EndFrame, p: &ConnectionParams) -> Parts {
    let (inward, side) = (-flat(f.out), flat(f.side));
    let t = p.thickness;
    let len = if p.length > 0.0 { p.length } else { p.edge };
    let inner = if p.width > 0.0 { p.width } else { 2.0 * f.half };
    let side_h = HANGER_SIDE * f.height();
    let mid = flat(f.face) + inward * (len / 2.0);
    // The base under the beam across the inside width; the sides from the
    // base's bottom up, beside it (no overlap).
    let mut blocks = vec![Block {
        center: mid + DVec3::Z * (f.bottom - t / 2.0),
        x: inward,
        y: side,
        size: DVec3::new(len, inner, t),
    }];
    for s in [-1.0, 1.0] {
        blocks.push(Block {
            center: mid + side * (s * (inner + t) / 2.0) + DVec3::Z * (f.bottom - t + (side_h + t) / 2.0),
            x: inward,
            y: DVec3::Z,
            size: DVec3::new(len, side_h + t, t),
        });
    }
    let reach = inner / 2.0 + t + BOLT_EXTRA / 2.0;
    let mut bolts = Vec::new();
    for z in spread(p.rows, side_h * 0.25) {
        for u in spread(p.columns, len * 0.25) {
            let at = mid + inward * u + DVec3::Z * (f.bottom + side_h / 2.0 + z);
            let shank = Rod { a: at - side * reach, b: at + side * reach, diameter: p.bolt_diameter };
            bolts.push(Bolt { shank, washers: Vec::new() });
        }
    }
    (vec![Plate { blocks }], bolts)
}

/// The parts of a beam end connection, `None` when the end does not suit it.
fn beam_parts(kind: ConnectionKind, f: &EndFrame, p: &ConnectionParams) -> Option<Parts> {
    if !suits(kind, f.joined) {
        return None;
    }
    match kind {
        ConnectionKind::EndPlate => Some(end_plate(f, p)),
        ConnectionKind::FinPlate => Some(fin_plate(f, p)),
        ConnectionKind::TimberBolts => Some(timber_bolts(f, p)),
        ConnectionKind::Hanger => Some(hanger(f, p)),
        ConnectionKind::BasePlate => None,
    }
}

/// Connection `id` as `c` and its member make it.
pub fn connection_geom(
    project: &Project,
    beams: &BTreeMap<ElementId, BeamGeom>,
    id: ElementId,
    c: &Connection,
) -> ConnectionGeom {
    let mut g = ConnectionGeom {
        element: id,
        member: c.member,
        kind: c.kind,
        plates: Vec::new(),
        bolts: Vec::new(),
        bolt_diameter: c.params.bolt_diameter,
        warning: None,
    };
    let Some(member) = project.element(c.member) else {
        g.warning = Some(ConnectionWarning::MissingMember);
        return g;
    };
    // Hostile values (a file not yet repaired) build with the kind's defaults.
    let mut params = c.params;
    params.reset_out_of_range(c.kind);
    let p = &params;
    g.bolt_diameter = p.bolt_diameter;
    let built = match &member.kind {
        ElementKind::Column(col) if c.kind.at_column() && c.end == MemberEnd::Base => {
            column_frame(project, member, col).map(|f| base_plate(&f, p))
        }
        ElementKind::Beam(_) if !c.kind.at_column() => {
            beams.get(&c.member).and_then(|b| end_frame(b, c.end)).and_then(|f| beam_parts(c.kind, &f, p))
        }
        _ => None,
    };
    match built.filter(finite) {
        Some((plates, bolts)) => (g.plates, g.bolts) = (plates, bolts),
        None => g.warning = Some(ConnectionWarning::Unsuitable),
    }
    g
}

/// Whether every point and size of the parts is a finite number.
fn finite((plates, bolts): &Parts) -> bool {
    let block = |b: &Block| b.center.is_finite() && b.x.is_finite() && b.y.is_finite() && b.size.is_finite();
    let rod = |r: &Rod| r.a.is_finite() && r.b.is_finite() && r.diameter.is_finite();
    plates.iter().flat_map(|p| &p.blocks).all(block) && bolts.iter().all(|b| rod(&b.shank) && b.washers.iter().all(rod))
}

/// Every connection of the project, from the resolved beams.
pub fn build_connections(
    project: &Project,
    beams: &BTreeMap<ElementId, BeamGeom>,
) -> BTreeMap<ElementId, ConnectionGeom> {
    project
        .elements
        .values()
        .filter_map(|e| match &e.kind {
            ElementKind::Connection(c) => Some((e.id, connection_geom(project, beams, e.id, c))),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ModelGeometry;
    use cc_core::{
        Beam, ColumnSection, Document, EditCtx, Element, ElementType, ElementTypeId, LevelId, SectionShape, TypeData,
        WallTop,
    };

    fn add(e: &mut EditCtx, level: LevelId, type_id: ElementTypeId, kind: ElementKind) -> ElementId {
        let id = ElementId(e.alloc_id());
        e.add_element(Element {
            id,
            level,
            type_id,
            mark: String::new(),
            comments: String::new(),
            kind,
            uid: Default::default(),
            phase_created: Default::default(),
            phase_demolished: None,
            design_option: None,
            paints: Vec::new(),
            plugin_data: Default::default(),
        })
    }

    /// A template project with a 0.30 x 0.20 column type (3 m high), a
    /// 0.20 x 0.40 beam type (top at 3 m) and a connection type.
    struct Site {
        doc: Document,
        level: LevelId,
        column: ElementTypeId,
        beam: ElementTypeId,
        connection: ElementTypeId,
    }

    fn site() -> Site {
        let mut doc = Document::new(Project::from_template());
        let level = doc.project.levels[0].id;
        let (column, beam, connection) = doc.edit("types", |e| {
            let mut add_type = |data: TypeData| {
                let id = ElementTypeId(e.alloc_id());
                e.add_type(ElementType {
                    id,
                    name: format!("T{}", id.0),
                    data,
                    info: Default::default(),
                    uid: Default::default(),
                })
            };
            let column = add_type(TypeData::Column {
                section: ColumnSection::Rectangular { width: 0.3, depth: 0.2 },
                material: "Steel".into(),
                profile: None,
                profile_rotation_deg: 0.0,
                profile_mirror: false,
            });
            let beam = add_type(TypeData::Beam {
                width: 0.2,
                height: 0.4,
                material: "Steel".into(),
                shape: SectionShape::Solid,
                profile: None,
                profile_rotation_deg: 0.0,
                profile_mirror: false,
            });
            (column, beam, add_type(TypeData::Connection {}))
        });
        Site { doc, level, column, beam, connection }
    }

    impl Site {
        fn z0(&self) -> f64 {
            self.doc.project.level_elevation(self.level)
        }

        fn column(&mut self, at: DVec2) -> ElementId {
            let (level, ty) = (self.level, self.column);
            let col = cc_core::Column {
                position: at,
                rotation_deg: 0.0,
                base_offset: 0.0,
                height: 3.0,
                top: WallTop::Unconnected,
                structural: true,
                base_support: Default::default(),
            };
            self.doc.edit("column", |e| add(e, level, ty, ElementKind::Column(col)))
        }

        fn beam(&mut self, a: DVec2, b: DVec2) -> ElementId {
            let (level, ty) = (self.level, self.beam);
            self.doc.edit("beam", |e| add(e, level, ty, ElementKind::Beam(Beam::new(a, b, 3.0))))
        }

        fn connect(
            &mut self,
            member: ElementId,
            end: MemberEnd,
            kind: ConnectionKind,
            f: impl FnOnce(&mut ConnectionParams),
        ) -> ElementId {
            let (level, ty) = (self.level, self.connection);
            let mut c = Connection::new(member, end, kind);
            f(&mut c.params);
            self.doc.edit("connection", |e| add(e, level, ty, ElementKind::Connection(c)))
        }

        fn geom(&self, id: ElementId) -> (ConnectionGeom, ModelGeometry) {
            let g = ModelGeometry::build(&self.doc.project);
            (g.connections.get(&id).cloned().expect("built"), g)
        }
    }

    fn close(a: DVec3, b: DVec3) -> bool {
        a.distance(b) < 1e-9
    }

    #[test]
    fn a_block_is_closed_and_outward() {
        let b = Block {
            center: DVec3::new(1.0, 2.0, 3.0),
            x: DVec3::new(0.6, 0.8, 0.0),
            y: DVec3::Z,
            size: DVec3::new(0.3, 0.2, 0.01),
        };
        let mut s = Solid::new(ElementId(1), Category::Connection, [0; 3]);
        s.add_polygons(b.faces());
        assert_eq!(s.faces.len(), 6);
        assert!((s.volume() - 0.3 * 0.2 * 0.01).abs() < 1e-12, "{}", s.volume());
    }

    #[test]
    fn a_base_plate_sits_under_its_column() {
        let mut s = site();
        let col = s.column(DVec2::new(2.0, 3.0));
        let id = s.connect(col, MemberEnd::Base, ConnectionKind::BasePlate, |_| {});
        let (g, model) = s.geom(id);
        assert_eq!(g.warning, None);
        let plate = g.plates[0].blocks[0];
        // Section 0.30 x 0.20 plus 2 x 0.05 at the edges, 25 mm thick, its top at the column's base.
        assert!(close(plate.size, DVec3::new(0.4, 0.3, 0.025)), "{:?}", plate.size);
        assert!(close(plate.center, DVec3::new(2.0, 3.0, s.z0() - 0.0125)), "{:?}", plate.center);
        assert_eq!(g.bolt_count(), 4);
        for b in &g.bolts {
            let (x, y) = ((b.shank.a.x - 2.0).abs(), (b.shank.a.y - 3.0).abs());
            assert!((x - 0.15).abs() < 1e-9 && (y - 0.1).abs() < 1e-9, "anchors at the edge distance: {x} {y}");
            assert!((b.shank.a.z - s.z0()).abs() < 1e-9, "from the plate's top");
            assert!((b.shank.length() - (0.025 + 0.4)).abs() < 1e-9);
        }
        assert!((g.plate_mass() - 0.4 * 0.3 * 0.025 * 7850.0).abs() < 1e-9);
        let solids: Vec<&Solid> = model.solids_of(id).collect();
        assert_eq!(solids.len(), 1);
        assert_eq!(solids[0].category, Category::Connection);
        // A given size wins over the section's.
        let other = s.column(DVec2::new(8.0, 0.0));
        let id = s.connect(other, MemberEnd::Base, ConnectionKind::BasePlate, |p| p.width = 0.5);
        assert!((s.geom(id).0.plates[0].blocks[0].size.x - 0.5).abs() < 1e-12);
    }

    #[test]
    fn anchors_come_in_rows_and_columns() {
        let mut s = site();
        let col = s.column(DVec2::ZERO);
        let id = s.connect(col, MemberEnd::Base, ConnectionKind::BasePlate, |p| (p.rows, p.columns) = (3, 4));
        assert_eq!(s.geom(id).0.bolt_count(), 12);
    }

    /// An end plate stands against the beam's end face, its normal along the
    /// beam's axis also when the beam is turned, bolted through along it.
    #[test]
    fn an_end_plate_follows_a_turned_beam() {
        let mut s = site();
        s.column(DVec2::ZERO);
        s.column(DVec2::new(3.0, 4.0));
        let beam = s.beam(DVec2::ZERO, DVec2::new(3.0, 4.0));
        let id = s.connect(beam, MemberEnd::End, ConnectionKind::EndPlate, |_| {});
        let (g, model) = s.geom(id);
        assert!(matches!(model.beams[&beam].end, BeamEnd::Column(_)));
        assert_eq!(g.warning, None);
        let axis = DVec3::new(0.6, 0.8, 0.0);
        let plate = g.plates[0].blocks[0];
        assert!(plate.z().cross(axis).length() < 1e-9, "normal {:?}", plate.z());
        assert!(plate.center.truncate().perp_dot(axis.truncate()).abs() < 1e-9, "on the axis");
        assert!((plate.center.z - (s.z0() + 2.8)).abs() < 1e-9, "mid-height of the beam");
        assert!(close(plate.size, DVec3::new(0.2 + 0.08, 0.4 + 0.08, 0.016)), "{:?}", plate.size);
        assert_eq!(g.bolt_count(), 4);
        for b in &g.bolts {
            assert!((b.shank.b - b.shank.a).cross(axis).length() < 1e-9);
            assert!((b.shank.length() - (0.016 + BOLT_EXTRA)).abs() < 1e-9);
        }
    }

    #[test]
    fn a_fin_plate_is_as_long_as_its_bolts_need() {
        let mut s = site();
        s.column(DVec2::ZERO);
        s.column(DVec2::new(5.0, 0.0));
        let beam = s.beam(DVec2::ZERO, DVec2::new(5.0, 0.0));
        let id = s.connect(beam, MemberEnd::Start, ConnectionKind::FinPlate, |p| p.columns = 3);
        let (g, _) = s.geom(id);
        let plate = g.plates[0].blocks[0];
        // 2 x 0.04 + 2 x 0.07 along the beam, 0.8 x 0.40 high, from the column face at x = 0.15.
        assert!((plate.size.x - 0.22).abs() < 1e-12 && (plate.size.y - 0.32).abs() < 1e-12, "{:?}", plate.size);
        let beside = 0.1 + plate.size.z / 2.0;
        assert!(close(plate.center, DVec3::new(0.15 + 0.11, beside, s.z0() + 2.8)), "{:?}", plate.center);
        assert_eq!(g.bolt_count(), 9, "three rows of three");
    }

    #[test]
    fn the_web_is_measured_at_mid_height() {
        use cc_core::SectionOutline;
        // An I: flanges 0.2 wide, a web 0.01 thick, 0.3 high.
        let p = |x: f64, y: f64| DVec2::new(x, y);
        let outer = vec![
            p(-0.1, 0.0),
            p(0.1, 0.0),
            p(0.1, 0.02),
            p(0.005, 0.02),
            p(0.005, 0.28),
            p(0.1, 0.28),
            p(0.1, 0.3),
            p(-0.1, 0.3),
            p(-0.1, 0.28),
            p(-0.005, 0.28),
            p(-0.005, 0.02),
            p(-0.1, 0.02),
        ];
        let i = SectionOutline { outer, holes: Vec::new() };
        assert!((width_at(&i, 0.15) - 0.01).abs() < 1e-12);
        assert!((width_at(&i, 0.01) - 0.2).abs() < 1e-12);
    }
    #[test]
    fn timber_bolts_cross_the_beam_with_washers() {
        let mut s = site();
        s.column(DVec2::ZERO);
        s.column(DVec2::new(5.0, 0.0));
        let beam = s.beam(DVec2::ZERO, DVec2::new(5.0, 0.0));
        let id = s.connect(beam, MemberEnd::End, ConnectionKind::TimberBolts, |_| {});
        let (g, _) = s.geom(id);
        assert!(g.plates.is_empty() && g.plate_volume() == 0.0);
        assert_eq!(g.bolt_count(), 4);
        for b in &g.bolts {
            assert!((b.shank.length() - (0.2 + 2.0 * 0.005)).abs() < 1e-9, "{}", b.shank.length());
            assert_eq!(b.washers.len(), 2);
            assert!(b.washers.iter().all(|w| (w.diameter - 3.0 * 0.012).abs() < 1e-12));
        }
        // The first column of bolts is the edge distance in from the face at x = 4.85.
        let nearest = g.bolts.iter().map(|b| b.shank.a.x).fold(f64::NEG_INFINITY, f64::max);
        assert!((nearest - (4.85 - 0.08)).abs() < 1e-9, "{nearest}");
    }

    #[test]
    fn a_hanger_is_a_bent_plate_under_the_beam_end() {
        let mut s = site();
        s.beam(DVec2::ZERO, DVec2::new(6.0, 0.0));
        let joist = s.beam(DVec2::new(3.0, 0.0), DVec2::new(3.0, 4.0));
        let id = s.connect(joist, MemberEnd::Start, ConnectionKind::Hanger, |_| {});
        let (g, model) = s.geom(id);
        assert_eq!(model.beams[&joist].start, BeamEnd::Beam);
        assert_eq!(g.warning, None);
        assert_eq!(g.plates.len(), 1);
        let blocks = &g.plates[0].blocks;
        assert_eq!(blocks.len(), 3, "base and two sides");
        let (t, len, inner, side) = (0.002, 0.1, 0.2, HANGER_SIDE * 0.4);
        let bottom = s.z0() + 2.6;
        let top = blocks.iter().map(|b| b.center.z + b.size.y / 2.0).fold(f64::NEG_INFINITY, f64::max);
        assert!((top - (bottom + side)).abs() < 1e-9, "sides up to 0.6 h: {top}");
        assert!((blocks[0].center.z - (bottom - t / 2.0)).abs() < 1e-9, "base under the beam");
        let volume = len * inner * t + 2.0 * len * (side + t) * t;
        assert!((g.plate_volume() - volume).abs() < 1e-12, "{} vs {volume}", g.plate_volume());
    }

    #[test]
    fn a_connection_moves_with_its_column() {
        let mut s = site();
        let col = s.column(DVec2::new(2.0, 3.0));
        let id = s.connect(col, MemberEnd::Base, ConnectionKind::BasePlate, |_| {});
        s.doc.edit("move", |e| e.element_mut(col).unwrap().translate(DVec2::new(1.0, 0.0)));
        let (g, _) = s.geom(id);
        assert!(close(g.plates[0].blocks[0].center, DVec3::new(3.0, 3.0, s.z0() - 0.0125)));
    }

    #[test]
    fn a_missing_member_or_a_free_end_builds_nothing() {
        let mut s = site();
        let lost = s.connect(ElementId(999_999), MemberEnd::Base, ConnectionKind::BasePlate, |_| {});
        s.column(DVec2::ZERO);
        let beam = s.beam(DVec2::ZERO, DVec2::new(5.0, 0.0));
        let free = s.connect(beam, MemberEnd::End, ConnectionKind::EndPlate, |_| {});
        let (g, model) = s.geom(lost);
        assert_eq!(g.warning, Some(ConnectionWarning::MissingMember));
        assert!(g.plates.is_empty() && g.bolts.is_empty());
        assert_eq!(model.solids_of(lost).count(), 0);
        let (g, model) = s.geom(free);
        assert_eq!(g.warning, Some(ConnectionWarning::Unsuitable));
        assert_eq!(model.solids_of(free).count(), 0);
    }

    /// Values a crafted file could carry build with the kind's defaults
    /// instead of huge or non-finite parts.
    #[test]
    fn hostile_values_build_with_the_defaults() {
        let mut s = site();
        let col = s.column(DVec2::ZERO);
        let mut c = Connection::new(col, MemberEnd::Base, ConnectionKind::BasePlate);
        c.params.thickness = f64::NAN;
        c.params.width = f64::INFINITY;
        c.params.anchor_length = -1.0;
        (c.params.rows, c.params.columns) = (u32::MAX, 0);
        let g = connection_geom(&s.doc.project, &BTreeMap::new(), ElementId(1), &c);
        assert_eq!(g.warning, None);
        assert_eq!(g.bolt_count(), 2 * 2);
        assert!(g.faces().iter().flatten().all(|p| p.is_finite()));
        assert!((g.plates[0].blocks[0].size.z - 0.025).abs() < 1e-12);
    }
}
