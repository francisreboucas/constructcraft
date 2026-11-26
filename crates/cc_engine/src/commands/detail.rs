//! Detail lines: 2D lines, arcs, circles and ellipses drawn in one view
//! (see [`cc_core::detail`]). They are moved, copied, rotated, mirrored,
//! trimmed, split, offset and deleted with the generic commands, and
//! `element_get`/`element_set` show and change their curve and style.

use super::sketch::CurveArg;
use super::{add_element, edit, ids_json};
use crate::error::{EngineError, Result};
use crate::refs::{self, Point, Ref, finite, point, positive};
use crate::registry::{Command, CommandKind};
use crate::session::Session;
use crate::t;
use cc_core::detail::{DetailLine, DetailStyle, LineStyleRef};
use cc_core::sketch::SketchCurve;
use cc_core::{Category, ElementKind, ElementType, ElementTypeId, TypeData};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Most curves one `detail_line_create` makes.
pub const MAX_CURVES: usize = cc_core::sketch::MAX_CURVES;

/// The detail line type, creating one if the project has none.
fn detail_type(e: &mut cc_core::EditCtx) -> ElementTypeId {
    if let Some(t) = e.project().default_type(Category::DetailLine) {
        return t;
    }
    let id = ElementTypeId(e.alloc_id());
    e.add_type(ElementType {
        id,
        name: t!("tpl-type-detail-line"),
        data: TypeData::DetailLine {},
        info: Default::default(),
        uid: Default::default(),
    })
}

/// A line style named by its code: a built-in style's code, or the name
/// of one of the project's line styles.
pub fn style_arg(p: &cc_core::Project, code: &str) -> Result<LineStyleRef> {
    if let Some(s) = DetailStyle::from_code(code) {
        return Ok(LineStyleRef::BuiltIn(s));
    }
    if let Some(s) = p.line_styles.values().find(|s| s.name.trim().eq_ignore_ascii_case(code.trim())) {
        return Ok(LineStyleRef::Project(s.id));
    }
    let codes: Vec<String> = DetailStyle::ALL
        .iter()
        .map(|s| s.code().to_string())
        .chain(p.line_styles.values().map(|s| s.name.clone()))
        .collect();
    Err(EngineError::invalid(format!("unknown line style '{code}' ({})", codes.join(", "))))
}

/// The view (and its level) detail lines go in: a plan, ceiling plan,
/// elevation, section, legend or drafting view.
fn detail_view(p: &cc_core::Project, r: &Ref) -> Result<(cc_core::ViewId, cc_core::LevelId)> {
    let id = refs::view(p, r)?;
    let view = p.view(id).ok_or_else(|| EngineError::not_found("view"))?;
    if !cc_views::hosts_cad(&view.kind) {
        return Err(EngineError::invalid(
            "detail lines go in plans, ceiling plans, elevations, sections, legends and drafting views",
        ));
    }
    let level =
        view.level().or_else(|| p.levels.first().map(|l| l.id)).ok_or_else(|| EngineError::rejected("no levels"))?;
    Ok((id, level))
}

/// Draw detail lines in a 2D view (plan, ceiling plan, elevation, section,
/// legend or drafting view): lines, arcs, circles and ellipses in the
/// view's drawing coordinates (meters, as for dimensions and text notes),
/// shown only in that view, on top of the model. Each curve becomes one
/// element (a polyline is its segments; rectangles and polygons their
/// sides), all in one undo step. A `radius` rounds the corners between
/// joined straight lines and an `offset` moves each chain sideways, as for
/// `walls_create` (ellipses then become splines). Detail lines are left out
/// of 3D views and IFC export; DXF export puts them on a layer named after
/// their style.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DetailLineCreate {
    /// View id or name.
    pub view: Ref,
    /// The curves to draw, as for sketches (`line`, `arc`, `circle`,
    /// `ellipse`, `rectangle`, `polygon`).
    pub curves: Vec<CurveArg>,
    /// Line style: thin (default), medium, wide, hidden, dashed, center or
    /// dotted, or the name of one of the project's line styles
    /// (`line_styles_list`).
    #[serde(default)]
    pub style: Option<String>,
    /// How far the lines lie from the curves given, meters: to the left of
    /// an open chain, outside a closed one, negative the other way
    /// (default 0).
    #[serde(default)]
    pub offset: Option<f64>,
    /// Radius in meters of the arcs that round the corners between joined
    /// straight lines (default 0: sharp corners).
    #[serde(default)]
    pub radius: Option<f64>,
}

impl Command for DetailLineCreate {
    const NAME: &'static str = "detail_line_create";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let (view, level) = detail_view(s.project(), &self.view)?;
        let style = self.style.as_deref().map(|c| style_arg(s.project(), c)).transpose()?.unwrap_or_default();
        let offset = finite(self.offset.unwrap_or(0.0), "offset")?;
        let radius = finite(self.radius.unwrap_or(0.0), "radius")?;
        if radius < 0.0 {
            return Err(EngineError::invalid("the radius cannot be negative"));
        }
        let mut curves = Vec::new();
        for arg in &self.curves {
            curves.extend(arg.curves()?);
            if curves.len() > MAX_CURVES {
                return Err(EngineError::invalid(format!("at most {MAX_CURVES} curves at once")));
            }
        }
        if offset != 0.0 || radius > 0.0 {
            curves = super::wall_draw::location_curves(&curves, offset, radius)?;
        }
        if curves.len() > MAX_CURVES {
            return Err(EngineError::invalid(format!("at most {MAX_CURVES} curves at once")));
        }
        let mut lines = Vec::with_capacity(curves.len());
        for curve in curves {
            let line = DetailLine { view, curve, style };
            if !line.is_valid() {
                return Err(EngineError::invalid("a curve is out of bounds"));
            }
            lines.push(line);
        }
        if lines.is_empty() {
            return Err(EngineError::invalid("give at least one curve"));
        }
        let ids = edit(s, t!("tx-create-detail-lines"), |e| {
            let type_id = detail_type(e);
            Ok(lines
                .into_iter()
                .map(|l| add_element(e, level, type_id, ElementKind::DetailLine(l)))
                .collect::<Vec<_>>())
        })?;
        Ok(json!({ "created": ids_json(&ids) }))
    }
}

/// Round the corner between two straight detail lines of one view with an
/// arc of `radius`: both are trimmed or extended to meet (keeping the part
/// each pick point is on, as `trim_corner` does), then cut back to where an
/// arc of `radius` tangent to both leaves them, and the arc, a detail line
/// in the style of the first, fills the gap. One undo step; returns the arc.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DetailLineFillet {
    /// First detail line id.
    pub a: u64,
    /// A point on the part of `a` to keep.
    pub pick_a: Point,
    /// Second detail line id.
    pub b: u64,
    /// A point on the part of `b` to keep.
    pub pick_b: Point,
    /// Radius of the arc, meters.
    pub radius: f64,
}

impl Command for DetailLineFillet {
    const NAME: &'static str = "detail_line_fillet";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let (a, b) = (refs::element(p, self.a)?, refs::element(p, self.b)?);
        if a == b {
            return Err(EngineError::invalid("pick two different detail lines"));
        }
        super::modify::not_pinned(p, a)?;
        super::modify::not_pinned(p, b)?;
        let radius = positive(self.radius, "radius")?;
        let line = |id| match p.element(id).map(|e| &e.kind) {
            Some(ElementKind::DetailLine(l)) if matches!(l.curve, SketchCurve::Line { .. }) => Some(l.view),
            _ => None,
        };
        let (Some(va), Some(vb)) = (line(a), line(b)) else {
            return Err(EngineError::invalid("a fillet joins two straight detail lines"));
        };
        if va != vb {
            return Err(EngineError::invalid("the detail lines are in different views"));
        }
        let (pa, pb) = (point(self.pick_a)?, point(self.pick_b)?);
        let arc = edit(s, t!("tx-detail-line-fillet"), |e| {
            e.trim_corner(a, pa, b, pb)?;
            let detail = |e: &cc_core::EditCtx, id| match e.project().element(id).map(|el| &el.kind) {
                Some(ElementKind::DetailLine(l)) => Some(l.clone()),
                _ => None,
            };
            let ends = |l: &DetailLine| match l.curve {
                SketchCurve::Line { a, b } => Some((a, b)),
                _ => None,
            };
            let (Some(la), Some(lb)) = (detail(e, a), detail(e, b)) else {
                return Err(EngineError::not_found("detail line"));
            };
            let (Some(sa), Some(sb)) = (ends(&la), ends(&lb)) else {
                return Err(EngineError::invalid("a fillet joins two straight detail lines"));
            };
            let corner = super::wall_draw::round_corner(sa, sb, radius, "lines")?;
            let set = |e: &mut cc_core::EditCtx, id, start: bool, to| {
                if let Some(ElementKind::DetailLine(l)) = e.element_mut(id).map(|el| &mut el.kind)
                    && let SketchCurve::Line { a, b } = &mut l.curve
                {
                    if start { *a = to } else { *b = to }
                }
            };
            set(e, a, corner.a_start, corner.ta);
            set(e, b, corner.b_start, corner.tb);
            let (level, type_id) = e
                .project()
                .element(a)
                .map(|el| (el.level, el.type_id))
                .ok_or_else(|| EngineError::not_found("detail line"))?;
            let arc = DetailLine {
                view: la.view,
                curve: SketchCurve::Arc { a: corner.ta, b: corner.tb, bulge: corner.bulge },
                style: la.style,
            };
            if !arc.is_valid() {
                return Err(EngineError::invalid("the arc is out of bounds"));
            }
            Ok(add_element(e, level, type_id, ElementKind::DetailLine(arc)))
        })?;
        Ok(json!({ "created": arc.0, "changed": [a.0, b.0] }))
    }
}

/// A `params` patch for a detail line with its style given by code
/// (`"style": "wide"`) turned into the model's form.
pub(crate) fn normalize_patch(p: &cc_core::Project, patch: &Value) -> Result<Value> {
    let mut patch = patch.clone();
    if let Some(code) = patch.get("style").and_then(Value::as_str) {
        let style = serde_json::to_value(style_arg(p, code)?).map_err(|e| EngineError::invalid(e.to_string()))?;
        if let Some(o) = patch.as_object_mut() {
            o.insert("style".into(), style);
        }
    }
    Ok(patch)
}

/// Check a patched detail line: the view stays, the curve is valid.
pub(crate) fn check(old: &DetailLine, new: &DetailLine) -> Result<()> {
    if new.view != old.view {
        return Err(EngineError::invalid("the view of a detail line cannot be changed"));
    }
    if !new.is_valid() {
        return Err(EngineError::invalid("the curve has no size or is out of bounds"));
    }
    Ok(())
}
