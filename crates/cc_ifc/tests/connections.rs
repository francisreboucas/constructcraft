//! Connections export as element assemblies of plates and mechanical
//! fasteners, tied to their members; importing them keeps the existing
//! fallback (no native round trip).

use cc_core::connection::{Connection, ConnectionKind, MemberEnd};
use cc_core::{
    Beam, Category, Column, Document, EditCtx, Element, ElementId, ElementKind, ElementTypeId, LevelId, Project,
    WallTop,
};
use cc_ifc::{ExportOptions, export, import};
use glam::DVec2;

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

/// Two columns 5 m apart with a beam between them and a beam from its
/// middle: base plates under the columns, end plates at the first beam's
/// ends and a hanger under the second beam's start.
pub(crate) fn connections_project() -> Project {
    let mut doc = Document::new(Project::from_template());
    let level = doc.project.levels[0].id;
    doc.edit("connections", |e| {
        let column_t = e.project().default_type(Category::Column).unwrap();
        let beam_t = e.project().default_type(Category::Beam).unwrap();
        let ty = ElementTypeId(e.alloc_id());
        e.add_type(cc_core::connection::host_type(ty, "Connection".into()));
        let column = |x: f64| {
            ElementKind::Column(Column {
                position: DVec2::new(x, 0.0),
                rotation_deg: 0.0,
                base_offset: 0.0,
                height: 3.0,
                top: WallTop::Unconnected,
                structural: true,
                base_support: Default::default(),
            })
        };
        let a = add(e, level, column_t, column(0.0));
        let b = add(e, level, column_t, column(5.0));
        let main = add(e, level, beam_t, ElementKind::Beam(Beam::new(DVec2::ZERO, DVec2::new(5.0, 0.0), 3.0)));
        let tee = add(e, level, beam_t, ElementKind::Beam(Beam::new(DVec2::new(2.5, 0.0), DVec2::new(2.5, 3.0), 3.0)));
        for (member, end, kind) in [
            (a, MemberEnd::Base, ConnectionKind::BasePlate),
            (b, MemberEnd::Base, ConnectionKind::BasePlate),
            (main, MemberEnd::Start, ConnectionKind::EndPlate),
            (main, MemberEnd::End, ConnectionKind::EndPlate),
            (tee, MemberEnd::Start, ConnectionKind::Hanger),
        ] {
            add(e, level, ty, ElementKind::Connection(Connection::new(member, end, kind)));
        }
    });
    doc.project
}

#[test]
fn connections_export_as_assemblies_of_plates_and_fasteners() {
    let project = connections_project();
    let geom = cc_geom::ModelGeometry::build(&project);
    assert!(geom.connections.values().all(|g| g.warning.is_none()), "{:?}", geom.connections);
    let text = export(&project, &ExportOptions::default());
    let count = |s: &str| text.matches(s).count();
    let plates: usize = geom.connections.values().map(|g| g.plates.len()).sum();
    let bolts: usize = geom.connections.values().map(|g| g.bolt_count()).sum();
    let anchors: usize =
        geom.connections.values().filter(|g| g.kind == ConnectionKind::BasePlate).map(|g| g.bolt_count()).sum();
    assert_eq!(count("IFCELEMENTASSEMBLY("), 5);
    assert_eq!(count("IFCRELCONNECTSELEMENTS("), 5);
    assert_eq!(count("IFCPLATE("), plates);
    assert_eq!(count("IFCMECHANICALFASTENER("), bolts);
    let anchor_lines =
        text.lines().filter(|l| l.contains("IFCMECHANICALFASTENER(") && l.contains(".ANCHORBOLT.")).count();
    assert_eq!(anchor_lines, anchors);
    assert_eq!(count("IFCPLATETYPE("), 1, "one plate type per connection type");
    assert_eq!(count("IFCMECHANICALFASTENERTYPE("), 2, "bolts and anchor bolts");
    for kind in ["'base_plate'", "'end_plate'", "'hanger'"] {
        assert!(text.contains(kind), "the kind as ObjectType: {kind}");
    }
}

#[test]
fn connections_come_back_without_a_native_round_trip() {
    let text = export(&connections_project(), &ExportOptions::default());
    let (imported, _) = import(&text, Project::from_template()).unwrap();
    assert_eq!(imported.elements_of(Category::Connection).count(), 0);
    assert_eq!(imported.elements_of(Category::Column).count(), 2, "the members come back");
}

/// Writes `fixtures/connections.ccad` from [`connections_project`]
/// (`cargo nextest run -p cc_ifc --test main write_connections_fixture --run-ignored only`).
#[test]
#[ignore]
fn write_connections_fixture() {
    let text = cc_core::to_ron(&connections_project()).unwrap();
    std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/connections.ccad"), text).unwrap();
}
