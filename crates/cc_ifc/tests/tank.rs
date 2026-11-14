//! The parts of a circular tank as another program writes them (IFC2X3, no
//! axes, no type objects, everything in world coordinates): columns whose
//! section is a piece of a ring, arc beams drawn as a band between two arcs,
//! a round raft, piles, and a set of over a thousand ties with its part of
//! the structure in a property. Each becomes the native element that holds
//! it exactly.

// Test helpers outside #[test] functions may fail loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cc_core::pile::PileKind;
use cc_core::structural_profile::ProfileShape;
use cc_core::{Category, ElementKind, FoundationShape, Project, TypeData};
use cc_geom::ModelGeometry;
use cc_ifc::import;
use std::f64::consts::PI;

/// A STEP file being written: entities numbered as they are added.
struct File {
    lines: Vec<String>,
    products: Vec<usize>,
}

impl File {
    fn new() -> Self {
        let lines = [
            "IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Tank',$,$,$,$,(#2),#3)",
            "IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#7,$)",
            "IFCUNITASSIGNMENT((#4))",
            "IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)",
            "IFCBUILDINGSTOREY('1XvctVUKr0kugbFTf53O9L',$,'Ground',$,$,#6,$,$,.ELEMENT.,0.)",
            "IFCLOCALPLACEMENT($,#7)",
            "IFCAXIS2PLACEMENT3D(#8,$,$)",
            "IFCCARTESIANPOINT((0.,0.,0.))",
            "IFCDIRECTION((0.,0.,1.))",
            "IFCAXIS2PLACEMENT2D(#11,$)",
            "IFCCARTESIANPOINT((0.,0.))",
        ];
        Self { lines: lines.iter().map(|l| l.to_string()).collect(), products: Vec::new() }
    }

    fn add(&mut self, entity: String) -> usize {
        self.lines.push(entity);
        self.lines.len()
    }

    fn point2(&mut self, x: f64, y: f64) -> usize {
        self.add(format!("IFCCARTESIANPOINT(({x:.6},{y:.6}))"))
    }

    /// The solid of `profile` extruded `depth` up from `z`, as a product's shape.
    fn extruded(&mut self, profile: usize, z: f64, depth: f64) -> usize {
        let at = self.add(format!("IFCCARTESIANPOINT((0.,0.,{z:.6}))"));
        let position = self.add(format!("IFCAXIS2PLACEMENT3D(#{at},$,$)"));
        let solid = self.add(format!("IFCEXTRUDEDAREASOLID(#{profile},#{position},#9,{depth:.6})"));
        let rep = self.add(format!("IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#{solid}))"));
        self.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}))"))
    }

    /// The piece of the ring between `r_in` and `r_out` from the angle `from` to `to` (degrees),
    /// as a closed profile of two trimmed circles and two radial lines.
    fn sector(&mut self, r_in: f64, r_out: f64, from: f64, to: f64) -> usize {
        let (a, b) = (from.to_radians(), to.to_radians());
        let mut at = |r: f64, t: f64| self.point2(r * t.cos(), r * t.sin());
        let (p0, p1, p2, p3) = (at(r_out, a), at(r_out, b), at(r_in, b), at(r_in, a));
        let outer = self.add(format!("IFCCIRCLE(#10,{r_out:.6})"));
        let inner = self.add(format!("IFCCIRCLE(#10,{r_in:.6})"));
        let trim = |c: usize, s: (usize, f64), e: (usize, f64), same: &str| {
            format!(
                "IFCTRIMMEDCURVE(#{c},(#{},IFCPARAMETERVALUE({:.6})),(#{},IFCPARAMETERVALUE({:.6})),.{same}.,.CARTESIAN.)",
                s.0, s.1, e.0, e.1
            )
        };
        let parts = [
            self.add(trim(outer, (p0, a), (p1, b), "T")),
            self.add(format!("IFCPOLYLINE((#{p1},#{p2}))")),
            self.add(trim(inner, (p2, b), (p3, a), "F")),
            self.add(format!("IFCPOLYLINE((#{p3},#{p0}))")),
        ];
        let segments: Vec<String> = parts
            .iter()
            .map(|c| format!("#{}", self.add(format!("IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#{c})"))))
            .collect();
        let curve = self.add(format!("IFCCOMPOSITECURVE(({}),.F.)", segments.join(",")));
        self.add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,'ring sector',#{curve})"))
    }

    /// A product of `class` named `name` and tagged `tag`, with `rest` after its tag.
    fn product(&mut self, class: &str, name: &str, tag: &str, shape: usize, rest: &str) -> usize {
        let guid = format!("2{:021}", self.lines.len());
        let id = self.add(format!("{class}('{guid}',$,'{name}',$,$,#6,#{shape},'{tag}'{rest})"));
        self.products.push(id);
        id
    }

    fn text(mut self) -> String {
        let all: Vec<String> = self.products.iter().map(|p| format!("#{p}")).collect();
        let contained =
            format!("IFCRELCONTAINEDINSPATIALSTRUCTURE('3XvctVUKr0kugbFTf53O9L',$,$,$,({}),#5)", all.join(","));
        self.add(contained);
        let mut out = String::from("ISO-10303-21;\nHEADER;\nFILE_SCHEMA(('IFC2X3'));\nENDSEC;\nDATA;\n");
        for (i, line) in self.lines.iter().enumerate() {
            out.push_str(&format!("#{}={line};\n", i + 1));
        }
        out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        out
    }
}

/// Six columns around a ring of radii 2.2 and 2.4 m, 20 degrees each, with an
/// arc beam of 40 degrees between each two; a raft of 3.2 m radius; a pile;
/// and 1200 ties in one set.
fn tank() -> String {
    let mut f = File::new();
    for k in 0..6 {
        let mid = 60.0 * k as f64;
        let profile = f.sector(2.2, 2.4, mid - 10.0, mid + 10.0);
        let shape = f.extruded(profile, 0.0, 3.0);
        f.product("IFCCOLUMN", "Sector column", &format!("P{}", k + 1), shape, "");
        let profile = f.sector(2.2, 2.4, mid + 10.0, mid + 50.0);
        let shape = f.extruded(profile, 2.55, 0.45);
        f.product("IFCBEAM", "Ring beam 20x45", &format!("V{}", k + 1), shape, "");
    }
    let circle = f.add("IFCCIRCLEPROFILEDEF(.AREA.,$,#10,3.2)".into());
    let shape = f.extruded(circle, -0.5, 0.5);
    f.product("IFCSLAB", "Raft", "R1", shape, ",.BASESLAB.");
    let circle = f.add("IFCCIRCLEPROFILEDEF(.AREA.,$,#10,0.15)".into());
    let shape = f.extruded(circle, -8.5, 8.0);
    f.product("IFCPILE", "Bored pile 30", "E1", shape, ",.FRICTION.,.CAST_IN_PLACE.");
    // Ties 0.2 m long up a wall, each a disk swept along two points.
    let disks: Vec<String> = (0..1200)
        .map(|i| {
            let (angle, z) = ((i % 60) as f64 * 6.0_f64.to_radians(), (i / 60) as f64 * 0.15);
            let a = f.add(format!("IFCCARTESIANPOINT(({:.6},{:.6},{z:.6}))", 2.2 * angle.cos(), 2.2 * angle.sin()));
            let b = f.add(format!("IFCCARTESIANPOINT(({:.6},{:.6},{z:.6}))", 2.4 * angle.cos(), 2.4 * angle.sin()));
            let line = f.add(format!("IFCPOLYLINE((#{a},#{b}))"));
            format!("#{}", f.add(format!("IFCSWEPTDISKSOLID(#{line},0.0025,$,$,$)")))
        })
        .collect();
    let rep = f.add(format!("IFCSHAPEREPRESENTATION(#2,'Body','AdvancedSweptSolid',({}))", disks.join(",")));
    let shape = f.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}))"));
    let guid = format!("2{:021}", f.lines.len());
    let bar = f.add(format!(
        "IFCREINFORCINGBAR('{guid}',$,'Wall ties',$,$,#6,#{shape},$,'CA-60',0.005,0.00002,0.2,.LIGATURE.,.TEXTURED.)"
    ));
    f.products.push(bar);
    // "Partição", as STEP writes it.
    let partition =
        f.add("IFCPROPERTYSINGLEVALUE('Parti\\X2\\00E7\\X0\\\\X2\\00E3\\X0\\o',$,IFCLABEL('WALL'),$)".into());
    let set = f.add(format!("IFCPROPERTYSET('4AvctVUKr0kugbFTf53O9L',$,'Quantities',$,(#{partition}))"));
    f.add(format!("IFCRELDEFINESBYPROPERTIES('4BvctVUKr0kugbFTf53O9L',$,$,$,(#{bar}),#{set})"));
    f.text()
}

#[test]
fn a_tank_from_another_program_becomes_native_elements() {
    let (p, report) = import(&tank(), Project::from_template()).unwrap();
    assert!(report.generic.is_empty(), "nothing is left as a generic object: {report:?}");
    let geom = ModelGeometry::build(&p);

    // The six columns are one annular sector profile and one type, each turned to its place on the ring.
    let columns: Vec<_> = p.elements_of(Category::Column).collect();
    assert_eq!(columns.len(), 6);
    let types: std::collections::BTreeSet<_> = columns.iter().map(|e| e.type_id).collect();
    assert_eq!(types.len(), 1, "one type for the six columns");
    let data = &p.element_type(columns[0].type_id).unwrap().data;
    let shape = &p.type_profile_def(data).expect("a profile").shape;
    assert_eq!(*shape, ProfileShape::AnnularSector { r_in: 2.2, r_out: 2.4, sweep: 20.0 });
    let sectors = p.profiles.values().filter(|d| matches!(d.shape, ProfileShape::AnnularSector { .. })).count();
    assert_eq!(sectors, 1, "one profile");
    let area = 20f64.to_radians() / 2.0 * (2.4f64.powi(2) - 2.2f64.powi(2));
    for (k, e) in columns.iter().enumerate() {
        let ElementKind::Column(c) = &e.kind else { panic!("a column") };
        // Its body lies on the ring: every corner of its outline between the two radii.
        let solid = geom.solids.iter().find(|s| s.element == e.id).expect("a body");
        let (lo, hi) = solid.faces.iter().flat_map(|f| f.verts.iter()).fold((f64::INFINITY, 0.0_f64), |(lo, hi), q| {
            let r = q.truncate().length();
            (lo.min(r), hi.max(r))
        });
        assert!(lo > 2.2 - 0.01 && hi < 2.4 + 1e-6, "column {k}: {lo}..{hi}");
        // ... around the bisector the file drew it on.
        let bisector = glam::DVec2::from_angle((60.0 * k as f64).to_radians());
        assert!(c.position.normalize().distance(bisector) < 1e-6, "column {k} at {}", c.position);
        assert!((geom.volume(e.id) - area * 3.0).abs() < 1e-6, "{}", geom.volume(e.id));
    }

    // The six beams are arc beams of 20 x 45 cm on the ring's middle circle, 40 degrees each.
    let beams: Vec<_> = p.elements_of(Category::Beam).collect();
    assert_eq!(beams.len(), 6);
    for e in &beams {
        let ElementKind::Beam(b) = &e.kind else { panic!("a beam") };
        let arc = b.arc().expect("an arc beam");
        assert!((arc.radius - 2.3).abs() < 1e-6 && (arc.sweep.abs() - 40f64.to_radians()).abs() < 1e-6, "{arc:?}");
        assert!(arc.center.length() < 1e-6);
        let Some(TypeData::Beam { width, height, .. }) = p.element_type(e.type_id).map(|t| &t.data) else { panic!() };
        assert!((width - 0.2).abs() < 1e-6 && (height - 0.45).abs() < 1e-6);
    }
    let one = &geom.beams[&beams[0].id];
    assert!((one.z_range.0 - 2.55).abs() < 1e-6 && (one.z_range.1 - 3.0).abs() < 1e-6, "{:?}", one.z_range);

    // The raft keeps its circle: the area and the volume are the disk's, not its chords'.
    let raft = p.elements_of(Category::Foundation).next().expect("the raft");
    let ElementKind::Foundation(f) = &raft.kind else { panic!("a foundation") };
    assert!(matches!(f.shape, FoundationShape::Slab { .. }) && f.sketch.as_ref().is_some_and(|s| s.is_curved()));
    assert!((geom.foundations[&raft.id].area() - PI * 3.2 * 3.2).abs() < 1e-9);
    assert!((geom.volume(raft.id) - PI * 3.2 * 3.2 * 0.5).abs() < 1e-9, "{}", geom.volume(raft.id));

    // The pile is cast in place: bored, whatever way it carries its load.
    let pile = p.elements_of(Category::Pile).next().expect("the pile");
    let Some(TypeData::Pile { kind, .. }) = p.element_type(pile.type_id).map(|t| &t.data) else { panic!() };
    assert_eq!(*kind, PileKind::Bored);

    // The 1200 ties are one reinforcement set, in the part of the structure the file says.
    let ties = p.elements_of(Category::Rebar).next().expect("the ties");
    let ElementKind::ShapedRebar(r) = &ties.kind else { panic!("shaped bars") };
    assert_eq!((r.bars.len(), r.partition.as_str()), (1200, "WALL"));
    let set = &geom.rebar[&ties.id];
    // The file rounds its points to the micrometer.
    assert!((set.total_length() - 1200.0 * 0.2).abs() < 5e-3, "{}", set.total_length());
}

/// Outlines that only look like a ring sector stay what they were.
#[test]
fn only_true_ring_sectors_are_read_as_such() {
    // A band whose inner arc turns less than its outer one: no ring sector, so the column keeps
    // its outline as a sketch and the beam, with no axis, its body.
    let mut f = File::new();
    let a = 10f64.to_radians();
    let mut at = |r: f64, t: f64| f.point2(r * t.cos(), r * t.sin());
    let (p0, p1, p2, p3) = (at(2.4, -a), at(2.4, a), at(2.2, a * 0.5), at(2.2, -a * 0.5));
    let outer = f.add("IFCCIRCLE(#10,2.4)".into());
    let inner = f.add("IFCCIRCLE(#10,2.2)".into());
    let parts = [
        f.add(format!("IFCTRIMMEDCURVE(#{outer},(#{p0}),(#{p1}),.T.,.CARTESIAN.)")),
        f.add(format!("IFCPOLYLINE((#{p1},#{p2}))")),
        f.add(format!("IFCTRIMMEDCURVE(#{inner},(#{p2}),(#{p3}),.F.,.CARTESIAN.)")),
        f.add(format!("IFCPOLYLINE((#{p3},#{p0}))")),
    ];
    let segments: Vec<String> = parts
        .iter()
        .map(|c| format!("#{}", f.add(format!("IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#{c})"))))
        .collect();
    let curve = f.add(format!("IFCCOMPOSITECURVE(({}),.F.)", segments.join(",")));
    let profile = f.add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,'wedge',#{curve})"));
    let shape = f.extruded(profile, 0.0, 3.0);
    f.product("IFCCOLUMN", "Wedge column", "P1", shape, "");
    let shape = f.extruded(profile, 2.55, 0.45);
    f.product("IFCBEAM", "Wedge beam", "V1", shape, "");
    let (p, _) = import(&f.text(), Project::from_template()).unwrap();
    assert!(p.profiles.values().all(|d| !matches!(d.shape, ProfileShape::AnnularSector { .. })));
    assert!(p.elements_of(Category::Beam).all(|e| !matches!(&e.kind, ElementKind::Beam(b) if b.arc().is_some())));
    assert_eq!(p.elements_of(Category::Column).count() + p.elements_of(Category::Component).count(), 2);
}
