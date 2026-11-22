//! Sloping IFC profiles must retain their elevations during import.

use cc_core::{ElementKind, Project};
use cc_geom::{ModelGeometry, Ray};
use glam::DVec3;

/// A sloping triangular profile extruded vertically, as used for terrain facets.
#[test]
fn sloping_profiles_keep_their_mesh_and_are_selectable() {
    let text = "ISO-10303-21;
HEADER; FILE_SCHEMA(('IFC2X3')); ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Test',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#7,$);
#3=IFCUNITASSIGNMENT((#4));
#4=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#6=IFCLOCALPLACEMENT($,#7);
#7=IFCAXIS2PLACEMENT3D(#8,$,$);
#8=IFCCARTESIANPOINT((0.,0.,0.));
#9=IFCCARTESIANPOINT((0.,0.));
#10=IFCCARTESIANPOINT((2.,0.));
#11=IFCCARTESIANPOINT((0.,2.));
#12=IFCPOLYLINE((#9,#10,#11,#9));
#13=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#12);
#14=IFCDIRECTION((0.,-0.7071067811865476,0.7071067811865476));
#15=IFCDIRECTION((1.,0.,0.));
#16=IFCAXIS2PLACEMENT3D(#8,#14,#15);
#17=IFCDIRECTION((0.,0.7071067811865476,0.7071067811865476));
#18=IFCEXTRUDEDAREASOLID(#13,#16,#17,0.3);
#19=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#18));
#20=IFCPRODUCTDEFINITIONSHAPE($,$,(#19));
#21=IFCSLAB('2AvctVUKr0kugbFTf53O9L',$,'Slope',$,$,#6,#20,$,.FLOOR.);
ENDSEC; END-ISO-10303-21;";
    let (project, report) = cc_ifc::import(text, Project::from_template()).unwrap();
    assert!(report.skipped.is_empty());
    let element = project.elements.values().find(|element| matches!(element.kind, ElementKind::Component(_))).unwrap();
    let geometry = ModelGeometry::build(&project);
    let ray = Ray { origin: DVec3::new(0.25, 0.25, 5.0), dir: -DVec3::Z };
    let (picked, distance) = geometry.pick(ray, &[]).unwrap();
    assert_eq!(picked, element.id);
    let expected_height = 0.25 + 0.3;
    assert!((5.0 - distance - expected_height).abs() < 1e-6);
}
