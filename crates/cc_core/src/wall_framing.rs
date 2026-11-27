//! Light steel and wood framing of walls: a host element attached to one
//! straight wall that owns ordinary columns (studs, king and jack studs,
//! cripples) and beams (tracks or plates, headers, sills, blocking) laid
//! out from its parameters, its wall and the doors and windows the wall
//! hosts. They are kept in step like a steel building's members (see
//! [`crate::member_host`]), and any change of the wall or its openings
//! regenerates them.
//!
//! The wall frame: `u` along the wall's body centerline
//! ([`Project::wall_axis`]) from its start to its length `L`, heights above
//! the wall's level.

use crate::member_host::{MemberHost, RoleKey, fix_type, reset, within};
use crate::steel_building::section_box;
use crate::{
    Beam, Category, Column, ElementId, ElementKind, ElementType, ElementTypeId, LevelId, Project, TypeData, WallTop,
};
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

/// Most members one framing makes; the layout stops there.
pub const MAX_MEMBERS: usize = 4000;
/// A jack, cripple or blocking piece shorter than this is left out, meters.
pub const MIN_PIECE: f64 = 0.05;
/// The index of the stud at the wall's end (its key does not depend on the length).
pub const END_STUD: u16 = u16::MAX;
pub const SPACING: RangeInclusive<f64> = 0.2..=1.2;
pub const LIGHT_STEEL_SPACING: f64 = 0.4;
pub const WOOD_SPACING: f64 = 0.6;

/// How the wall is framed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FramingSystem {
    /// Cold-formed steel studs between tracks.
    #[default]
    LightSteel,
    /// Sawn timber studs between plates.
    Wood,
}

impl FramingSystem {
    pub const ALL: [FramingSystem; 2] = [FramingSystem::LightSteel, FramingSystem::Wood];

    pub fn code(self) -> &'static str {
        match self {
            FramingSystem::LightSteel => "light_steel",
            FramingSystem::Wood => "wood",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.code().eq_ignore_ascii_case(code.trim()))
    }

    /// The i18n key of its name.
    pub fn key(self) -> &'static str {
        match self {
            FramingSystem::LightSteel => "framing-light-steel",
            FramingSystem::Wood => "framing-wood",
        }
    }

    pub fn default_spacing(self) -> f64 {
        match self {
            FramingSystem::LightSteel => LIGHT_STEEL_SPACING,
            FramingSystem::Wood => WOOD_SPACING,
        }
    }

    /// Wood frames get a double top plate by default.
    pub fn default_double_top(self) -> bool {
        self == FramingSystem::Wood
    }
}

/// What a member is in its framing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FramingRoleKind {
    BottomTrack,
    TopTrack,
    Stud,
    KingStud,
    JackStud,
    Header,
    Sill,
    Cripple,
    Blocking,
}

impl FramingRoleKind {
    pub const ALL: [FramingRoleKind; 9] = [
        FramingRoleKind::BottomTrack,
        FramingRoleKind::TopTrack,
        FramingRoleKind::Stud,
        FramingRoleKind::KingStud,
        FramingRoleKind::JackStud,
        FramingRoleKind::Header,
        FramingRoleKind::Sill,
        FramingRoleKind::Cripple,
        FramingRoleKind::Blocking,
    ];

    /// Studs are columns; tracks, headers, sills and blocking are beams.
    pub fn is_stud(self) -> bool {
        matches!(
            self,
            FramingRoleKind::Stud | FramingRoleKind::KingStud | FramingRoleKind::JackStud | FramingRoleKind::Cripple
        )
    }
}

/// A member's place in its framing:
/// - tracks: index 0 (the top plates from the topmost down);
/// - studs: index `k` for the stud at `k · spacing`, 0 at the wall start, [`END_STUD`] at its end;
/// - king and jack studs, headers, sills: `opening` is the opening's order along the wall, index 0 on the start side, 1 on the end side;
/// - cripples: index `2k` above the header and `2k + 1` below the sill;
/// - blocking: keyed by its left stud, `(0, k)` after stud `k` and `(j + 1, 0)`/`(j + 1, 1)` after opening `j`'s end-side/start-side king.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FramingRole {
    pub kind: FramingRoleKind,
    pub opening: u16,
    pub index: u16,
}

impl FramingRole {
    pub fn fits(self, kind: &ElementKind) -> bool {
        if self.kind.is_stud() { matches!(kind, ElementKind::Column(_)) } else { matches!(kind, ElementKind::Beam(_)) }
    }

    /// Its key for the shared member-host code (same order as the role).
    pub fn key(self) -> RoleKey {
        RoleKey::new(self.kind as u8, self.opening, self.index)
    }

    pub fn from_key(key: RoleKey) -> Option<Self> {
        let (kind, opening, index) = key.parts();
        Some(Self { kind: *FramingRoleKind::ALL.get(usize::from(kind))?, opening, index })
    }
}

/// A member the framing manages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FramingMember {
    pub role: FramingRole,
    pub id: ElementId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallFraming {
    /// The framed wall (straight; one framing per wall).
    pub wall: ElementId,
    #[serde(default)]
    pub system: FramingSystem,
    /// Column type of the studs, king and jack studs and cripples.
    pub stud_type: ElementTypeId,
    /// Beam type of the bottom and top tracks or plates, and of the blocking.
    pub track_type: ElementTypeId,
    /// Beam type of the headers and sills.
    pub header_type: ElementTypeId,
    /// Stud spacing on center from the wall start, meters.
    pub spacing: f64,
    /// An extra top plate (wood only; ignored for light steel).
    #[serde(default)]
    pub double_top: bool,
    /// A row of blocking at mid-height between full-height studs.
    #[serde(default)]
    pub blocking: bool,
    #[serde(default)]
    pub members: Vec<FramingMember>,
    #[serde(default)]
    pub detached: Vec<FramingRole>,
}

/// The type a project gets with its first wall framing.
pub fn host_type(id: ElementTypeId, name: String) -> ElementType {
    ElementType { id, name, data: TypeData::WallFraming {}, info: Default::default(), uid: Default::default() }
}

/// Members collected by a layout; it stops collecting one past [`MAX_MEMBERS`],
/// which is enough to know the framing is too big.
struct Members(Vec<(FramingRole, ElementTypeId, ElementKind)>);

impl Members {
    fn push(&mut self, kind: FramingRoleKind, opening: u16, index: u16, type_id: ElementTypeId, element: ElementKind) {
        if self.0.len() <= MAX_MEMBERS {
            self.0.push((FramingRole { kind, opening, index }, type_id, element));
        }
    }
}

/// The wall frame: the centerline's start and direction, its length, and
/// the wall's base and top above the wall's level.
#[derive(Debug, Clone, Copy)]
struct Frame {
    origin: DVec2,
    dir: DVec2,
    length: f64,
    base: f64,
    top: f64,
}

/// A door or window in the wall frame: its order along the wall, its span
/// clipped to the wall, and its sill and head above the wall's level.
#[derive(Debug, Clone, Copy)]
struct Opening {
    index: u16,
    u0: f64,
    u1: f64,
    sill: f64,
    head: f64,
    window: bool,
}

impl WallFraming {
    /// A framing of `wall` with the system's default spacing and top plates, no blocking.
    pub fn new(
        wall: ElementId,
        system: FramingSystem,
        stud_type: ElementTypeId,
        track_type: ElementTypeId,
        header_type: ElementTypeId,
    ) -> Self {
        Self {
            wall,
            system,
            stud_type,
            track_type,
            header_type,
            spacing: system.default_spacing(),
            double_top: system.default_double_top(),
            blocking: false,
            members: Vec::new(),
            detached: Vec::new(),
        }
    }

    /// Top tracks or plates: two for a wood frame with `double_top`, else one.
    pub fn top_plates(&self) -> u16 {
        if self.system == FramingSystem::Wood && self.double_top { 2 } else { 1 }
    }

    /// Whether the spacing is in range.
    pub fn check(&self) -> Result<(), String> {
        within("spacing", self.spacing, &SPACING)
    }

    /// Whether the stud type is a column type and the track and header types beam types.
    pub fn check_types(&self, project: &Project) -> Result<(), String> {
        let is = |t: ElementTypeId, c: Category| project.element_type(t).is_some_and(|x| x.category() == c);
        if !is(self.stud_type, Category::Column) {
            return Err(format!("type {} is not a column type", self.stud_type.0));
        }
        match [self.track_type, self.header_type].into_iter().find(|t| !is(*t, Category::Beam)) {
            Some(t) => Err(format!("type {} is not a beam type", t.0)),
            None => Ok(()),
        }
    }

    /// Whether its wall is a wall of the project.
    pub fn check_wall(&self, project: &Project) -> Result<(), String> {
        match project.element(self.wall) {
            Some(e) if e.as_wall().is_some() => Ok(()),
            _ => Err(format!("element {} is not a wall", self.wall.0)),
        }
    }

    /// Put an out-of-range spacing back to the system's default; returns how many (0 or 1).
    pub fn reset_out_of_range(&mut self) -> usize {
        reset(&mut self.spacing, &SPACING, self.system.default_spacing())
    }

    /// The member types it uses.
    pub fn type_ids(&self) -> impl Iterator<Item = ElementTypeId> + '_ {
        [self.stud_type, self.track_type, self.header_type].into_iter()
    }

    /// Its wall's centerline length (0 when the wall is gone).
    pub fn wall_length(&self, project: &Project) -> f64 {
        project.element(self.wall).and_then(|e| e.as_wall().map(|w| project.wall_length(e, w))).unwrap_or(0.0)
    }

    /// The wall frame and the wall's openings; `None` when the wall is gone,
    /// curved or degenerate.
    fn frame(&self, project: &Project) -> Option<(Frame, Vec<Opening>)> {
        let e = project.element(self.wall)?;
        let w = e.as_wall()?;
        if w.is_curved() {
            return None;
        }
        let (a, b) = project.wall_axis(e, w);
        let length = a.distance(b);
        let dir = (b - a).try_normalize()?;
        let floor = project.level_elevation(e.level);
        let (base, top) = (project.wall_base_z(e, w) - floor, project.wall_top_z(e, w) - floor);
        if !(length.is_finite() && base.is_finite() && top.is_finite()) {
            return None;
        }
        let mut hosted: Vec<(&crate::Element, &crate::Hosted)> =
            // Niches do not open the wall through: the framing passes behind them.
            project
                .hosted_by(self.wall)
                .filter(|o| !matches!(o.kind, ElementKind::Niche(_)))
                .filter_map(|o| o.as_hosted().map(|h| (o, h)))
                .collect();
        hosted.sort_by(|x, y| x.1.offset.total_cmp(&y.1.offset).then(x.0.id.cmp(&y.0.id)));
        let mut openings = Vec::new();
        for (j, (o, h)) in hosted.into_iter().enumerate() {
            let Ok(index) = u16::try_from(j) else { break };
            let Some((width, height)) = project.element_type(o.type_id).and_then(|t| t.opening_size()) else {
                continue;
            };
            if !(width.is_finite() && height.is_finite() && h.offset.is_finite() && width > 0.0 && height > 0.0) {
                continue;
            }
            let window = matches!(o.kind, ElementKind::Window(_));
            let sill = if window && h.sill_height.is_finite() { h.sill_height.max(0.0) } else { 0.0 };
            let u0 = (h.offset - width / 2.0).clamp(0.0, length);
            let u1 = (h.offset + width / 2.0).clamp(0.0, length);
            if u1 - u0 > 1e-6 {
                openings.push(Opening { index, u0, u1, sill: base + sill, head: base + sill + height, window });
            }
        }
        Some((Frame { origin: a, dir, length, base, top }, openings))
    }

    /// The regular stud positions `(index, u)`: the start stud flush with the
    /// wall start, stud `k` at `k · spacing` while it clears the stud before
    /// it and the end stud by a stud width, and the end stud flush with the end.
    fn positions(&self, length: f64, w: f64) -> Vec<(u16, f64)> {
        let s = w / 2.0;
        let end = length - s;
        let mut out = vec![(0, s)];
        for k in 1..END_STUD {
            let u = f64::from(k) * self.spacing;
            if u > end - w + 1e-9 {
                break;
            }
            if out.last().is_some_and(|&(_, last)| u - last >= w - 1e-9) {
                out.push((k, u));
            }
        }
        out.push((END_STUD, end));
        out
    }

    /// Its members: role, type and element. Empty when the spacing is out of
    /// range or the wall is gone, curved, shorter than two studs or too low
    /// for its tracks, and when the members would be more than
    /// [`MAX_MEMBERS`] (see [`WallFraming::check_size`]): never cut short.
    pub fn layout(&self, project: &Project) -> Vec<(FramingRole, ElementTypeId, ElementKind)> {
        self.build(project).unwrap_or_default()
    }

    /// Whether the members fit in [`MAX_MEMBERS`].
    pub fn check_size(&self, project: &Project) -> Result<(), String> {
        self.build(project).map(|_| ())
    }

    fn build(&self, project: &Project) -> Result<Vec<(FramingRole, ElementTypeId, ElementKind)>, String> {
        use FramingRoleKind as K;
        let mut out = Members(Vec::new());
        if self.check().is_err() {
            return Ok(Vec::new());
        }
        let Some((f, openings)) = self.frame(project) else { return Ok(Vec::new()) };
        let (w, _) = section_box(project, self.stud_type);
        let (_, t) = section_box(project, self.track_type);
        let (_, hd) = section_box(project, self.header_type);
        let s = w / 2.0;
        let plates = self.top_plates();
        // Studs stand on the bottom track and stop under the top ones.
        let (bottom, top) = (f.base + t, f.top - f64::from(plates) * t);
        if f.length < 2.0 * w || top - bottom < MIN_PIECE {
            return Ok(Vec::new());
        }
        // The section's width (local x) along the wall, its depth (local y) across it.
        let angle = f.dir.to_angle().to_degrees();
        let at = |u: f64| f.origin + f.dir * u;
        let stud = |u: f64, z0: f64, z1: f64| {
            ElementKind::Column(Column {
                position: at(u),
                rotation_deg: angle,
                base_offset: z0,
                height: z1 - z0,
                top: WallTop::Unconnected,
                structural: true,
                base_support: Default::default(),
            })
        };
        let beam = |u0: f64, u1: f64, top_z: f64| {
            let mut b = Beam::new(at(u0), at(u1), top_z);
            (b.join_start, b.join_end) = (false, false);
            ElementKind::Beam(b)
        };
        let (studs, tracks, headers) = (self.stud_type, self.track_type, self.header_type);
        out.push(K::BottomTrack, 0, 0, tracks, beam(0.0, f.length, bottom));
        for i in 0..plates {
            out.push(K::TopTrack, 0, i, tracks, beam(0.0, f.length, f.top - f64::from(i) * t));
        }
        let positions = self.positions(f.length, w);
        // What an opening's framing takes: its jacks and kings on both sides.
        let taken = |u: f64| openings.iter().any(|o| u + s > o.u0 - 2.0 * w + 1e-9 && u - s < o.u1 + 2.0 * w - 1e-9);
        // Full-height studs, with the key of the blocking that starts at each.
        let mut full: Vec<(f64, Option<(u16, u16)>)> = Vec::new();
        for &(k, u) in &positions {
            if !taken(u) {
                out.push(K::Stud, 0, k, studs, stud(u, bottom, top));
                full.push((u, Some((0, k))));
            }
        }
        let on_wall = |u: f64| u >= s - 1e-9 && u <= f.length - s + 1e-9;
        for o in &openings {
            let j = o.index;
            let elsewhere = |u: f64| openings.iter().any(|x| x.index != j && u > x.u0 && u < x.u1);
            // The header's underside: the opening's head, kept under the top tracks.
            let head = o.head.min(top - hd);
            for (side, sign, edge) in [(0u16, -1.0, o.u0), (1, 1.0, o.u1)] {
                let king = edge + sign * 3.0 * s;
                if on_wall(king) && !elsewhere(king) {
                    out.push(K::KingStud, j, side, studs, stud(king, bottom, top));
                    full.push((king, j.checked_add(1).map(|g| (g, 1 - side))));
                }
                let jack = edge + sign * s;
                if on_wall(jack) && !elsewhere(jack) && head - bottom >= MIN_PIECE {
                    out.push(K::JackStud, j, side, studs, stud(jack, bottom, head));
                }
            }
            if head - bottom >= MIN_PIECE {
                out.push(K::Header, j, 0, headers, beam((o.u0 - w).max(0.0), (o.u1 + w).min(f.length), head + hd));
            }
            // A window's sill, when it clears the bottom track.
            let sill = (o.window && o.sill - hd >= bottom - 1e-9).then_some(o.sill);
            if let Some(z) = sill {
                out.push(K::Sill, j, 0, headers, beam(o.u0, o.u1, z));
            }
            for &(k, u) in positions.iter().filter(|(k, _)| *k != END_STUD) {
                if u < o.u0 + s - 1e-9 || u > o.u1 - s + 1e-9 || elsewhere(u) {
                    continue;
                }
                let (Ok(above), Ok(below)) = (u16::try_from(2 * u32::from(k)), u16::try_from(2 * u32::from(k) + 1))
                else {
                    continue;
                };
                if top - (head + hd) >= MIN_PIECE {
                    out.push(K::Cripple, j, above, studs, stud(u, head + hd, top));
                }
                if let Some(z) = sill
                    && z - hd - bottom >= MIN_PIECE
                {
                    out.push(K::Cripple, j, below, studs, stud(u, bottom, z - hd));
                }
            }
        }
        if self.blocking {
            full.sort_by(|a, b| a.0.total_cmp(&b.0));
            // Centered at mid-height between the tracks.
            let z = (bottom + top) / 2.0 + t / 2.0;
            for pair in full.windows(2) {
                let ((a, key), (b, _)) = (pair[0], pair[1]);
                let (a0, b0) = (a + s, b - s);
                let across = openings.iter().any(|o| o.u0 < b0 && o.u1 > a0);
                if let Some((group, index)) = key
                    && b0 - a0 >= MIN_PIECE
                    && !across
                {
                    out.push(K::Blocking, group, index, tracks, beam(a0, b0, z));
                }
            }
        }
        if out.0.len() > MAX_MEMBERS {
            return Err(format!("the framing would have more than {MAX_MEMBERS} members: use a wider spacing"));
        }
        Ok(out.0)
    }
}

impl MemberHost for WallFraming {
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
            members.iter().filter_map(|&(k, id)| Some(FramingMember { role: FramingRole::from_key(k)?, id })).collect();
    }

    fn detached_roles(&self) -> Vec<RoleKey> {
        self.detached.iter().map(|r| r.key()).collect()
    }

    fn set_detached_roles(&mut self, roles: &[RoleKey]) {
        self.detached = roles.iter().filter_map(|k| FramingRole::from_key(*k)).collect();
    }

    fn role_fits(&self, role: RoleKey, kind: &ElementKind) -> bool {
        FramingRole::from_key(role).is_some_and(|r| r.fits(kind))
    }

    fn check_parameters(&self, project: &Project) -> Result<(), String> {
        self.check()?;
        self.check_types(project)?;
        self.check_wall(project)
    }

    fn repair_parameters(&mut self, project: &Project) -> Option<usize> {
        self.check_wall(project).ok()?;
        let mut n = self.reset_out_of_range();
        n += fix_type(project, &mut self.stud_type, Category::Column)?;
        n += fix_type(project, &mut self.track_type, Category::Beam)?;
        n += fix_type(project, &mut self.header_type, Category::Beam)?;
        Some(n)
    }

    fn inputs(&self, project: &Project) -> Vec<ElementId> {
        let mut ids = vec![self.wall];
        ids.extend(project.hosted_by(self.wall).map(|e| e.id));
        ids
    }

    fn fed_by(&self, e: &crate::Element) -> bool {
        e.as_wall().is_some() || e.as_hosted().is_some()
    }

    fn member_level(&self, project: &Project) -> Option<LevelId> {
        project.element(self.wall).map(|e| e.level)
    }
}

impl Project {
    /// The framings of `wall` (one in a consistent model).
    pub fn framings_of(&self, wall: ElementId) -> impl Iterator<Item = ElementId> + '_ {
        self.elements
            .values()
            .filter(move |e| matches!(&e.kind, ElementKind::WallFraming(f) if f.wall == wall))
            .map(|e| e.id)
    }

    /// The framing of `wall`, if it has one.
    pub fn wall_framing_of(&self, wall: ElementId) -> Option<ElementId> {
        self.framings_of(wall).next()
    }

    /// One framing per wall: the later ones (higher ids) go, their members
    /// staying as plain elements. Returns how many went.
    pub(crate) fn repair_wall_framings(&mut self) -> usize {
        let mut seen = std::collections::BTreeSet::new();
        let extra: Vec<ElementId> = self
            .elements
            .values()
            .filter_map(|e| match &e.kind {
                ElementKind::WallFraming(f) => (!seen.insert(f.wall)).then_some(e.id),
                _ => None,
            })
            .collect();
        for id in &extra {
            self.elements.remove(id);
        }
        extra.len()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{ColumnSection, Document, Element, Hosted, PlanTransform, Wall};

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

    pub(crate) fn element(id: ElementId, level: LevelId, type_id: ElementTypeId, kind: ElementKind) -> Element {
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

    pub(crate) struct Site {
        pub doc: Document,
        pub wall: ElementId,
        pub host: ElementId,
        /// A 1.2 x 1.2 window type, sill 0.9.
        pub window: ElementTypeId,
        /// A 0.9 x 2.1 door type.
        pub door: ElementTypeId,
    }

    /// A wall 3 m high from (0, 0) to (`length`, 0) on the ground level,
    /// framed (light steel, 0.4 m) with 0.04 x 0.09 studs, 0.04 deep tracks
    /// and 0.14 deep headers; `f` changes the framing before it is added.
    pub(crate) fn site_with(length: f64, f: impl FnOnce(&mut WallFraming)) -> Site {
        let mut doc = Document::new(Project::from_template());
        let (wall, host, window, door) = doc.edit("site", |e| {
            let level = e.project().levels[0].id;
            let material = e.project().materials[0].name.clone();
            let wall_type = e.project().default_type(Category::Wall).expect("a wall type");
            let beam = |width: f64, height: f64| TypeData::Beam {
                width,
                height,
                material: material.clone(),
                shape: Default::default(),
                profile: None,
                profile_rotation_deg: 0.0,
                profile_mirror: false,
            };
            let track = add_type(e, beam(0.09, 0.04));
            let header = add_type(e, beam(0.09, 0.14));
            let stud = add_type(
                e,
                TypeData::Column {
                    section: ColumnSection::Rectangular { width: 0.04, depth: 0.09 },
                    material: material.clone(),
                    profile: None,
                    profile_rotation_deg: 0.0,
                    profile_mirror: false,
                },
            );
            let window = add_type(
                e,
                TypeData::Window { width: 1.2, height: 1.2, default_sill: 0.9, material: material.clone() },
            );
            let door = add_type(e, TypeData::Door { width: 0.9, height: 2.1, material: material.clone() });
            let host_type = add_type(e, TypeData::WallFraming {});
            let wall = ElementId(e.alloc_id());
            let w = Wall { start: DVec2::ZERO, end: DVec2::new(length, 0.0), height: 3.0, ..Default::default() };
            e.add_element(element(wall, level, wall_type, ElementKind::Wall(w)));
            let mut framing = WallFraming::new(wall, FramingSystem::LightSteel, stud, track, header);
            f(&mut framing);
            let host = ElementId(e.alloc_id());
            e.add_element(element(host, level, host_type, ElementKind::WallFraming(Box::new(framing))));
            (wall, host, window, door)
        });
        Site { doc, wall, host, window, door }
    }

    /// Add a window (`type_id == site.window`, sill 0.9) or a door centered
    /// `offset` along the site's wall.
    pub(crate) fn add_opening(site: &mut Site, type_id: ElementTypeId, offset: f64) -> ElementId {
        let (wall, window) = (site.wall, type_id == site.window);
        site.doc.edit("opening", |e| {
            let level = e.project().levels[0].id;
            let sill_height = if window { 0.9 } else { 0.0 };
            let hosted = Hosted { host: wall, offset, sill_height, flip_facing: false, flip_hand: false };
            let kind = if window { ElementKind::Window(hosted) } else { ElementKind::Door(hosted) };
            let id = ElementId(e.alloc_id());
            e.add_element(element(id, level, type_id, kind))
        })
    }

    pub(crate) fn framing(doc: &Document, host: ElementId) -> WallFraming {
        match doc.project.element(host).map(|e| &e.kind) {
            Some(ElementKind::WallFraming(f)) => (**f).clone(),
            _ => panic!("not a wall framing"),
        }
    }

    fn layout(site: &Site) -> Vec<(FramingRole, ElementTypeId, ElementKind)> {
        framing(&site.doc, site.host).layout(&site.doc.project)
    }

    /// Members per kind, in `FramingRoleKind::ALL` order: [bottom tracks,
    /// top tracks, studs, king studs, jack studs, headers, sills, cripples, blocking].
    fn counts(site: &Site) -> [usize; 9] {
        let l = layout(site);
        FramingRoleKind::ALL.map(|k| l.iter().filter(|(r, _, _)| r.kind == k).count())
    }

    fn find(site: &Site, kind: FramingRoleKind, opening: u16, index: u16) -> ElementKind {
        layout(site)
            .into_iter()
            .find(|(r, _, _)| *r == FramingRole { kind, opening, index })
            .map(|(_, _, k)| k)
            .unwrap_or_else(|| panic!("no {kind:?} {opening} {index}"))
    }

    fn column(k: ElementKind) -> Column {
        match k {
            ElementKind::Column(c) => c,
            other => panic!("not a column: {other:?}"),
        }
    }

    fn beam(k: ElementKind) -> Beam {
        match k {
            ElementKind::Beam(b) => b,
            other => panic!("not a beam: {other:?}"),
        }
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn a_bare_wall_gets_studs_at_the_spacing_between_tracks() {
        let site = site_with(3.6, |_| {});
        // 0, 0.4, ..., 3.2 and the end stud.
        assert_eq!(counts(&site), [1, 1, 10, 0, 0, 0, 0, 0, 0]);
        let first = column(find(&site, FramingRoleKind::Stud, 0, 0));
        assert!(near(first.position.x, 0.02) && near(first.position.y, 0.0), "flush with the wall start");
        assert!(near(first.base_offset, 0.04) && near(first.height, 2.92), "between the tracks");
        assert!(near(first.rotation_deg, 0.0), "turned with the wall");
        assert!(near(column(find(&site, FramingRoleKind::Stud, 0, 1)).position.x, 0.4));
        assert!(near(column(find(&site, FramingRoleKind::Stud, 0, END_STUD)).position.x, 3.58));
        let bottom = beam(find(&site, FramingRoleKind::BottomTrack, 0, 0));
        assert!(near(bottom.start.x, 0.0) && near(bottom.end.x, 3.6) && near(bottom.top_offset, 0.04));
        assert!(!bottom.join_start && !bottom.join_end, "tracks keep their length");
        assert!(near(beam(find(&site, FramingRoleKind::TopTrack, 0, 0)).top_offset, 3.0));
    }

    #[test]
    fn a_window_gets_kings_jacks_a_header_a_sill_and_cripples() {
        let mut site = site_with(3.6, |_| {});
        let window = site.window;
        add_opening(&mut site, window, 1.8);
        // The studs at 1.2, 1.6, 2.0 and 2.4 give way to the opening's framing.
        assert_eq!(counts(&site), [1, 1, 6, 2, 2, 1, 1, 4, 0]);
        for k in 3..=6 {
            assert!(
                layout(&site)
                    .iter()
                    .all(|(r, _, _)| *r != FramingRole { kind: FramingRoleKind::Stud, opening: 0, index: k })
            );
        }
        assert!(near(column(find(&site, FramingRoleKind::KingStud, 0, 0)).position.x, 1.14));
        assert!(near(column(find(&site, FramingRoleKind::KingStud, 0, 1)).position.x, 2.46));
        let jack = column(find(&site, FramingRoleKind::JackStud, 0, 1));
        assert!(near(jack.position.x, 2.42) && near(jack.base_offset, 0.04) && near(jack.height, 2.06));
        let header = beam(find(&site, FramingRoleKind::Header, 0, 0));
        assert!(near(header.start.x, 1.16) && near(header.end.x, 2.44) && near(header.top_offset, 2.24));
        let sill = beam(find(&site, FramingRoleKind::Sill, 0, 0));
        assert!(near(sill.start.x, 1.2) && near(sill.end.x, 2.4) && near(sill.top_offset, 0.9));
        // At 1.6 (k = 4): above the header (index 8) and below the sill (index 9).
        let above = column(find(&site, FramingRoleKind::Cripple, 0, 8));
        assert!(near(above.position.x, 1.6) && near(above.base_offset, 2.24) && near(above.height, 0.72));
        let below = column(find(&site, FramingRoleKind::Cripple, 0, 9));
        assert!(near(below.base_offset, 0.04) && near(below.height, 0.72));
    }

    #[test]
    fn a_door_has_no_sill_and_no_cripples_below() {
        let mut site = site_with(3.6, |_| {});
        let door = site.door;
        add_opening(&mut site, door, 1.0);
        assert_eq!(counts(&site), [1, 1, 8, 2, 2, 1, 0, 2, 0]);
        assert!(near(column(find(&site, FramingRoleKind::JackStud, 0, 0)).height, 2.06));
    }

    #[test]
    fn wood_gets_a_double_top_plate_and_light_steel_does_not() {
        let site = site_with(3.6, |f| (f.system, f.double_top) = (FramingSystem::Wood, true));
        assert_eq!(counts(&site)[1], 2);
        assert!(near(beam(find(&site, FramingRoleKind::TopTrack, 0, 1)).top_offset, 2.96));
        assert!(near(column(find(&site, FramingRoleKind::Stud, 0, 0)).height, 2.88));
        let site = site_with(3.6, |f| f.double_top = true);
        assert_eq!(counts(&site)[1], 1, "double top is wood only");
        assert!(FramingSystem::Wood.default_double_top() && !FramingSystem::LightSteel.default_double_top());
    }

    #[test]
    fn blocking_runs_between_full_height_studs_but_not_across_openings() {
        let mut site = site_with(3.6, |f| f.blocking = true);
        assert_eq!(counts(&site)[8], 9);
        let first = beam(find(&site, FramingRoleKind::Blocking, 0, 0));
        assert!(near(first.start.x, 0.04) && near(first.end.x, 0.38) && near(first.top_offset, 1.52));
        let window = site.window;
        add_opening(&mut site, window, 1.8);
        assert_eq!(counts(&site)[8], 6);
        // After the end-side king of opening 0.
        let after_king = beam(find(&site, FramingRoleKind::Blocking, 1, 0));
        assert!(near(after_king.start.x, 2.48));
    }

    #[test]
    fn an_opening_at_the_wall_start_is_clipped() {
        let mut site = site_with(3.6, |_| {});
        let window = site.window;
        add_opening(&mut site, window, 0.3);
        let c = counts(&site);
        assert_eq!((c[3], c[4]), (1, 1), "only the end-side king and jack");
        assert!(near(beam(find(&site, FramingRoleKind::Header, 0, 0)).start.x, 0.0));
    }

    #[test]
    fn curved_short_and_out_of_range_get_nothing() {
        let mut site = site_with(3.6, |_| {});
        let wall = site.wall;
        site.doc.edit("bend", |e| {
            if let Some(ElementKind::Wall(w)) = e.element_mut(wall).map(|x| &mut x.kind) {
                w.bulge = 0.2;
            }
        });
        assert!(layout(&site).is_empty(), "curved");
        assert!(layout(&site_with(0.05, |_| {})).is_empty(), "shorter than two studs");
        let mut f = framing(&site.doc, site.host);
        for bad in [0.1, 1.3, f64::NAN] {
            f.spacing = bad;
            assert!(f.check().is_err() && f.layout(&site.doc.project).is_empty(), "{bad}");
        }
        assert_eq!(f.reset_out_of_range(), 1);
        assert!(near(f.spacing, LIGHT_STEEL_SPACING));
        (f.system, f.spacing) = (FramingSystem::Wood, 5.0);
        f.reset_out_of_range();
        assert!(near(f.spacing, WOOD_SPACING));
    }

    #[test]
    fn layouts_over_the_limit_are_refused_not_cut_short() {
        let site = site_with(2000.0, |f| f.spacing = 0.2);
        let f = framing(&site.doc, site.host);
        assert!(layout(&site).is_empty());
        assert!(f.check_size(&site.doc.project).is_err_and(|e| e.contains("members")));
        let site = site_with(700.0, |f| f.spacing = 0.2);
        let n = layout(&site).len();
        assert!(n > 3000 && n <= MAX_MEMBERS, "{n}");
        assert!(framing(&site.doc, site.host).check_size(&site.doc.project).is_ok());
    }

    #[test]
    fn systems_and_roles() {
        for s in FramingSystem::ALL {
            assert_eq!(FramingSystem::from_code(s.code()), Some(s));
        }
        assert_eq!(FramingSystem::from_code(" WOOD "), Some(FramingSystem::Wood));
        assert_eq!(FramingSystem::from_code("steel"), None);
        let stud = FramingRole { kind: FramingRoleKind::Stud, opening: 0, index: 1 };
        assert!(stud.fits(&ElementKind::Column(column(find(&site_with(3.6, |_| {}), FramingRoleKind::Stud, 0, 1)))));
        let header = FramingRole { kind: FramingRoleKind::Header, opening: 0, index: 0 };
        assert!(!header.fits(&ElementKind::Column(column(find(&site_with(3.6, |_| {}), FramingRoleKind::Stud, 0, 1)))));
    }

    fn member(f: &WallFraming, kind: FramingRoleKind, opening: u16, index: u16) -> ElementId {
        let role = FramingRole { kind, opening, index };
        f.members.iter().find(|m| m.role == role).map(|m| m.id).unwrap_or_else(|| panic!("no {role:?}"))
    }

    fn set(doc: &mut Document, host: ElementId, f: impl FnOnce(&mut WallFraming)) {
        doc.edit("set", |e| {
            if let Some(ElementKind::WallFraming(x)) = e.element_mut(host).map(|x| &mut x.kind) {
                f(x);
            }
        });
    }

    fn of_kind(f: &WallFraming, kind: FramingRoleKind) -> usize {
        f.members.iter().filter(|m| m.role.kind == kind).count()
    }

    fn position(doc: &Document, id: ElementId) -> DVec2 {
        match doc.project.element(id).map(|e| &e.kind) {
            Some(ElementKind::Column(c)) => c.position,
            _ => panic!("not a column"),
        }
    }

    #[test]
    fn role_keys_round_trip_in_order() {
        let roles = [
            FramingRole { kind: FramingRoleKind::BottomTrack, opening: 0, index: 0 },
            FramingRole { kind: FramingRoleKind::Stud, opening: 0, index: END_STUD },
            FramingRole { kind: FramingRoleKind::KingStud, opening: 2, index: 1 },
            FramingRole { kind: FramingRoleKind::Blocking, opening: 3, index: 0 },
        ];
        for w in roles.windows(2) {
            assert!(w[0] < w[1] && w[0].key() < w[1].key());
        }
        for r in roles {
            assert_eq!(FramingRole::from_key(r.key()), Some(r));
        }
        assert_eq!(FramingRole::from_key(RoleKey::new(9, 0, 0)), None);
    }

    #[test]
    fn a_new_framing_gets_its_members() {
        let site = site_with(3.6, |_| {});
        let f = framing(&site.doc, site.host);
        assert_eq!(f.members.len(), 12);
        let level = site.doc.project.element(site.wall).map(|e| e.level);
        for m in &f.members {
            let e = site.doc.project.element(m.id).expect("the member exists");
            assert!(m.role.fits(&e.kind));
            assert_eq!(Some(e.level), level);
        }
        assert_eq!(site.doc.project.member_host_of(f.members[0].id), Some(site.host));
        assert_eq!(site.doc.project.wall_framing_of(site.wall), Some(site.host));
        assert_eq!(site.doc.project.clone().repair(), 0, "a consistent model");
    }

    #[test]
    fn moving_the_wall_moves_the_studs_and_keeps_their_ids() {
        let mut site = site_with(3.6, |_| {});
        let before = framing(&site.doc, site.host);
        let stud = member(&before, FramingRoleKind::Stud, 0, 1);
        let wall = site.wall;
        site.doc.edit("move", |e| e.transform_elements(&[wall], &PlanTransform::translate(DVec2::new(0.0, 2.0))));
        let after = framing(&site.doc, site.host);
        assert_eq!(member(&after, FramingRoleKind::Stud, 0, 1), stud, "same id");
        assert!(position(&site.doc, stud).distance(DVec2::new(0.4, 2.0)) < 1e-9);
        assert!(after.detached.is_empty(), "moved by its wall, not detached");
        assert!(site.doc.undo());
        assert_eq!(framing(&site.doc, site.host), before);
    }

    #[test]
    fn a_window_regenerates_the_framing_and_undo_restores_it() {
        let mut site = site_with(3.6, |_| {});
        let before = framing(&site.doc, site.host);
        let start = site.doc.project.elements.clone();
        let kept = member(&before, FramingRoleKind::Stud, 0, 1);
        let dropped = member(&before, FramingRoleKind::Stud, 0, 4);
        let window = site.window;
        add_opening(&mut site, window, 1.8);
        let f = framing(&site.doc, site.host);
        assert_eq!(
            [FramingRoleKind::Stud, FramingRoleKind::KingStud, FramingRoleKind::JackStud].map(|k| of_kind(&f, k)),
            [6, 2, 2]
        );
        assert_eq!(
            [FramingRoleKind::Header, FramingRoleKind::Sill, FramingRoleKind::Cripple].map(|k| of_kind(&f, k)),
            [1, 1, 4]
        );
        assert_eq!(member(&f, FramingRoleKind::Stud, 0, 1), kept);
        assert!(site.doc.project.element(dropped).is_none(), "the stud inside the opening is gone");
        assert!(site.doc.undo());
        assert_eq!(site.doc.project.elements, start);
    }

    #[test]
    fn moving_one_opening_keeps_the_keys_of_the_other() {
        let mut site = site_with(6.0, |_| {});
        let window = site.window;
        add_opening(&mut site, window, 1.2);
        let second = add_opening(&mut site, window, 4.0);
        let f = framing(&site.doc, site.host);
        let (first_header, second_header) =
            (member(&f, FramingRoleKind::Header, 0, 0), member(&f, FramingRoleKind::Header, 1, 0));
        site.doc.edit("slide", |e| {
            if let Some(h) = e.element_mut(second).and_then(|x| x.as_hosted_mut()) {
                h.offset = 4.4;
            }
        });
        let f = framing(&site.doc, site.host);
        assert_eq!(member(&f, FramingRoleKind::Header, 0, 0), first_header);
        assert_eq!(member(&f, FramingRoleKind::Header, 1, 0), second_header);
        let Some(ElementKind::Beam(b)) = site.doc.project.element(second_header).map(|e| &e.kind) else {
            panic!("a beam")
        };
        assert!((b.start.x - 3.76).abs() < 1e-9, "moved with its window");
    }

    #[test]
    fn deleting_the_window_brings_the_studs_back() {
        let mut site = site_with(3.6, |_| {});
        let window = site.window;
        let w = add_opening(&mut site, window, 1.8);
        site.doc.edit("delete", |e| e.remove_element(w));
        let f = framing(&site.doc, site.host);
        assert_eq!((of_kind(&f, FramingRoleKind::Stud), of_kind(&f, FramingRoleKind::KingStud)), (10, 0));
    }

    #[test]
    fn a_window_type_edit_regenerates_the_header() {
        let mut site = site_with(3.6, |_| {});
        let window = site.window;
        add_opening(&mut site, window, 1.8);
        let header = member(&framing(&site.doc, site.host), FramingRoleKind::Header, 0, 0);
        site.doc.edit("wider", |e| {
            if let Some(TypeData::Window { width, .. }) = e.type_mut(window).map(|t| &mut t.data) {
                *width = 1.6;
            }
        });
        let Some(ElementKind::Beam(b)) = site.doc.project.element(header).map(|e| &e.kind) else { panic!("a beam") };
        assert!((b.start.x - 0.96).abs() < 1e-9, "1.0 - 0.04");
    }

    #[test]
    fn deleting_the_wall_deletes_the_framing_and_its_members() {
        let mut site = site_with(3.6, |_| {});
        let ids = site.doc.project.host_members_of(site.host);
        assert_eq!(ids.len(), 12);
        let wall = site.wall;
        site.doc.edit("delete", |e| e.remove_element(wall));
        assert!(site.doc.project.element(site.host).is_none());
        assert!(ids.iter().all(|id| site.doc.project.element(*id).is_none()));
        assert!(site.doc.undo());
        assert!(ids.iter().all(|id| site.doc.project.element(*id).is_some()));
        assert!(site.doc.project.element(site.host).is_some());
    }

    #[test]
    fn a_detached_stud_is_left_alone() {
        let mut site = site_with(3.6, |_| {});
        let stud = member(&framing(&site.doc, site.host), FramingRoleKind::Stud, 0, 2);
        site.doc.edit("move", |e| e.transform_elements(&[stud], &PlanTransform::translate(DVec2::new(0.0, 1.0))));
        let role = FramingRole { kind: FramingRoleKind::Stud, opening: 0, index: 2 };
        assert!(framing(&site.doc, site.host).detached.contains(&role));
        set(&mut site.doc, site.host, |f| f.spacing = 0.4 + 1e-6);
        assert!(position(&site.doc, stud).distance(DVec2::new(0.8, 1.0)) < 1e-9, "where the user left it");
        assert_eq!(framing(&site.doc, site.host).members.len(), 11, "and not made again");
    }

    #[test]
    fn the_members_follow_the_wall_to_another_level() {
        let mut site = site_with(3.6, |_| {});
        let upper = site.doc.project.levels[1].id;
        let wall = site.wall;
        site.doc.edit("level", |e| {
            if let Some(w) = e.element_mut(wall) {
                w.level = upper;
            }
        });
        let ids = site.doc.project.host_members_of(site.host);
        assert!(ids.iter().all(|id| site.doc.project.element(*id).is_some_and(|e| e.level == upper)));
    }

    #[test]
    fn a_copied_wall_takes_its_framing_along_and_a_framing_alone_is_not_copied() {
        let mut site = site_with(3.6, |_| {});
        let (wall, host) = (site.wall, site.host);
        let originals = site.doc.project.host_members_of(host);
        let copies =
            site.doc.edit("copy", |e| e.copy_elements(&[wall], &[PlanTransform::translate(DVec2::new(0.0, 5.0))]));
        let new_wall =
            copies.iter().copied().find(|id| site.doc.project.element(*id).is_some_and(|e| e.as_wall().is_some()));
        let new_wall = new_wall.expect("the wall's copy");
        let copy = site.doc.project.wall_framing_of(new_wall).expect("its framing's copy");
        assert_ne!(copy, host);
        let f = framing(&site.doc, copy);
        assert_eq!(f.members.len(), 12);
        assert!(f.members.iter().all(|m| !originals.contains(&m.id)));
        assert_eq!(framing(&site.doc, host).members.len(), 12, "the original keeps its own");
        let alone =
            site.doc.edit("copy", |e| e.copy_elements(&[host], &[PlanTransform::translate(DVec2::new(0.0, 9.0))]));
        assert!(alone.is_empty(), "its wall has one already");
    }

    #[test]
    fn repair_keeps_one_framing_per_wall_and_drops_those_without_a_wall() {
        let site = site_with(3.6, |_| {});
        let mut p = site.doc.project.clone();
        let mut twin = p.element(site.host).cloned().expect("the framing");
        twin.id = ElementId(p.alloc_id());
        if let ElementKind::WallFraming(f) = &mut twin.kind {
            (f.members, f.detached) = (Vec::new(), Vec::new());
        }
        p.elements.insert(twin.id, twin.clone());
        assert_eq!(p.repair(), 1, "the later twin goes");
        assert!(p.element(twin.id).is_none() && p.element(site.host).is_some());
        assert_eq!(p.repair(), 0);
        let mut p = site.doc.project.clone();
        p.elements.remove(&site.wall);
        assert_eq!(p.repair(), 1, "the framing of a missing wall goes (its members stay as plain elements)");
        assert!(p.element(site.host).is_none());
        assert_eq!(p.repair(), 0);
    }
}
