//! Built-in profile catalogs: the standard series of rolled, cold-formed,
//! timber and hollow sections, as tables (cells split by `;`, since the
//! designations write decimals with a comma) compiled into the binary and
//! parsed once. Each row is a designation (the full name the standard gives,
//! e.g. `W150x22,5 (H)`) and the dimensions of the series'
//! [`ProfileShape`] in millimeters (taken from public dimension tables), plus
//! the tabulated area in cm², kept only to check the computed outlines.
//! Catalogs are not stored in projects: importing a row copies it in as a
//! [`ProfileDef`] that remembers its [`CatalogRef`].

use crate::structural_profile::{CatalogRef, ProfileDef, ProfileMaterial, ProfileShape};
use crate::{ProfileId, Uid};
use std::sync::OnceLock;

/// Most rows read from one table.
pub const MAX_ROWS: usize = 5000;

/// Regional / normative library a catalog series belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileLibrary {
    Brazil,
    Europe,
    Timber,
}

impl ProfileLibrary {
    pub fn code(self) -> &'static str {
        match self {
            Self::Brazil => "brazil",
            Self::Europe => "europe",
            Self::Timber => "timber",
        }
    }
}

/// A catalog series: rows of one shape and material.
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    /// Stable code, e.g. `"W"`, `"NBR6355-Ue"`.
    pub code: &'static str,
    /// The i18n key of the series' name.
    pub name_key: &'static str,
    pub library: ProfileLibrary,
    pub material: ProfileMaterial,
    /// The [`ProfileShape::code`] of the rows.
    pub shape: &'static str,
    pub rows: Vec<CatalogRow>,
}

/// One row of a series.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogRow {
    pub designation: String,
    /// The shape's values by field name: meters, `angle` in degrees.
    pub values: Vec<(String, f64)>,
    /// The tabulated area, m² (reference only; 0 when the table has none).
    pub area: f64,
}

/// What a series is: its code, name key, library, material, shape and table.
struct Spec {
    code: &'static str,
    name_key: &'static str,
    library: ProfileLibrary,
    material: ProfileMaterial,
    shape: &'static str,
    table: &'static str,
}

const fn spec(
    code: &'static str,
    name_key: &'static str,
    library: ProfileLibrary,
    material: ProfileMaterial,
    shape: &'static str,
    table: &'static str,
) -> Spec {
    Spec { code, name_key, library, material, shape, table }
}

use ProfileLibrary::{Brazil, Europe, Timber as LibTimber};
use ProfileMaterial::{ColdFormed, Steel, Timber};

/// The series: W and HP as rolled in Brazil (ABNT NBR 15980:2024, ASTM A6/A6M
/// dimensions; `(H)` marks the wide-flange column sizes), CS and CVS welded
/// sections (ABNT NBR 5884:2005), IPE, HEA and HEB (EN 10365), the NBR 6355
/// cold-formed sections (Z45 with lips at 45°, Z90 with square lips), commercial
/// sawn timber and glulam, round steel bars (BR) and the hollow sections
/// (EN 10219 sizes and corner radii).
const SPECS: &[Spec] = &[
    spec("W", "profile-series-w", Brazil, Steel, "i", include_str!("../assets/profiles/w.csv")),
    spec("HP", "profile-series-hp", Brazil, Steel, "i", include_str!("../assets/profiles/hp.csv")),
    spec("CS", "profile-series-cs", Brazil, Steel, "i", include_str!("../assets/profiles/cs.csv")),
    spec("CVS", "profile-series-cvs", Brazil, Steel, "i", include_str!("../assets/profiles/cvs.csv")),
    spec("U-LAM", "profile-series-u-lam", Brazil, Steel, "channel", include_str!("../assets/profiles/u_lam.csv")),
    spec("L-LAM", "profile-series-l-lam", Brazil, Steel, "angle", include_str!("../assets/profiles/l_lam.csv")),
    spec("IPE", "profile-series-ipe", Europe, Steel, "i", include_str!("../assets/profiles/ipe.csv")),
    spec("HEA", "profile-series-hea", Europe, Steel, "i", include_str!("../assets/profiles/hea.csv")),
    spec("HEB", "profile-series-heb", Europe, Steel, "i", include_str!("../assets/profiles/heb.csv")),
    spec(
        "NBR6355-U",
        "profile-series-nbr6355-u",
        Brazil,
        ColdFormed,
        "cold_u",
        include_str!("../assets/profiles/nbr6355_u.csv"),
    ),
    spec(
        "NBR6355-Ue",
        "profile-series-nbr6355-ue",
        Brazil,
        ColdFormed,
        "cold_lipped_u",
        include_str!("../assets/profiles/nbr6355_ue.csv"),
    ),
    spec(
        "NBR6355-Z",
        "profile-series-nbr6355-z",
        Brazil,
        ColdFormed,
        "cold_z",
        include_str!("../assets/profiles/nbr6355_z.csv"),
    ),
    spec(
        "NBR6355-Ze",
        "profile-series-nbr6355-ze",
        Brazil,
        ColdFormed,
        "cold_z",
        include_str!("../assets/profiles/nbr6355_ze.csv"),
    ),
    spec(
        "NBR6355-L",
        "profile-series-nbr6355-l",
        Brazil,
        ColdFormed,
        "angle",
        include_str!("../assets/profiles/nbr6355_l.csv"),
    ),
    spec(
        "NBR6355-Cartola",
        "profile-series-nbr6355-cartola",
        Brazil,
        ColdFormed,
        "cold_hat",
        include_str!("../assets/profiles/nbr6355_cartola.csv"),
    ),
    spec(
        "Timber-Sawn",
        "profile-series-timber-sawn",
        LibTimber,
        Timber,
        "rect",
        include_str!("../assets/profiles/timber_sawn.csv"),
    ),
    spec("Glulam", "profile-series-glulam", LibTimber, Timber, "rect", include_str!("../assets/profiles/glulam.csv")),
    spec("SHS", "profile-series-shs", Europe, Steel, "rect_tube", include_str!("../assets/profiles/shs.csv")),
    spec("RHS", "profile-series-rhs", Europe, Steel, "rect_tube", include_str!("../assets/profiles/rhs.csv")),
    spec("CHS", "profile-series-chs", Europe, Steel, "round_tube", include_str!("../assets/profiles/chs.csv")),
    spec("BR", "profile-series-br", Brazil, Steel, "round", include_str!("../assets/profiles/br.csv")),
];

/// Every built-in series, parsed on first use; rows that do not parse are left out.
pub fn catalog() -> &'static [Series] {
    static CATALOG: OnceLock<Vec<Series>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        SPECS
            .iter()
            .map(|s| {
                let names: Vec<String> = ProfileShape::default_for(s.shape)
                    .map(|sh| sh.params().into_iter().map(|(n, _)| n).collect())
                    .unwrap_or_default();
                let fields: Vec<&str> = names.iter().map(String::as_str).collect();
                Series {
                    code: s.code,
                    name_key: s.name_key,
                    library: s.library,
                    material: s.material,
                    shape: s.shape,
                    rows: parse_rows(s.table, &fields),
                }
            })
            .collect()
    })
}

/// The series of that code.
pub fn series(code: &str) -> Option<&'static Series> {
    catalog().iter().find(|s| s.code == code)
}

/// A profile for a row (id 0 and a new uid; the caller sets the id), named
/// by its designation, or `None` when the row does not make a valid outline.
/// Values the row lacks are 0, except a Z's lip angle, which is square (90°).
pub fn row_to_def(series: &Series, row: &CatalogRow) -> Option<ProfileDef> {
    let mut shape = ProfileShape::default_for(series.shape)?;
    for (name, _) in shape.params() {
        let v = if name == "angle" { 90.0 } else { 0.0 };
        shape.set_param(&name, v);
    }
    for (name, v) in &row.values {
        if !shape.set_param(name, *v) {
            return None;
        }
    }
    shape.outline().ok()?;
    Some(ProfileDef {
        id: ProfileId(0),
        uid: Uid::generate(),
        name: row.designation.clone(),
        material: series.material,
        shape,
        catalog: Some(CatalogRef { series: series.code.to_string(), designation: row.designation.clone() }),
        description: String::new(),
    })
}

/// Reads a table, cells split by `;`: a header row naming the columns
/// (`designation`, any of the shape's `fields`, and optionally `area_cm2`), then
/// one row per line.
/// Lengths are millimeters (converted to meters; `angle` stays in degrees)
/// and the area cm² (converted to m²). Blank lines and lines starting with
/// `#` are ignored; rows with a wrong number of cells, no designation or a
/// value that is not a finite number (non-negative, the area positive) are
/// skipped; at most [`MAX_ROWS`] rows are read. Other columns are ignored.
fn parse_rows(text: &str, fields: &[&str]) -> Vec<CatalogRow> {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#'));
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let columns: Vec<&str> = header.split(';').map(str::trim).collect();
    let mut rows = Vec::new();
    for line in lines {
        if rows.len() >= MAX_ROWS {
            break;
        }
        let cells: Vec<&str> = line.split(';').map(str::trim).collect();
        if cells.len() != columns.len() {
            continue;
        }
        if let Some(row) = parse_row(&columns, &cells, fields) {
            rows.push(row);
        }
    }
    rows
}

fn parse_row(columns: &[&str], cells: &[&str], fields: &[&str]) -> Option<CatalogRow> {
    let mut designation = None;
    let mut values = Vec::new();
    let mut area = 0.0;
    for (&col, &cell) in columns.iter().zip(cells) {
        if col == "designation" {
            if cell.is_empty() {
                return None;
            }
            designation = Some(cell.to_string());
            continue;
        }
        let is_area = col == "area_cm2";
        if !is_area && !fields.contains(&col) {
            continue;
        }
        let v: f64 = cell.parse().ok()?;
        if !v.is_finite() || v < 0.0 || (is_area && v == 0.0) {
            return None;
        }
        if is_area {
            area = v / 10_000.0;
        } else {
            values.push((col.to_string(), if col == "angle" { v } else { v / 1000.0 }));
        }
    }
    Some(CatalogRow { designation: designation?, values, area })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_series_parses_and_has_rows() {
        let codes: Vec<&str> = catalog().iter().map(|s| s.code).collect();
        for c in [
            "W",
            "HP",
            "CS",
            "CVS",
            "IPE",
            "HEA",
            "HEB",
            "NBR6355-U",
            "NBR6355-Ue",
            "NBR6355-Z",
            "NBR6355-Ze",
            "NBR6355-L",
            "NBR6355-Cartola",
            "U-LAM",
            "L-LAM",
            "Timber-Sawn",
            "Glulam",
            "SHS",
            "RHS",
            "CHS",
            "BR",
        ] {
            assert!(codes.contains(&c), "{c}");
            assert!(!series(c).unwrap().rows.is_empty(), "{c}");
        }
    }

    #[test]
    fn computed_areas_match_the_tables() {
        for s in catalog() {
            for row in &s.rows {
                let def = row_to_def(s, row).unwrap_or_else(|| panic!("{} {}", s.code, row.designation));
                let a = def.outline().unwrap().properties(0.0).area;
                let err = (a - row.area).abs() / row.area;
                let tol = match s.code {
                    "NBR6355-L" => 0.07,
                    "U-LAM" | "L-LAM" => 0.05,
                    "NBR6355-Cartola" => 0.04,
                    _ => 0.02,
                };
                assert!(err < tol, "{} {}: computed {a} table {}", s.code, row.designation, row.area);
            }
        }
    }

    #[test]
    fn reference_rows() {
        let ipe = series("IPE").unwrap();
        let r = ipe.rows.iter().find(|r| r.designation == "IPE 200").unwrap();
        let def = row_to_def(ipe, r).unwrap();
        assert_eq!(def.name, "IPE 200");
        assert_eq!(def.shape, ProfileShape::I { d: 0.2, bf: 0.1, tw: 0.0056, tf: 0.0085, r: 0.012 });
    }

    #[test]
    fn malformed_csv_rows_are_skipped() {
        let rows = parse_rows("designation;d;bf\nok;100;50\nbad;x;50\n;1\n", &["d", "bf"]);
        assert_eq!(rows.len(), 1);
    }

    fn def(code: &str, des: &str) -> ProfileDef {
        let s = series(code).unwrap();
        let r = s.rows.iter().find(|r| r.designation == des).unwrap_or_else(|| panic!("{code} {des}"));
        row_to_def(s, r).unwrap()
    }

    #[test]
    fn named_reference_rows_and_lsf_sizes() {
        let w = def("W", "W200x26,6");
        assert_eq!(w.name, "W200x26,6");
        assert_eq!(def("W", "W150x13").name, "W150x13", "a whole mass has no decimals");
        assert_eq!(
            def("W", "W150x22,5 (H)").shape,
            ProfileShape::I { d: 0.152, bf: 0.152, tw: 0.0058, tf: 0.0066, r: 0.01 }
        );
        assert_eq!(def("HP", "HP310x125 (H)").name, "HP310x125 (H)");
        let h = series("W").unwrap().rows.iter().filter(|r| r.designation.ends_with(" (H)")).count();
        assert_eq!(h, 22, "the wide-flange column sizes are marked (H)");
        assert_eq!(w.catalog, Some(CatalogRef { series: "W".into(), designation: "W200x26,6".into() }));
        assert_eq!(w.id, ProfileId(0));
        assert_eq!(w.material, ProfileMaterial::Steel);
        assert_eq!(def("HEA", "HEA 200").shape, ProfileShape::I { d: 0.19, bf: 0.2, tw: 0.0065, tf: 0.01, r: 0.018 });
        assert_eq!(def("HEB", "HEB 200").shape, ProfileShape::I { d: 0.2, bf: 0.2, tw: 0.009, tf: 0.015, r: 0.018 });
        assert_eq!(def("NBR6355-Ue", "Ue 90x40x12x0,95").name, "Ue 90x40x12x0,95");
        assert_eq!(def("Timber-Sawn", "Peça 6x16").name, "Peça 6x16");
        assert_eq!(def("Timber-Sawn", "Peça 6x16").material, ProfileMaterial::Timber);
        for d in ["90x40x12", "140x40x12", "200x40x12"] {
            for t in ["0,80", "0,95", "1,25"] {
                def("NBR6355-Ue", &format!("Ue {d}x{t}"));
            }
        }
        for d in ["92x38", "142x38", "202x38"] {
            def("NBR6355-U", &format!("U {d}x0,95"));
        }
        let z45 = def("NBR6355-Z", "Z45 200x75x20x2,00").shape;
        assert!(matches!(z45, ProfileShape::ColdZ { angle, .. } if angle == 45.0));
        let z90 = def("NBR6355-Ze", "Z90 200x75x20x2,00").shape;
        assert!(matches!(z90, ProfileShape::ColdZ { angle, .. } if angle == 90.0));
        let cs = def("CS", "CS 150x25");
        assert_eq!(cs.name, "CS 150x25");
        assert_eq!(cs.shape, ProfileShape::I { d: 0.150, bf: 0.150, tw: 0.0063, tf: 0.008, r: 0.0 });
        let cvs = def("CVS", "CVS 150x15");
        assert_eq!(cvs.name, "CVS 150x15");
        assert_eq!(cvs.shape, ProfileShape::I { d: 0.150, bf: 0.100, tw: 0.00475, tf: 0.0063, r: 0.0 });
        let br = def("BR", "BR 1/2\"");
        assert_eq!(br.name, "BR 1/2\"");
        assert_eq!(br.shape, ProfileShape::Round { d: 0.0127 });
    }

    #[test]
    fn missing_values_default_and_bad_rows_give_none() {
        let s = series("NBR6355-Ze").unwrap();
        let no_angle = CatalogRow {
            designation: "x".into(),
            values: vec![("d".into(), 0.2), ("bf".into(), 0.075), ("lip".into(), 0.02), ("t".into(), 0.002)],
            area: 0.0,
        };
        let d = row_to_def(s, &no_angle).unwrap();
        assert_eq!(d.shape, ProfileShape::ColdZ { d: 0.2, bf: 0.075, lip: 0.02, t: 0.002, ri: 0.0, angle: 90.0 });
        let unknown = CatalogRow { values: vec![("nope".into(), 1.0)], ..no_angle.clone() };
        assert!(row_to_def(s, &unknown).is_none());
        let impossible = CatalogRow { values: vec![("t".into(), 0.5)], ..no_angle };
        assert!(row_to_def(s, &impossible).is_none());
    }

    #[test]
    fn parsing_is_bounded_and_rejects_bad_numbers() {
        let mut text = String::from("designation;d;area_cm2\n");
        for i in 0..MAX_ROWS + 10 {
            text.push_str(&format!("r{i};100;1\n"));
        }
        assert_eq!(parse_rows(&text, &["d"]).len(), MAX_ROWS);
        let rows = parse_rows("designation;d;area_cm2\na;NaN;1\nb;inf;1\nc;-1;1\nd;10;0\ne;10;1;2\nf;10;2\n", &["d"]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].values, vec![("d".to_string(), 0.01)]);
        assert!((rows[0].area - 2e-4).abs() < 1e-15);
        assert!(parse_rows("", &["d"]).is_empty());
    }
}
