//! Steel and timber connections: an element at one end of a column or beam
//! (`member`, `end`) of a standard kind, whose plates and bolts
//! `cc_geom::connection` derives from the member's geometry on every build.
//! It has no placement of its own: it is removed with its member
//! ([`crate::EditCtx::remove_element`]), follows its phases and design option
//! ([`crate::phase::phase_host`]), is never copied ([`Project::copy_set`]),
//! and a member end carries at most one ([`Project::repair_connections`]
//! keeps the lowest id).

use crate::{Element, ElementId, ElementKind, ElementType, ElementTypeId, Project, StructuralMaterial, TypeData};
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

/// Every length of a connection, meters. A plate's `width` and `length`
/// may also be 0: taken from the member's section plus `edge`.
pub const LENGTH: RangeInclusive<f64> = 0.001..=2.0;
/// Bolt rows and columns.
pub const COUNT: RangeInclusive<u32> = 1..=10;

/// The standard joints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub enum ConnectionKind {
    /// A steel plate under a column with anchor bolts.
    #[default]
    BasePlate,
    /// A steel plate on a beam end that stops at a column face, bolted through.
    EndPlate,
    /// A steel plate along the web of a beam end at a column or a beam.
    FinPlate,
    /// A group of bolts with washers through a timber beam end at a column.
    TimberBolts,
    /// A bent steel plate under a timber beam end at a beam.
    Hanger,
}

impl ConnectionKind {
    pub const ALL: [ConnectionKind; 5] = [
        ConnectionKind::BasePlate,
        ConnectionKind::EndPlate,
        ConnectionKind::FinPlate,
        ConnectionKind::TimberBolts,
        ConnectionKind::Hanger,
    ];

    pub fn code(self) -> &'static str {
        match self {
            ConnectionKind::BasePlate => "base_plate",
            ConnectionKind::EndPlate => "end_plate",
            ConnectionKind::FinPlate => "fin_plate",
            ConnectionKind::TimberBolts => "timber_bolts",
            ConnectionKind::Hanger => "hanger",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.code().eq_ignore_ascii_case(code.trim()))
    }

    /// The i18n key of its name.
    pub fn key(self) -> &'static str {
        match self {
            ConnectionKind::BasePlate => "connection-base-plate",
            ConnectionKind::EndPlate => "connection-end-plate",
            ConnectionKind::FinPlate => "connection-fin-plate",
            ConnectionKind::TimberBolts => "connection-timber-bolts",
            ConnectionKind::Hanger => "connection-hanger",
        }
    }

    /// Whether it sits at a column's base (else at a beam's start or end).
    pub fn at_column(self) -> bool {
        self == ConnectionKind::BasePlate
    }

    /// The Structure tab material whose *Connections* group offers it.
    pub fn material(self) -> StructuralMaterial {
        match self {
            ConnectionKind::TimberBolts | ConnectionKind::Hanger => StructuralMaterial::Timber,
            _ => StructuralMaterial::Steel,
        }
    }

    /// The parameters a new connection of this kind gets.
    pub fn defaults(self) -> ConnectionParams {
        let p = |thickness, bolt_diameter, rows, columns, spacing, edge| ConnectionParams {
            thickness,
            width: 0.0,
            length: 0.0,
            bolt_diameter,
            rows,
            columns,
            spacing,
            edge,
            anchor_length: 0.4,
        };
        match self {
            ConnectionKind::BasePlate => p(0.025, 0.020, 2, 2, 0.15, 0.05),
            ConnectionKind::EndPlate => p(0.016, 0.016, 2, 2, 0.10, 0.04),
            ConnectionKind::FinPlate => p(0.010, 0.016, 3, 1, 0.07, 0.04),
            ConnectionKind::TimberBolts => p(0.005, 0.012, 2, 2, 0.10, 0.08),
            ConnectionKind::Hanger => p(0.002, 0.010, 1, 2, 0.05, 0.10),
        }
    }
}

/// Which end of its member a connection sits at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MemberEnd {
    /// A column's base.
    Base,
    /// A beam's start.
    Start,
    /// A beam's end.
    End,
}

impl MemberEnd {
    pub const ALL: [MemberEnd; 3] = [MemberEnd::Base, MemberEnd::Start, MemberEnd::End];

    pub fn code(self) -> &'static str {
        match self {
            MemberEnd::Base => "base",
            MemberEnd::Start => "start",
            MemberEnd::End => "end",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            MemberEnd::Base => "member-end-base",
            MemberEnd::Start => "prop-start",
            MemberEnd::End => "prop-end",
        }
    }
}

/// The sizes of a connection, meters (see the kind for what each means).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConnectionParams {
    /// Plate thickness (a bolted joint's washers).
    pub thickness: f64,
    /// Plate width; 0: from the member's section plus `edge`.
    pub width: f64,
    /// Plate length; 0: from the member's section (or bolts) plus `edge`.
    pub length: f64,
    pub bolt_diameter: f64,
    pub rows: u32,
    pub columns: u32,
    /// Bolt to bolt.
    pub spacing: f64,
    /// Bolt to the plate's edge (or to the member's end).
    pub edge: f64,
    /// Anchor bolts below a base plate.
    pub anchor_length: f64,
}

impl Default for ConnectionParams {
    fn default() -> Self {
        ConnectionKind::BasePlate.defaults()
    }
}

impl ConnectionParams {
    /// Whether every value is in range.
    pub fn check(&self) -> Result<(), String> {
        use crate::member_host::within;
        for (name, v) in [
            ("thickness", self.thickness),
            ("bolt_diameter", self.bolt_diameter),
            ("spacing", self.spacing),
            ("edge", self.edge),
            ("anchor_length", self.anchor_length),
        ] {
            within(name, v, &LENGTH)?;
        }
        for (name, v) in [("width", self.width), ("length", self.length)] {
            if v != 0.0 {
                within(name, v, &LENGTH).map_err(|e| format!("{e} (or 0: from the member)"))?;
            }
        }
        for (name, n) in [("rows", self.rows), ("columns", self.columns)] {
            if !COUNT.contains(&n) {
                return Err(format!("{name} must be between {} and {}", COUNT.start(), COUNT.end()));
            }
        }
        Ok(())
    }

    /// Values out of range back to `kind`'s defaults (a bad width or length
    /// to 0); returns how many changed.
    pub fn reset_out_of_range(&mut self, kind: ConnectionKind) -> usize {
        use crate::member_host::reset;
        let d = kind.defaults();
        let mut n = reset(&mut self.thickness, &LENGTH, d.thickness)
            + reset(&mut self.bolt_diameter, &LENGTH, d.bolt_diameter)
            + reset(&mut self.spacing, &LENGTH, d.spacing)
            + reset(&mut self.edge, &LENGTH, d.edge)
            + reset(&mut self.anchor_length, &LENGTH, d.anchor_length);
        for v in [&mut self.width, &mut self.length] {
            if *v != 0.0 {
                n += reset(v, &LENGTH, 0.0);
            }
        }
        for (v, default) in [(&mut self.rows, d.rows), (&mut self.columns, d.columns)] {
            if !COUNT.contains(v) {
                *v = default;
                n += 1;
            }
        }
        n
    }
}

/// A connection at one end of a column or beam.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    /// The column or beam.
    pub member: ElementId,
    pub end: MemberEnd,
    pub kind: ConnectionKind,
    #[serde(default)]
    pub params: ConnectionParams,
}

impl Connection {
    /// A connection with the kind's default parameters.
    pub fn new(member: ElementId, end: MemberEnd, kind: ConnectionKind) -> Self {
        Self { member, end, kind, params: kind.defaults() }
    }

    /// Whether it can sit on `member` at its end: a base plate on a column's
    /// base, the other kinds on a beam's start or end.
    pub fn fits(&self, member: &ElementKind) -> bool {
        match (member, self.end) {
            (ElementKind::Column(_), MemberEnd::Base) => self.kind.at_column(),
            (ElementKind::Beam(_), MemberEnd::Start | MemberEnd::End) => !self.kind.at_column(),
            _ => false,
        }
    }
}

/// The type a project gets with its first connection.
pub fn host_type(id: ElementTypeId, name: String) -> ElementType {
    ElementType { id, name, data: TypeData::Connection {}, info: Default::default(), uid: Default::default() }
}

impl Project {
    /// The connections on `member`.
    pub fn connections_of(&self, member: ElementId) -> impl Iterator<Item = (&Element, &Connection)> {
        self.elements.values().filter_map(move |e| match &e.kind {
            ElementKind::Connection(c) if c.member == member => Some((e, c)),
            _ => None,
        })
    }

    /// The connection at `member`'s `end`, if it has one.
    pub fn connection_at(&self, member: ElementId, end: MemberEnd) -> Option<ElementId> {
        self.connections_of(member).find(|(_, c)| c.end == end).map(|(e, _)| e.id)
    }

    /// From a file: one connection per member end (the lowest id stays),
    /// numbers back in range; returns the repairs.
    pub(crate) fn repair_connections(&mut self) -> usize {
        let mut repairs = 0;
        let mut seen = std::collections::BTreeSet::new();
        let mut extra = Vec::new();
        for e in self.elements.values_mut() {
            let ElementKind::Connection(c) = &mut e.kind else { continue };
            if !seen.insert((c.member, c.end)) {
                extra.push(e.id);
                continue;
            }
            repairs += c.params.reset_out_of_range(c.kind);
        }
        repairs += extra.len();
        for id in extra {
            self.elements.remove(&id);
        }
        repairs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Column, Document, Element, LevelId, WallTop};
    use glam::DVec2;

    fn element(id: ElementId, level: LevelId, type_id: ElementTypeId, kind: ElementKind) -> Element {
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

    /// A template project with one column at (2, 3) and a base plate under
    /// it: `(document, column, connection)`.
    fn site() -> (Document, ElementId, ElementId) {
        let mut doc = Document::new(Project::from_template());
        let level = doc.project.levels[0].id;
        let (column, connection) = doc.edit("site", |e| {
            let column_type = e.project().default_type(crate::Category::Column).expect("a column type");
            let ty = ElementTypeId(e.alloc_id());
            e.add_type(host_type(ty, "Connection".into()));
            let col = Column {
                position: DVec2::new(2.0, 3.0),
                rotation_deg: 0.0,
                base_offset: 0.0,
                height: 3.0,
                top: WallTop::Unconnected,
                structural: true,
                base_support: Default::default(),
            };
            let column = ElementId(e.alloc_id());
            e.add_element(element(column, level, column_type, ElementKind::Column(col)));
            let id = ElementId(e.alloc_id());
            let c = Connection::new(column, MemberEnd::Base, ConnectionKind::BasePlate);
            e.add_element(element(id, level, ty, ElementKind::Connection(c)));
            (column, id)
        });
        (doc, column, connection)
    }

    #[test]
    fn codes_round_trip() {
        for k in ConnectionKind::ALL {
            assert_eq!(ConnectionKind::from_code(k.code()), Some(k));
            assert_eq!(k.defaults().check(), Ok(()), "{k:?} defaults are in range");
        }
        assert_eq!(ConnectionKind::from_code(" End_Plate "), Some(ConnectionKind::EndPlate));
        assert_eq!(ConnectionKind::from_code("weld"), None);
        assert!(ConnectionKind::BasePlate.at_column() && !ConnectionKind::Hanger.at_column());
        assert_eq!(ConnectionKind::TimberBolts.material(), crate::StructuralMaterial::Timber);
        assert_eq!(ConnectionKind::FinPlate.material(), crate::StructuralMaterial::Steel);
    }

    #[test]
    fn bounds_are_checked() {
        let ok = ConnectionKind::EndPlate.defaults();
        let bad = |f: fn(&mut ConnectionParams)| {
            let mut p = ok;
            f(&mut p);
            p.check()
        };
        assert!(bad(|p| p.width = 0.0).is_ok(), "0: from the member");
        assert!(bad(|p| p.width = 0.0005).is_err());
        assert!(bad(|p| p.length = 2.5).is_err());
        assert!(bad(|p| p.thickness = 0.0).is_err(), "only width and length may be 0");
        assert!(bad(|p| p.bolt_diameter = f64::NAN).is_err());
        assert!(bad(|p| p.rows = 0).is_err());
        assert!(bad(|p| p.columns = 11).is_err());
        assert!(bad(|p| p.anchor_length = 2.0).is_ok());
    }

    #[test]
    fn a_connection_fits_its_member_end() {
        let column = ElementKind::Column(Column {
            position: DVec2::ZERO,
            rotation_deg: 0.0,
            base_offset: 0.0,
            height: 3.0,
            top: WallTop::Unconnected,
            structural: true,
            base_support: Default::default(),
        });
        let beam = ElementKind::Beam(crate::Beam::new(DVec2::ZERO, DVec2::X, 3.0));
        let c = |end, kind| Connection::new(ElementId(1), end, kind);
        assert!(c(MemberEnd::Base, ConnectionKind::BasePlate).fits(&column));
        assert!(!c(MemberEnd::Start, ConnectionKind::BasePlate).fits(&column));
        assert!(!c(MemberEnd::Base, ConnectionKind::BasePlate).fits(&beam));
        assert!(c(MemberEnd::End, ConnectionKind::Hanger).fits(&beam));
        assert!(!c(MemberEnd::Base, ConnectionKind::EndPlate).fits(&column));
    }

    /// Repair keeps the first connection of a member end and resets numbers
    /// out of range to the kind's defaults.
    #[test]
    fn repair_keeps_one_per_end_and_resets_numbers() {
        let (doc, column, first) = site();
        let mut p = doc.project.clone();
        let mut second = p.element(first).unwrap().clone();
        second.id = ElementId(p.alloc_id());
        second.uid = crate::Uid::generate();
        p.elements.insert(second.id, second.clone());
        if let Some(ElementKind::Connection(c)) = p.elements.get_mut(&first).map(|e| &mut e.kind) {
            c.params.rows = 0;
            c.params.thickness = f64::NAN;
        }
        assert_eq!(p.repair(), 3, "the second one, rows and thickness");
        assert!(p.element(second.id).is_none());
        let Some(ElementKind::Connection(c)) = p.element(first).map(|e| &e.kind) else { panic!("kept") };
        assert_eq!((c.params.rows, c.params.thickness), (2, 0.025));
        assert_eq!(p.connection_at(column, MemberEnd::Base), Some(first));
    }

    #[test]
    fn a_connection_on_the_wrong_member_is_dropped() {
        let (doc, _, first) = site();
        let mut p = doc.project.clone();
        if let Some(ElementKind::Connection(c)) = p.elements.get_mut(&first).map(|e| &mut e.kind) {
            c.end = MemberEnd::Start;
        }
        assert_eq!(p.repair(), 1);
        assert!(p.element(first).is_none());
    }

    #[test]
    fn it_goes_with_its_member_and_is_not_copied() {
        let (mut doc, column, connection) = site();
        assert!(doc.project.copy_set(&[column, connection]).iter().all(|e| e.id != connection));
        assert_eq!(crate::phase::phase_host(doc.project.element(connection).unwrap()), Some(column));
        doc.edit("delete", |e| e.remove_element(column));
        assert!(doc.project.element(connection).is_none());
        assert_eq!(doc.project.repairs_needed(), 0);
    }
}
