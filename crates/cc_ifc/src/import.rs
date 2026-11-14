//! IFC import: maps the spatial structure and building elements of an IFC
//! file (IFC2X3 or IFC4) onto a ConstructCraft project.
//!
//! Geometry is read from extruded area solids (the usual representation of
//! walls, slabs, columns and beams), with faceted B-reps, face sets and
//! bounding boxes used for extents. Storeys become levels; walls, doors,
//! windows, slabs, ceilings, roofs, spaces, columns and beams become native
//! elements with matching or new types. Products that cannot be interpreted
//! are counted in the report instead of failing the import.

use crate::geometry::{Extrusion, Geometry, Solid, affine, clean_loop};
use crate::guid;
use crate::parse::{Entity, Param, ParseError, StepFile, parse};
use cc_core::component::IfcClass;
use cc_core::component::convex_hull;
use cc_core::curtain::{CurtainEdits, CurtainPanel, CurtainSystem};
use cc_core::properties::{Atom, IfcValue, Property, PropertySet, PropertyValue};
use cc_core::sketch::Sketch;
use cc_core::structural_profile::{
    MAX_NAME, MAX_OPENINGS as MAX_SKETCH_OPENINGS, MAX_POINTS as MAX_SKETCH_POINTS, MAX_PROFILES, ParamSketch,
    ProfileDef, ProfileMaterial, ProfileShape,
};
use cc_core::{
    Beam, Category, Ceiling, Column, ColumnSection, Component, ComponentDef, ComponentItem, ComponentKind,
    ComponentShape, Element, ElementId, ElementKind, ElementType, ElementTypeId, Floor, Grid, Hosted, Layer,
    LayerFunction, Level, LevelId, Material, Project, Roof, RoofShape, Room, SectionShape, ShapedRebar, TypeData, Uid,
    View, ViewId, ViewKind, Wall, WallFunction, WallTop,
};
use cc_core::{DoorOperation, ProfileId, ProfilePoint, WindowOperation};
use cc_core::{Foundation, FoundationKind, FoundationShape};
use cc_geom::polygon;
use glam::{DAffine3, DVec2, DVec3};
use std::collections::{BTreeMap, HashMap};

/// Deepest nesting of complex properties.
const MAX_PROPERTY_DEPTH: usize = 8;
/// Largest IFC file imported, bytes: the parsed instances take many times
/// the file's size in memory.
pub const MAX_IFC_BYTES: u64 = 512 << 20;
/// Most properties (nested ones included) read from one property set.
const MAX_SET_PROPERTIES: usize = 10_000;
/// Most openings cut from one product.
const MAX_OPENINGS: usize = 1000;
/// Window panels read from one type.
const MAX_PANELS: usize = 16;
const MAX_PARENT_DEPTH: usize = 16;
/// Most construction phases read from a file.
const MAX_PHASES: usize = 100;
/// Length tolerance when matching types (meters).
const TYPE_TOL: f64 = 0.001;
/// How far apart (m) the vertices of two imported pieces may be for the
/// pieces to be the same (the mesh vertices are welded at 1/100 mm).
const PIECE_TOL: f64 = 1e-5;

/// What an import brought in.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportReport {
    /// IFC schema of the file (`IFC4`, `IFC2X3`…).
    pub schema: String,
    pub levels: usize,
    pub imported: BTreeMap<Category, usize>,
    /// Objects brought in as generic components keeping their IFC class, by class.
    pub generic: BTreeMap<String, usize>,
    /// Products that were not imported, by IFC entity name.
    pub skipped: BTreeMap<String, usize>,
}

impl ImportReport {
    pub fn imported_total(&self) -> usize {
        self.imported.values().sum()
    }

    pub fn skipped_total(&self) -> usize {
        self.skipped.values().sum()
    }
}

#[derive(Debug)]
pub enum ImportError {
    Io(std::io::Error),
    Parse(ParseError),
    /// The file has no `IFCPROJECT`.
    NotIfc,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Io(e) => write!(f, "{e}"),
            ImportError::Parse(e) => write!(f, "{e}"),
            ImportError::NotIfc => write!(f, "no IFCPROJECT found"),
        }
    }
}

impl std::error::Error for ImportError {}

/// Import the IFC file at `path` into `base` (see [`import`]).
/// Files larger than [`MAX_IFC_BYTES`] are refused before they are read
/// (an [`ImportError::Io`] of kind `FileTooLarge`).
pub fn import_file(path: &std::path::Path, base: Project) -> Result<(Project, ImportReport), ImportError> {
    let len = std::fs::metadata(path).map_err(ImportError::Io)?.len();
    if len > MAX_IFC_BYTES {
        return Err(ImportError::Io(std::io::Error::new(
            std::io::ErrorKind::FileTooLarge,
            format!("the IFC file is larger than {} MB", MAX_IFC_BYTES >> 20),
        )));
    }
    let bytes = std::fs::read(path).map_err(ImportError::Io)?;
    import(&String::from_utf8_lossy(&bytes), base)
}

/// Import IFC `text`. `base` supplies materials, types and project-wide
/// views (usually the default template); its levels, elements, plans and
/// sheets are replaced by the imported building.
pub fn import(text: &str, base: Project) -> Result<(Project, ImportReport), ImportError> {
    let file = parse(text).map_err(ImportError::Parse)?;
    let Some((project_id, _)) = file.of_type("IFCPROJECT").next() else { return Err(ImportError::NotIfc) };
    let mut im = Importer::new(&file, base, project_id);
    im.run();
    Ok((im.p, im.report))
}

/// Body and axis of a product, in world coordinates.
#[derive(Debug, Clone, Default)]
struct Body {
    solids: Vec<Solid>,
    axis: Option<(DVec2, DVec2)>,
    /// The axis curve in plan, as points (arcs tessellated).
    axis_points: Vec<DVec2>,
}

impl Body {
    fn points(&self) -> impl Iterator<Item = DVec3> + '_ {
        self.solids.iter().flat_map(|s| s.points.iter().copied())
    }

    fn z_range(&self) -> Option<(f64, f64)> {
        let mut it = self.points().map(|p| p.z).peekable();
        it.peek()?;
        Some(it.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), z| (lo.min(z), hi.max(z))))
    }

    /// The first extrusion (vertical ones preferred when `vertical`).
    fn extrusion(&self, vertical: bool) -> Option<&Extrusion> {
        let mut all = self.solids.iter().filter_map(|s| s.extrusion.as_ref());
        if vertical { all.find(|e| e.is_vertical()) } else { all.next() }
    }

    /// The extrusion of a body that is nothing but one uncut extrusion of a
    /// solid profile (what slabs, columns and beams can hold exactly).
    fn sole_extrusion(&self) -> Option<&Extrusion> {
        self.sole_extrusion_with_voids().filter(|e| e.profile.holes.is_empty())
    }

    /// Like [`Body::sole_extrusion`], the profile possibly with voids (slabs
    /// hold them as openings).
    fn sole_extrusion_with_voids(&self) -> Option<&Extrusion> {
        match self.solids.as_slice() {
            [s] if !s.clipped => s.extrusion.as_ref().filter(|e| e.profile.closed),
            _ => None,
        }
    }

    /// A slab's extrusion and the extrusions its body subtracts from it
    /// (shafts some tools cut out of the body as well as with openings);
    /// `None` when anything else changed it.
    fn slab_extrusion(&self) -> Option<(&Extrusion, &[Extrusion])> {
        match self.solids.as_slice() {
            [s] if !s.other_cut && (!s.clipped || !s.cuts.is_empty()) => {
                s.extrusion.as_ref().filter(|e| e.profile.closed).map(|e| (e, s.cuts.as_slice()))
            }
            _ => None,
        }
    }
}

/// Outline loops (the outer one first, counter-clockwise; then openings)
/// and slope of a roof body: its vertical faces stand on the outline, its
/// sloped faces share one slope (0 when there are none); also the uphill
/// direction of the first upward sloped face. `None` when the faces do not
/// close into loops or slopes differ.
fn roof_outline(faces: &[Vec<DVec3>]) -> Option<(Vec<Vec<DVec2>>, f64, Option<DVec2>)> {
    let mut segments: Vec<(DVec2, DVec2)> = Vec::new();
    let mut slope: Option<f64> = None;
    let mut uphill: Option<DVec2> = None;
    for f in faces.iter().take(100_000) {
        let n = f.len();
        if n < 3 {
            continue;
        }
        let normal: DVec3 = (0..n).map(|i| f[i].cross(f[(i + 1) % n])).sum();
        let len = normal.length();
        if !len.is_finite() || len <= 1e-12 {
            continue;
        }
        let normal = normal / len;
        let run = normal.truncate().length();
        if normal.z.abs() < 1e-9 {
            // A vertical face: its plan extent along its own direction.
            let dir = normal.truncate().perp().normalize_or_zero();
            let (lo, hi) = f.iter().map(|p| p.truncate()).fold((None::<DVec2>, None::<DVec2>), |(lo, hi), p| {
                let t = p.dot(dir);
                (Some(lo.filter(|l| l.dot(dir) <= t).unwrap_or(p)), Some(hi.filter(|h| h.dot(dir) >= t).unwrap_or(p)))
            });
            if let (Some(a), Some(b)) = (lo, hi)
                && a.distance(b) > 1e-6
            {
                segments.push((a, b));
            }
        } else if run > 1e-9 {
            let s = run / normal.z.abs();
            if normal.z > 0.0 && uphill.is_none() {
                uphill = (-normal.truncate()).try_normalize();
            }
            match slope {
                Some(x) if (x - s).abs() > 1e-6 * x.max(1.0) => return None,
                _ => slope = Some(s),
            }
        }
    }
    // Chain the plan segments into loops (each point joins two segments).
    let tol = 1e-6;
    let mut loops: Vec<Vec<DVec2>> = Vec::new();
    let mut rest = segments;
    let mut guard = rest.len() + 1;
    while let Some((a, b)) = rest.pop() {
        let mut ring = vec![a];
        let mut at = b;
        while at.distance(a) > tol && guard > 0 {
            guard -= 1;
            let i = rest.iter().position(|(p, q)| p.distance(at) <= tol || q.distance(at) <= tol)?;
            let (p, q) = rest.swap_remove(i);
            ring.push(at);
            at = if p.distance(at) <= tol { q } else { p };
        }
        // Points where the outline goes straight on are not corners.
        let n = ring.len();
        let ring: Vec<DVec2> = (0..n)
            .filter(|&i| {
                let (p, c, q) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
                (c - p).perp_dot(q - c).abs() > 1e-9 * (c - p).length() * (q - c).length() || (c - p).dot(q - c) < 0.0
            })
            .map(|i| ring[i])
            .collect();
        if ring.len() >= 3 && polygon::area(&ring) > 1e-6 {
            loops.push(ring);
        }
    }
    if loops.is_empty() {
        return None;
    }
    loops.sort_by(|a, b| polygon::area(b).total_cmp(&polygon::area(a)));
    if let Some(outer) = loops.first_mut() {
        *outer = polygon::ccw(outer);
    }
    Some((loops, slope.unwrap_or(0.0), uphill))
}

/// An arc wall (or arc beam): an axis on a circular arc and a footprint whose points all
/// lie on two circles concentric with it, at equal distances from the axis.
/// Returns its ends, bulge and thickness.
fn arc_wall(axis: &[DVec2], footprint: &[DVec2]) -> Option<(DVec2, DVec2, f64, f64)> {
    if axis.len() < 3 || footprint.len() < 4 {
        return None;
    }
    let (start, end) = (*axis.first()?, *axis.last()?);
    let mid = *axis.get(axis.len() / 2)?;
    let bulge = cc_core::arc::Arc::bulge_through(start, mid, end)?;
    let arc = cc_core::arc::Arc::from_bulge(start, end, bulge)?;
    let tol = 1e-3;
    if axis.iter().any(|p| (p.distance(arc.center) - arc.radius).abs() > tol) {
        return None;
    }
    let radii: Vec<f64> = footprint.iter().map(|p| p.distance(arc.center)).collect();
    let (lo, hi) = radii.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), r| (lo.min(*r), hi.max(*r)));
    let thickness = hi - lo;
    let on_faces = radii.iter().all(|r| (r - lo).abs() < tol || (r - hi).abs() < tol);
    let centered = ((lo + hi) * 0.5 - arc.radius).abs() < tol;
    (on_faces && centered && thickness >= 0.005 && arc.length() >= 0.01).then_some((start, end, bulge, thickness))
}

/// The piece of a ring between two radii: its ring's center, the radii, the
/// angle it turns through (radians) and the unit direction of its bisector,
/// out of the ring.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RingSector {
    center: DVec2,
    r_in: f64,
    r_out: f64,
    sweep: f64,
    axis: DVec2,
}

/// The ring sector `curves` draw: two arcs about one center, of different
/// radii and the same turn, and two straight sides between their ends along
/// radii. `None` for any other outline, or a turn past the profile's limit.
fn ring_sector(curves: &[cc_core::sketch::SketchCurve]) -> Option<RingSector> {
    use cc_core::sketch::SketchCurve;
    if curves.len() != 4 {
        return None;
    }
    let tol = 1e-4;
    let mut arcs: Vec<(cc_core::arc::Arc, DVec2, DVec2)> = Vec::new();
    let mut lines: Vec<(DVec2, DVec2)> = Vec::new();
    for curve in curves {
        match *curve {
            SketchCurve::Arc { a, b, bulge } => arcs.push((cc_core::arc::Arc::from_bulge(a, b, bulge)?, a, b)),
            SketchCurve::Line { a, b } => lines.push((a, b)),
            _ => return None,
        }
    }
    let [(first, ..), (second, ..)] = arcs.as_slice() else { return None };
    if lines.len() != 2 || first.center.distance(second.center) > tol {
        return None;
    }
    let (inner, outer) = if first.radius < second.radius { (&arcs[0], &arcs[1]) } else { (&arcs[1], &arcs[0]) };
    let (center, r_in, r_out) = (outer.0.center, inner.0.radius, outer.0.radius);
    let sweep = outer.0.sweep.abs();
    let limit = ProfileShape::MAX_SECTOR_SWEEP.to_radians();
    if r_out - r_in < 1e-4 || (inner.0.sweep.abs() - sweep).abs() > 1e-4 || !(1e-4..limit - 1e-6).contains(&sweep) {
        return None;
    }
    // The straight sides run along radii: each joins an end of one arc to the end of the other on its ray.
    let radial = |(a, b): (DVec2, DVec2)| {
        let (da, db) = ((a - center).normalize_or_zero(), (b - center).normalize_or_zero());
        da.distance(db) < 1e-4 && ((a.distance(center) - b.distance(center)).abs() - (r_out - r_in)).abs() < tol
    };
    if !lines.iter().copied().all(radial) {
        return None;
    }
    // Under half a turn, the middle of the outer arc's chord lies on the bisector.
    let axis = ((outer.1 + outer.2) / 2.0 - center).try_normalize()?;
    Some(RingSector { center, r_in, r_out, sweep, axis })
}

/// Entities that are not physical objects: spatial structure, zones,
/// features cut from their hosts, ports, annotations and analysis models.
fn is_non_physical(name: &str) -> bool {
    matches!(
        name,
        "IFCPROJECT"
            | "IFCBUILDING"
            | "IFCBUILDINGSTOREY"
            | "IFCSPATIALZONE"
            | "IFCEXTERNALSPATIALELEMENT"
            | "IFCOPENINGELEMENT"
            | "IFCOPENINGSTANDARDCASE"
            | "IFCVOIDINGFEATURE"
            | "IFCSURFACEFEATURE"
            | "IFCANNOTATION"
            | "IFCPORT"
            | "IFCDISTRIBUTIONPORT"
            | "IFCVIRTUALELEMENT"
            | "IFCREFERENT"
    ) || name.starts_with("IFCSTRUCTURAL")
        || name.starts_with("IFCALIGNMENT")
        || name.starts_with("IFCFACILITY")
}

/// Component kind of a generic object, by IFC class.
fn kind_of_class(name: &str) -> ComponentKind {
    match name {
        "IFCFURNITURE" | "IFCFURNISHINGELEMENT" | "IFCSYSTEMFURNITUREELEMENT" => ComponentKind::Furniture,
        "IFCSANITARYTERMINAL" | "IFCWASTETERMINAL" => ComponentKind::Plumbing,
        "IFCLIGHTFIXTURE" | "IFCLAMP" => ComponentKind::Lighting,
        _ => ComponentKind::Generic,
    }
}

struct Importer<'a> {
    f: &'a StepFile,
    g: Geometry<'a>,
    p: Project,
    report: ImportReport,
    project_id: u32,
    /// Meters per file length unit.
    scale: f64,
    /// SI values of the file's area and volume units.
    area: f64,
    volume: f64,
    storeys: HashMap<u32, LevelId>,
    /// Product → its containing structure or aggregating whole.
    parent: HashMap<u32, u32>,
    type_of: HashMap<u32, u32>,
    material_of: HashMap<u32, u32>,
    psets_of: HashMap<u32, Vec<u32>>,
    /// Opening → host element; filler → opening.
    host_of_opening: HashMap<u32, u32>,
    /// Host → the openings voiding it.
    openings_of: HashMap<u32, Vec<u32>>,
    opening_of_filler: HashMap<u32, u32>,
    /// Surface style colors by style name and by styled item.
    style_colors: HashMap<String, [u8; 3]>,
    item_colors: HashMap<u32, [u8; 3]>,
    /// Surface style name of styled items.
    item_styles: HashMap<u32, String>,
    /// Imported walls by IFC instance.
    walls: HashMap<u32, ElementId>,
    /// Products with parts aggregated into them (e.g. roofs made of slabs).
    has_parts: HashMap<u32, usize>,
    /// Roofs imported with their framing restored (from [`crate::export::ROOFING_PSET`]):
    /// the members aggregated in them are derived again, not imported.
    framed_roofs: std::collections::HashSet<u32>,
    /// Piles this program wrote under a cap: the pile, the cap's GlobalId and
    /// the pile's place in its layout (linked when the import ends).
    cap_piles: Vec<(ElementId, String, u32)>,
    /// Pile cap types and the name of their piles' type (resolved when the import ends).
    cap_types: Vec<(ElementTypeId, String)>,
    /// Curtain walls imported as such: their plates and members come in with them.
    curtain_walls: std::collections::HashSet<u32>,
    /// Imported elements and their IFC instances (their phases are read at the end).
    products: Vec<(ElementId, u32)>,
    /// Types this import added, by category and the name they were asked
    /// for (they may have been renamed to keep names unique).
    added_types: HashMap<(Category, String), Vec<ElementTypeId>>,
}

/// The opening size of a door or window type of `category` (of either kind).
fn is_opening_type(d: &TypeData, category: Category) -> Option<(f64, f64)> {
    match d {
        TypeData::Door { width, height, .. } if category == Category::Door => Some((*width, *height)),
        TypeData::Window { width, height, .. } if category == Category::Window => Some((*width, *height)),
        TypeData::Component { def, .. } if def.category() == category => def.opening_size(),
        _ => None,
    }
}

/// How the panels of a window type open.
fn window_operation(d: &TypeData) -> WindowOperation {
    match d {
        TypeData::Component { def, .. } => def.opening.as_ref().map(|o| o.operation).unwrap_or_default(),
        _ => WindowOperation::Fixed,
    }
}

/// How the leaves of a door type swing.
fn door_operation(d: &TypeData) -> DoorOperation {
    match d {
        TypeData::Component { def, .. } => def.opening.as_ref().map(|o| o.door_operation).unwrap_or_default(),
        _ => DoorOperation::SingleSwingLeft,
    }
}

fn clean_polygon(pts: Vec<DVec2>) -> Vec<DVec2> {
    clean_loop(pts, |a, b| a.distance(*b) < 1e-9)
}

/// How far an opening may stop short of a slab's faces and still pass through it (m).
const SLAB_TOL: f64 = 1e-3;

/// Whether two outlines have the same corners (within a millimeter), in any order.
fn same_outline(a: &[DVec2], b: &[DVec2]) -> bool {
    let covers = |a: &[DVec2], b: &[DVec2]| a.iter().all(|p| b.iter().any(|q| p.distance(*q) < 1e-3));
    a.len() == b.len() && covers(a, b) && covers(b, a)
}

/// Whether `opening` is an outline with an area strictly inside `outline`.
fn openings_fit(outline: &[DVec2], opening: &[DVec2]) -> bool {
    opening.len() >= 3
        && polygon::area(opening) > 1e-4
        && opening.iter().all(|p| polygon::contains(outline, *p))
        && !edges_meet(outline, opening)
}

/// Whether two outlines cross, touch or one lies in the other.
fn outlines_meet(a: &[DVec2], b: &[DVec2]) -> bool {
    edges_meet(a, b) || b.iter().any(|p| polygon::contains(a, *p)) || a.iter().any(|p| polygon::contains(b, *p))
}

/// Whether the edges of two outlines cross or touch.
fn edges_meet(a: &[DVec2], b: &[DVec2]) -> bool {
    let near = |p: DVec2, q: DVec2, r: DVec2| polygon::closest_on_segment(p, q, r).0.distance(r) < 1e-6;
    let crosses = |p: DVec2, q: DVec2, r: DVec2, s: DVec2| {
        let side = |a: DVec2, b: DVec2, c: DVec2| (b - a).perp_dot(c - a);
        let (d1, d2, d3, d4) = (side(r, s, p), side(r, s, q), side(p, q, r), side(p, q, s));
        (d1 * d2 < 0.0 && d3 * d4 < 0.0) || near(r, s, p) || near(r, s, q) || near(p, q, r) || near(p, q, s)
    };
    let (n, m) = (a.len(), b.len());
    (0..n).any(|i| (0..m).any(|j| crosses(a[i], a[(i + 1) % n], b[j], b[(j + 1) % m])))
}

/// Whether an outline `thickness` thick is shaped like a slab: at least
/// [`SLAB_ASPECT`] times as wide as thick (pile caps are blocks, not rafts).
fn slab_like(outline: &[DVec2], thickness: f64) -> bool {
    let n = outline.len();
    // The narrowest extent over the outline's edge directions (exact for convex outlines).
    let width = (0..n)
        .filter_map(|i| (outline[(i + 1) % n] - outline[i]).try_normalize())
        .map(|d| {
            let across: Vec<f64> = outline.iter().map(|p| p.dot(d.perp())).collect();
            across.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                - across.iter().copied().fold(f64::INFINITY, f64::min)
        })
        .fold(f64::INFINITY, f64::min);
    width.is_finite() && thickness * SLAB_ASPECT <= width
}

/// How many times wider than thick a base slab is to be a raft.
const SLAB_ASPECT: f64 = 3.0;

/// Type names from other tools often end in an instance number ("Wall:Generic 200mm:123456").
fn clean_name(name: &str) -> String {
    let name = name.trim();
    match name.rsplit_once(':') {
        Some((head, tail)) if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) => head.to_string(),
        _ => name.to_string(),
    }
}

impl<'a> Importer<'a> {
    fn new(f: &'a StepFile, base: Project, project_id: u32) -> Self {
        let mut im = Self {
            f,
            g: Geometry::new(f, 1.0, 1.0),
            p: base,
            report: ImportReport { schema: f.schema.clone(), ..Default::default() },
            project_id,
            scale: 1.0,
            area: 1.0,
            volume: 1.0,
            storeys: HashMap::new(),
            parent: HashMap::new(),
            type_of: HashMap::new(),
            material_of: HashMap::new(),
            psets_of: HashMap::new(),
            host_of_opening: HashMap::new(),
            openings_of: HashMap::new(),
            opening_of_filler: HashMap::new(),
            style_colors: HashMap::new(),
            item_colors: HashMap::new(),
            item_styles: HashMap::new(),
            walls: HashMap::new(),
            has_parts: HashMap::new(),
            framed_roofs: Default::default(),
            cap_piles: Vec::new(),
            cap_types: Vec::new(),
            curtain_walls: Default::default(),
            products: Vec::new(),
            added_types: HashMap::new(),
        };
        im.scale = im.unit("LENGTHUNIT");
        im.area = im.unit_opt("AREAUNIT").unwrap_or(im.scale * im.scale);
        im.volume = im.unit_opt("VOLUMEUNIT").unwrap_or(im.scale.powi(3));
        im.g = Geometry::new(f, im.scale, im.unit("PLANEANGLEUNIT"));
        im.index_relationships();
        im.index_styles();
        im
    }

    fn run(&mut self) {
        self.prepare_base();
        self.read_project_info();
        self.read_storeys();
        let f = self.f;
        // Walls first: doors and windows need their hosts.
        for (id, e) in f.entities.iter() {
            if matches!(e.name.as_str(), "IFCWALL" | "IFCWALLSTANDARDCASE" | "IFCWALLELEMENTEDCASE") {
                let ok = self.import_wall(*id, e);
                self.or_generic(*id, e, ok);
            } else if e.name == "IFCCURTAINWALL" {
                let ok = self.import_curtain_wall(*id, e);
                self.or_generic(*id, e, ok);
            }
        }
        for (id, e) in f.entities.iter() {
            let (id, name) = (*id, e.name.as_str());
            let ok = match name {
                "IFCWALL" | "IFCWALLSTANDARDCASE" | "IFCWALLELEMENTEDCASE" | "IFCCURTAINWALL" => continue,
                // The panels and mullions of a curtain wall rebuilt from its grid.
                "IFCPLATE" | "IFCMEMBER" if self.parent.get(&id).is_some_and(|w| self.curtain_walls.contains(w)) => {
                    continue;
                }
                "IFCDOOR" | "IFCDOORSTANDARDCASE" => self.import_filler(id, e, true),
                "IFCWINDOW" | "IFCWINDOWSTANDARDCASE" => self.import_filler(id, e, false),
                "IFCSLAB" | "IFCSLABSTANDARDCASE" | "IFCSLABELEMENTEDCASE" => self.import_slab(id, e),
                "IFCCOVERING" => self.import_covering(id, e),
                "IFCROOF" => self.import_roof(id, e),
                "IFCGEOGRAPHICELEMENT" => self.import_topography(id, e),
                "IFCCOLUMN" | "IFCCOLUMNSTANDARDCASE" => self.import_column(id, e),
                "IFCBEAM" | "IFCBEAMSTANDARDCASE" => self.import_beam(id, e),
                "IFCREINFORCINGBAR" => self.import_bar(id, e),
                "IFCFOOTING" => self.import_pile_cap(id, e),
                "IFCPILE" => self.import_pile(id, e),
                // The framing of a roof whose type now draws it again.
                "IFCMEMBER" if self.parent.get(&id).is_some_and(|r| self.framed_roofs.contains(r)) => continue,
                "IFCSPACE" => {
                    if !self.import_space(id, e) {
                        self.skip(e);
                    }
                    continue;
                }
                "IFCGRID" => {
                    self.import_grid(id, e);
                    continue;
                }
                // Terrain and other site geometry.
                "IFCSITE" => {
                    if self.is_shaped(e) {
                        self.import_generic(id, e);
                    }
                    continue;
                }
                _ if is_non_physical(name) || !self.is_product(e) => continue,
                // Every other physical object keeps its class and geometry.
                _ => false,
            };
            self.or_generic(id, e, ok);
        }
        self.finish();
    }

    /// A pile cap this program wrote, with its placement and layout in
    /// [`crate::export::PILE_CAP_PSET`], when its body still stands there:
    /// the native cap, its top at the body's top and its type (found or made)
    /// of that thickness and layout. Its piles come in on their own and are
    /// linked to it when the import ends. Caps of other programs come in as
    /// components.
    fn import_pile_cap(&mut self, id: u32, e: &Entity) -> bool {
        if e.arg(8).as_enum() != Some("PILE_CAP") || self.has_openings(id) {
            return false;
        }
        let set = |n: &str| n == crate::export::PILE_CAP_PSET;
        let first = |name: &str| self.pset_texts(id, &set, name).first().map(|s| s.to_string());
        let (Some(placement), Some(cap)) = (first("Placement"), first("Cap")) else { return false };
        let pile_type = first("PileType").unwrap_or_default();
        let Some(([x, y], rotation_deg)) = cc_core::curtain::from_text::<([f64; 2], f64)>(&placement) else {
            return false;
        };
        let Some((thickness, mut layout)) = cc_core::curtain::from_text::<(f64, cc_core::pile::PileCap)>(&cap) else {
            return false;
        };
        let position = DVec2::new(x, y);
        layout.pile_type = None;
        let valid = position.is_finite()
            && rotation_deg.is_finite()
            && thickness.is_finite()
            && thickness > 0.0
            && thickness <= cc_core::StripHaunch::MAX_HEIGHT
            && layout.clone().repair() == 0;
        if !valid {
            return false;
        }
        // The body must still stand where the cap was placed.
        let (_, body) = self.product(e);
        let Some((_, z1)) = body.z_range() else { return false };
        let (lo, hi) = body
            .points()
            .fold((DVec2::INFINITY, DVec2::NEG_INFINITY), |(lo, hi), p| (lo.min(p.truncate()), hi.max(p.truncate())));
        let tol = 0.01;
        if !(z1.is_finite() && position.cmpge(lo - tol).all() && position.cmple(hi + tol).all()) {
            return false;
        }
        let (level, elevation) = self.level_for(id, z1);
        let name = self.type_name(id, e);
        let material = self.main_material(id, e);
        let uid = self.type_uid(id);
        let (low, high) = layout.outline().bounds();
        let size = high - low;
        let wanted = layout.clone();
        let same = move |d: &TypeData| {
            matches!(d, TypeData::Foundation { kind: FoundationKind::PileCap, thickness: t, pile_cap: Some(c), .. }
                if (t - thickness).abs() < TYPE_TOL
                    && cc_core::pile::PileCap { pile_type: None, ..c.clone() } == wanted)
        };
        let fallback = format!("{} x {:.0} cm", layout.layout.count(), thickness * 100.0);
        let type_id = self.find_or_add_type(Category::Foundation, name, fallback, uid, same, || TypeData::Foundation {
            kind: FoundationKind::PileCap,
            width: size.x,
            length: size.y,
            thickness,
            material,
            haunch: None,
            pile_cap: Some(layout),
        });
        if !pile_type.trim().is_empty() {
            self.cap_types.push((type_id, pile_type));
        }
        let foundation = Foundation::new(FoundationShape::Pad { position, rotation_deg }, z1 - elevation);
        self.add_element(id, e, level, type_id, Self::tag(e), ElementKind::Foundation(foundation));
        true
    }

    /// A pile whose body is a vertical extrusion of a circle or a square: a
    /// native pile, its head and toe at the body's top and bottom, of a pile
    /// type (found or made) of that section and kind. One this program wrote
    /// keeps its kind and embedment from [`crate::export::PILE_PSET`] and is
    /// linked to its cap when the import ends; another program's takes its
    /// kind from its predefined and construction types. Other piles come in
    /// as components.
    fn import_pile(&mut self, id: u32, e: &Entity) -> bool {
        use cc_core::pile::{Pile, PileKind, PileSection};
        if self.has_openings(id) {
            return false;
        }
        let (_, body) = self.product(e);
        let Some(ex) = body.sole_extrusion().filter(|e| e.is_vertical()).cloned() else { return false };
        let Some((z0, z1)) = body.z_range() else { return false };
        // The profile plane seen from above, unscaled, so the section is the profile's.
        let (xw, yw) = (ex.plane_vector(DVec2::X).truncate(), ex.plane_vector(DVec2::Y).truncate());
        let rigid = (xw.length() - 1.0).abs() < 1e-6 && (yw.length() - 1.0).abs() < 1e-6 && xw.dot(yw).abs() < 1e-6;
        if !(rigid && xw.perp_dot(yw) > 0.0) {
            return false;
        }
        let Some(read) = self.read_section(id, &ex) else { return false };
        let center = ex.plane_point(read.center).truncate();
        let turn = xw.y.atan2(xw.x).to_degrees() + read.rotation_deg;
        let (section, rotation_deg) = match read.shape {
            ProfileShape::Round { d } => (PileSection::Round { diameter: d }, 0.0),
            ProfileShape::Rect { b, h, chamfer } if chamfer == 0.0 && (b - h).abs() <= 1e-6 * b.max(1.0) => {
                (PileSection::Square { side: b }, turn)
            }
            _ => return false,
        };
        if !(section.is_valid() && cc_core::pile::length_ok(z1 - z0) && center.is_finite() && rotation_deg.is_finite())
        {
            return false;
        }
        let own = |n: &str| n == crate::export::PILE_PSET;
        let first = |name: &str| self.pset_texts(id, &own, name).first().map(|s| s.to_string());
        let kind = first("Kind").and_then(|k| PileKind::from_code(&k)).unwrap_or_else(|| {
            let object_type = e.arg(4).as_str().unwrap_or_default().to_ascii_uppercase();
            match (e.arg(8).as_enum(), e.arg(9).as_enum()) {
                (Some("BORED"), _) => PileKind::Bored,
                (Some("DRIVEN"), Some("PREFAB_STEEL")) => PileKind::Steel,
                (Some("DRIVEN"), _) => PileKind::PrecastDriven,
                (Some("USERDEFINED"), _) if object_type.contains("AUGER") => PileKind::ContinuousFlightAuger,
                (Some("USERDEFINED"), _) if object_type.contains("MICRO") => PileKind::Micro,
                // The predefined type of older files says how the pile carries its load
                // (friction, cohesion, support): the construction type says how it is made.
                (_, Some("CAST_IN_PLACE")) => PileKind::Bored,
                (_, Some("PRECAST_CONCRETE")) => PileKind::PrecastDriven,
                (_, Some("PREFAB_STEEL")) => PileKind::Steel,
                _ => PileKind::Other,
            }
        });
        let embedment = first("Embedment")
            .and_then(|t| cc_core::curtain::from_text::<f64>(&t))
            .filter(|v| v.is_finite() && (0.0..=cc_core::pile::MAX_EMBEDMENT).contains(v))
            .unwrap_or(0.0);
        let cap = first("Cap").zip(first("CapIndex").and_then(|i| i.trim().parse::<u32>().ok()));
        let (level, elevation) = self.level_for(id, z1);
        let name = self.type_name(id, e);
        let material = self.main_material(id, e);
        let uid = self.type_uid(id);
        let fallback = format!("{:.0} cm", section.size() * 100.0);
        let same = move |d: &TypeData| {
            matches!(d, TypeData::Pile { section: s, kind: k, .. }
                if *k == kind && std::mem::discriminant(s) == std::mem::discriminant(&section)
                    && (s.size() - section.size()).abs() < TYPE_TOL)
        };
        let type_id = self.find_or_add_type(Category::Pile, name, fallback, uid, same, || TypeData::Pile {
            section,
            kind,
            material,
        });
        let pile =
            Pile { position: center, rotation_deg, cut_off: z1 - elevation, toe: z0 - elevation, embedment, cap: None };
        let element = self.add_element(id, e, level, type_id, Self::tag(e), ElementKind::Pile(pile));
        if let Some((guid, index)) = cap {
            self.cap_piles.push((element, guid, index));
        }
        true
    }

    /// Pile caps take their piles' type by name, and piles this program
    /// wrote under a cap belong to it again (standing where it wants them).
    fn link_piles(&mut self) {
        for (type_id, name) in std::mem::take(&mut self.cap_types) {
            let pile_type = self.p.types_of(Category::Pile).find(|t| t.name == name).map(|t| t.id);
            if let Some(TypeData::Foundation { pile_cap: Some(cap), .. }) =
                self.p.types.get_mut(&type_id).map(|t| &mut t.data)
            {
                cap.pile_type = pile_type;
            }
        }
        for (pile, guid, index) in std::mem::take(&mut self.cap_piles) {
            let Some(uid) = guid::decode(guid.trim()).map(Uid) else { continue };
            let Some(cap) = self.p.elements.values().find(|c| c.uid == uid && self.p.pile_cap(c).is_some()) else {
                continue;
            };
            let level = cap.level;
            let Some((_, type_id, want)) = self.p.cap_piles(cap).into_iter().find(|(i, _, _)| *i == index) else {
                continue;
            };
            if let Some(e) = self.p.elements.get_mut(&pile) {
                e.level = level;
                e.type_id = type_id;
                e.kind = ElementKind::Pile(want);
            }
        }
    }

    /// The part of the structure a product belongs to, as other programs
    /// write it: the text of a property named "Partition" (in English,
    /// Portuguese, Spanish or French, with or without accents) in any of its
    /// property sets.
    fn foreign_partition(&self, owner: u32) -> Option<String> {
        let f = self.f;
        let plain = |name: &str| -> String {
            name.trim()
                .to_lowercase()
                .chars()
                .map(|c| match c {
                    'ç' => 'c',
                    'ã' | 'á' | 'à' | 'â' => 'a',
                    'ó' | 'õ' | 'ô' => 'o',
                    'í' => 'i',
                    other => other,
                })
                .collect()
        };
        let wanted = ["partition", "particao", "particion", "partie"];
        for pset in self.psets_of.get(&owner).into_iter().flatten().filter_map(|s| f.get(*s)) {
            if pset.name != "IFCPROPERTYSET" {
                continue;
            }
            for prop in pset.arg(4).as_list().iter().filter_map(|p| f.deref(p)) {
                if prop.name != "IFCPROPERTYSINGLEVALUE"
                    || !prop.arg(0).as_str().is_some_and(|n| wanted.contains(&plain(n).as_str()))
                {
                    continue;
                }
                let text = prop.arg(2).as_str().map(clean_name).filter(|t| !t.is_empty());
                if let Some(text) = text.filter(|t| t.chars().count() <= cc_core::bar_marks::MAX_MARK_TEXT) {
                    return Some(text);
                }
            }
        }
        None
    }

    /// After a native import attempt: anything not imported natively comes in
    /// as a generic object, and is reported only when it has no usable geometry.
    fn or_generic(&mut self, id: u32, e: &Entity, native: bool) {
        if !native && !self.import_generic(id, e) {
            self.skip(e);
        }
    }

    /// A placed object with a shape (an `IfcProduct` with geometry).
    fn is_product(&self, e: &Entity) -> bool {
        let placed = self.f.deref(e.arg(5)).is_some_and(|p| {
            matches!(p.name.as_str(), "IFCLOCALPLACEMENT" | "IFCGRIDPLACEMENT" | "IFCLINEARPLACEMENT")
        });
        placed && self.is_shaped(e)
    }

    fn is_shaped(&self, e: &Entity) -> bool {
        self.f.deref(e.arg(6)).is_some_and(|s| s.name == "IFCPRODUCTDEFINITIONSHAPE")
    }

    fn skip(&mut self, e: &Entity) {
        *self.report.skipped.entry(e.name.clone()).or_default() += 1;
    }

    // --- Setup ---

    /// Clear what the import replaces: levels, elements, plans and sheets.
    fn prepare_base(&mut self) {
        self.p.elements.clear();
        self.p.levels.clear();
        self.p.views.retain(|_, v| {
            !matches!(v.kind, ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. } | ViewKind::Sheet { .. })
        });
    }

    fn read_project_info(&mut self) {
        let Some(e) = self.f.get(self.project_id) else { return };
        if let Some(name) = e.arg(2).as_str().filter(|s| !s.is_empty()) {
            self.p.info.name = name.to_string();
        }
        self.p.info.number = e.arg(5).as_str().unwrap_or_default().to_string();
        self.p.info.uid = e.arg(0).as_str().and_then(guid::decode).map(Uid).unwrap_or_default();
        self.p.info.client.clear();
        self.p.info.author.clear();
        self.p.info.address.clear();
        let address = self.f.of_type("IFCBUILDING").find_map(|(_, b)| self.f.deref(b.arg(11)));
        if let Some(lines) = address.map(|a| a.arg(4).as_list()) {
            let lines: Vec<&str> = lines.iter().filter_map(Param::as_str).collect();
            self.p.info.address = lines.join(", ");
        }
        self.p.info.site = self.read_site();
    }

    /// The site location: the IfcSite's reference latitude, longitude and
    /// elevation, its long name and postal address (else the building's),
    /// and true north from the model context.
    fn read_site(&self) -> Option<cc_core::site::SiteLocation> {
        let (_, site) = self.f.of_type("IFCSITE").next()?;
        let angle = |i: usize| {
            let parts: Vec<i64> = site.arg(i).as_list().iter().filter_map(|p| p.as_f64()).map(|v| v as i64).collect();
            cc_core::site::from_dms(&parts)
        };
        let mut location = cc_core::site::SiteLocation::at(angle(9)?, angle(10)?);
        location.elevation = site.arg(11).as_f64().map_or(0.0, |z| z * self.unit("LENGTHUNIT"));
        location.place = site.arg(7).as_str().unwrap_or_default().to_string();
        let address = self
            .f
            .deref(site.arg(13))
            .or_else(|| self.f.of_type("IFCBUILDING").find_map(|(_, b)| self.f.deref(b.arg(11))));
        if let Some(a) = address.filter(|a| a.name == "IFCPOSTALADDRESS") {
            let text = |i: usize| a.arg(i).as_str().unwrap_or_default().to_string();
            location.address.city = text(6);
            location.address.region = text(7);
            location.address.postcode = text(8);
            location.address.country = text(9);
        }
        let north = self
            .f
            .of_type("IFCGEOMETRICREPRESENTATIONCONTEXT")
            .find(|(_, c)| c.arg(1).as_str() == Some("Model"))
            .and_then(|(_, c)| self.f.deref(c.arg(5)))
            .map(|d| d.arg(0).as_list().iter().filter_map(|p| p.as_f64()).collect::<Vec<_>>());
        if let Some([x, y, ..]) = north.as_deref()
            && (x * x + y * y) > 1e-12
        {
            location.true_north = (-x).atan2(*y).to_degrees();
        }
        let (valid, _) = location.repair();
        valid.then_some(location)
    }

    /// SI value of one file unit of `kind` (`LENGTHUNIT`: meters; `PLANEANGLEUNIT`: radians), 1 if unset.
    fn unit(&self, kind: &str) -> f64 {
        self.unit_opt(kind).unwrap_or(1.0)
    }

    /// SI value of one file unit of `kind`, if the file assigns one.
    fn unit_opt(&self, kind: &str) -> Option<f64> {
        let f = self.f;
        let assignment = f
            .get(self.project_id)
            .and_then(|p| f.deref(p.arg(8)))
            .or_else(|| f.of_type("IFCUNITASSIGNMENT").next().map(|(_, e)| e))?;
        assignment
            .arg(0)
            .as_list()
            .iter()
            .filter_map(|u| f.deref(u))
            .filter(|u| u.arg(1).as_enum() == Some(kind))
            .find_map(|u| self.unit_factor(u))
    }

    /// SI value of one `unit` (an `IfcSIUnit` or conversion-based unit).
    fn unit_factor(&self, unit: &Entity) -> Option<f64> {
        let power = |kind: Option<&str>| match kind {
            Some("AREAUNIT") => 2,
            Some("VOLUMEUNIT") => 3,
            _ => 1,
        };
        // A prefix applies to the metre before squaring or cubing (mm², mm³).
        let si = |u: &Entity| si_prefix(u.arg(2).as_enum()).powi(power(u.arg(1).as_enum()));
        let factor = match unit.name.as_str() {
            "IFCSIUNIT" => si(unit),
            "IFCCONVERSIONBASEDUNIT" | "IFCCONVERSIONBASEDUNITWITHOFFSET" => {
                let measure = self.f.deref(unit.arg(3));
                let base =
                    measure.and_then(|m| self.f.deref(m.arg(1))).filter(|b| b.name == "IFCSIUNIT").map_or(1.0, si);
                let value = measure.and_then(|m| m.arg(0).as_f64()).map(|v| v * base);
                match value {
                    Some(v) if v.is_finite() && v > 0.0 => v,
                    // Degrees without a usable conversion factor.
                    _ if unit.arg(1).as_enum() == Some("PLANEANGLEUNIT")
                        && unit.arg(2).as_str().is_some_and(|n| n.eq_ignore_ascii_case("degree")) =>
                    {
                        std::f64::consts::PI / 180.0
                    }
                    _ => return None,
                }
            }
            _ => return None,
        };
        (factor.is_finite() && factor > 0.0).then_some(factor)
    }

    fn index_relationships(&mut self) {
        let f = self.f;
        let refs = |p: &Param| p.as_list().iter().filter_map(Param::as_ref).collect::<Vec<_>>();
        for e in f.entities.values() {
            match e.name.as_str() {
                "IFCRELCONTAINEDINSPATIALSTRUCTURE" => {
                    let Some(whole) = e.arg(5).as_ref() else { continue };
                    for part in refs(e.arg(4)) {
                        self.parent.insert(part, whole);
                    }
                }
                "IFCRELAGGREGATES" | "IFCRELNESTS" => {
                    let Some(whole) = e.arg(4).as_ref() else { continue };
                    for part in refs(e.arg(5)) {
                        self.parent.entry(part).or_insert(whole);
                        *self.has_parts.entry(whole).or_default() += 1;
                    }
                }
                "IFCRELDEFINESBYTYPE" => {
                    let Some(ty) = e.arg(5).as_ref() else { continue };
                    for object in refs(e.arg(4)) {
                        self.type_of.insert(object, ty);
                    }
                }
                "IFCRELASSOCIATESMATERIAL" => {
                    let Some(material) = e.arg(5).as_ref() else { continue };
                    for object in refs(e.arg(4)) {
                        self.material_of.insert(object, material);
                    }
                }
                "IFCRELDEFINESBYPROPERTIES" => {
                    let Some(set) = e.arg(5).as_ref() else { continue };
                    for object in refs(e.arg(4)) {
                        self.psets_of.entry(object).or_default().push(set);
                    }
                }
                "IFCRELVOIDSELEMENT" => {
                    if let (Some(host), Some(opening)) = (e.arg(4).as_ref(), e.arg(5).as_ref()) {
                        self.host_of_opening.insert(opening, host);
                        self.openings_of.entry(host).or_default().push(opening);
                    }
                }
                "IFCRELFILLSELEMENT" => {
                    if let (Some(opening), Some(filler)) = (e.arg(4).as_ref(), e.arg(5).as_ref()) {
                        self.opening_of_filler.insert(filler, opening);
                    }
                }
                _ => {}
            }
        }
        // Type objects can also carry their property sets directly.
        for (id, e) in f.entities.iter() {
            if e.name.ends_with("TYPE") || e.name.ends_with("STYLE") {
                let sets = refs(e.arg(5));
                let is_set =
                    |s: &u32| f.get(*s).is_some_and(|x| x.name == "IFCPROPERTYSET" || x.name == "IFCELEMENTQUANTITY");
                if !sets.is_empty() && sets.iter().all(is_set) {
                    self.psets_of.entry(*id).or_default().extend(sets);
                }
            }
        }
    }

    fn index_styles(&mut self) {
        let f = self.f;
        let color_of_style = |style: &Entity| -> Option<[u8; 3]> {
            for s in style.arg(2).as_list().iter().filter_map(|s| f.deref(s)) {
                if s.name.starts_with("IFCSURFACESTYLE") {
                    let rgb = f.deref(s.arg(0))?;
                    let c = |i| (rgb.arg(i).as_f64().unwrap_or(0.8).clamp(0.0, 1.0) * 255.0).round() as u8;
                    return Some([c(1), c(2), c(3)]);
                }
            }
            None
        };
        for (_, style) in f.of_type("IFCSURFACESTYLE") {
            if let (Some(name), Some(color)) = (style.arg(0).as_str(), color_of_style(style)) {
                self.style_colors.entry(name.to_string()).or_insert(color);
            }
        }
        for (_, styled) in f.of_type("IFCSTYLEDITEM") {
            let Some(item) = styled.arg(0).as_ref() else { continue };
            // IFC2X3 wraps styles in presentation style assignments.
            let mut styles: Vec<&Entity> = Vec::new();
            for s in styled.arg(1).as_list().iter().filter_map(|s| f.deref(s)) {
                if s.name == "IFCPRESENTATIONSTYLEASSIGNMENT" {
                    styles.extend(s.arg(0).as_list().iter().filter_map(|x| f.deref(x)));
                } else {
                    styles.push(s);
                }
            }
            let surface = styles.iter().filter(|s| s.name == "IFCSURFACESTYLE");
            if let Some((style, color)) = surface.clone().find_map(|s| Some((s, color_of_style(s)?))) {
                self.item_colors.insert(item, color);
                if let Some(name) = style.arg(0).as_str().filter(|n| !n.trim().is_empty()) {
                    self.item_styles.insert(item, name.to_string());
                }
            }
        }
    }

    fn read_storeys(&mut self) {
        let f = self.f;
        let mut storeys: Vec<(u32, String, f64)> = Vec::new();
        for (id, e) in f.of_type("IFCBUILDINGSTOREY") {
            let world = self.g.placement(e.arg(5));
            let attr = e.arg(9).as_f64().map(|z| z * self.scale);
            let z = world.translation.z;
            // Prefer the placement (consistent with element coordinates).
            let elevation = if z.abs() < 1e-9 { attr.unwrap_or(0.0) } else { z };
            let name = e.arg(2).as_str().or(e.arg(7).as_str()).unwrap_or_default().to_string();
            storeys.push((id, name, elevation));
        }
        storeys.sort_by(|a, b| a.2.total_cmp(&b.2));
        for (i, (id, name, elevation)) in storeys.into_iter().enumerate() {
            let level = LevelId(self.p.alloc_id());
            let name = if name.is_empty() { format!("{i}") } else { name };
            self.p.levels.push(Level { id: level, name, elevation });
            self.storeys.insert(id, level);
        }
        if self.p.levels.is_empty() {
            let level = LevelId(self.p.alloc_id());
            self.p.levels.push(Level { id: level, name: "0".into(), elevation: 0.0 });
        }
        self.report.levels = self.p.levels.len();
    }

    /// Text values of property `name` in the property sets of `owner` whose
    /// name passes `set` (single and enumerated values).
    fn pset_texts(&self, owner: u32, set: &dyn Fn(&str) -> bool, name: &str) -> Vec<&'a str> {
        let f = self.f;
        let mut out = Vec::new();
        for pset in self.psets_of.get(&owner).into_iter().flatten().filter_map(|s| f.get(*s)) {
            if pset.name != "IFCPROPERTYSET" || !pset.arg(2).as_str().is_some_and(set) {
                continue;
            }
            for prop in pset.arg(4).as_list().iter().filter_map(|p| f.deref(p)) {
                if prop.arg(0).as_str() != Some(name) {
                    continue;
                }
                match prop.name.as_str() {
                    "IFCPROPERTYSINGLEVALUE" => out.extend(prop.arg(2).as_str()),
                    "IFCPROPERTYENUMERATEDVALUE" => out.extend(prop.arg(2).as_list().iter().filter_map(Param::as_str)),
                    _ => {}
                }
            }
        }
        out
    }

    /// Design options as this program writes them ([`crate::export::DESIGN_OPTION_PSET`]):
    /// sets and options by name, the first option of a set its primary (exports
    /// write primary options only); hosted elements then follow their hosts.
    fn import_design_options(&mut self) {
        use cc_core::design_option::{MAX_OPTION_SETS, MAX_OPTIONS};
        use cc_core::{DesignOption, OptionId, OptionSet, OptionSetId};
        let is_set = |n: &str| n == crate::export::DESIGN_OPTION_PSET;
        let mut found: Vec<(ElementId, String, String)> = Vec::new();
        for (element, ifc) in &self.products {
            let first = |name: &str| self.pset_texts(*ifc, &is_set, name).first().map(|s| s.trim().to_string());
            let (Some(set), Some(option)) = (first("OptionSet"), first("Option")) else { continue };
            if set.is_empty() || option.is_empty() {
                continue;
            }
            found.push((*element, set, option));
        }
        for (element, set_name, option_name) in found {
            let si = match self.p.option_sets.iter().position(|s| s.name.eq_ignore_ascii_case(&set_name)) {
                Some(i) => i,
                None if self.p.option_sets.len() < MAX_OPTION_SETS => {
                    let id = OptionSetId(self.p.alloc_id());
                    self.p.option_sets.push(OptionSet {
                        id,
                        name: set_name,
                        options: Vec::new(),
                        primary: OptionId(0),
                    });
                    self.p.option_sets.len() - 1
                }
                None => continue,
            };
            let existing = self
                .p
                .option_sets
                .get(si)
                .and_then(|s| s.options.iter().find(|o| o.name.eq_ignore_ascii_case(&option_name)).map(|o| o.id));
            let option = match existing {
                Some(o) => o,
                None => {
                    let id = OptionId(self.p.alloc_id());
                    let Some(s) = self.p.option_sets.get_mut(si).filter(|s| s.options.len() < MAX_OPTIONS) else {
                        continue;
                    };
                    s.options.push(DesignOption { id, name: option_name });
                    id
                }
            };
            if let Some(s) = self.p.option_sets.get_mut(si)
                && !s.contains(s.primary)
            {
                s.primary = option;
            }
            if let Some(e) = self.p.elements.get_mut(&element) {
                e.design_option = Some(option);
            }
        }
        for (id, option) in self.p.option_fixes() {
            if let Some(e) = self.p.elements.get_mut(&id) {
                e.design_option = option;
            }
        }
    }

    /// Construction phases: the project's phases when the file lists them
    /// (as this program writes them), then each element's phases by name,
    /// or else from its IFC `Status` relative to the last phase (existing:
    /// built in the phase before it; demolished: built before it and
    /// demolished in it; new and temporary: built in it).
    fn import_phases(&mut self) {
        let phases_set = |n: &str| n == crate::export::PHASES_PSET;
        let mut names: Vec<(usize, String)> = Vec::new();
        for pset in self.psets_of.get(&self.project_id).into_iter().flatten().filter_map(|s| self.f.get(*s)) {
            if pset.name != "IFCPROPERTYSET" || !pset.arg(2).as_str().is_some_and(phases_set) {
                continue;
            }
            for prop in pset.arg(4).as_list().iter().filter_map(|p| self.f.deref(p)).take(MAX_PHASES) {
                let (Some(index), Some(name)) =
                    (prop.arg(0).as_str().and_then(|i| i.trim().parse::<usize>().ok()), prop.arg(2).as_str())
                else {
                    continue;
                };
                let name = name.trim();
                if !name.is_empty() && !names.iter().any(|(_, n)| n.eq_ignore_ascii_case(name)) {
                    names.push((index, name.to_string()));
                }
            }
        }
        names.sort_by_key(|(i, _)| *i);
        if !names.is_empty() {
            self.p.phases = names
                .into_iter()
                .map(|(_, name)| cc_core::Phase {
                    id: cc_core::PhaseId(self.p.alloc_id()),
                    name,
                    description: String::new(),
                })
                .collect();
        }
        let Some(last) = self.p.phases.len().checked_sub(1) else { return };
        let phase_at = |p: &Project, i: usize| p.phases.get(i).map(|ph| ph.id);
        let by_name =
            |p: &Project, n: &str| p.phases.iter().find(|ph| ph.name.eq_ignore_ascii_case(n.trim())).map(|ph| ph.id);
        let previous = last.saturating_sub(1);
        for (element, ifc) in std::mem::take(&mut self.products) {
            let phasing = |n: &str| n == crate::export::PHASING_PSET;
            let created = self.pset_texts(ifc, &phasing, "PhaseCreated").first().and_then(|n| by_name(&self.p, n));
            let demolished =
                self.pset_texts(ifc, &phasing, "PhaseDemolished").first().and_then(|n| by_name(&self.p, n));
            let common = |n: &str| n.starts_with("Pset_") && n.ends_with("Common");
            let status = self.pset_texts(ifc, &common, "Status").into_iter().find_map(cc_core::PhaseStatus::from_ifc);
            let (created, demolished) = match (created, status) {
                (Some(c), _) => (Some(c), demolished),
                (None, Some(st)) => {
                    use cc_core::PhaseStatus as S;
                    let (c, d) = match st {
                        S::New => (last, None),
                        S::Existing => (previous, None),
                        S::Demolished => (previous, Some(last)),
                        S::Temporary => (last, Some(last)),
                    };
                    (phase_at(&self.p, c), d.and_then(|d| phase_at(&self.p, d)))
                }
                (None, None) => (None, None),
            };
            let Some(e) = self.p.elements.get_mut(&element) else { continue };
            if let Some(c) = created {
                e.phase_created = c;
            }
            e.phase_demolished = demolished;
        }
    }

    fn finish(&mut self) {
        self.link_piles();
        self.import_design_options();
        self.import_phases();
        let levels: Vec<(LevelId, String)> = self.p.levels.iter().map(|l| (l.id, l.name.clone())).collect();
        let add_view = |p: &mut Project, name: &str, kind: ViewKind| {
            let id = ViewId(p.alloc_id());
            p.views.insert(id, View::new(id, name, 50, kind));
        };
        for (level, name) in &levels {
            add_view(&mut self.p, name, ViewKind::floor_plan(*level));
        }
        for (level, name) in &levels {
            add_view(&mut self.p, name, ViewKind::ceiling_plan(*level));
        }
        self.p.ensure_annotation_types();
        self.p.ensure_uids();
        // New plans show the last phase with the default phase filter.
        let (last, filter) = (self.p.last_phase().unwrap_or_default(), self.p.default_phase_filter());
        for v in self.p.views.values_mut().filter(|v| self.p.phases.iter().all(|ph| ph.id != v.phase)) {
            v.phase = last;
            v.phase_filter = v.phase_filter.or(filter);
        }
        self.p.repair();
    }

    // --- Lookups ---

    fn storey_of(&self, mut id: u32) -> Option<LevelId> {
        for _ in 0..MAX_PARENT_DEPTH {
            id = *self.parent.get(&id)?;
            if let Some(level) = self.storeys.get(&id) {
                return Some(*level);
            }
        }
        None
    }

    /// Level of a product: its storey, else the highest level at or below `z`.
    fn level_for(&self, id: u32, z: f64) -> (LevelId, f64) {
        let level = self.storey_of(id).or_else(|| {
            let below = self.p.levels.iter().filter(|l| l.elevation <= z + 1e-3);
            below.max_by(|a, b| a.elevation.total_cmp(&b.elevation)).map(|l| l.id)
        });
        let level = level.or_else(|| self.p.levels.first().map(|l| l.id)).unwrap_or(LevelId(0));
        (level, self.p.level_elevation(level))
    }

    fn type_entity(&self, id: u32) -> Option<&'a Entity> {
        self.type_of.get(&id).and_then(|t| self.f.get(*t))
    }

    /// Type name: the IFC type object's, else the product's name.
    fn type_name(&self, id: u32, e: &Entity) -> Option<String> {
        self.type_entity(id)
            .and_then(|t| t.arg(2).as_str())
            .or(e.arg(2).as_str())
            .map(clean_name)
            .filter(|s| !s.is_empty())
    }

    /// A property value from the product's (or its type's) property sets.
    fn property(&self, id: u32, set: &str, name: &str) -> Option<&'a Param> {
        let f = self.f;
        let mut owners = vec![id];
        owners.extend(self.type_of.get(&id));
        for owner in owners {
            for set_id in self.psets_of.get(&owner).into_iter().flatten() {
                let Some(pset) = f.get(*set_id).filter(|s| s.name == "IFCPROPERTYSET") else { continue };
                if pset.arg(2).as_str() != Some(set) {
                    continue;
                }
                for prop in pset.arg(4).as_list().iter().filter_map(|p| f.deref(p)) {
                    if prop.name == "IFCPROPERTYSINGLEVALUE" && prop.arg(0).as_str() == Some(name) {
                        return Some(prop.arg(2));
                    }
                }
            }
        }
        None
    }

    /// Numeric properties of a property set of the product.
    fn numeric_properties(&self, id: u32, set: &str) -> Vec<(String, f64, Option<String>)> {
        let f = self.f;
        let mut out = Vec::new();
        for set_id in self.psets_of.get(&id).into_iter().flatten() {
            let Some(pset) = f.get(*set_id).filter(|s| s.name == "IFCPROPERTYSET" && s.arg(2).as_str() == Some(set))
            else {
                continue;
            };
            for prop in pset.arg(4).as_list().iter().filter_map(|p| f.deref(p)) {
                let (Some(name), Some(v)) = (prop.arg(0).as_str(), prop.arg(2).as_f64()) else { continue };
                let measure = match prop.arg(2) {
                    Param::Typed(t, _) => Some(t.clone()),
                    _ => None,
                };
                out.push((name.to_string(), v, measure));
            }
        }
        out
    }

    /// Material layers (name, thickness) and the main material name of a
    /// product or its type.
    fn materials(&self, id: u32) -> (Vec<(String, f64)>, Option<String>) {
        let f = self.f;
        let mut owners = vec![id];
        owners.extend(self.type_of.get(&id));
        for owner in owners {
            let Some(mut m) = self.material_of.get(&owner).and_then(|m| f.get(*m)) else { continue };
            for _ in 0..4 {
                match m.name.as_str() {
                    "IFCMATERIALLAYERSETUSAGE" => match f.deref(m.arg(0)) {
                        Some(set) => m = set,
                        None => break,
                    },
                    "IFCMATERIALLAYERSET" => {
                        let layers: Vec<(String, f64)> = m
                            .arg(0)
                            .as_list()
                            .iter()
                            .filter_map(|l| f.deref(l))
                            .map(|l| {
                                let name = f.deref(l.arg(0)).and_then(|mat| mat.arg(0).as_str()).unwrap_or("IFC");
                                (name.to_string(), l.arg(1).as_f64().unwrap_or(0.0) * self.scale)
                            })
                            .collect();
                        let main = layers.first().map(|l| l.0.clone());
                        return (layers, main);
                    }
                    "IFCMATERIAL" => return (Vec::new(), m.arg(0).as_str().map(str::to_string)),
                    "IFCMATERIALLIST" => {
                        let first = m.arg(0).as_list().first().and_then(|x| f.deref(x));
                        return (Vec::new(), first.and_then(|x| x.arg(0).as_str()).map(str::to_string));
                    }
                    "IFCMATERIALCONSTITUENTSET" | "IFCMATERIALPROFILESET" => {
                        let first = m.arg(2).as_list().first().and_then(|x| f.deref(x));
                        let material = first.and_then(|x| f.deref(x.arg(2))).and_then(|x| x.arg(0).as_str());
                        return (Vec::new(), material.map(str::to_string));
                    }
                    "IFCMATERIALPROFILESETUSAGE" => match f.deref(m.arg(0)) {
                        Some(set) => m = set,
                        None => break,
                    },
                    _ => break,
                }
            }
        }
        (Vec::new(), None)
    }

    /// Ensure a project material named `name`, colored by a style of the same
    /// name or by `fallback`.
    fn material(&mut self, name: &str, fallback: Option<[u8; 3]>) -> String {
        if !self.p.materials.iter().any(|m| m.name == name) {
            let color = self.style_colors.get(name).copied().or(fallback).unwrap_or([200, 200, 200]);
            self.p.materials.push(Material::new(name, color));
        }
        name.to_string()
    }

    /// Main material of a product (created if needed), colored like its body.
    fn main_material(&mut self, id: u32, e: &Entity) -> String {
        let (_, main) = self.materials(id);
        let color = self.body_color(e);
        let name = main.unwrap_or_else(|| "IFC".to_string());
        self.material(&name, color)
    }

    fn body_color(&self, e: &Entity) -> Option<[u8; 3]> {
        let f = self.f;
        let shape = f.deref(e.arg(6))?;
        for rep in shape.arg(2).as_list().iter().filter_map(|r| f.deref(r)) {
            for item in rep.arg(3).as_list().iter().filter_map(Param::as_ref) {
                if let Some(c) = self.item_colors.get(&item) {
                    return Some(*c);
                }
            }
        }
        None
    }

    /// Layers for a layered type of `thickness`: the file's layer set when it
    /// adds up, else one layer of the main material.
    fn layers(&mut self, id: u32, e: &Entity, thickness: f64) -> Vec<Layer> {
        let (layers, main) = self.materials(id);
        let color = self.body_color(e);
        let total: f64 = layers.iter().map(|l| l.1).sum();
        if !layers.is_empty() && (total - thickness).abs() < TYPE_TOL && layers.iter().all(|l| l.1 > 0.0) {
            let n = layers.len();
            return layers
                .into_iter()
                .enumerate()
                .map(|(i, (name, t))| {
                    let material = self.material(&name, color);
                    // Outer layers of a multi-layer set are finishes.
                    let function =
                        if n > 2 && (i == 0 || i == n - 1) { LayerFunction::Finish } else { LayerFunction::Structure };
                    Layer { material, thickness: t, function }
                })
                .collect();
        }
        let material = self.material(&main.unwrap_or_else(|| "IFC".to_string()), color);
        vec![Layer { material, thickness, function: LayerFunction::Structure }]
    }

    /// A type of `category` named `name` whose data satisfies `same`, else a
    /// new one from `make`.
    fn find_or_add_type(
        &mut self,
        category: Category,
        name: Option<String>,
        fallback_name: String,
        uid: Option<Uid>,
        same: impl Fn(&TypeData) -> bool,
        make: impl FnOnce() -> TypeData,
    ) -> ElementTypeId {
        let named = |t: &ElementType| name.as_deref().is_none_or(|n| t.name == n);
        if let Some(t) = self.p.types_of(category).find(|t| named(t) && same(&t.data)) {
            return t.id;
        }
        if name.is_none()
            && let Some(t) = self.p.types_of(category).find(|t| same(&t.data))
        {
            return t.id;
        }
        // The type an IFC type object already became (also in a project it
        // was imported into before).
        if let Some(uid) = uid.filter(|u| *u != Uid::default())
            && let Some(t) = self.p.types_of(category).find(|t| t.uid == uid && same(&t.data))
        {
            return t.id;
        }
        let mut base = name.unwrap_or(fallback_name);
        // A type this import added for the same name, renamed because the
        // project had one: the next occurrences of that name reuse it.
        let key = (category, base.clone());
        let types = &self.p.types;
        let added = self.added_types.get(&key).into_iter().flatten();
        if let Some(&id) = added.into_iter().find(|id| types.get(id).is_some_and(|t| same(&t.data))) {
            return id;
        }
        if self.p.types_of(category).any(|t| t.name == base) {
            base = format!("{base} (IFC)");
        }
        let mut unique = base.clone();
        let mut n = 2;
        while self.p.types_of(category).any(|t| t.name == unique) {
            unique = format!("{base} {n}");
            n += 1;
        }
        let id = ElementTypeId(self.p.alloc_id());
        // An IFC type whose occurrences differ (in size, say) makes several
        // types: only the first keeps its GlobalId, so uids stay unique.
        let uid = uid.filter(|u| !self.p.types.values().any(|t| t.uid == *u)).unwrap_or_default();
        let ty = ElementType { id, name: unique, data: make(), info: Default::default(), uid };
        self.p.types.insert(id, ty);
        self.added_types.entry(key).or_default().push(id);
        id
    }

    /// How the panels of product `id` open: the `IfcWindowPanelProperties`
    /// among its type's property sets, from the left; fixed when it has none
    /// or panels no [`WindowOperation`] stands for.
    fn window_operation(&self, id: u32) -> WindowOperation {
        let Some(t) = self.type_entity(id) else { return WindowOperation::Fixed };
        let rank = |position: &str| match position {
            "LEFT" | "TOP" => 0,
            "RIGHT" | "BOTTOM" => 2,
            _ => 1,
        };
        let mut panels: Vec<(u8, &str)> = t
            .arg(5)
            .as_list()
            .iter()
            .filter_map(|p| self.f.get(p.as_ref()?))
            .filter(|e| e.name == "IFCWINDOWPANELPROPERTIES")
            .take(MAX_PANELS)
            .filter_map(|e| Some((rank(e.arg(5).as_enum().unwrap_or_default()), e.arg(4).as_enum()?)))
            .collect();
        panels.sort_by_key(|(rank, _)| *rank);
        let operations: Vec<&str> = panels.into_iter().map(|(_, op)| op).collect();
        WindowOperation::from_ifc_panels(&operations).unwrap_or_default()
    }

    /// How the leaves of door `e` (product `id`) swing, and whether it is
    /// placed mirrored: the operation type of its type (`IfcDoorType`, or
    /// an IFC2X3 `IfcDoorStyle`), else its own; a door whose own operation
    /// is its type's with the other hand is that type flipped. Doors of
    /// operations no [`DoorOperation`] stands for swing on the left.
    fn door_operation(&self, id: u32, e: &Entity) -> (DoorOperation, bool) {
        let typed = self.type_entity(id).and_then(|t| match t.name.as_str() {
            "IFCDOORTYPE" => t.arg(10).as_enum(),
            "IFCDOORSTYLE" => t.arg(8).as_enum(),
            _ => None,
        });
        let typed = typed.and_then(DoorOperation::from_ifc);
        let own = e.arg(11).as_enum().and_then(DoorOperation::from_ifc);
        match (typed, own) {
            (Some(t), Some(o)) => (t, o != t && o == t.mirrored()),
            (Some(op), None) | (None, Some(op)) => (op, false),
            (None, None) => (DoorOperation::SingleSwingLeft, false),
        }
    }

    fn type_uid(&self, id: u32) -> Option<Uid> {
        self.type_entity(id).and_then(|t| t.arg(0).as_str()).and_then(guid::decode).map(Uid)
    }

    fn add_element(
        &mut self,
        ifc: u32,
        e: &Entity,
        level: LevelId,
        type_id: ElementTypeId,
        mark: String,
        kind: ElementKind,
    ) -> ElementId {
        let id = ElementId(self.p.alloc_id());
        self.products.push((id, ifc));
        let uid = e.arg(0).as_str().and_then(guid::decode).map(Uid).unwrap_or_default();
        let comments = e.arg(3).as_str().unwrap_or_default().to_string();
        let element = Element {
            id,
            level,
            type_id,
            mark,
            comments,
            kind,
            uid,
            phase_created: Default::default(),
            phase_demolished: None,
            design_option: None,
            paints: Vec::new(),
        };
        *self.report.imported.entry(element.category()).or_default() += 1;
        self.p.elements.insert(id, element);
        id
    }

    fn tag(e: &Entity) -> String {
        e.arg(7).as_str().unwrap_or_default().to_string()
    }

    /// Body solids and axis of a product placed at `world`.
    fn body(&self, e: &Entity, world: DAffine3) -> Body {
        let f = self.f;
        let mut body = Body::default();
        let Some(shape) = f.deref(e.arg(6)) else { return body };
        let reps: Vec<&Entity> = shape.arg(2).as_list().iter().filter_map(|r| f.deref(r)).collect();
        let id = |r: &Entity| r.arg(1).as_str().unwrap_or_default().to_ascii_lowercase();
        let has_body = reps.iter().any(|r| id(r) == "body");
        for rep in &reps {
            let ident = id(rep);
            if ident == "axis" {
                let pts: Vec<DVec2> = rep
                    .arg(3)
                    .as_list()
                    .iter()
                    .flat_map(|c| self.g.curve(c, 0))
                    .map(|p| world.transform_point3(p).truncate())
                    .collect();
                if let (Some(a), Some(b)) = (pts.first(), pts.last())
                    && a.distance(*b) > 1e-6
                {
                    body.axis = Some((*a, *b));
                }
                body.axis_points = pts;
                continue;
            }
            let usable = if has_body {
                ident == "body"
            } else {
                !matches!(ident.as_str(), "footprint" | "annotation" | "box" | "clearance" | "profile")
            };
            if usable {
                for item in rep.arg(3).as_list() {
                    self.g.items(item, world, &mut body.solids);
                }
            }
        }
        if body.solids.is_empty() {
            // Last resort: a bounding box representation.
            for rep in reps.iter().filter(|r| id(r) == "box") {
                for item in rep.arg(3).as_list() {
                    self.g.items(item, world, &mut body.solids);
                }
            }
        }
        body
    }

    fn product(&self, e: &Entity) -> (DAffine3, Body) {
        let world = self.g.placement(e.arg(5));
        let body = self.body(e, world);
        (world, body)
    }

    // --- Elements ---

    fn import_wall(&mut self, id: u32, e: &Entity) -> bool {
        let (world, body) = self.product(e);
        let Some(ex) = body.extrusion(true) else { return false };
        let Some((z0, z1)) = body.z_range() else { return false };
        let footprint = ex.footprint();
        if let Some((start, end, bulge, thickness)) = arc_wall(&body.axis_points, &footprint) {
            return self.add_wall(id, e, (z0, z1), thickness, Wall { start, end, bulge, ..Default::default() });
        }
        // Wall direction: the axis representation, else the placement's X axis
        // (or Y when the footprint is longer that way).
        let mut dir = world.matrix3.x_axis.truncate().try_normalize().unwrap_or(DVec2::X);
        let mut origin = world.translation.truncate();
        if let Some((a, b)) = body.axis {
            dir = (b - a).try_normalize().unwrap_or(dir);
            origin = a;
        }
        let along = |d: DVec2| {
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            let (mut vlo, mut vhi) = (f64::INFINITY, f64::NEG_INFINITY);
            for p in &footprint {
                let r = *p - origin;
                lo = lo.min(r.dot(d));
                hi = hi.max(r.dot(d));
                vlo = vlo.min(r.dot(d.perp()));
                vhi = vhi.max(r.dot(d.perp()));
            }
            (lo, hi, vlo, vhi)
        };
        let (mut u0, mut u1, mut v0, mut v1) = along(dir);
        if body.axis.is_none() && (v1 - v0) > (u1 - u0) {
            dir = dir.perp();
            (u0, u1, v0, v1) = along(dir);
        }
        let thickness = v1 - v0;
        let on_a_face = |p: &DVec2| {
            let v = (*p - origin).dot(dir.perp());
            (v - v0).abs() < 1e-3 || (v - v1).abs() < 1e-3
        };
        if !footprint.iter().all(on_a_face) {
            return false;
        }
        let n = dir.perp() * ((v0 + v1) / 2.0);
        let (start, end) = match body.axis {
            // The axis keeps its own length (body ends include joins).
            Some((a, b)) => (a + n, b + n),
            None => (origin + dir * u0 + n, origin + dir * u1 + n),
        };
        if !(start.is_finite() && end.is_finite()) || start.distance(end) < 0.01 || thickness < 0.005 {
            return false;
        }
        self.add_wall(id, e, (z0, z1), thickness, Wall { start, end, ..Default::default() })
    }

    /// Add a wall of `thickness` spanning `z` with the type and properties of `e`.
    fn add_wall(&mut self, id: u32, e: &Entity, (z0, z1): (f64, f64), thickness: f64, wall: Wall) -> bool {
        let (level, elevation) = self.level_for(id, z0);
        let name = self.type_name(id, e);
        let fallback = format!("{:.0} cm", thickness * 100.0);
        let layers = self.layers(id, e, thickness);
        let external = self.property(id, "Pset_WallCommon", "IsExternal").and_then(Param::as_bool);
        let structural = self.property(id, "Pset_WallCommon", "LoadBearing").and_then(Param::as_bool);
        let function = if external == Some(false) { WallFunction::Interior } else { WallFunction::Exterior };
        let uid = self.type_uid(id);
        let type_id = self.find_or_add_type(
            Category::Wall,
            name,
            fallback,
            uid,
            |d| matches!(d, TypeData::Wall { layers, curtain: None, .. } if (layers.iter().map(|l| l.thickness).sum::<f64>() - thickness).abs() < TYPE_TOL),
            || TypeData::Wall { layers, function, curtain: None },
        );
        let wall =
            Wall { base_offset: z0 - elevation, height: z1 - z0, structural: structural.unwrap_or(false), ..wall };
        self.add_element(id, e, level, type_id, Self::tag(e), ElementKind::Wall(wall));
        if let Some(element) = self.p.elements.keys().next_back().copied() {
            self.walls.insert(id, element);
        }
        true
    }

    /// A curtain wall: rebuilt from [`crate::export::CURTAIN_PSET`] when the
    /// file was written by this program, else from its aggregated plates and
    /// members when they make a straight grid of rectangular panels (see
    /// [`Self::curtain_from_parts`]). Its plates and members then come in with
    /// it; otherwise they are imported one by one, as before.
    fn import_curtain_wall(&mut self, id: u32, e: &Entity) -> bool {
        let (world, body) = self.product(e);
        let set = |n: &str| n == crate::export::CURTAIN_PSET;
        let text = |name: &str| self.pset_texts(id, &set, name).first().map(|s| s.to_string());
        let system = text("System").and_then(|t| cc_core::curtain::from_text::<CurtainSystem>(&t));
        let rebuilt = match (system, body.axis_points.first(), body.axis_points.last()) {
            (Some(system), Some(&start), Some(&end)) if start.distance(end) > 0.01 => {
                let edits: CurtainEdits =
                    text("Edits").and_then(|t| cc_core::curtain::from_text(&t)).unwrap_or_default();
                let numbers = self.numeric_properties(id, crate::export::CURTAIN_PSET);
                let number = |name: &str| numbers.iter().find(|n| n.0 == name).map(|n| n.1);
                let height = number("Height").map(|h| h * self.scale).filter(|h| h.is_finite() && *h > 0.01);
                let bulge = number("Bulge").filter(|b| b.is_finite()).unwrap_or(0.0);
                let z0 = world.translation.z;
                height.map(|h| (system, edits, start, end, bulge, (z0, z0 + h)))
            }
            _ => None,
        };
        let Some((mut system, mut edits, start, end, bulge, z)) =
            rebuilt.or_else(|| self.curtain_from_parts(id, &world, &body))
        else {
            return false;
        };
        // Component panels name types of the project that wrote the file: they become the type's glass.
        let glass = || CurtainPanel::Glass { thickness: 0.024, material: String::new() };
        if matches!(system.panel, CurtainPanel::Component { .. }) {
            system.panel = glass();
        }
        let fallback = system.panel.clone();
        for swap in &mut edits.panels {
            if matches!(swap.panel, CurtainPanel::Component { .. }) {
                swap.panel = fallback.clone();
            }
        }
        system.repair(&|_| false);
        edits.repair(&|_| false);
        if !(start.is_finite() && end.is_finite() && z.0.is_finite() && z.1 > z.0) {
            return false;
        }
        let (level, elevation) = self.level_for(id, z.0);
        let external = self.property(id, "Pset_CurtainWallCommon", "IsExternal").and_then(Param::as_bool);
        let function = if external == Some(false) { WallFunction::Interior } else { WallFunction::Exterior };
        let name = self.type_name(id, e);
        let uid = self.type_uid(id);
        let wanted = system.clone();
        let type_id = self.find_or_add_type(
            Category::Wall,
            name,
            "Curtain Wall".to_string(),
            uid,
            |d| matches!(d, TypeData::Wall { curtain: Some(c), .. } if *c == wanted),
            || TypeData::Wall { layers: Vec::new(), function, curtain: Some(system) },
        );
        let wall = Wall {
            start,
            end,
            bulge,
            base_offset: z.0 - elevation,
            height: z.1 - z.0,
            curtain: edits,
            ..Default::default()
        };
        self.add_element(id, e, level, type_id, Self::tag(e), ElementKind::Wall(wall));
        self.curtain_walls.insert(id);
        true
    }

    /// A curtain wall of another program from its aggregated plates and
    /// members: straight (every part within a meter across one line), every
    /// plate a box filling one cell of the grid its edges make, mullions
    /// sized by the members. The wall keeps those lines as its own; the type
    /// spaces none.
    #[allow(clippy::type_complexity)]
    fn curtain_from_parts(
        &mut self,
        id: u32,
        world: &DAffine3,
        body: &Body,
    ) -> Option<(CurtainSystem, CurtainEdits, DVec2, DVec2, f64, (f64, f64))> {
        let f = self.f;
        let parts: Vec<(u32, &'a Entity)> = f
            .entities
            .iter()
            .filter(|(pid, p)| self.parent.get(pid) == Some(&id) && matches!(p.name.as_str(), "IFCPLATE" | "IFCMEMBER"))
            .map(|(pid, p)| (*pid, p))
            .take(4 * cc_core::curtain::MAX_LINES)
            .collect();
        if !parts.iter().any(|(_, p)| p.name == "IFCPLATE") {
            return None;
        }
        // The wall's line: its axis, else its placement's X axis.
        let (origin, dir) = match body.axis {
            Some((a, b)) => (a, (b - a).try_normalize()?),
            None => (world.translation.truncate(), world.matrix3.x_axis.truncate().try_normalize()?),
        };
        // Each part's extent along the line, across it and up.
        type Extent = ((f64, f64), (f64, f64), (f64, f64));
        let extent = |b: &Body| -> Option<Extent> {
            let mut it = b.points().map(|p| {
                let r = p.truncate() - origin;
                (r.dot(dir), r.dot(dir.perp()), p.z)
            });
            let first = it.next()?;
            let grow = |(lo, hi): (f64, f64), v: f64| (lo.min(v), hi.max(v));
            let (u, v, z) = it.fold(((first.0, first.0), (first.1, first.1), (first.2, first.2)), |(u, v, z), p| {
                (grow(u, p.0), grow(v, p.1), grow(z, p.2))
            });
            [u.0, u.1, v.0, v.1, z.0, z.1].iter().all(|x| x.is_finite()).then_some((u, v, z))
        };
        let mut plates = Vec::new();
        let mut members = Vec::new();
        for (pid, p) in &parts {
            let (_, b) = self.product(p);
            let ext = extent(&b)?;
            if p.name == "IFCPLATE" { plates.push((*pid, *p, ext)) } else { members.push((*pid, *p, ext)) }
        }
        let all = plates.iter().chain(&members).map(|x| x.2);
        let (mut u0, mut u1, mut v0, mut v1, mut z0, mut z1) =
            (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for (u, v, z) in all {
            (u0, u1, v0, v1, z0, z1) = (u0.min(u.0), u1.max(u.1), v0.min(v.0), v1.max(v.1), z0.min(z.0), z1.max(z.1));
        }
        if v1 - v0 > 1.0 || u1 - u0 < 0.05 || z1 - z0 < 0.05 {
            return None;
        }
        // Mullions: the narrowest face width and the depth of the members.
        let median = |mut xs: Vec<f64>| -> Option<f64> {
            xs.sort_by(f64::total_cmp);
            xs.get(xs.len() / 2).copied().filter(|x| x.is_finite() && *x > 1e-3)
        };
        let width = median(members.iter().map(|(_, _, (u, _, z))| (u.1 - u.0).min(z.1 - z.0)).collect());
        let depth = median(members.iter().map(|(_, _, (_, v, _))| v.1 - v.0).collect());
        let mullion = match (width, depth, members.first()) {
            (Some(width), Some(depth), Some((mid, m, _))) => {
                Some(cc_core::curtain::Mullion { width, depth, material: self.main_material(*mid, m) })
            }
            _ => None,
        };
        let w = mullion.as_ref().map_or(0.0, |m| m.width);
        // Grid lines where plate edges meet, within a mullion's width.
        let tol = w + 0.02;
        let lines = |edges: Vec<f64>, lo: f64, hi: f64| -> Vec<f64> {
            let mut edges: Vec<f64> = edges.into_iter().filter(|x| *x > lo + tol && *x < hi - tol).collect();
            edges.sort_by(f64::total_cmp);
            let mut out: Vec<Vec<f64>> = Vec::new();
            for x in edges {
                match out.last_mut() {
                    Some(group) if group.last().is_some_and(|l| x - l <= tol) => group.push(x),
                    _ => out.push(vec![x]),
                }
            }
            out.iter().map(|g| g.iter().sum::<f64>() / g.len() as f64).collect()
        };
        let us = lines(plates.iter().flat_map(|(_, _, (u, _, _))| [u.0, u.1]).collect(), u0, u1);
        let zs = lines(plates.iter().flat_map(|(_, _, (_, _, z))| [z.0, z.1]).collect(), z0, z1);
        let cells = (us.len() + 1) * (zs.len() + 1);
        if plates.len() != cells || cells > 4 * cc_core::curtain::MAX_LINES {
            return None;
        }
        // Every plate fills its own cell.
        let cell_of = |lines: &[f64], x: f64| lines.iter().filter(|l| **l < x).count();
        let mut seen = std::collections::HashSet::new();
        for (_, _, (u, _, z)) in &plates {
            let key = (cell_of(&us, (u.0 + u.1) * 0.5), cell_of(&zs, (z.0 + z.1) * 0.5));
            if !seen.insert(key) {
                return None;
            }
        }
        // The panel: glass when its material says so, else solid; as thick as the plates.
        let (pid, plate, (_, pv, _)) = plates.first()?;
        let material = self.main_material(*pid, plate);
        let lower = material.to_lowercase();
        let thickness = (pv.1 - pv.0).max(0.001);
        let panel = if ["glass", "glaz", "vidro", "verre", "glas", "vetro", "vidrio"].iter().any(|g| lower.contains(g))
        {
            CurtainPanel::Glass { thickness, material }
        } else {
            CurtainPanel::Solid { thickness, material }
        };
        let none = cc_core::curtain::CurtainGrid { layout: cc_core::curtain::GridLayout::None, ..Default::default() };
        let system = CurtainSystem {
            vertical: none,
            horizontal: none,
            panel,
            vertical_mullion: mullion.clone(),
            horizontal_mullion: mullion.clone(),
            border_mullion: mullion,
        };
        let edits = CurtainEdits {
            vertical: Some(us.iter().map(|u| u - u0).collect()),
            horizontal: Some(zs.iter().map(|z| z - z0).collect()),
            ..Default::default()
        };
        let across = dir.perp() * ((v0 + v1) * 0.5);
        let (start, end) = (origin + dir * u0 + across, origin + dir * u1 + across);
        Some((system, edits, start, end, 0.0, (z0, z1)))
    }

    fn import_filler(&mut self, id: u32, e: &Entity, door: bool) -> bool {
        let opening = self.opening_of_filler.get(&id).copied();
        let host_ifc = opening.and_then(|o| self.host_of_opening.get(&o)).copied();
        let (world, body) = self.product(e);
        let opening_body = opening.and_then(|o| self.f.get(o)).map(|o| self.product(o).1);
        // Points that span the opening: the opening's body, else the filler's.
        let mut points: Vec<DVec3> = opening_body.as_ref().map(|b| b.points().collect()).unwrap_or_default();
        if points.is_empty() {
            points = body.points().collect();
        }
        let center = if points.is_empty() {
            world.translation
        } else {
            let (lo, hi) =
                points.iter().fold((DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)), |(lo, hi), p| {
                    (lo.min(*p), hi.max(*p))
                });
            (lo + hi) * 0.5
        };
        // Host: the voided wall, else the nearest wall within reach.
        let host = host_ifc.and_then(|h| self.walls.get(&h)).copied().or_else(|| {
            self.p
                .walls()
                .map(|(w, wall)| {
                    let c = center.truncate();
                    let distance = match wall.arc() {
                        Some(_) => {
                            let (u, v) = wall.project(c);
                            if (0.0..=wall.length()).contains(&u) { v.abs() } else { f64::INFINITY }
                        }
                        None => polygon::closest_on_segment(wall.start, wall.end, c).0.distance(c),
                    };
                    (w.id, distance, self.p.thickness_of(w))
                })
                .filter(|(_, d, t)| *d <= t / 2.0 + 0.1)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(id, _, _)| id)
        });
        let Some(host) = host else { return false };
        let Some(host_element) = self.p.element(host).cloned() else { return false };
        let Some(wall) = host_element.as_wall().cloned() else { return false };
        let base_z = self.p.wall_base_z(&host_element, &wall);
        let (mut t0, mut t1, mut z0, mut z1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for p in &points {
            let t = wall.project(p.truncate()).0;
            t0 = t0.min(t);
            t1 = t1.max(t);
            z0 = z0.min(p.z);
            z1 = z1.max(p.z);
        }
        if points.is_empty() {
            let t = wall.project(center.truncate()).0;
            (t0, t1, z0, z1) = (t, t, center.z, center.z);
        }
        let attr = |i: usize| e.arg(i).as_f64().map(|v| v * self.scale).filter(|v| v.is_finite() && *v > 0.0);
        let width = attr(9).unwrap_or(t1 - t0);
        let height = attr(8).unwrap_or(z1 - z0);
        if !(width > 0.01 && height > 0.01) {
            return false;
        }
        let offset = (t0 + t1) / 2.0;
        let sill = if (z0 - base_z).abs() < 0.005 { 0.0 } else { (z0 - base_z).max(0.0) };
        let name = self.type_name(id, e);
        let material = self.main_material(id, e);
        let same = move |w: f64, h: f64| (w - width).abs() < TYPE_TOL && (h - height).abs() < TYPE_TOL;
        let uid = self.type_uid(id);
        let fallback = format!("{:.0} x {:.0}", width * 100.0, height * 100.0);
        let (operation, flip_hand) = if door { self.door_operation(id, e) } else { Default::default() };
        let (category, type_id) = if door {
            let type_id = self.find_or_add_type(
                Category::Door,
                name,
                fallback,
                uid,
                |d| {
                    is_opening_type(d, Category::Door).is_some_and(|(w, h)| same(w, h))
                        && door_operation(d) == operation
                },
                || {
                    let names = cc_core::openings::OpeningNames::default();
                    let def = cc_core::openings::operable_door_def(&names, (width, height), &material, operation);
                    TypeData::Component { component: names.door_for(operation, false).clone(), def }
                },
            );
            (Category::Door, type_id)
        } else {
            let operation = self.window_operation(id);
            let type_id = self.find_or_add_type(
                Category::Window,
                name,
                fallback,
                uid,
                |d| {
                    is_opening_type(d, Category::Window).is_some_and(|(w, h)| same(w, h))
                        && window_operation(d) == operation
                },
                || {
                    let names = cc_core::openings::OpeningNames::default();
                    let def =
                        cc_core::openings::operable_window_def(&names, (width, height, sill), &material, operation);
                    let component = names.window_for(operation).clone();
                    TypeData::Component { component, def }
                },
            );
            (Category::Window, type_id)
        };
        let hosted = Hosted { host, offset, sill_height: sill, flip_facing: false, flip_hand };
        let kind = if category == Category::Door { ElementKind::Door(hosted) } else { ElementKind::Window(hosted) };
        self.add_element(id, e, host_element.level, type_id, Self::tag(e), kind);
        true
    }

    /// Footprint, z range and vertical thickness of a body that is one
    /// vertical extrusion; with `approximate`, other bodies give their hull.
    fn sketch(body: &Body, approximate: bool) -> Option<(Vec<DVec2>, f64, f64, f64)> {
        let (z0, z1) = body.z_range()?;
        let (footprint, thickness) = match body.sole_extrusion().filter(|e| e.is_vertical()) {
            Some(ex) => (ex.footprint(), ex.vector.z.abs()),
            None if approximate => (convex_hull(&body.points().map(|p| p.truncate()).collect::<Vec<_>>()), z1 - z0),
            None => return None,
        };
        let footprint = clean_polygon(footprint);
        (footprint.len() >= 3 && polygon::area(&footprint) > 1e-4).then_some((footprint, z0, z1, thickness))
    }

    /// Openings voiding a product that a native element (other than a wall,
    /// whose doors and windows cut it) could not show.
    fn has_openings(&self, id: u32) -> bool {
        self.openings_of.get(&id).is_some_and(|o| !o.is_empty())
    }

    /// The outline, openings, z range and thickness of a slab that is one
    /// vertical extrusion: the openings are the voids of its profile and the
    /// openings voiding it that pass through it vertically. `None` when the
    /// slab or an opening is anything else, or an opening is not inside the
    /// outline and apart from the others (natives could not show it). When
    /// the profile is drawn with arcs or circles, also the sketch of the
    /// outline and openings, which then come from it (#264).
    #[allow(clippy::type_complexity)]
    fn slab_sketch(
        &self,
        id: u32,
        body: &Body,
    ) -> Option<(Vec<DVec2>, Vec<Vec<DVec2>>, f64, f64, f64, Option<Sketch>)> {
        let (ex, cuts) = body.slab_extrusion().filter(|(e, _)| e.is_vertical())?;
        let (z0, z1) = ex.z_range();
        let mut outline = clean_polygon(ex.footprint());
        if outline.len() < 3 || polygon::area(&outline) <= 1e-4 {
            return None;
        }
        let mut openings: Vec<Vec<DVec2>> = ex.void_footprints().into_iter().map(clean_polygon).collect();
        // An opening passes through the whole slab (a recess would leave part of it).
        let through = |(c0, c1): (f64, f64)| c0 <= z0 + SLAB_TOL && c1 >= z1 - SLAB_TOL;
        for cut in cuts {
            if !(cut.is_vertical() && cut.profile.holes.is_empty() && through(cut.z_range())) {
                return None;
            }
            openings.push(clean_polygon(cut.footprint()));
        }
        let f = self.f;
        for o in self.openings_of.get(&id).into_iter().flatten() {
            let (_, cutter) = self.product(f.get(*o)?);
            let cut = cutter.sole_extrusion().filter(|e| e.is_vertical())?;
            if !through(cutter.z_range()?) {
                return None;
            }
            // The same shaft may be cut out of the body and voided by an opening.
            let footprint = clean_polygon(cut.footprint());
            if !openings.iter().any(|o| same_outline(o, &footprint)) {
                openings.push(footprint);
            }
        }
        // The profile's curves, with the other openings as lines: the outline
        // and openings are then the sketch's (arcs split as natives split them).
        let voids = ex.profile.holes.len();
        let sketch = ex.footprint_curves().and_then(|mut curves| {
            for o in openings.iter().skip(voids) {
                curves.extend(Sketch::from_polygons(o, &[]).curves);
            }
            let sketch = Sketch { curves };
            let p = sketch.profile().ok()?;
            (p.openings.len() == openings.len()).then_some((sketch, p))
        });
        let sketch = sketch.map(|(sketch, p)| {
            outline = p.outline;
            openings = p.openings;
            sketch
        });
        if outline.len() < 3 || polygon::area(&outline) <= 1e-4 {
            return None;
        }
        openings.iter().all(|o| openings_fit(&outline, o)).then_some(())?;
        for (i, a) in openings.iter().enumerate() {
            for b in openings.iter().skip(i + 1) {
                if outlines_meet(a, b) {
                    return None;
                }
            }
        }
        Some((outline, openings, z0, z1, ex.vector.z.abs(), sketch))
    }

    fn import_slab(&mut self, id: u32, e: &Entity) -> bool {
        let kind = e.arg(8).as_enum().unwrap_or("NOTDEFINED");
        let (_, body) = self.product(e);
        let Some((boundary, openings, z0, z1, thickness, sketch)) = self.slab_sketch(id, &body) else { return false };
        match kind {
            "ROOF" => return self.add_roof_with_holes(id, e, boundary, openings, z0, thickness, RoofShape::Flat),
            // A foundation slab, if it is shaped like one: piles and pile
            // caps written as base slabs stay generic, keeping their class.
            "BASESLAB" => {
                return openings.is_empty()
                    && slab_like(&boundary, thickness)
                    && self.add_foundation_slab(id, e, boundary, sketch, z1, thickness);
            }
            _ => {}
        }
        let (level, elevation) = self.level_for(id, z1);
        let name = self.type_name(id, e);
        let layers = self.layers(id, e, thickness);
        let uid = self.type_uid(id);
        let type_id = self.find_or_add_type(
            Category::Floor,
            name,
            format!("{:.0} cm", thickness * 100.0),
            uid,
            |d| matches!(d, TypeData::Floor { layers } if (layers.iter().map(|l| l.thickness).sum::<f64>() - thickness).abs() < TYPE_TOL),
            || TypeData::Floor { layers },
        );
        let mut floor = Floor::new(boundary, z1 - elevation);
        floor.openings = openings;
        floor.sketch = sketch;
        self.add_element(id, e, level, type_id, Self::tag(e), ElementKind::Floor(floor));
        true
    }

    /// A foundation slab with its top at `z1`.
    fn add_foundation_slab(
        &mut self,
        id: u32,
        e: &Entity,
        boundary: Vec<DVec2>,
        sketch: Option<cc_core::sketch::Sketch>,
        z1: f64,
        thickness: f64,
    ) -> bool {
        if thickness < 1e-3 {
            return false;
        }
        let (level, elevation) = self.level_for(id, z1);
        let name = self.type_name(id, e);
        let material = self.main_material(id, e);
        let uid = self.type_uid(id);
        let type_id = self.find_or_add_type(
            Category::Foundation,
            name,
            format!("{:.0} cm", thickness * 100.0),
            uid,
            |d| {
                matches!(d, TypeData::Foundation { kind: FoundationKind::Slab, thickness: t, .. }
                    if (t - thickness).abs() < TYPE_TOL)
            },
            || TypeData::Foundation {
                kind: FoundationKind::Slab,
                width: 1.0,
                length: 1.0,
                thickness,
                material,
                haunch: None,
                pile_cap: None,
            },
        );
        // The circles and arcs of its profile, so a round raft keeps its true area and volume.
        let mut foundation = Foundation::new(FoundationShape::Slab { boundary }, z1 - elevation);
        foundation.sketch = sketch;
        self.add_element(id, e, level, type_id, Self::tag(e), ElementKind::Foundation(foundation));
        true
    }

    fn import_covering(&mut self, id: u32, e: &Entity) -> bool {
        if self.has_openings(id) {
            return false;
        }
        if e.arg(8).as_enum() != Some("CEILING") {
            return false;
        }
        let (_, body) = self.product(e);
        let Some((boundary, z0, _, thickness)) = Self::sketch(&body, false) else { return false };
        let (level, elevation) = self.level_for(id, z0);
        let name = self.type_name(id, e);
        let material = self.main_material(id, e);
        let uid = self.type_uid(id);
        let type_id = self.find_or_add_type(
            Category::Ceiling,
            name,
            format!("{:.0} cm", thickness * 100.0),
            uid,
            |d| matches!(d, TypeData::Ceiling { thickness: t, .. } if (t - thickness).abs() < TYPE_TOL),
            || TypeData::Ceiling { thickness, material },
        );
        let ceiling = Ceiling { boundary, height_offset: z0 - elevation, cuts: Vec::new(), sketch: None };
        self.add_element(id, e, level, type_id, Self::tag(e), ElementKind::Ceiling(ceiling));
        true
    }

    fn import_roof(&mut self, id: u32, e: &Entity) -> bool {
        if self.has_openings(id) {
            return false;
        }
        let (_, body) = self.product(e);
        if body.solids.is_empty() {
            // Roofs made of aggregated slabs are imported through the slabs.
            return self.has_parts.contains_key(&id);
        }
        let predefined = e.arg(8).as_enum().unwrap_or("NOTDEFINED");
        let (layers, _) = self.materials(id);
        let layered: f64 = layers.iter().map(|l| l.1).sum();
        let tilted = |ex: &&Extrusion| ex.xf.matrix3.z_axis.try_normalize().is_some_and(|n| n.z.abs() < 0.9999);
        if let Some(ex) = body.sole_extrusion().filter(tilted) {
            // A sloped profile extruded vertically: the slope runs along the
            // steepest direction of the profile plane.
            let normal = ex.xf.matrix3.z_axis.normalize_or_zero();
            let downhill = normal.truncate();
            let run = downhill.length();
            if run < 1e-9 || normal.z.abs() < 1e-9 {
                return false;
            }
            let slope = run / normal.z.abs();
            // The roof rises away from the normal's horizontal component.
            let direction = -downhill * normal.z.signum();
            let footprint = clean_polygon(ex.footprint());
            let z0 = ex.profile.pts.iter().map(|p| ex.plane_point(*p).z).fold(f64::INFINITY, f64::min);
            let thickness = ex.vector.z.abs() / (1.0 + slope * slope).sqrt();
            let shape = RoofShape::Shed { slope, direction_deg: direction.y.atan2(direction.x).to_degrees() };
            if footprint.len() < 3 {
                return false;
            }
            return self.add_roof(id, e, footprint, z0, thickness, shape);
        }
        if let Some((footprint, z0, _, thickness)) = Self::sketch(&body, false) {
            return self.add_roof(id, e, footprint, z0, thickness, RoofShape::Flat);
        }
        // Roofs as written by this program: one closed body whose vertical
        // faces (eaves and gables) outline the roof and its openings, and
        // whose sloped faces share one slope.
        if matches!(predefined, "GABLE_ROOF" | "HIP_ROOF" | "FLAT_ROOF" | "SHED_ROOF")
            && let [solid] = body.solids.as_slice()
            && let Some((loops, slope, uphill)) = roof_outline(&solid.mesh)
            && let Some((z0, z1)) = body.z_range()
        {
            let mut loops = loops.into_iter();
            if let Some(outer) = loops.next() {
                let shape = match predefined {
                    "HIP_ROOF" if slope > 0.0 => RoofShape::Hip { slope },
                    "GABLE_ROOF" if slope > 0.0 => {
                        let n = outer.len();
                        let longest = (0..n)
                            .map(|i| outer[(i + 1) % n] - outer[i])
                            .max_by(|a, b| a.length().total_cmp(&b.length()))
                            .unwrap_or(DVec2::X);
                        RoofShape::Gable { slope, ridge_deg: longest.y.atan2(longest.x).to_degrees() }
                    }
                    "SHED_ROOF" if slope > 0.0 => {
                        let d = uphill.unwrap_or(DVec2::X);
                        RoofShape::Shed { slope, direction_deg: d.y.atan2(d.x).to_degrees() }
                    }
                    _ => RoofShape::Flat,
                };
                let thickness = match shape {
                    RoofShape::Flat => z1 - z0,
                    _ if layered > 0.0 => layered,
                    _ => 0.15,
                };
                let holes: Vec<Vec<DVec2>> = loops.collect();
                return self.add_roof_with_holes(id, e, outer, holes, z0, thickness, shape);
            }
        }
        // Pitched roofs as written by this program (one body with the roof
        // shape named): footprint hull, slope from the rise over the half width.
        // Other bodies keep their exact geometry as generic objects.
        if !matches!(predefined, "GABLE_ROOF" | "HIP_ROOF") || body.solids.len() != 1 {
            return false;
        }
        let Some((footprint, z0, z1, _)) = Self::sketch(&body, true) else { return false };
        let thickness = if layered > 0.0 { layered } else { 0.15 };
        let shape = match predefined {
            "GABLE_ROOF" | "HIP_ROOF" => {
                // Longest hull edge sets the ridge direction.
                let n = footprint.len();
                let (mut ridge, mut best) = (DVec2::X, 0.0);
                for i in 0..n {
                    let d = footprint[(i + 1) % n] - footprint[i];
                    if d.length() > best {
                        best = d.length();
                        ridge = d.normalize();
                    }
                }
                let across: Vec<f64> = footprint.iter().map(|p| p.dot(ridge.perp())).collect();
                let width = across.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                    - across.iter().cloned().fold(f64::INFINITY, f64::min);
                let half = (width / 2.0).max(0.01);
                let rise = z1 - z0;
                // rise = slope * half + thickness * sqrt(1 + slope²), solved by iteration.
                let mut slope = rise / half;
                for _ in 0..20 {
                    slope = ((rise - thickness * (1.0 + slope * slope).sqrt()) / half).max(0.0);
                }
                if predefined == "HIP_ROOF" {
                    RoofShape::Hip { slope }
                } else {
                    RoofShape::Gable { slope, ridge_deg: ridge.y.atan2(ridge.x).to_degrees() }
                }
            }
            _ => RoofShape::Flat,
        };
        let thickness = if matches!(shape, RoofShape::Flat) { z1 - z0 } else { thickness };
        self.add_roof(id, e, footprint, z0, thickness, shape)
    }

    fn add_roof(
        &mut self,
        id: u32,
        e: &Entity,
        footprint: Vec<DVec2>,
        z0: f64,
        thickness: f64,
        shape: RoofShape,
    ) -> bool {
        self.add_roof_with_holes(id, e, footprint, Vec::new(), z0, thickness, shape)
    }

    #[allow(clippy::too_many_arguments)]
    fn add_roof_with_holes(
        &mut self,
        id: u32,
        e: &Entity,
        footprint: Vec<DVec2>,
        holes: Vec<Vec<DVec2>>,
        z0: f64,
        thickness: f64,
        shape: RoofShape,
    ) -> bool {
        if thickness < 1e-3 {
            return false;
        }
        let (level, elevation) = self.level_for(id, z0);
        let name = self.type_name(id, e);
        let layers = self.layers(id, e, thickness);
        let uid = self.type_uid(id);
        let (covering, framing) = self.roofing(id);
        if !framing.material.is_empty() {
            self.material(&framing.material, None);
        }
        let type_id = self.find_or_add_type(
            Category::Roof,
            name,
            format!("{:.0} cm", thickness * 100.0),
            uid,
            |d| {
                matches!(d, TypeData::Roof { layers, covering: c, framing: f }
                    if (layers.iter().map(|l| l.thickness).sum::<f64>() - thickness).abs() < TYPE_TOL
                        && *c == covering && *f == framing)
            },
            || TypeData::Roof { layers, covering, framing: framing.clone() },
        );
        if framing.system != cc_core::roofing::FramingSystem::None {
            self.framed_roofs.insert(id);
        }
        let mut roof = Roof::new(footprint, z0 - elevation, 0.0, shape);
        roof.holes = holes;
        self.add_element(id, e, level, type_id, Self::tag(e), ElementKind::Roof(roof));
        true
    }

    /// A roof's covering and framing as this program writes them
    /// ([`crate::export::ROOFING_PSET`]); none and the default framing otherwise.
    fn roofing(&self, id: u32) -> (Option<cc_core::roofing::TileKind>, cc_core::roofing::RoofFraming) {
        use cc_core::roofing::{FramingSystem, RoofFraming, TileKind};
        let set = |n: &str| n == crate::export::ROOFING_PSET;
        let text = |name: &str| self.pset_texts(id, &set, name).first().map(|s| s.to_string());
        let mut framing = RoofFraming::default();
        let Some(system) = text("FramingSystem").and_then(|s| FramingSystem::from_code(&s)) else {
            return (None, framing);
        };
        framing.system = system;
        let covering = text("Covering").and_then(|s| TileKind::from_code(&s));
        if let Some(m) = text("FramingMaterial") {
            framing.material = m;
        }
        let numbers = self.numeric_properties(id, crate::export::ROOFING_PSET);
        let length = |name: &str| numbers.iter().find(|n| n.0 == name).map(|n| n.1 * self.scale);
        let sizes = [&mut framing.purlin, &mut framing.rafter, &mut framing.batten, &mut framing.truss];
        for ((_, w, h), size) in crate::export::ROOFING_SIZES.iter().zip(sizes) {
            if let (Some(w), Some(h)) = (length(w), length(h)) {
                *size = cc_core::roofing::MemberSize::new(w, h);
            }
        }
        for (name, v) in [
            ("PurlinSpacing", &mut framing.purlin_spacing),
            ("RafterSpacing", &mut framing.rafter_spacing),
            ("TrussSpacing", &mut framing.truss_spacing),
        ] {
            if let Some(x) = length(name) {
                *v = x;
            }
        }
        // Hostile files: invalid sizes and spacings go back to the defaults.
        framing.repair();
        (covering, framing)
    }

    /// Terrain as this program writes it ([`crate::export::TOPOGRAPHY_PSET`]):
    /// rebuilt from its points and boundary. Other terrain stays generic.
    fn import_topography(&mut self, id: u32, e: &Entity) -> bool {
        use cc_core::topography::{
            DEFAULT_BASE_DEPTH, DEFAULT_CONTOUR_INTERVAL, DEFAULT_MAJOR_EVERY, EARTH_COLOR, MAX_OUTLINE, MAX_POINTS,
            MIN_CONTOUR_INTERVAL, Topography, clean_points,
        };
        let set = |n: &str| n == crate::export::TOPOGRAPHY_PSET;
        let text = |name: &str| self.pset_texts(id, &set, name).first().map(|s| s.to_string());
        let (Some(points), Some(boundary)) = (text("Points"), text("Boundary")) else { return false };
        let s = self.scale;
        let points: Vec<DVec3> = crate::point_list::decode(&points, 3, MAX_POINTS)
            .into_iter()
            .map(|p| DVec3::new(p[0], p[1], p[2]) * s)
            .collect();
        let boundary: Vec<DVec2> = crate::point_list::decode(&boundary, 2, MAX_OUTLINE)
            .into_iter()
            .map(|p| DVec2::new(p[0], p[1]) * s)
            .collect();
        let numbers = self.numeric_properties(id, crate::export::TOPOGRAPHY_PSET);
        let number = |name: &str| numbers.iter().find(|n| n.0 == name).map(|n| n.1).filter(|v| v.is_finite());
        let mut topo = Topography {
            points: clean_points(&points),
            boundary,
            base_depth: number("BaseDepth").map_or(DEFAULT_BASE_DEPTH, |d| d * s),
            sketch: None,
        };
        topo.repair();
        if !topo.is_valid() {
            return false;
        }
        let interval = number("ContourInterval")
            .map(|v| v * s)
            .filter(|v| *v >= MIN_CONTOUR_INTERVAL)
            .unwrap_or(DEFAULT_CONTOUR_INTERVAL);
        let major = number("MajorEvery").filter(|v| *v >= 1.0 && *v <= 1e6).map_or(DEFAULT_MAJOR_EVERY, |v| v as u32);
        // Without a material named, the project's earth material, or the
        // template's (by its name, in the earth class).
        let material = match text("Material").filter(|m| !m.trim().is_empty()) {
            Some(m) => self.material(&m, Some(EARTH_COLOR)),
            None => match cc_core::topography::earth_material(&self.p) {
                Some(m) => m.to_string(),
                None => {
                    let name = cc_core::template_default("tpl-mat-earth");
                    let added = !self.p.materials.iter().any(|m| m.name == name);
                    let name = self.material(name, Some(EARTH_COLOR));
                    if added && let Some(m) = self.p.materials.iter_mut().find(|m| m.name == name) {
                        cc_core::material::template_properties(m, cc_core::material::MaterialClass::Earth);
                    }
                    name
                }
            },
        };
        let (level, _) = self.level_for(id, 0.0);
        let name = self.type_name(id, e);
        let uid = self.type_uid(id);
        let type_id = self.find_or_add_type(
            Category::Topography,
            name,
            cc_core::template_default("tpl-type-topography").to_string(),
            uid,
            |d| {
                matches!(d, TypeData::Topography { material: m, contour_interval: c, major_every: k }
                    if *m == material && (c - interval).abs() < 1e-9 && *k == major)
            },
            || TypeData::Topography { material: material.clone(), contour_interval: interval, major_every: major },
        );
        self.add_element(id, e, level, type_id, Self::tag(e), ElementKind::Topography(topo));
        if let Some(&(terrain, _)) = self.products.last() {
            self.import_pads(id, terrain, level);
        }
        true
    }

    /// The building pads of terrain `terrain`, from its
    /// [`crate::export::PADS_PSET`] (new ids and uids: IFC4 has no pad object).
    fn import_pads(&mut self, id: u32, terrain: ElementId, level: LevelId) {
        use cc_core::pad::{DEFAULT_CUT_SLOPE, DEFAULT_FILL_SLOPE, MAX_OUTLINE, Pad, clamp_slope};
        let set = |n: &str| n == crate::export::PADS_PSET;
        let numbers = self.numeric_properties(id, crate::export::PADS_PSET);
        let number = |name: &str| numbers.iter().find(|n| n.0 == name).map(|n| n.1).filter(|v| v.is_finite());
        let s = self.scale;
        // Bounded: a pad per outline, counted from 1.
        for n in 1..=10_000 {
            let Some(outline) = self.pset_texts(id, &set, &format!("Pad{n}Outline")).first().map(|t| t.to_string())
            else {
                break;
            };
            let outline: Vec<DVec2> = crate::point_list::decode(&outline, 2, MAX_OUTLINE)
                .into_iter()
                .map(|p| DVec2::new(p[0], p[1]) * s)
                .collect();
            let mut pad = Pad {
                host: terrain,
                outline,
                elevation: number(&format!("Pad{n}Elevation")).map_or(0.0, |z| z * s),
                cut_slope: number(&format!("Pad{n}CutSlope")),
                fill_slope: number(&format!("Pad{n}FillSlope")),
                sketch: None,
            };
            pad.repair();
            if !pad.is_valid() {
                continue;
            }
            let name = self.pset_texts(id, &set, &format!("Pad{n}Type")).first().map(|t| t.to_string());
            let ratio = |key: &str, d: f64| number(key).map_or(d, |r| clamp_slope(r, d));
            let cut = ratio(&format!("Pad{n}TypeCutSlope"), DEFAULT_CUT_SLOPE);
            let fill = ratio(&format!("Pad{n}TypeFillSlope"), DEFAULT_FILL_SLOPE);
            let type_id = self.find_or_add_type(
                Category::Pad,
                name.filter(|n| !n.trim().is_empty()),
                cc_core::template_default("tpl-type-pad").to_string(),
                None,
                |d| {
                    matches!(d, TypeData::Pad { cut_slope, fill_slope }
                        if (cut_slope - cut).abs() < 1e-9 && (fill_slope - fill).abs() < 1e-9)
                },
                || TypeData::Pad { cut_slope: cut, fill_slope: fill },
            );

            let pid = ElementId(self.p.alloc_id());
            let element = Element {
                id: pid,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                kind: ElementKind::Pad(pad),
                uid: Default::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
            };
            *self.report.imported.entry(Category::Pad).or_default() += 1;
            self.p.elements.insert(pid, element);
        }
    }

    fn import_space(&mut self, id: u32, e: &Entity) -> bool {
        let (world, body) = self.product(e);
        let (z0, z1) = body.z_range().unwrap_or((world.translation.z, world.translation.z));
        let outline = Self::sketch(&body, true).map(|s| s.0).unwrap_or_default();
        // A point inside the outline: its centroid when inside, else the
        // centroid of one of its triangles.
        let position = if outline.len() >= 3 {
            let area = polygon::signed_area(&outline);
            let n = outline.len();
            let mut c = DVec2::ZERO;
            for i in 0..n {
                let (a, b) = (outline[i], outline[(i + 1) % n]);
                c += (a + b) * a.perp_dot(b);
            }
            let centroid = if area.abs() > 1e-9 { c / (6.0 * area) } else { outline[0] };
            if polygon::contains(&outline, centroid) {
                centroid
            } else {
                polygon::triangulate(&outline)
                    .first()
                    .map(|t| (outline[t[0]] + outline[t[1]] + outline[t[2]]) / 3.0)
                    .unwrap_or(centroid)
            }
        } else {
            world.translation.truncate()
        };
        if !position.is_finite() {
            return false;
        }
        let (level, _) = self.level_for(id, z0);
        let height = if z1 - z0 > 0.1 { z1 - z0 } else { self.p.level_height(level, 2.8) };
        let Some(type_id) = self.p.default_type(Category::Room).or_else(|| {
            self.p.ensure_annotation_types();
            self.p.default_type(Category::Room)
        }) else {
            return false;
        };
        let room = Room {
            position,
            number: e.arg(2).as_str().unwrap_or_default().to_string(),
            name: e.arg(7).as_str().unwrap_or_default().to_string(),
            height,
        };
        self.add_element(id, e, level, type_id, String::new(), ElementKind::Room(room));
        true
    }

    fn import_column(&mut self, id: u32, e: &Entity) -> bool {
        if self.has_openings(id) {
            return false;
        }
        let (_, body) = self.product(e);
        let Some(ex) = body.sole_extrusion_with_voids().filter(|e| e.is_vertical()).cloned() else {
            return false;
        };
        let Some((z0, z1)) = body.z_range() else { return false };
        // The profile plane seen from above, unscaled: the section turns
        // with the column; otherwise its footprint is the section.
        let (xw, yw) = (ex.plane_vector(DVec2::X).truncate(), ex.plane_vector(DVec2::Y).truncate());
        let rigid = (xw.length() - 1.0).abs() < 1e-6 && (yw.length() - 1.0).abs() < 1e-6 && xw.dot(yw).abs() < 1e-6;
        // The piece of a ring (two concentric arcs joined by radial sides): the annular sector
        // itself, placed by its ring, so the columns around a tank share one profile and type.
        let sector = ex.footprint_curves().as_deref().and_then(ring_sector).filter(|s| s.r_in > 1e-6);
        let (read, position, rotation) = if let Some(s) = sector {
            // To the hundredth of a millimeter and the thousandth of a degree: files round
            // their points, and columns alike but for that noise are one profile.
            let (length, angle) = (|v: f64| (v * 1e5).round() / 1e5, |v: f64| (v * 1e3).round() / 1e3);
            let shape = ProfileShape::AnnularSector {
                r_in: length(s.r_in),
                r_out: length(s.r_out),
                sweep: angle(s.sweep.to_degrees()),
            };
            let Some(center) = shape.sector_center() else { return false };
            let name = self.read_section(id, &ex).and_then(|r| r.name);
            let read = ReadSection { shape, name, rotation_deg: 0.0, center: DVec2::ZERO };
            // The sector's own Y axis points out of the ring along its bisector.
            (read, s.center - s.axis * center.y, s.axis.to_angle().to_degrees() - 90.0)
        } else if rigid && xw.perp_dot(yw) > 0.0 {
            let Some(read) = self.read_section(id, &ex) else { return false };
            let position = ex.plane_point(read.center).truncate();
            (read, position, xw.y.atan2(xw.x).to_degrees())
        } else {
            let Some(read) = loose_section(&ex.footprint(), &ex.void_footprints()) else { return false };
            let position = read.center;
            (read, position, 0.0)
        };
        if !(position.is_finite() && rotation.is_finite()) {
            return false;
        }
        let (level, elevation) = self.level_for(id, z0);
        let name = self.type_name(id, e);
        let material = self.main_material(id, e);
        let profile = self.profile_for(id, &read, &material);
        // Without a profile (the project has no room for more) only a box or a round is held exactly.
        let held = matches!(
            read.shape,
            ProfileShape::Rect { chamfer: 0.0, .. } | ProfileShape::Round { .. } | ProfileShape::RoundTube { .. }
        );
        if profile.is_none() && (!held || read.rotation_deg != 0.0) {
            return false;
        }
        let turn = read.rotation_deg;
        let mut data = TypeData::Column {
            section: ColumnSection::Rectangular { width: 1.0, depth: 1.0 },
            material,
            profile,
            profile_rotation_deg: turn,
            profile_mirror: false,
        };
        // The legacy section is the profile's box (or its round), as edits keep it.
        let size = match profile {
            Some(_) => self.p.type_section(&data).map(|s| s.outline.bbox()).map(|(lo, hi)| hi - lo),
            None => read.shape.outline().ok().map(|o| o.bbox()).map(|(lo, hi)| hi - lo),
        };
        let Some(size) = size.filter(|s| s.is_finite() && s.x > 1e-6 && s.y > 1e-6) else { return false };
        let legacy = match read.shape {
            ProfileShape::Round { d } if turn == 0.0 => ColumnSection::Round { diameter: d },
            ProfileShape::RoundTube { d, t } if turn == 0.0 => ColumnSection::Pipe { diameter: d, wall: t },
            _ => ColumnSection::Rectangular { width: size.x, depth: size.y },
        };
        if let TypeData::Column { section, .. } = &mut data {
            *section = legacy;
        }
        let fallback = profile
            .and_then(|p| self.p.profile(p))
            .map_or_else(|| format!("{:.0} x {:.0}", size.x * 100.0, size.y * 100.0), |p| p.name.clone());
        let uid = self.type_uid(id);
        let type_id = self.find_or_add_type(
            Category::Column,
            name,
            fallback,
            uid,
            |d| same_section(d, profile, turn, &data),
            || data.clone(),
        );
        let column = Column {
            position,
            rotation_deg: rotation,
            base_offset: z0 - elevation,
            height: z1 - z0,
            top: WallTop::Unconnected,
            structural: true,
        };
        self.add_element(id, e, level, type_id, Self::tag(e), ElementKind::Column(column));
        true
    }

    /// The section of a column or beam extrusion: the parametric shape of
    /// its profile definition when one holds it (and makes a valid outline),
    /// else the profile's outline as a sketch (`None` when neither holds it). Its
    /// name is the definition's `ProfileName`, else the name of the material
    /// profile that holds the definition in the material profile set of the
    /// product or its type.
    fn read_section(&self, id: u32, ex: &Extrusion) -> Option<ReadSection> {
        let source = ex.source.and_then(|s| self.f.get(s).map(|e| (s, e)));
        let name = source.and_then(|(s, e)| profile_name(e).or_else(|| self.material_profile_name(id, s)));
        let parametric = source.and_then(|(_, e)| {
            let shape = self.g.parametric_shape(e).filter(|s| s.outline().is_ok())?;
            let x = self.g.axis_placement(e.arg(2)).transform_vector3(DVec3::X).truncate();
            let turn = if x.length() > 1e-9 { x.y.atan2(x.x) } else { 0.0 };
            // The middle of the drawn outline's box, in the shape's frame, is
            // where the shape's outline is centered.
            let back = DVec2::from_angle(-turn);
            let (lo, hi) = polygon_bounds(ex.profile.pts.iter().map(|p| back.rotate(*p)))?;
            let center = DVec2::from_angle(turn).rotate((lo + hi) * 0.5);
            let rotation_deg = if turn.abs() < 1e-12 { 0.0 } else { turn.to_degrees() };
            Some(ReadSection { shape, name: name.clone(), rotation_deg, center })
        });
        parametric.or_else(|| loose_section(&ex.profile.pts, &ex.profile.holes).map(|r| ReadSection { name, ..r }))
    }

    /// The material profile set the product `id` or its type uses (IFC4).
    fn material_profile_set(&self, id: u32) -> Option<&'a Entity> {
        let f = self.f;
        let mut owners = vec![id];
        owners.extend(self.type_of.get(&id));
        owners.into_iter().find_map(|owner| {
            let m = self.material_of.get(&owner).and_then(|m| f.get(*m))?;
            let set = if m.name == "IFCMATERIALPROFILESETUSAGE" { f.deref(m.arg(0))? } else { m };
            (set.name == "IFCMATERIALPROFILESET").then_some(set)
        })
    }

    /// The name of the material profile that holds profile definition
    /// `profile` in the material profile set of product `id` or its type.
    fn material_profile_name(&self, id: u32, profile: u32) -> Option<String> {
        let f = self.f;
        let set = self.material_profile_set(id)?;
        let item = set
            .arg(2)
            .as_list()
            .iter()
            .filter_map(|x| f.deref(x))
            .find(|i| i.name == "IFCMATERIALPROFILE" && i.arg(3).as_ref() == Some(profile))?;
        item.arg(0).as_str().map(clean_profile_name).filter(|n| !n.is_empty())
    }

    /// Whether the material profile of product `id` (or its type) is marked
    /// cold-formed ([`crate::export::COLD_FORMED_CATEGORY`]).
    fn cold_formed(&self, id: u32) -> bool {
        let f = self.f;
        self.material_profile_set(id).is_some_and(|set| {
            set.arg(2).as_list().iter().filter_map(|x| f.deref(x)).any(|i| {
                i.name == "IFCMATERIALPROFILE"
                    && i.arg(5).as_str().is_some_and(|c| c.eq_ignore_ascii_case(crate::export::COLD_FORMED_CATEGORY))
            })
        })
    }

    /// The section of a beam written as a `width` x `height` box: the
    /// material profile of product `id` (or its type), with its name, when
    /// it is an unturned solid rectangle of that size; else that rectangle.
    fn rect_section(&self, id: u32, width: f64, height: f64) -> ReadSection {
        let f = self.f;
        let from_set = || {
            let set = self.material_profile_set(id)?;
            let item =
                set.arg(2).as_list().first().and_then(|x| f.deref(x)).filter(|i| i.name == "IFCMATERIALPROFILE")?;
            let def = f.deref(item.arg(3))?;
            let turned = self.g.axis_placement(def.arg(2)).transform_vector3(DVec3::X);
            match self.g.parametric_shape(def)? {
                shape @ ProfileShape::Rect { b, h, .. }
                    if (turned.x - 1.0).abs() < 1e-9
                        && (b - width).abs() < TYPE_TOL
                        && (h - height).abs() < TYPE_TOL =>
                {
                    let name = profile_name(def)
                        .or_else(|| item.arg(0).as_str().map(clean_profile_name))
                        .filter(|n| !n.is_empty());
                    Some(ReadSection { shape, name, rotation_deg: 0.0, center: DVec2::ZERO })
                }
                _ => None,
            }
        };
        from_set().unwrap_or(ReadSection {
            shape: ProfileShape::Rect { b: width, h: height, chamfer: 0.0 },
            name: None,
            rotation_deg: 0.0,
            center: DVec2::ZERO,
        })
    }

    /// The project profile for a read section: the one of its name when it
    /// has the same shape, else a new one (named after the section, under a
    /// free name when another profile has it); without a name, one of the
    /// same material and shape. `None` when the project has no room for more.
    fn profile_for(&mut self, id: u32, read: &ReadSection, material: &str) -> Option<ProfileId> {
        // Float noise in a file reuses a profile rather than making "name 2".
        const TOL: f64 = 1e-6;
        let class = match self.p.profile_material_for(material) {
            ProfileMaterial::Steel if read.shape.is_cold_formed() || self.cold_formed(id) => {
                ProfileMaterial::ColdFormed
            }
            class => class,
        };
        let name = match &read.name {
            Some(name) => match self.p.profile_named(name) {
                Some(p) if p.shape.close_to(&read.shape, TOL) => return Some(p.id),
                Some(_) => self.p.free_profile_name(name),
                None => name.clone(),
            },
            None => {
                if let Some(p) =
                    self.p.profiles.values().find(|p| p.material == class && p.shape.close_to(&read.shape, TOL))
                {
                    return Some(p.id);
                }
                self.p.free_profile_name(&cc_core::profiles_project::section_name_in(
                    &read.shape,
                    class,
                    self.p.measurement_system(),
                ))
            }
        };
        if self.p.profiles.len() >= MAX_PROFILES {
            return None;
        }
        let id = ProfileId(self.p.alloc_id());
        let def = ProfileDef {
            id,
            uid: Uid::generate(),
            name,
            material: class,
            shape: read.shape.clone(),
            catalog: None,
            description: String::new(),
        };
        self.p.profiles.insert(id, def);
        Some(id)
    }

    /// Any physical object without a native counterpart (or whose geometry
    /// a native element cannot hold): a component that keeps its IFC class.
    /// A reinforcing bar drawn as disks swept along center lines becomes a
    /// shaped bar set of the reinforcement category (#263), its type found or
    /// made by diameter and steel (grade or material), so that bars of one
    /// size share it. Its property sets stay with the element. `false` for
    /// bars of other geometry (they come in as components).
    fn import_bar(&mut self, id: u32, e: &Entity) -> bool {
        let (_, body) = self.product(e);
        let mut radius: Option<f64> = None;
        let mut lines = Vec::new();
        for s in &body.solids {
            let Some((path, r)) = s.disk.as_ref().filter(|_| !s.clipped && !s.other_cut) else { return false };
            // One diameter per set: the type holds it.
            if radius.is_some_and(|r0| (r0 - r).abs() > 1e-6) {
                return false;
            }
            radius = Some(*r);
            lines.push(path.clone());
        }
        let Some(radius) = radius else { return false };
        let z0 = lines.iter().flatten().map(|p| p.z).fold(f64::INFINITY, f64::min);
        if !z0.is_finite() {
            return false;
        }
        let (level, elevation) = self.level_for(id, z0);
        let bars = lines.into_iter().map(|l| l.into_iter().map(|p| p - DVec3::Z * elevation).collect()).collect();
        // The partition and the mark given by hand, as this program writes them.
        let own = |name: &str| {
            let is_set = |n: &str| n == crate::export::REBAR_PSET;
            let text = self.pset_texts(id, &is_set, name).first().map(|t| clean_name(t));
            text.filter(|t| !t.is_empty() && t.chars().count() <= cc_core::bar_marks::MAX_MARK_TEXT)
        };
        let (partition, bar_mark, own_grade) = (own("Partition"), own("BarMark"), own("Grade"));
        // Another program's sets say their part of the structure in a property of that name.
        let partition = partition.or_else(|| self.foreign_partition(id)).unwrap_or_default();
        let rebar = ShapedRebar { bars, properties: self.property_sets(id), partition, bar_mark };
        if !rebar.is_valid() {
            return false;
        }
        // The nominal diameter, unless it is far from the drawn one.
        let drawn = 2.0 * radius;
        let diameter = e
            .arg(9)
            .as_f64()
            .map(|d| d * self.scale)
            .filter(|d| d.is_finite() && (d - drawn).abs() <= 0.25 * drawn)
            .unwrap_or(drawn);
        let grade = e.arg(8).as_str().map(clean_name).filter(|g| !g.is_empty());
        let steel = grade.clone().or_else(|| self.materials(id).1);
        let color = self.body_color(e);
        let material = steel.as_ref().map(|m| self.material(m, color));
        // Without a grade or material, any bar type of the size will do.
        let existing = self.p.types_of(Category::Rebar).find_map(|t| match &t.data {
            TypeData::Rebar { material, .. } => Some(material.clone()),
            _ => None,
        });
        let fallback_material = match (&material, existing) {
            (Some(m), _) => m.clone(),
            (None, Some(m)) => m,
            (None, None) => self.material("IFC", color),
        };
        let size = format!("Ø {:.1} mm", diameter * 1000.0);
        // The type's steel grade: this program's own, else the bar's.
        let fits = |g: &String| g.chars().count() <= cc_core::bar_marks::MAX_MARK_TEXT;
        let type_grade = own_grade.or_else(|| grade.clone().filter(fits)).unwrap_or_default();
        let wanted_grade = type_grade.clone();
        let fallback = grade.map_or_else(|| size.clone(), |g| format!("{size} {g}"));
        let name = self.type_entity(id).and_then(|t| t.arg(2).as_str()).map(clean_name).filter(|s| !s.is_empty());
        let uid = self.type_uid(id);
        let type_id = self.find_or_add_type(
            Category::Rebar,
            name,
            fallback,
            uid,
            |t| match t {
                // A bar of a grade takes a type of that grade.
                TypeData::Rebar { diameter: d, material: m, grade: g, .. } => {
                    (d - diameter).abs() <= 1e-5
                        && material.as_ref().is_none_or(|want| m == want)
                        && (wanted_grade.is_empty() || *g == wanted_grade)
                }
                _ => false,
            },
            || TypeData::Rebar { diameter, material: fallback_material, grade: type_grade, mass_per_meter: None },
        );
        // The bar mark: the tag, else the name.
        let mark = Some(Self::tag(e)).filter(|t| !t.is_empty()).or_else(|| e.arg(2).as_str().map(clean_name));
        self.add_element(id, e, level, type_id, mark.unwrap_or_default(), ElementKind::ShapedRebar(rebar));
        true
    }

    fn import_generic(&mut self, id: u32, e: &Entity) -> bool {
        let ok = self.import_component(id, e, kind_of_class(&e.name));
        if ok {
            *self.report.generic.entry(crate::schema::display_name(&e.name)).or_default() += 1;
        }
        ok
    }

    /// IFC class, predefined type, object type and type property sets of a product.
    fn ifc_class(&self, id: u32, e: &Entity) -> IfcClass {
        let properties = self.type_of.get(&id).map(|t| self.property_sets(*t)).unwrap_or_default();
        // Site geometry (terrain) is written back as a geographic element.
        if e.name == "IFCSITE" {
            return IfcClass {
                entity: "IFCGEOGRAPHICELEMENT".into(),
                predefined: "TERRAIN".into(),
                object_type: String::new(),
                properties,
            };
        }
        // Predefined types are read from IFC4 files only (IFC2X3 enumerations differ).
        let predefined = crate::schema::element_class(&e.name)
            .and_then(|c| c.predefined)
            .filter(|_| self.report.schema.starts_with("IFC4"))
            .and_then(|i| e.arg(i).as_enum())
            .unwrap_or_default();
        IfcClass {
            entity: e.name.clone(),
            predefined: predefined.to_string(),
            object_type: e.arg(4).as_str().unwrap_or_default().to_string(),
            properties,
        }
    }

    /// Property and quantity sets attached to `owner` (an object or a type).
    /// The program's own component parameters are left out (they are read
    /// back as overrides).
    fn property_sets(&self, owner: u32) -> Vec<PropertySet> {
        let f = self.f;
        let mut out = Vec::new();
        for set in self.psets_of.get(&owner).into_iter().flatten().filter_map(|s| f.get(*s)) {
            let name = set.arg(2).as_str().unwrap_or_default().to_string();
            if name == crate::export::COMPONENT_PSET
                || name == crate::export::PHASING_PSET
                || name == crate::export::DESIGN_OPTION_PSET
                || name == crate::export::REBAR_PSET
            {
                continue;
            }
            let (quantities, properties) = match set.name.as_str() {
                "IFCPROPERTYSET" => {
                    let (mut path, mut left) = (Vec::new(), MAX_SET_PROPERTIES);
                    let properties =
                        set.arg(4).as_list().iter().filter_map(|p| self.read_property(p, &mut path, &mut left));
                    (false, properties.collect::<Vec<_>>())
                }
                "IFCELEMENTQUANTITY" => (true, set.arg(5).as_list().iter().filter_map(|q| self.quantity(q)).collect()),
                // Predefined property sets (door linings…) and others.
                _ => continue,
            };
            // A common set's `Status` is read as the element's phases.
            let mut properties = properties;
            if name.starts_with("Pset_") && name.ends_with("Common") {
                properties.retain(|p| p.name != "Status");
            }
            if !properties.is_empty() {
                out.push(PropertySet { name, quantities, properties });
            }
        }
        out
    }

    /// A property of a set. Complex properties nest at most
    /// [`MAX_PROPERTY_DEPTH`] deep (`path` holds those being read, and one
    /// already on it is refused) and a set reads at most `left` properties
    /// in all, so shared nested properties cannot multiply.
    fn read_property(&self, param: &Param, path: &mut Vec<u32>, left: &mut usize) -> Option<Property> {
        let id = param.as_ref()?;
        *left = left.checked_sub(1)?;
        let p = self.f.get(id)?;
        let name = p.arg(0).as_str()?.to_string();
        let values = |i: usize, unit: &Param| -> Vec<IfcValue> {
            p.arg(i).as_list().iter().filter_map(|v| self.ifc_value(v, unit)).collect()
        };
        let value = match p.name.as_str() {
            "IFCPROPERTYSINGLEVALUE" => PropertyValue::Single(self.ifc_value(p.arg(2), p.arg(3))),
            "IFCPROPERTYENUMERATEDVALUE" => PropertyValue::Enumerated(values(2, &Param::Null)),
            "IFCPROPERTYLISTVALUE" => PropertyValue::List(values(2, p.arg(3))),
            "IFCPROPERTYBOUNDEDVALUE" => PropertyValue::Bounded {
                upper: self.ifc_value(p.arg(2), p.arg(4)),
                lower: self.ifc_value(p.arg(3), p.arg(4)),
                set_point: self.ifc_value(p.arg(5), p.arg(4)),
            },
            "IFCPROPERTYTABLEVALUE" => {
                PropertyValue::Table { defining: values(2, p.arg(5)), defined: values(3, p.arg(6)) }
            }
            "IFCCOMPLEXPROPERTY" if path.len() < MAX_PROPERTY_DEPTH && !path.contains(&id) => {
                path.push(id);
                let properties = p.arg(3).as_list().iter().filter_map(|x| self.read_property(x, path, left)).collect();
                path.pop();
                PropertyValue::Complex { usage: p.arg(2).as_str().unwrap_or_default().to_string(), properties }
            }
            _ => return None,
        };
        Some(Property { name, value })
    }

    fn quantity(&self, param: &Param) -> Option<Property> {
        let q = self.f.deref(param)?;
        let name = q.arg(0).as_str()?.to_string();
        let explicit = self.f.deref(q.arg(2)).and_then(|u| self.unit_factor(u));
        let factor = match q.name.as_str() {
            "IFCQUANTITYLENGTH" => explicit.unwrap_or(self.scale),
            "IFCQUANTITYAREA" => explicit.unwrap_or(self.area),
            "IFCQUANTITYVOLUME" => explicit.unwrap_or(self.volume),
            "IFCQUANTITYCOUNT" | "IFCQUANTITYWEIGHT" | "IFCQUANTITYTIME" => explicit.unwrap_or(1.0),
            _ => return None,
        };
        let value = q.arg(3).as_f64().map(|v| v * factor).filter(|v| v.is_finite())?;
        Some(Property { name, value: PropertyValue::Quantity { kind: q.name.clone(), value } })
    }

    /// A typed value converted to SI (lengths, areas, volumes, angles), by
    /// its explicit `unit` or the file's units.
    fn ifc_value(&self, param: &Param, unit: &Param) -> Option<IfcValue> {
        let Param::Typed(measure, inner) = param else { return None };
        let explicit = self.f.deref(unit).and_then(|u| self.unit_factor(u));
        let factor = match measure.as_str() {
            "IFCLENGTHMEASURE" | "IFCPOSITIVELENGTHMEASURE" | "IFCNONNEGATIVELENGTHMEASURE" => {
                explicit.unwrap_or(self.scale)
            }
            "IFCAREAMEASURE" => explicit.unwrap_or(self.area),
            "IFCVOLUMEMEASURE" => explicit.unwrap_or(self.volume),
            "IFCPLANEANGLEMEASURE" | "IFCPOSITIVEPLANEANGLEMEASURE" => explicit.unwrap_or(self.g.angle),
            _ => 1.0,
        };
        let value = match inner.as_ref() {
            Param::Real(x) => Atom::Real(x * factor),
            Param::Int(i) if factor == 1.0 => Atom::Integer(*i),
            Param::Int(i) => Atom::Real(*i as f64 * factor),
            Param::Str(s) => Atom::Text(s.clone()),
            Param::Enum(e) if e == "T" => Atom::Bool(true),
            Param::Enum(e) if e == "F" => Atom::Bool(false),
            Param::Enum(e) => Atom::Enum(e.clone()),
            _ => return None,
        };
        value.is_finite().then(|| IfcValue { measure: measure.clone(), value })
    }

    /// Furniture, fixtures and any other object: pieces in the product's own
    /// frame (vertical extrusions as profiles, anything else as a fixed mesh),
    /// colored by their item styles. An existing component type with the
    /// same name and size is reused, so parametric components survive a
    /// round trip.
    fn import_component(&mut self, id: u32, e: &Entity, kind: ComponentKind) -> bool {
        let (world, mut body) = self.product(e);
        if body.solids.is_empty() {
            return false;
        }
        self.cut_openings(id, &mut body);
        let x = world.matrix3.x_axis.truncate().try_normalize().unwrap_or(DVec2::X);
        let rotation = x.y.atan2(x.x);
        let mut origin = world.translation;
        // Plan frame of the component: origin and rotation in plan, Z up.
        let mut frame = affine(x.extend(0.0), x.perp().extend(0.0), DVec3::Z, origin);
        let mut inv = frame.inverse();
        // Geometry written in world coordinates (a placement away from the
        // body) with no type object: the origin goes to the bottom center of
        // the body, so that identical objects at different places (bars, say)
        // have the same pieces and share a type.
        if !self.type_of.contains_key(&id)
            && let Some((lo, hi)) = local_bounds(&body.solids, &inv)
            && !((lo - TYPE_TOL).cmple(DVec3::ZERO).all() && (hi + TYPE_TOL).cmpge(DVec3::ZERO).all())
        {
            origin = frame.transform_point3(DVec3::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0, lo.z));
            frame = affine(x.extend(0.0), x.perp().extend(0.0), DVec3::Z, origin);
            inv = frame.inverse();
        }
        let main = self.main_material(id, e);
        let n = |v: f64| literal(v);
        let base_name = self.type_name(id, e).unwrap_or_else(|| crate::schema::display_name(&e.name));
        let mut items = Vec::new();
        for (i, solid) in body.solids.iter().enumerate() {
            let name = format!("{base_name} {}", i + 1);
            let material = match self.item_styles.get(&solid.item).cloned() {
                Some(style) => {
                    let color = self.item_colors.get(&solid.item).copied();
                    self.material(&style, color)
                }
                None => main.clone(),
            };
            let plain = solid
                .extrusion
                .as_ref()
                .filter(|ex| !solid.clipped && ex.is_vertical() && ex.profile.closed && ex.profile.holes.is_empty());
            let shape = match plain {
                Some(ex) => {
                    let pts: Vec<DVec2> =
                        ex.profile.pts.iter().map(|p| inv.transform_point3(ex.plane_point(*p)).truncate()).collect();
                    let z0 = ex
                        .profile
                        .pts
                        .iter()
                        .map(|p| inv.transform_point3(ex.plane_point(*p)).z)
                        .fold(f64::INFINITY, f64::min);
                    let z0 = z0.min(z0 + ex.vector.z);
                    let points = clean_polygon(pts).into_iter().map(|p| (n(p.x), n(p.y))).collect::<Vec<_>>();
                    if points.len() < 3 {
                        continue;
                    }
                    ComponentShape::Extrusion { points, z: n(z0), height: n(ex.vector.z.abs()) }
                }
                None => {
                    let Some(mesh) = local_mesh(&solid.mesh, &inv) else { continue };
                    mesh
                }
            };
            items.push(ComponentItem {
                name,
                shape,
                material,
                in_plan: true,
                void: false,
                plan_only: false,
                detail: Default::default(),
            });
        }
        let ifc = Some(self.ifc_class(id, e));
        let properties = self.property_sets(id);
        let def = ComponentDef {
            kind,
            params: Vec::new(),
            items,
            ifc,
            mep: None,
            opening: None,
            connectors: Vec::new(),
            ..Default::default()
        };
        let Some((lo, hi)) = def_bounds(&def, &Default::default()) else { return false };
        // Instance parameter values written by ConstructCraft.
        let overrides: std::collections::BTreeMap<String, f64> = self
            .numeric_properties(id, crate::export::COMPONENT_PSET)
            .into_iter()
            .map(|(name, v, measure)| {
                let v = match measure.as_deref() {
                    Some("IFCLENGTHMEASURE") => v * self.scale,
                    Some("IFCPLANEANGLEMEASURE") => v.to_degrees(),
                    _ => v,
                };
                (name, v)
            })
            .filter(|(_, v)| v.is_finite())
            .collect();
        let check = overrides.clone();
        let (level, elevation) = self.level_for(id, origin.z + lo.z);
        let name = self.type_name(id, e);
        let fallback = format!("{} {}", crate::schema::display_name(&e.name), id);
        let uid = self.type_uid(id);
        let component = name.clone().unwrap_or_else(|| fallback.clone());
        // Same name and same size: the existing parametric type. A type
        // imported before as fixed pieces must have the same pieces and class
        // (equal bounds are not the same shape).
        let new_def = def.clone();
        let same = move |d: &TypeData| match d {
            TypeData::Component { def, .. } if def.params.is_empty() && def.ifc.is_some() => {
                def.ifc == new_def.ifc && same_items(&def.items, &new_def.items)
            }
            TypeData::Component { def, .. } => def_bounds(def, &check)
                .is_some_and(|(a, b)| (a - lo).abs().max_element() < 1e-3 && (b - hi).abs().max_element() < 1e-3),
            _ => false,
        };
        let type_id = self.find_or_add_type(Category::Component, name, fallback, uid, same, || TypeData::Component {
            component,
            def,
        });
        // Overrides only make sense for a parametric type that declares them.
        let overrides = match self.p.element_type(type_id).map(|t| &t.data) {
            Some(TypeData::Component { def, .. }) => overrides
                .into_iter()
                .filter(|(k, v)| def.param(k).is_some_and(|p| p.instance && p.value != *v))
                .collect(),
            _ => Default::default(),
        };
        let c = Component {
            system: None,
            position: origin.truncate(),
            rotation_deg: rotation.to_degrees(),
            base_offset: origin.z - elevation,
            overrides,
            properties,
            ..Default::default()
        };
        self.add_element(id, e, level, type_id, Self::tag(e), ElementKind::Component(c));
        true
    }

    /// Subtract the bodies of the openings voiding `id` from its solids
    /// (openings too complex to cut are left uncut).
    fn cut_openings(&self, id: u32, body: &mut Body) {
        let f = self.f;
        let openings = self.openings_of.get(&id).into_iter().flatten().filter_map(|o| f.get(*o));
        for opening in openings.take(MAX_OPENINGS) {
            let (_, cutter) = self.product(opening);
            for c in &cutter.solids {
                for s in &mut body.solids {
                    if let Some(mesh) = crate::geometry::subtract(&s.mesh, &c.mesh) {
                        s.replace_mesh(mesh);
                    }
                }
            }
        }
        body.solids.retain(|s| !s.mesh.is_empty());
    }

    /// Grid axes of an `IfcGrid` as grid lines.
    fn import_grid(&mut self, id: u32, e: &Entity) {
        let f = self.f;
        let world = self.g.placement(e.arg(5));
        let Some(type_id) = self.p.default_type(Category::Grid).or_else(|| {
            self.p.ensure_annotation_types();
            self.p.default_type(Category::Grid)
        }) else {
            return;
        };
        let (level, _) = self.level_for(id, world.translation.z);
        for axis in (7..10).flat_map(|i| e.arg(i).as_list().iter()).filter_map(|a| f.deref(a)) {
            let pts = self.g.curve(axis.arg(1), 0);
            let (Some(a), Some(b)) = (pts.first(), pts.last()) else { continue };
            let (mut start, mut end) = (world.transform_point3(*a).truncate(), world.transform_point3(*b).truncate());
            if axis.arg(2).as_bool() == Some(false) {
                std::mem::swap(&mut start, &mut end);
            }
            if start.distance(end) < 1e-6 || !(start.is_finite() && end.is_finite()) {
                continue;
            }
            let name = axis.arg(0).as_str().unwrap_or_default().to_string();
            let element = ElementId(self.p.alloc_id());
            let grid = Grid { start, end, name };
            let el = Element {
                id: element,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                kind: ElementKind::Grid(grid),
                uid: Uid::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
            };
            *self.report.imported.entry(Category::Grid).or_default() += 1;
            self.p.elements.insert(element, el);
        }
    }

    fn import_beam(&mut self, id: u32, e: &Entity) -> bool {
        if self.has_openings(id) {
            return false;
        }
        let (_, body) = self.product(e);
        let Some((_, z1)) = body.z_range() else { return false };
        // An arc beam: an axis on a circular arc and a band between two
        // circles concentric with it, extruded upwards.
        if let Some(ex) = body.sole_extrusion().filter(|e| e.is_vertical())
            && let Some((start, end, bulge, width)) = arc_wall(&body.axis_points, &ex.footprint())
        {
            let read = self.rect_section(id, width, ex.vector.z.abs());
            return self.add_beam(id, e, (start, end), bulge, read, z1);
        }
        // The same band drawn without an axis: its profile is two concentric
        // arcs joined by radial ends, and the beam's axis runs midway.
        if body.axis_points.is_empty()
            && let Some(ex) = body.sole_extrusion().filter(|e| e.is_vertical())
            && let Some(s) = ex.footprint_curves().as_deref().and_then(ring_sector).filter(|s| s.r_in > 1e-6)
        {
            let (radius, half) = ((s.r_in + s.r_out) / 2.0, s.sweep / 2.0);
            let end_at = |turn: f64| s.center + DVec2::from_angle(turn).rotate(s.axis) * radius;
            // Counter-clockwise from start to end: a positive bulge.
            let (start, end, bulge) = (end_at(-half), end_at(half), (s.sweep / 4.0).tan());
            let read = self.rect_section(id, s.r_out - s.r_in, ex.vector.z.abs());
            return self.add_beam(id, e, (start, end), bulge, read, z1);
        }
        // A plan footprint extruded upwards along an axis: the axis keeps the
        // beam's own length (the body's ends include its joins).
        if let Some((a, b)) = body.axis
            && let Some(ex) = body.sole_extrusion().filter(|e| e.is_vertical())
        {
            let Some(dir) = (b - a).try_normalize() else { return false };
            let footprint = ex.footprint();
            let across: Vec<f64> = footprint.iter().map(|p| (*p - a).dot(dir.perp())).collect();
            let (v0, v1) =
                across.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
            if !across.iter().all(|v| (v - v0).abs() < 1e-3 || (v - v1).abs() < 1e-3) {
                return false;
            }
            let n = dir.perp() * ((v0 + v1) / 2.0);
            let (width, height) = (v1 - v0, ex.vector.z.abs());
            let read = self.rect_section(id, width, height);
            return self.add_beam(id, e, (a + n, b + n), 0.0, read, z1);
        }
        let Some(ex) = body.sole_extrusion_with_voids().cloned() else { return false };
        if ex.is_vertical() {
            // Plan rectangle extruded upwards: the long side is the beam.
            if !ex.profile.is_simple() {
                return false;
            }
            let (center, x, w, h) = ex.profile.rect();
            let c = ex.plane_point(center).truncate();
            let xw = ex.plane_vector(x).truncate().try_normalize().unwrap_or(DVec2::X);
            let (along, len, across) = if w >= h { (xw, w, h) } else { (xw.perp(), h, w) };
            let height = ex.vector.z.abs();
            let read = self.rect_section(id, across, height);
            return self.add_beam(id, e, (c - along * len / 2.0, c + along * len / 2.0), 0.0, read, z1);
        }
        // A section extruded along the beam, level.
        let Some(dir) = ex.vector.truncate().try_normalize().filter(|_| ex.vector.normalize().z.abs() < 0.1) else {
            return false;
        };
        let n = dir.perp();
        let across = n.extend(0.0);
        // The profile plane's axes across the beam (to its left) and up.
        let (xw, yw) = (ex.plane_vector(DVec2::X), ex.plane_vector(DVec2::Y));
        let (xv, xz, yv, yz) = (xw.dot(across), xw.z, yw.dot(across), yw.z);
        let rigid = (xw.length() - 1.0).abs() < 1e-6 && (yw.length() - 1.0).abs() < 1e-6 && xw.dot(yw).abs() < 1e-6;
        let world: Vec<DVec3> = ex.profile.pts.iter().map(|p| ex.plane_point(*p)).collect();
        let origin = ex.plane_point(DVec2::ZERO);
        let read = if rigid && xv * yz - xz * yv > 0.0 {
            let Some(read) = self.read_section(id, &ex) else { return false };
            let turn = xz.atan2(xv).to_degrees();
            ReadSection { rotation_deg: read.rotation_deg + if turn.abs() < 1e-9 { 0.0 } else { turn }, ..read }
        } else {
            // The section as seen along the beam.
            let in_section = |w: DVec3| DVec2::new((w - origin).dot(across), w.z);
            let holes: Vec<Vec<DVec2>> =
                ex.profile.holes.iter().map(|h| h.iter().map(|p| in_section(ex.plane_point(*p))).collect()).collect();
            let outer: Vec<DVec2> = world.iter().map(|w| in_section(*w)).collect();
            let Some(read) = loose_section(&outer, &holes) else { return false };
            read
        };
        // The body's middle across the beam, on the axis when it has one along the beam.
        let (a, b) = match body.axis {
            Some((a, b)) if (b - a).try_normalize().is_some_and(|d| d.dot(dir) > 1.0 - 1e-6) => (a, b),
            _ => (origin.truncate(), origin.truncate() + ex.vector.truncate()),
        };
        let Some((v0, v1)) = range(world.iter().map(|w| (w.truncate() - a).dot(n))) else { return false };
        let shift = n * ((v0 + v1) / 2.0);
        self.add_beam(id, e, (a + shift, b + shift), 0.0, read, z1)
    }

    /// A native beam along `start`-`end` (an arc with a `bulge`) with its top
    /// at `z1`, its type of the section `read`.
    fn add_beam(
        &mut self,
        id: u32,
        e: &Entity,
        (start, end): (DVec2, DVec2),
        bulge: f64,
        read: ReadSection,
        z1: f64,
    ) -> bool {
        if start.distance(end) < 0.01 || !(start.is_finite() && end.is_finite() && z1.is_finite()) {
            return false;
        }
        let material = self.main_material(id, e);
        let profile = self.profile_for(id, &read, &material);
        // Without a profile (the project has no room for more) only a box is held exactly.
        if profile.is_none()
            && (!matches!(read.shape, ProfileShape::Rect { chamfer: 0.0, .. }) || read.rotation_deg != 0.0)
        {
            return false;
        }
        let turn = read.rotation_deg;
        let mut data = TypeData::Beam {
            width: 1.0,
            height: 1.0,
            material,
            shape: SectionShape::Solid,
            profile,
            profile_rotation_deg: turn,
            profile_mirror: false,
        };
        // The legacy size is the profile's box, as edits keep it.
        let size = match profile {
            Some(_) => self.p.type_section(&data).map(|s| s.outline.bbox()).map(|(lo, hi)| hi - lo),
            None => read.shape.outline().ok().map(|o| o.bbox()).map(|(lo, hi)| hi - lo),
        };
        let Some(size) = size.filter(|s| s.is_finite() && s.x > 1e-6 && s.y > 1e-6) else { return false };
        if let TypeData::Beam { width, height, .. } = &mut data {
            *width = size.x;
            *height = size.y;
        }
        let (level, elevation) = self.level_for(id, z1 - size.y);
        let name = self.type_name(id, e);
        let uid = self.type_uid(id);
        let fallback = profile
            .and_then(|p| self.p.profile(p))
            .map_or_else(|| format!("{:.0} x {:.0}", size.x * 100.0, size.y * 100.0), |p| p.name.clone());
        let type_id = self.find_or_add_type(
            Category::Beam,
            name,
            fallback,
            uid,
            |d| same_section(d, profile, turn, &data),
            || data.clone(),
        );
        let mut beam = Beam::new(start, end, z1 - elevation);
        beam.bulge = bulge;
        self.add_element(id, e, level, type_id, Self::tag(e), ElementKind::Beam(beam));
        true
    }
}

/// A column or beam section read from an IFC profile.
struct ReadSection {
    shape: ProfileShape,
    /// The profile's name in the file.
    name: Option<String>,
    /// Turn of the shape in the profile plane, degrees counter-clockwise.
    rotation_deg: f64,
    /// The profile-plane point the shape's outline is centered on (its box's middle).
    center: DVec2,
}

/// A section drawn as `outer` with `holes`: a sketch of those points
/// (centered on their box). `None` when it does not make a valid outline,
/// has more points or openings than a sketch holds, or a point is not
/// finite: the product then keeps its exact mesh as a component.
fn loose_section(outer: &[DVec2], holes: &[Vec<DVec2>]) -> Option<ReadSection> {
    let (lo, hi) = polygon_bounds(outer.iter().copied())?;
    let center = (lo + hi) * 0.5;
    let points = outer.len() + holes.iter().map(Vec::len).sum::<usize>();
    let sketch = (holes.len() <= MAX_SKETCH_OPENINGS && points <= MAX_SKETCH_POINTS).then(|| {
        let ring = |r: &[DVec2]| -> Vec<ProfilePoint> {
            r.iter().map(|p| ProfilePoint::new(literal(p.x - center.x), literal(p.y - center.y))).collect()
        };
        ProfileShape::Sketch(ParamSketch {
            params: Vec::new(),
            outline: ring(outer),
            openings: holes.iter().map(|h| ring(h)).collect(),
        })
    });
    let shape = sketch.filter(|s| s.outline().is_ok())?;
    Some(ReadSection { shape, name: None, rotation_deg: 0.0, center })
}

/// The corners of the box around `pts`; `None` when there are none or one is not finite.
fn polygon_bounds(pts: impl Iterator<Item = DVec2>) -> Option<(DVec2, DVec2)> {
    let mut found = None;
    for p in pts {
        if !p.is_finite() {
            return None;
        }
        found = Some(found.map_or((p, p), |(lo, hi): (DVec2, DVec2)| (lo.min(p), hi.max(p))));
    }
    found
}

/// The least and greatest of `values`; `None` when there are none or one is not finite.
fn range(values: impl Iterator<Item = f64>) -> Option<(f64, f64)> {
    polygon_bounds(values.map(DVec2::splat)).map(|(lo, hi)| (lo.x, hi.x))
}

/// A profile definition's `ProfileName`, cleaned.
fn profile_name(e: &Entity) -> Option<String> {
    e.arg(1).as_str().map(clean_profile_name).filter(|n| !n.is_empty())
}

/// A name trimmed and cut to a profile name's length.
fn clean_profile_name(name: &str) -> String {
    name.trim().chars().take(MAX_NAME).collect::<String>().trim().to_string()
}

/// Whether a column or beam type draws the section of `made`: the same
/// profile and turn, unmirrored, and without a profile the same legacy section.
fn same_section(d: &TypeData, profile: Option<ProfileId>, turn: f64, made: &TypeData) -> bool {
    match (d, made) {
        (
            TypeData::Column { profile: p, profile_rotation_deg: r, profile_mirror: m, section: a, .. },
            TypeData::Column { section: b, .. },
        ) => *p == profile && !*m && (r - turn).abs() < 1e-6 && (profile.is_some() || a == b),
        (
            TypeData::Beam { profile: p, profile_rotation_deg: r, profile_mirror: m, width: w0, height: h0, .. },
            TypeData::Beam { width: w1, height: h1, .. },
        ) => {
            *p == profile
                && !*m
                && (r - turn).abs() < 1e-6
                && (profile.is_some() || ((w0 - w1).abs() < TYPE_TOL && (h0 - h1).abs() < TYPE_TOL))
        }
        _ => false,
    }
}

/// A world mesh as a fixed component piece in the frame `inv` maps to.
fn local_mesh(mesh: &[Vec<DVec3>], inv: &DAffine3) -> Option<ComponentShape> {
    let mut vertices: Vec<[f64; 3]> = Vec::new();
    let mut index: HashMap<[i64; 3], u32> = HashMap::new();
    let mut faces = Vec::with_capacity(mesh.len());
    for face in mesh {
        let mut f: Vec<u32> = Vec::with_capacity(face.len());
        for p in face {
            let q = inv.transform_point3(*p);
            if !q.is_finite() {
                continue;
            }
            // Snap to 1/100 mm so shared corners share a vertex.
            let key = [(q.x * 1e5).round() as i64, (q.y * 1e5).round() as i64, (q.z * 1e5).round() as i64];
            let i = *index.entry(key).or_insert_with(|| {
                vertices.push([q.x, q.y, q.z]);
                (vertices.len() - 1) as u32
            });
            if f.last() != Some(&i) {
                f.push(i);
            }
        }
        if f.len() > 1 && f.first() == f.last() {
            f.pop();
        }
        if f.len() >= 3 {
            faces.push(f);
        }
    }
    let faces = cc_geom::coplanar::merge(&vertices, faces);
    (!faces.is_empty()).then_some(ComponentShape::Mesh { vertices: vertices.into(), faces: faces.into() })
}

/// Bounds of the solids' vertices in the frame `inv` maps to.
fn local_bounds(solids: &[Solid], inv: &DAffine3) -> Option<(DVec3, DVec3)> {
    solids
        .iter()
        .flat_map(|s| s.mesh.iter().flatten())
        .map(|p| inv.transform_point3(*p))
        .filter(|p| p.is_finite())
        .fold(None, |acc, p| Some(acc.map_or((p, p), |(lo, hi): (DVec3, DVec3)| (lo.min(p), hi.max(p)))))
}

/// Pieces imported from two objects are the same (within [`PIECE_TOL`]):
/// same shapes, materials and flags. Only fixed meshes and literal
/// extrusions are compared by value; other shapes must be equal.
fn same_items(a: &[ComponentItem], b: &[ComponentItem]) -> bool {
    let num = |x: &str, y: &str| match (x.parse::<f64>(), y.parse::<f64>()) {
        (Ok(x), Ok(y)) => (x - y).abs() <= PIECE_TOL,
        _ => x == y,
    };
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.material == b.material
                && a.in_plan == b.in_plan
                && a.void == b.void
                && a.plan_only == b.plan_only
                && a.detail == b.detail
                && match (&a.shape, &b.shape) {
                    (
                        ComponentShape::Mesh { vertices: va, faces: fa },
                        ComponentShape::Mesh { vertices: vb, faces: fb },
                    ) => {
                        fa == fb
                            && va.len() == vb.len()
                            && va
                                .iter()
                                .zip(vb.iter())
                                .all(|(p, q)| p.iter().zip(q).all(|(x, y)| (x - y).abs() <= PIECE_TOL))
                    }
                    (
                        ComponentShape::Extrusion { points: pa, z: za, height: ha },
                        ComponentShape::Extrusion { points: pb, z: zb, height: hb },
                    ) => {
                        num(za, zb)
                            && num(ha, hb)
                            && pa.len() == pb.len()
                            && pa.iter().zip(pb).all(|(p, q)| num(&p.0, &q.0) && num(&p.1, &q.1))
                    }
                    (sa, sb) => sa == sb,
                }
        })
}

/// A number as an expression literal.
fn literal(v: f64) -> String {
    let s = format!("{:.6}", if v.abs() < 5e-7 { 0.0 } else { v });
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".to_string() } else { s.to_string() }
}

/// Bounds of a definition's pieces in its own frame.
fn def_bounds(def: &ComponentDef, overrides: &BTreeMap<String, f64>) -> Option<(DVec3, DVec3)> {
    let (parts, errors) = def.resolve(overrides);
    if !errors.is_empty() {
        return None;
    }
    parts
        .iter()
        .flat_map(|p| p.footprint.iter().flat_map(move |q| [q.extend(p.z0), q.extend(p.z1)]))
        .fold(None, |acc, p| Some(acc.map_or((p, p), |(lo, hi): (DVec3, DVec3)| (lo.min(p), hi.max(p)))))
}

fn si_prefix(prefix: Option<&str>) -> f64 {
    match prefix {
        Some("MILLI") => 0.001,
        Some("CENTI") => 0.01,
        Some("DECI") => 0.1,
        Some("KILO") => 1000.0,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hull_and_names() {
        let pts = [DVec2::ZERO, DVec2::new(2.0, 0.0), DVec2::new(1.0, 0.5), DVec2::new(2.0, 1.0), DVec2::new(0.0, 1.0)];
        let hull = convex_hull(&pts);
        assert_eq!(hull.len(), 4);
        assert!((polygon::signed_area(&hull) - 2.0).abs() < 1e-12);
        assert_eq!(clean_name("Basic Wall:Generic 200mm:123456"), "Basic Wall:Generic 200mm");
        assert_eq!(clean_name("Alvenaria 15 cm"), "Alvenaria 15 cm");
    }

    #[test]
    fn non_ifc_and_garbage_are_errors() {
        assert!(matches!(import("garbage", Project::empty()), Err(ImportError::Parse(_))));
        let empty = "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\nENDSEC;\nEND-ISO-10303-21;\n";
        assert!(matches!(import(empty, Project::empty()), Err(ImportError::NotIfc)));
    }

    /// A minimal IFC2X3-style file in millimeters with one wall, a storey and
    /// dangling or cyclic references, as other tools may write.
    #[test]
    fn foreign_file_in_millimeters() {
        let text = "ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Foreign',$,$,$,$,(#2),#3);
#3=IFCUNITASSIGNMENT((#4));
#4=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#10=IFCBUILDINGSTOREY('1XvctVUKr0kugbFTf53O9L',$,'Level 1',$,$,#11,$,$,.ELEMENT.,3000.);
#11=IFCLOCALPLACEMENT($,#12);
#12=IFCAXIS2PLACEMENT3D(#13,$,$);
#13=IFCCARTESIANPOINT((0.,0.,3000.));
#20=IFCWALLSTANDARDCASE('2XvctVUKr0kugbFTf53O9L',$,'Basic Wall:Generic 200mm:4711',$,$,#21,#30,'W1');
#21=IFCLOCALPLACEMENT(#11,#22);
#22=IFCAXIS2PLACEMENT3D(#23,#24,#25);
#23=IFCCARTESIANPOINT((1000.,2000.,0.));
#24=IFCDIRECTION((0.,0.,1.));
#25=IFCDIRECTION((0.,1.,0.));
#30=IFCPRODUCTDEFINITIONSHAPE($,$,(#31));
#31=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#32));
#32=IFCEXTRUDEDAREASOLID(#33,$,#24,2700.);
#33=IFCRECTANGLEPROFILEDEF(.AREA.,$,#34,5000.,200.);
#34=IFCAXIS2PLACEMENT2D(#35,$);
#35=IFCCARTESIANPOINT((2500.,0.));
#40=IFCRELCONTAINEDINSPATIALSTRUCTURE('3XvctVUKr0kugbFTf53O9L',$,$,$,(#20,#99),#10);
#50=IFCLOCALPLACEMENT(#51,#12);
#51=IFCLOCALPLACEMENT(#50,#12);
#60=IFCCOLUMN('4XvctVUKr0kugbFTf53O9L',$,'C',$,$,#50,#61,$);
#61=IFCPRODUCTDEFINITIONSHAPE($,$,(#62));
#62=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#63));
#63=IFCEXTRUDEDAREASOLID(#64,$,#24,3000.);
#64=IFCCIRCLEPROFILEDEF(.AREA.,$,$,150.);
#70=IFCFURNISHINGELEMENT('5XvctVUKr0kugbFTf53O9L',$,'Chair',$,$,#11,#30,$);
ENDSEC;
END-ISO-10303-21;
";
        let (p, report) = import(text, Project::from_template()).unwrap();
        assert_eq!(report.schema, "IFC2X3");
        assert_eq!(p.levels.len(), 1);
        assert_eq!(p.levels[0].name, "Level 1");
        assert!((p.levels[0].elevation - 3.0).abs() < 1e-9);
        let (wall_el, wall) = p.walls().next().unwrap();
        // The wall runs along the placement's X axis (world +Y) from (1, 2).
        assert!(wall.start.distance(DVec2::new(1.0, 2.0)) < 1e-9, "{}", wall.start);
        assert!(wall.end.distance(DVec2::new(1.0, 7.0)) < 1e-9, "{}", wall.end);
        assert!((wall.height - 2.7).abs() < 1e-9);
        assert!(wall.base_offset.abs() < 1e-9);
        assert!((p.thickness_of(wall_el) - 0.2).abs() < 1e-9);
        assert_eq!(p.type_name(wall_el.type_id), "Basic Wall:Generic 200mm");
        assert_eq!(wall_el.mark, "W1");
        // The cyclic placement does not hang; the column still imports.
        assert_eq!(report.imported.get(&Category::Column), Some(&1));
        // Furnishings become components (here a box from the wall's body).
        assert_eq!(report.imported.get(&Category::Component), Some(&1));
        assert!(report.skipped.is_empty(), "{:?}", report.skipped);
        assert!(p.views_of(cc_core::ViewCategory::FloorPlan).count() == 1);
    }

    /// Slabs as other tools write them (millimeters): a floor voided by a
    /// shaft through it, one with a recess, one whose profile has a void, one
    /// with the shaft cut out of its body and voided by an opening too, a
    /// raft, and a pile and two pile caps written as base slabs.
    #[test]
    fn slabs_with_openings_and_base_slabs() {
        let text = "ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Slabs',$,$,$,$,$,#3);
#3=IFCUNITASSIGNMENT((#4));
#4=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#10=IFCBUILDINGSTOREY('1XvctVUKr0kugbFTf53O9L',$,'Level 1',$,$,#11,$,$,.ELEMENT.,0.);
#11=IFCLOCALPLACEMENT($,#12);
#12=IFCAXIS2PLACEMENT3D(#13,$,$);
#13=IFCCARTESIANPOINT((0.,0.,0.));
#14=IFCDIRECTION((0.,0.,1.));
#15=IFCAXIS2PLACEMENT3D(#16,$,$);
#16=IFCCARTESIANPOINT((0.,0.,-300.));
#100=IFCSLAB('2XvctVUKr0kugbFTf53O9L',$,'Floor:Generic 300mm:1',$,$,#101,#102,'1',.FLOOR.);
#101=IFCLOCALPLACEMENT(#11,#12);
#102=IFCPRODUCTDEFINITIONSHAPE($,$,(#103));
#103=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#104));
#104=IFCEXTRUDEDAREASOLID(#105,#15,#14,300.);
#105=IFCRECTANGLEPROFILEDEF(.AREA.,$,#106,10000.,6000.);
#106=IFCAXIS2PLACEMENT2D(#107,$);
#107=IFCCARTESIANPOINT((5000.,3000.));
#110=IFCOPENINGELEMENT('3XvctVUKr0kugbFTf53O9L',$,'Shaft',$,$,#111,#112,$,.OPENING.);
#111=IFCLOCALPLACEMENT(#101,#12);
#112=IFCPRODUCTDEFINITIONSHAPE($,$,(#113));
#113=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#114));
#114=IFCEXTRUDEDAREASOLID(#115,#15,#14,300.);
#115=IFCRECTANGLEPROFILEDEF(.AREA.,$,#116,2000.,2000.);
#116=IFCAXIS2PLACEMENT2D(#117,$);
#117=IFCCARTESIANPOINT((2000.,2000.));
#118=IFCRELVOIDSELEMENT('4XvctVUKr0kugbFTf53O9L',$,$,$,#100,#110);
#140=IFCSLAB('5XvctVUKr0kugbFTf53O9L',$,'Floor:Generic 300mm:2',$,$,#141,#102,'2',.FLOOR.);
#141=IFCLOCALPLACEMENT(#11,#142);
#142=IFCAXIS2PLACEMENT3D(#143,$,$);
#143=IFCCARTESIANPOINT((20000.,0.,0.));
#150=IFCOPENINGELEMENT('6XvctVUKr0kugbFTf53O9L',$,'Recess',$,$,#141,#151,$,.RECESS.);
#151=IFCPRODUCTDEFINITIONSHAPE($,$,(#152));
#152=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#153));
#153=IFCEXTRUDEDAREASOLID(#115,#15,#14,100.);
#154=IFCRELVOIDSELEMENT('7XvctVUKr0kugbFTf53O9L',$,$,$,#140,#150);
#200=IFCSLAB('8XvctVUKr0kugbFTf53O9L',$,'Raft:400mm:3',$,$,#201,#202,'3',.BASESLAB.);
#201=IFCLOCALPLACEMENT(#11,#203);
#203=IFCAXIS2PLACEMENT3D(#204,$,$);
#204=IFCCARTESIANPOINT((0.,20000.,-1000.));
#202=IFCPRODUCTDEFINITIONSHAPE($,$,(#205));
#205=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#206));
#206=IFCEXTRUDEDAREASOLID(#207,$,#14,400.);
#207=IFCRECTANGLEPROFILEDEF(.AREA.,$,#106,12000.,12000.);
#300=IFCSLAB('9XvctVUKr0kugbFTf53O9L',$,'Pile:500mm Diameter:4',$,$,#201,#302,'4',.BASESLAB.);
#302=IFCPRODUCTDEFINITIONSHAPE($,$,(#303));
#303=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#304));
#304=IFCEXTRUDEDAREASOLID(#305,$,#14,6000.);
#305=IFCCIRCLEPROFILEDEF(.AREA.,$,$,250.);
#310=IFCSLAB('AXvctVUKr0kugbFTf53O9L',$,'Pile Cap:800 x 1800:5',$,$,#201,#312,'5',.BASESLAB.);
#312=IFCPRODUCTDEFINITIONSHAPE($,$,(#313));
#313=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#314));
#314=IFCEXTRUDEDAREASOLID(#315,$,#14,1200.);
#315=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,1800.,800.);
#320=IFCSLAB('GXvctVUKr0kugbFTf53O9L',$,'Pile Cap:2000 x 2000:8',$,$,#201,#322,'8',.BASESLAB.);
#322=IFCPRODUCTDEFINITIONSHAPE($,$,(#323));
#323=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#324));
#324=IFCEXTRUDEDAREASOLID(#325,$,#14,900.);
#325=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,2000.,2000.);
#400=IFCSLAB('BXvctVUKr0kugbFTf53O9L',$,'Floor:Voided:6',$,$,#401,#402,'6',.FLOOR.);
#401=IFCLOCALPLACEMENT(#11,#403);
#403=IFCAXIS2PLACEMENT3D(#404,$,$);
#404=IFCCARTESIANPOINT((40000.,0.,0.));
#402=IFCPRODUCTDEFINITIONSHAPE($,$,(#405));
#405=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#406));
#406=IFCEXTRUDEDAREASOLID(#407,#15,#14,300.);
#407=IFCARBITRARYPROFILEDEFWITHVOIDS(.AREA.,$,#408,(#409));
#408=IFCPOLYLINE((#410,#411,#412,#413,#410));
#409=IFCPOLYLINE((#414,#415,#416,#417,#414));
#410=IFCCARTESIANPOINT((0.,0.));
#411=IFCCARTESIANPOINT((8000.,0.));
#412=IFCCARTESIANPOINT((8000.,8000.));
#413=IFCCARTESIANPOINT((0.,8000.));
#414=IFCCARTESIANPOINT((3000.,3000.));
#415=IFCCARTESIANPOINT((3000.,5000.));
#416=IFCCARTESIANPOINT((5000.,5000.));
#417=IFCCARTESIANPOINT((5000.,3000.));
#600=IFCSLAB('DXvctVUKr0kugbFTf53O9L',$,'Floor:Cut:7',$,$,#601,#602,'7',.FLOOR.);
#601=IFCLOCALPLACEMENT(#11,#603);
#603=IFCAXIS2PLACEMENT3D(#604,$,$);
#604=IFCCARTESIANPOINT((60000.,0.,0.));
#602=IFCPRODUCTDEFINITIONSHAPE($,$,(#605));
#605=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#606));
#606=IFCBOOLEANRESULT(.DIFFERENCE.,#104,#114);
#610=IFCOPENINGELEMENT('EXvctVUKr0kugbFTf53O9L',$,'Shaft',$,$,#611,#112,$,.OPENING.);
#611=IFCLOCALPLACEMENT(#601,#12);
#612=IFCRELVOIDSELEMENT('FXvctVUKr0kugbFTf53O9L',$,$,$,#600,#610);
#500=IFCRELCONTAINEDINSPATIALSTRUCTURE('CXvctVUKr0kugbFTf53O9L',$,$,$,(#100,#140,#200,#300,#310,#320,#400,#600),#10);
ENDSEC;
END-ISO-10303-21;
";
        let (p, report) = import(text, Project::from_template()).unwrap();
        let floors: Vec<&cc_core::Floor> = p
            .elements
            .values()
            .filter_map(|e| match &e.kind {
                ElementKind::Floor(f) => Some(f),
                _ => None,
            })
            .collect();
        // The shafted floors and the voided one are native, with their openings.
        assert_eq!(floors.len(), 3, "{report:?}");
        let cut = floors.iter().find(|f| f.boundary.iter().all(|q| q.x >= 60.0 - 1e-9)).unwrap();
        assert_eq!(cut.openings.len(), 1, "the shaft counts once");
        assert!((cut.area() - 56.0).abs() < 1e-9, "{}", cut.area());
        let shafted = floors.iter().find(|f| f.boundary.iter().all(|q| q.x < 15.0)).unwrap();
        assert_eq!(shafted.openings.len(), 1);
        assert!((shafted.area() - (60.0 - 4.0)).abs() < 1e-9, "{}", shafted.area());
        assert!(shafted.height_offset.abs() < 1e-9);
        let voided = floors.iter().find(|f| f.boundary.iter().all(|q| q.x >= 40.0 - 1e-9)).unwrap();
        assert_eq!(voided.openings.len(), 1);
        assert!((voided.area() - 60.0).abs() < 1e-9, "{}", voided.area());
        // The raft is a foundation slab, its top 0.6 m below the level.
        let rafts: Vec<&cc_core::Foundation> = p
            .elements
            .values()
            .filter_map(|e| match &e.kind {
                ElementKind::Foundation(f) => Some(f),
                _ => None,
            })
            .collect();
        assert_eq!(rafts.len(), 1);
        assert!(matches!(rafts[0].shape, cc_core::FoundationShape::Slab { .. }));
        assert!((rafts[0].top_offset + 0.6).abs() < 1e-9, "{}", rafts[0].top_offset);
        // The recessed floor, the pile and the pile caps keep their class as components.
        assert_eq!(report.generic.values().sum::<usize>(), 4, "{:?}", report.generic);
        let slab_components = p
            .types
            .values()
            .filter(|t| match &t.data {
                TypeData::Component { def, .. } => def.ifc.as_ref().is_some_and(|c| c.entity == "IFCSLAB"),
                _ => false,
            })
            .count();
        assert!(slab_components >= 1);
    }

    /// Profiles as other tools write them (IFC2X3, millimeters, no material
    /// profile sets): a named I column, an I whose root radius does not fit
    /// (its drawn outline comes in as a sketch), an angle thicker than its
    /// legs (an outline that crosses itself: a component), and a channel beam
    /// extruded along its axis.
    #[test]
    fn foreign_and_hostile_profiles() {
        let text = "ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Foreign',$,$,$,$,(#2),#3);
#3=IFCUNITASSIGNMENT((#4));
#4=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#10=IFCBUILDINGSTOREY('1XvctVUKr0kugbFTf53O9L',$,'Level 1',$,$,#11,$,$,.ELEMENT.,0.);
#11=IFCLOCALPLACEMENT($,#12);
#12=IFCAXIS2PLACEMENT3D(#13,$,$);
#13=IFCCARTESIANPOINT((0.,0.,0.));
#24=IFCDIRECTION((0.,0.,1.));
#30=IFCAXIS2PLACEMENT2D(#13,$);
#40=IFCRELCONTAINEDINSPATIALSTRUCTURE('3XvctVUKr0kugbFTf53O9L',$,$,$,(#50,#60,#70,#80),#10);
#50=IFCCOLUMN('4XvctVUKr0kugbFTf53O9L',$,'A',$,$,#11,#51,$);
#51=IFCPRODUCTDEFINITIONSHAPE($,$,(#52));
#52=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#53));
#53=IFCEXTRUDEDAREASOLID(#54,$,#24,3000.);
#54=IFCISHAPEPROFILEDEF(.AREA.,'IPE200',#30,100.,200.,5.6,8.5,12.,$,$);
#60=IFCCOLUMN('5XvctVUKr0kugbFTf53O9L',$,'B',$,$,#61,#64,$);
#61=IFCLOCALPLACEMENT(#11,#62);
#62=IFCAXIS2PLACEMENT3D(#63,$,$);
#63=IFCCARTESIANPOINT((2000.,0.,0.));
#64=IFCPRODUCTDEFINITIONSHAPE($,$,(#65));
#65=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#66));
#66=IFCEXTRUDEDAREASOLID(#67,$,#24,3000.);
#67=IFCISHAPEPROFILEDEF(.AREA.,'Bad',#30,100.,200.,5.6,8.5,500.,$,$);
#70=IFCCOLUMN('6XvctVUKr0kugbFTf53O9L',$,'C',$,$,#71,#74,$);
#71=IFCLOCALPLACEMENT(#11,#72);
#72=IFCAXIS2PLACEMENT3D(#73,$,$);
#73=IFCCARTESIANPOINT((4000.,0.,0.));
#74=IFCPRODUCTDEFINITIONSHAPE($,$,(#75));
#75=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#76));
#76=IFCEXTRUDEDAREASOLID(#77,$,#24,3000.);
#77=IFCLSHAPEPROFILEDEF(.AREA.,$,#30,100.,100.,150.,$,$,$);
#80=IFCBEAM('7XvctVUKr0kugbFTf53O9L',$,'D',$,$,#81,#84,$);
#81=IFCLOCALPLACEMENT(#11,#82);
#82=IFCAXIS2PLACEMENT3D(#83,$,$);
#83=IFCCARTESIANPOINT((0.,5000.,3000.));
#84=IFCPRODUCTDEFINITIONSHAPE($,$,(#85));
#85=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#86));
#86=IFCEXTRUDEDAREASOLID(#87,#88,#24,4000.);
#87=IFCUSHAPEPROFILEDEF(.AREA.,'U100',#30,100.,50.,6.,8.5,$,$,$);
#88=IFCAXIS2PLACEMENT3D(#13,#89,#90);
#89=IFCDIRECTION((1.,0.,0.));
#90=IFCDIRECTION((0.,1.,0.));
ENDSEC;
END-ISO-10303-21;
";
        let (p, report) = import(text, Project::from_template()).unwrap();
        let shape_of = |name: &str| {
            let e = p.elements.values().find(|e| p.type_name(e.type_id) == name).unwrap();
            let ty = p.element_type(e.type_id).unwrap();
            let (TypeData::Column { profile: Some(id), .. } | TypeData::Beam { profile: Some(id), .. }) = &ty.data
            else {
                panic!("{name} has no profile: {:?}", ty.data);
            };
            p.profile(*id).unwrap().clone()
        };
        assert_eq!(report.imported.get(&Category::Column), Some(&2), "{report:?}");
        assert_eq!(report.imported.get(&Category::Beam), Some(&1), "{report:?}");
        let a = shape_of("A");
        assert_eq!(a.name, "IPE200");
        let ProfileShape::I { d, bf, tw, tf, r } = a.shape else { panic!("{:?}", a.shape) };
        for (got, want) in [(d, 0.2), (bf, 0.1), (tw, 0.0056), (tf, 0.0085), (r, 0.012)] {
            assert!((got - want).abs() < 1e-9, "{got} vs {want}");
        }
        let b = shape_of("B");
        assert_eq!(b.name, "Bad");
        assert!(matches!(b.shape, ProfileShape::Sketch(_)), "{:?}", b.shape);
        // Its drawn outline crosses itself: no profile, a component of its class.
        let c = p.elements.values().find(|e| p.type_name(e.type_id) == "C").unwrap();
        assert_eq!(c.category(), Category::Component);
        assert_eq!(report.generic.get("IfcColumn"), Some(&1), "{report:?}");
        let d = shape_of("D");
        assert_eq!(d.name, "U100");
        assert!(matches!(d.shape, ProfileShape::Channel { .. }), "{:?}", d.shape);
        let (_, beam) = p
            .elements
            .values()
            .find_map(|e| match &e.kind {
                ElementKind::Beam(b) => Some((e, b.clone())),
                _ => None,
            })
            .unwrap();
        assert!(beam.start.distance(DVec2::new(0.0, 5.0)) < 1e-9, "{}", beam.start);
        assert!(beam.end.distance(DVec2::new(4.0, 5.0)) < 1e-9, "{}", beam.end);
        let mut again = p.clone();
        assert_eq!(again.repair(), 0);
    }

    /// An outline of more points than a sketch holds (or with a point that is
    /// not a number) gives no section: the column stays a component.
    #[test]
    fn oversized_outlines_become_components() {
        let n = MAX_SKETCH_POINTS + 1;
        let circle: Vec<DVec2> =
            (0..n).map(|i| DVec2::from_angle(std::f64::consts::TAU * i as f64 / n as f64) * 0.1).collect();
        assert!(loose_section(&circle, &[]).is_none());
        let mut bad = circle[..8].to_vec();
        bad[3] = DVec2::new(f64::NAN, 0.0);
        assert!(loose_section(&bad, &[]).is_none());

        // A column extruded from that outline keeps its exact mesh.
        let mut text = String::from(
            "ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Foreign',$,$,$,$,(#2),#3);
#3=IFCUNITASSIGNMENT((#4));
#4=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#10=IFCBUILDINGSTOREY('1XvctVUKr0kugbFTf53O9L',$,'Level 1',$,$,#11,$,$,.ELEMENT.,0.);
#11=IFCLOCALPLACEMENT($,#12);
#12=IFCAXIS2PLACEMENT3D(#13,$,$);
#13=IFCCARTESIANPOINT((0.,0.,0.));
#24=IFCDIRECTION((0.,0.,1.));
#40=IFCRELCONTAINEDINSPATIALSTRUCTURE('3XvctVUKr0kugbFTf53O9L',$,$,$,(#50),#10);
#50=IFCCOLUMN('4XvctVUKr0kugbFTf53O9L',$,'Round',$,$,#11,#51,$);
#51=IFCPRODUCTDEFINITIONSHAPE($,$,(#52));
#52=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#53));
#53=IFCEXTRUDEDAREASOLID(#54,$,#24,3.);
#54=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,'Many',#55);
",
        );
        let ids: Vec<String> = (0..n).map(|i| format!("#{}", 100 + i)).collect();
        text.push_str(&format!("#55=IFCPOLYLINE(({},#100));\n", ids.join(",")));
        for (i, p) in circle.iter().enumerate() {
            text.push_str(&format!("#{}=IFCCARTESIANPOINT(({:.9},{:.9}));\n", 100 + i, p.x, p.y));
        }
        text.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        let (p, report) = import(&text, Project::from_template()).unwrap();
        assert_eq!(report.imported.get(&Category::Column), None, "{report:?}");
        assert_eq!(report.imported.get(&Category::Component), Some(&1), "{report:?}");
        assert!(p.profiles.values().all(|f| f.name != "Many"));
    }

    /// Slabs drawn with circles and arcs come in as floors with the curves as
    /// their sketch, so areas and volumes are those of the true curves (#264):
    /// a round slab with a square shaft, a slab with a half-round end (a
    /// composite curve with a trimmed circle) and a square slab with a round
    /// void.
    #[test]
    fn round_slabs_keep_their_curves() {
        use std::f64::consts::PI;
        let text = "ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Round',$,$,$,$,$,#3);
#3=IFCUNITASSIGNMENT((#4));
#4=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#10=IFCBUILDINGSTOREY('1XvctVUKr0kugbFTf53O9L',$,'Level 1',$,$,#11,$,$,.ELEMENT.,0.);
#11=IFCLOCALPLACEMENT($,#12);
#12=IFCAXIS2PLACEMENT3D(#13,$,$);
#13=IFCCARTESIANPOINT((0.,0.,0.));
#14=IFCDIRECTION((0.,0.,1.));
#15=IFCAXIS2PLACEMENT3D(#16,$,$);
#16=IFCCARTESIANPOINT((0.,0.,-200.));
#100=IFCSLAB('2XvctVUKr0kugbFTf53O9L',$,'Round slab',$,$,#101,#102,'1',.FLOOR.);
#101=IFCLOCALPLACEMENT(#11,#12);
#102=IFCPRODUCTDEFINITIONSHAPE($,$,(#103));
#103=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#104));
#104=IFCEXTRUDEDAREASOLID(#105,#15,#14,200.);
#105=IFCCIRCLEPROFILEDEF(.AREA.,$,#106,2000.);
#106=IFCAXIS2PLACEMENT2D(#107,$);
#107=IFCCARTESIANPOINT((5000.,5000.));
#110=IFCOPENINGELEMENT('3XvctVUKr0kugbFTf53O9L',$,'Shaft',$,$,#111,#112,$,.OPENING.);
#111=IFCLOCALPLACEMENT(#101,#12);
#112=IFCPRODUCTDEFINITIONSHAPE($,$,(#113));
#113=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#114));
#114=IFCEXTRUDEDAREASOLID(#115,#15,#14,200.);
#115=IFCRECTANGLEPROFILEDEF(.AREA.,$,#106,400.,400.);
#118=IFCRELVOIDSELEMENT('4XvctVUKr0kugbFTf53O9L',$,$,$,#100,#110);
#200=IFCSLAB('5XvctVUKr0kugbFTf53O9L',$,'Half-round end',$,$,#201,#202,'2',.FLOOR.);
#201=IFCLOCALPLACEMENT(#11,#203);
#203=IFCAXIS2PLACEMENT3D(#204,$,$);
#204=IFCCARTESIANPOINT((20000.,0.,0.));
#202=IFCPRODUCTDEFINITIONSHAPE($,$,(#205));
#205=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#206));
#206=IFCEXTRUDEDAREASOLID(#207,#15,#14,200.);
#207=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#208);
#208=IFCCOMPOSITECURVE((#209,#210,#211),.F.);
#209=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#212);
#210=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#213);
#211=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#214);
#212=IFCPOLYLINE((#220,#221));
#213=IFCTRIMMEDCURVE(#215,(#221),(#222),.T.,.CARTESIAN.);
#214=IFCPOLYLINE((#222,#223,#220));
#215=IFCCIRCLE(#216,2000.);
#216=IFCAXIS2PLACEMENT2D(#217,$);
#217=IFCCARTESIANPOINT((4000.,2000.));
#220=IFCCARTESIANPOINT((0.,0.));
#221=IFCCARTESIANPOINT((4000.,0.));
#222=IFCCARTESIANPOINT((4000.,4000.));
#223=IFCCARTESIANPOINT((0.,4000.));
#300=IFCSLAB('6XvctVUKr0kugbFTf53O9L',$,'Round void',$,$,#301,#302,'3',.FLOOR.);
#301=IFCLOCALPLACEMENT(#11,#303);
#303=IFCAXIS2PLACEMENT3D(#304,$,$);
#304=IFCCARTESIANPOINT((40000.,0.,0.));
#302=IFCPRODUCTDEFINITIONSHAPE($,$,(#305));
#305=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#306));
#306=IFCEXTRUDEDAREASOLID(#307,#15,#14,200.);
#307=IFCARBITRARYPROFILEDEFWITHVOIDS(.AREA.,$,#308,(#309));
#308=IFCPOLYLINE((#220,#310,#311,#312,#220));
#309=IFCCIRCLE(#313,1000.);
#310=IFCCARTESIANPOINT((6000.,0.));
#311=IFCCARTESIANPOINT((6000.,6000.));
#312=IFCCARTESIANPOINT((0.,6000.));
#313=IFCAXIS2PLACEMENT2D(#314,$);
#314=IFCCARTESIANPOINT((3000.,3000.));
#500=IFCRELCONTAINEDINSPATIALSTRUCTURE('CXvctVUKr0kugbFTf53O9L',$,$,$,(#100,#200,#300),#10);
ENDSEC;
END-ISO-10303-21;
";
        let (p, report) = import(text, Project::from_template()).unwrap();
        let geom = cc_geom::ModelGeometry::build(&p);
        let floors: Vec<(cc_core::ElementId, &cc_core::Floor)> = p
            .elements
            .values()
            .filter_map(|e| match &e.kind {
                ElementKind::Floor(f) => Some((e.id, f)),
                _ => None,
            })
            .collect();
        assert_eq!(floors.len(), 3, "{report:?}");
        let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
        let at = |x: f64| floors.iter().find(|(_, f)| f.boundary.iter().all(|q| q.x >= x - 1e-6 && q.x < x + 15.0));
        // Round: pi R^2 less the 0.4 x 0.4 shaft, 0.2 m thick.
        let (id, round) = at(0.0).unwrap();
        let sketch = round.sketch.as_ref().expect("the circle is kept");
        assert!(
            sketch
                .curves
                .iter()
                .any(|c| matches!(c, cc_core::sketch::SketchCurve::Circle { radius, .. } if near(*radius, 2.0)))
        );
        assert_eq!(round.openings.len(), 1);
        let area = PI * 4.0 - 0.16;
        assert!(near(round.area(), area), "{} != {area}", round.area());
        assert!(near(geom.volume(*id), area * 0.2), "{}", geom.volume(*id));
        // The outline follows the circle as closely as natives do (well inside 1 %).
        let drawn = polygon::area(&round.boundary) - 0.16;
        assert!((area - drawn) / area < 1e-3, "{drawn}");
        // Half-round end: 4 x 4 plus half a circle of radius 2.
        let (id, half) = at(20.0).unwrap();
        assert!(half.sketch.as_ref().is_some_and(|s| s.is_curved()));
        let area = 16.0 + 2.0 * PI;
        assert!(near(half.area(), area), "{} != {area}", half.area());
        assert!(near(geom.volume(*id), area * 0.2), "{}", geom.volume(*id));
        // Square with a round void.
        let (id, voided) = at(40.0).unwrap();
        assert_eq!(voided.openings.len(), 1);
        let area = 36.0 - PI;
        assert!(near(voided.area(), area), "{} != {area}", voided.area());
        assert!(near(geom.volume(*id), area * 0.2), "{}", geom.volume(*id));
    }

    /// Geometry a hostile file can throw at the importer: cyclic booleans and
    /// mapped items, out-of-range and negative indices, absurd knot
    /// multiplicities and angles, empty paths. Nothing may panic or hang.
    #[test]
    fn files_past_the_size_limit_are_refused_unread() {
        let dir = std::env::temp_dir().join(format!("ccad-ifc-size-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("huge.ifc");
        std::fs::File::create(&path).unwrap().set_len(MAX_IFC_BYTES + 1).unwrap();
        let err = import_file(&path, Project::empty()).unwrap_err();
        std::fs::remove_dir_all(&dir).ok();
        assert!(matches!(&err, ImportError::Io(e) if e.kind() == std::io::ErrorKind::FileTooLarge), "{err}");
    }

    /// Complex properties that contain themselves, or that fan out through
    /// distinct ones, are read once and within a budget, not 10^8 times.
    #[test]
    fn complex_properties_that_cycle_or_fan_out_are_bounded() {
        // #100 holds itself ten times; #101..#108 each hold the next ten times.
        let mut props = format!("#100=IFCCOMPLEXPROPERTY('Self',$,'u',({}));", ["#100"; 10].join(","));
        for k in 101..=108 {
            let inner = if k == 108 { "#109".to_string() } else { vec![format!("#{}", k + 1); 10].join(",") };
            props += &format!("#{k}=IFCCOMPLEXPROPERTY('Fan',$,'u',({inner}));");
        }
        props += "#109=IFCPROPERTYSINGLEVALUE('Leaf',$,IFCLABEL('x'),$);";
        let text = format!(
            "ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,$,#3);
#3=IFCUNITASSIGNMENT((#4));
#4=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#10=IFCBUILDINGSTOREY('1XvctVUKr0kugbFTf53O9L',$,'Level 1',$,$,#11,$,$,.ELEMENT.,0.);
#11=IFCLOCALPLACEMENT($,#12);
#12=IFCAXIS2PLACEMENT3D(#13,$,$);
#13=IFCCARTESIANPOINT((0.,0.,0.));
#20=IFCFURNISHINGELEMENT('5XvctVUKr0kugbFTf53O9L',$,'Chair',$,$,#11,#30,$);
#30=IFCPRODUCTDEFINITIONSHAPE($,$,(#31));
#31=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#32));
#32=IFCBLOCK(#12,1.,1.,1.);
#40=IFCRELCONTAINEDINSPATIALSTRUCTURE('3XvctVUKr0kugbFTf53O9L',$,$,$,(#20),#10);
#50=IFCPROPERTYSET('6XvctVUKr0kugbFTf53O9L',$,'Pset_Bomb',$,(#100,#101));
#51=IFCRELDEFINESBYPROPERTIES('7XvctVUKr0kugbFTf53O9L',$,$,$,(#20),#50);
{props}
ENDSEC;
END-ISO-10303-21;
"
        );
        let (p, _) = import(&text, Project::from_template()).unwrap();
        fn count(props: &[cc_core::properties::Property]) -> usize {
            props
                .iter()
                .map(|p| match &p.value {
                    cc_core::properties::PropertyValue::Complex { properties, .. } => 1 + count(properties),
                    _ => 1,
                })
                .sum()
        }
        let sets: Vec<_> = p
            .elements
            .values()
            .filter_map(|e| match &e.kind {
                ElementKind::Component(c) => Some(&c.properties),
                _ => None,
            })
            .flatten()
            .filter(|s| s.name == "Pset_Bomb")
            .collect();
        let [set] = sets.as_slice() else { panic!("{sets:?}") };
        let total = count(&set.properties);
        assert!(total <= MAX_SET_PROPERTIES, "{total}");
        // The self-holding property is read once, with nothing inside.
        let first = set.properties.first().unwrap();
        assert!(
            matches!(&first.value, cc_core::properties::PropertyValue::Complex { properties, .. } if properties.is_empty())
        );
    }

    #[test]
    fn hostile_geometry_is_survived() {
        let text = "ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Hostile',$,$,$,$,$,$);
#2=IFCLOCALPLACEMENT($,#3);
#3=IFCAXIS2PLACEMENT3D(#4,$,$);
#4=IFCCARTESIANPOINT((0.,0.,0.));
#10=IFCPIPESEGMENT('1',$,'A',$,$,#2,#11,$,.RIGIDSEGMENT.);
#11=IFCPRODUCTDEFINITIONSHAPE($,$,(#12));
#12=IFCSHAPEREPRESENTATION($,'Body','X',(#13,#14,#15,#16,#17,#18,#19,#20,#21,#22));
#13=IFCBOOLEANRESULT(.UNION.,#13,#13);
#14=IFCMAPPEDITEM(#30,$);
#30=IFCREPRESENTATIONMAP(#3,#31);
#31=IFCSHAPEREPRESENTATION($,'Body','X',(#14));
#15=IFCPOLYGONALFACESET(#32,$,(#33,#34),(0,-5,99999999999));
#32=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(1.,0.,0.),(0.,1.,0.)));
#33=IFCINDEXEDPOLYGONALFACE((1,2,3));
#34=IFCINDEXEDPOLYGONALFACE((-1,0,1E300));
#16=IFCSWEPTDISKSOLID(#35,10.,$,$,$);
#35=IFCBSPLINECURVEWITHKNOTS(3,(#4,#4,#4,#4),.UNSPECIFIED.,.F.,.F.,(1E30,1E30),(0.,1.),.UNSPECIFIED.);
#17=IFCREVOLVEDAREASOLID(#36,$,#37,1E308);
#36=IFCCIRCLEPROFILEDEF(.AREA.,$,$,-5.);
#37=IFCAXIS1PLACEMENT(#4,$);
#18=IFCEXTRUDEDAREASOLID(#38,$,$,1.);
#38=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#39);
#39=IFCCOMPOSITECURVE((#40),.F.);
#40=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#39);
#19=IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE.,#21,#41);
#41=IFCHALFSPACESOLID(#42,.T.);
#42=IFCPLANE($);
#20=IFCSWEPTDISKSOLID(#43,1.,$,$,$);
#43=IFCPOLYLINE((#4,#4,#4));
#21=IFCBLOCK($,1.,1.,1.);
#22=IFCFACETEDBREP(#44);
#44=IFCCLOSEDSHELL((#45));
#45=IFCFACE((#46));
#46=IFCFACEOUTERBOUND(#47,.T.);
#47=IFCEDGELOOP((#48));
#48=IFCORIENTEDEDGE(*,*,#49,.T.);
#49=IFCEDGECURVE(#50,#50,#51,.T.);
#50=IFCVERTEXPOINT(#4);
#51=IFCCIRCLE(#3,0.);
ENDSEC;
END-ISO-10303-21;
";
        let (p, report) = import(text, Project::from_template()).unwrap();
        // The block (clipped to nothing or not) still makes an object.
        assert_eq!(report.imported_total(), p.elements.len());
        assert!(report.generic.contains_key("IfcPipeSegment") || !report.skipped.is_empty());
    }
}
