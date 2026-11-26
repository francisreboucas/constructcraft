use crate::drawing_index::DrawingIndex;
use cc_core::ElementId;
use glam::DVec2;

/// Graphic weight of a line, resolved to pen widths by the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineStyle {
    /// Element cut by the view plane.
    Cut,
    /// Visible element beyond the cut plane.
    Projection,
    /// Fine detail (layers, glass, door swings).
    Thin,
    /// Dashed: element above the cut plane or hidden.
    Hidden,
    /// Levels, section marks, text leaders.
    Annotation,
    /// Dash-dot: grid lines and center lines.
    Center,
    /// Background linework (the architecture under building services).
    Faint,
    /// Lines of a cut pattern (drawn like `Thin`; view graphics replace them
    /// when they override the cut pattern).
    Hatch,
}

/// Graphic overrides of one line (view graphics); `None` keeps the style's pen.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pen {
    pub color: Option<[u8; 3]>,
    /// Width on paper, millimeters.
    pub weight: Option<f64>,
    pub pattern: Option<cc_core::LinePattern>,
    /// Mixed with white.
    pub halftone: bool,
    /// The function of the layer the line belongs to (a cut wall's layer
    /// lines in plans), so view graphics give it its subcategory's lines (#452).
    pub layer: Option<cc_core::LayerFunction>,
}

/// A line's pen resolved: width in paper millimeters, color and dash lengths
/// in paper millimeters (empty: continuous).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedPen {
    pub width: f64,
    pub color: [u8; 3],
    pub dashes: &'static [f64],
}

impl LineStyle {
    /// The default pen of the style: width (paper mm) and color.
    pub fn pen(self) -> (f64, [u8; 3]) {
        match self {
            LineStyle::Cut => (0.5, [10, 10, 10]),
            LineStyle::Projection => (0.25, [45, 45, 45]),
            LineStyle::Thin | LineStyle::Hatch => (0.13, [95, 95, 95]),
            LineStyle::Hidden => (0.18, [120, 120, 120]),
            LineStyle::Annotation => (0.18, [30, 90, 170]),
            LineStyle::Center => (0.18, [30, 90, 170]),
            LineStyle::Faint => (0.13, [175, 175, 175]),
        }
    }

    /// Dash pattern of the style, paper millimeters.
    pub fn dashes(self) -> &'static [f64] {
        match self {
            LineStyle::Hidden => cc_core::LinePattern::ShortDash.dashes_mm(),
            LineStyle::Center => cc_core::LinePattern::DashDot.dashes_mm(),
            _ => &[],
        }
    }

    /// The pen of a line of this style with its overrides.
    pub fn resolve(self, pen: Option<&Pen>) -> ResolvedPen {
        let (width, color) = self.pen();
        let mut out = ResolvedPen { width, color, dashes: self.dashes() };
        if let Some(p) = pen {
            if let Some(c) = p.color {
                out.color = c;
            }
            if let Some(w) = p.weight.filter(|w| w.is_finite() && *w > 0.0) {
                out.width = w.min(10.0);
            }
            if let Some(pattern) = p.pattern {
                out.dashes = pattern.dashes_mm();
            }
            if p.halftone {
                out.color = halftone(out.color);
            }
        }
        out
    }
}

/// A color mixed two thirds with white (faint drawing).
pub fn halftone(c: [u8; 3]) -> [u8; 3] {
    c.map(|v| (f64::from(v) + (255.0 - f64::from(v)) * 0.66).round() as u8)
}

/// Which point of a text box sits at its position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    #[default]
    LeftBottom,
    CenterBottom,
    /// The end of the baseline (right-aligned).
    RightBottom,
    Center,
}

/// Drawing primitive, in view coordinates (meters on the view plane).
#[derive(Debug, Clone, PartialEq)]
pub enum Prim {
    Line {
        a: DVec2,
        b: DVec2,
        style: LineStyle,
        element: Option<ElementId>,
        /// Overrides from the view's graphics.
        pen: Option<Pen>,
    },
    /// Simple polygon fill (any winding, may be concave).
    Fill {
        pts: Vec<DVec2>,
        color: [u8; 4],
        element: Option<ElementId>,
        /// A cut region (plans, sections), not a surface seen beyond the cut.
        cut: bool,
    },
    /// Text placed by `anchor` at `pos`, rotated by `angle` radians
    /// (counter-clockwise); `size` is paper height in millimeters.
    Text {
        pos: DVec2,
        text: String,
        size: f64,
        anchor: Anchor,
        angle: f64,
        /// The element the text annotates (hidden with it).
        element: Option<ElementId>,
        /// The text's own color (`None`: the annotation ink, or what the
        /// view's graphics give its element).
        color: Option<[u8; 3]>,
    },
    /// A picture filling the parallelogram from `origin` (its bottom-left
    /// corner) along `u` (its width) and `v` (its height).
    Image {
        origin: DVec2,
        u: DVec2,
        v: DVec2,
        image: std::sync::Arc<cc_core::raster::RasterImage>,
        element: Option<ElementId>,
        /// Mixed two thirds with white.
        faded: bool,
    },
}

impl Prim {
    /// The primitive with its points moved by the affine map `f` (an image's
    /// sides follow from where its corners land).
    pub fn mapped(&self, f: impl Fn(DVec2) -> DVec2) -> Prim {
        match self {
            Prim::Line { a, b, style, element, pen } => {
                Prim::Line { a: f(*a), b: f(*b), style: *style, element: *element, pen: *pen }
            }
            Prim::Fill { pts, color, element, cut } => {
                Prim::Fill { pts: pts.iter().map(|p| f(*p)).collect(), color: *color, element: *element, cut: *cut }
            }
            Prim::Text { pos, text, size, anchor, angle, element, color } => Prim::Text {
                pos: f(*pos),
                text: text.clone(),
                size: *size,
                anchor: *anchor,
                angle: *angle,
                element: *element,
                color: *color,
            },
            Prim::Image { origin, u, v, image, element, faded } => {
                let o = f(*origin);
                Prim::Image {
                    origin: o,
                    u: f(*origin + *u) - o,
                    v: f(*origin + *v) - o,
                    image: image.clone(),
                    element: *element,
                    faded: *faded,
                }
            }
        }
    }

    /// The four corners of an image (bottom-left first, counter-clockwise when not mirrored).
    pub fn image_corners(origin: DVec2, u: DVec2, v: DVec2) -> [DVec2; 4] {
        [origin, origin + u, origin + u + v, origin + v]
    }

    /// The element a primitive belongs to, if any.
    pub fn element(&self) -> Option<ElementId> {
        match self {
            Prim::Line { element, .. }
            | Prim::Fill { element, .. }
            | Prim::Text { element, .. }
            | Prim::Image { element, .. } => *element,
        }
    }
}

/// The most elements [`Drawing2D::hits_at`] gives.
pub const MAX_HITS: usize = 64;

/// Ordered list of primitives, painted first to last.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Drawing2D {
    pub prims: Vec<Prim>,
    /// Fit the view to the geometry of elements only (plans: the markers of
    /// elevations and sections stand outside the model).
    pub fit_elements: bool,
    /// Elements drawn as background (imported CAD drawings): picked only
    /// where nothing else is.
    pub background: std::collections::BTreeSet<ElementId>,
    /// Datum elements (grids, levels): their lines are picked after any
    /// model line within reach, however near (see [`Self::hits_at`]).
    pub datum: std::collections::BTreeSet<ElementId>,
    /// Elements whose outline often lies on another's (floors, ceilings,
    /// roofs, foundations, terrain, pads): among lines as near, picked last.
    pub yielding: std::collections::BTreeSet<ElementId>,
    /// Tags that can be dragged: where each is and how far it was moved.
    pub tags: Vec<TagHandle>,
    /// Labels of fittings in services plans that can be dragged, by key
    /// ([`cc_core::tag::fitting_key`]).
    pub fitting_tags: Vec<FittingTagHandle>,
    /// How many of the first `prims` are the plans of other levels drawn
    /// under a plan ([`cc_core::Underlay`]): halftone, with no element, not
    /// picked or snapped to.
    pub underlay: usize,
    /// How many of the last `prims` are an overlay drawn over the view (the
    /// sun path diagram, [`crate::sun_path`]): with no element, not picked
    /// or snapped to, but part of the extents to fit the view to.
    pub overlay: usize,
    /// The marks of sections in a plan: picked, moved and flipped there.
    pub sections: Vec<SectionHandle>,
    /// The marks of callouts drawn in the view: picked, moved and resized there.
    pub callouts: Vec<crate::callout::CalloutHandle>,
    /// The markers of elevations in a plan: picked and moved there.
    pub marks: Vec<MarkHandle>,
    /// The bubbles of grids: picked and moved in the view.
    pub bubbles: Vec<BubbleHandle>,
    /// The arrows of interior elevation markers in a plan: a double click
    /// opens the arrow's view.
    pub interior_arrows: Vec<InteriorArrow>,
    /// The levels of an elevation or a section: picked, and their heads
    /// moved, in the view.
    pub levels: Vec<LevelHandle>,
    /// Lookups over `prims`, built the first time they are needed (see
    /// [`DrawingIndex`]); a copy of the drawing starts without them.
    pub index: DrawingIndex,
}

/// An arrow of an interior elevation marker, as drawn: picked within
/// `radius` of `center`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InteriorArrow {
    pub view: cc_core::ViewId,
    pub center: DVec2,
    pub radius: f64,
}

/// The label of a fitting, as drawn: its key in the view's
/// `fitting_placements` and its box (`tag.element` is the fitting's first run).
#[derive(Debug, Clone, PartialEq)]
pub struct FittingTagHandle {
    pub key: String,
    pub tag: TagHandle,
}

/// A section's mark in a plan, as drawn (meters on the plan).
#[derive(Debug, Clone, PartialEq)]
pub struct SectionHandle {
    pub view: cc_core::ViewId,
    pub start: DVec2,
    pub end: DVec2,
    /// The view's split line (see [`cc_core::ViewKind::Section`]).
    pub segments: Option<[f64; 2]>,
    /// The pieces of line drawn between the heads.
    pub pieces: Vec<(DVec2, DVec2)>,
    /// Radius of a head's circle.
    pub radius: f64,
    /// How far from each end the line starts (half the triangle's base).
    pub head: f64,
}

impl SectionHandle {
    /// Whether `p` lies on the mark: within `tol` of its line or in a head.
    pub fn hits(&self, p: DVec2, tol: f64) -> bool {
        let near = |a: DVec2, b: DVec2| cc_geom::polygon::closest_on_segment(a, b, p).0.distance(p) <= tol;
        [self.start, self.end].iter().any(|c| c.distance(p) <= self.radius + tol)
            || self.pieces.iter().any(|&(a, b)| near(a, b))
    }

    /// The inner ends of a split line's pieces, next to the start and to
    /// the end: dragged closer to or away from their head.
    pub fn segment_ends(&self) -> Option<[DVec2; 2]> {
        let [s0, s1] = self.segments?;
        let dir = (self.end - self.start).normalize_or_zero();
        Some([self.start + dir * s0, self.end - dir * s1])
    }
}

/// A tag of an element in a view, as drawn: its box, `half` its size
/// along and across the tag around `center`, turned `rotation` radians
/// counter-clockwise; `offset` is how far the view moved it from its
/// default spot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TagHandle {
    pub element: ElementId,
    pub center: DVec2,
    pub half: DVec2,
    pub rotation: f64,
    pub offset: DVec2,
}

impl TagHandle {
    /// Whether `p` lies in the tag's turned box.
    pub fn contains(&self, p: DVec2) -> bool {
        let local = DVec2::from_angle(-self.rotation).rotate(p - self.center);
        local.abs().cmple(self.half).all()
    }

    /// The corners of the turned box around `center`, counter-clockwise
    /// from the bottom left.
    pub fn corners_at(&self, center: DVec2) -> [DVec2; 4] {
        let r = DVec2::from_angle(self.rotation);
        let h = self.half;
        [DVec2::new(-h.x, -h.y), DVec2::new(h.x, -h.y), h, DVec2::new(-h.x, h.y)].map(|c| center + r.rotate(c))
    }

    /// Where the grip that turns the tag sits: `gap` meters past the middle
    /// of the box's right side.
    pub fn rotate_grip(&self, gap: f64) -> DVec2 {
        self.center + DVec2::from_angle(self.rotation) * (self.half.x + gap)
    }
}

/// The marker of an elevation in a plan, as drawn: picked within `half` of
/// `center`; `offset` is how far the plan moved it from where it sits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkHandle {
    pub view: cc_core::ViewId,
    pub center: DVec2,
    pub half: DVec2,
    pub offset: DVec2,
}

/// A grid's bubble, as drawn: picked within `radius` of `center`; `start`
/// for the bubble at the grid's start, and `offset` how far the view moved it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BubbleHandle {
    pub element: ElementId,
    pub start: bool,
    pub center: DVec2,
    pub radius: f64,
    pub offset: DVec2,
}

/// A level's line in an elevation or a section, as drawn (meters in the view).
#[derive(Debug, Clone, PartialEq)]
pub struct LevelHandle {
    pub level: cc_core::LevelId,
    /// The line's ends: the start on the left.
    pub a: DVec2,
    pub b: DVec2,
    /// The heads drawn at its ends.
    pub heads: Vec<LevelHeadHandle>,
}

/// One head of a level's line, as drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelHeadHandle {
    /// At the line's start (else its end).
    pub start: bool,
    /// The end of the line the head belongs to.
    pub tip: DVec2,
    /// Where the head's symbol points: `tip`, or past its elbow.
    pub point: DVec2,
    /// The box the symbol and the texts take.
    pub center: DVec2,
    pub half: DVec2,
    /// How far the head is moved up from the line, meters in the view.
    pub elbow: f64,
}

impl LevelHeadHandle {
    pub fn contains(&self, p: DVec2) -> bool {
        (p - self.center).abs().cmple(self.half).all()
    }
}

impl LevelHandle {
    /// Whether `p` lies within `tol` of the line.
    pub fn on_line(&self, p: DVec2, tol: f64) -> bool {
        cc_geom::polygon::closest_on_segment(self.a, self.b, p).0.distance(p) <= tol
    }
}

impl Drawing2D {
    /// The primitives of the view itself, without its underlay and its overlay.
    pub fn own_prims(&self) -> &[Prim] {
        let end = self.prims.len().saturating_sub(self.overlay);
        self.prims.get(self.underlay..end).unwrap_or_default()
    }

    /// The lines of the view itself that tools snap to by their ends: its
    /// linework without the pattern lines of hatches, which are not geometry.
    pub fn snap_lines(&self) -> Vec<(DVec2, DVec2)> {
        self.own_prims()
            .iter()
            .filter_map(|p| match p {
                Prim::Line { a, b, style, .. } if *style != LineStyle::Hatch => Some((*a, *b)),
                _ => None,
            })
            .collect()
    }

    /// The overlay drawn over the view (see [`Self::overlay`]).
    pub fn overlay_prims(&self) -> &[Prim] {
        self.prims.get(self.prims.len().saturating_sub(self.overlay)..).unwrap_or_default()
    }

    /// The drawing without its underlay (printed when the view says so).
    pub fn without_underlay(&self) -> Drawing2D {
        let prims = self.prims.get(self.underlay..).unwrap_or_default().to_vec();
        Drawing2D { prims, underlay: 0, ..self.clone() }
    }

    /// The section mark at `p` (the last drawn on top), if any.
    pub fn section_at(&self, p: DVec2, tol: f64) -> Option<&SectionHandle> {
        self.sections.iter().rev().find(|s| s.hits(p, tol))
    }

    /// The callout mark at `p` (the last drawn on top), if any.
    pub fn callout_at(&self, p: DVec2, tol: f64) -> Option<&crate::callout::CalloutHandle> {
        self.callouts.iter().rev().find(|c| c.hits(p, tol))
    }

    /// The view of the interior elevation arrow at `p` (within `tol`), if any.
    pub fn interior_arrow_at(&self, p: DVec2, tol: f64) -> Option<cc_core::ViewId> {
        self.interior_arrows.iter().rev().find(|a| a.center.distance(p) <= a.radius + tol).map(|a| a.view)
    }

    /// The tag at `p` (the last drawn on top), if any.
    pub fn tag_at(&self, p: DVec2) -> Option<TagHandle> {
        self.tags.iter().rev().find(|t| t.contains(p)).copied()
    }

    /// The fitting label at `p` (the last drawn on top), if any.
    pub fn fitting_tag_at(&self, p: DVec2) -> Option<FittingTagHandle> {
        self.fitting_tags.iter().rev().find(|t| t.tag.contains(p)).cloned()
    }

    /// The elevation marker at `p` (the last drawn on top), if any.
    pub fn mark_at(&self, p: DVec2) -> Option<MarkHandle> {
        self.marks.iter().rev().find(|m| (p - m.center).abs().cmple(m.half).all()).copied()
    }

    /// The level whose head is at `p` (with which head: `true` at the
    /// start), else whose line passes within `tol` of it.
    pub fn level_at(&self, p: DVec2, tol: f64) -> Option<(cc_core::LevelId, Option<bool>)> {
        let head = self.levels.iter().rev().find_map(|l| {
            let h = l.heads.iter().find(|h| h.contains(p))?;
            Some((l.level, Some(h.start)))
        });
        head.or_else(|| self.levels.iter().rev().find(|l| l.on_line(p, tol)).map(|l| (l.level, None)))
    }

    /// The grid bubble at `p` (the last drawn on top), if any.
    pub fn bubble_at(&self, p: DVec2) -> Option<BubbleHandle> {
        self.bubbles.iter().rev().find(|b| b.center.distance(p) <= b.radius).copied()
    }
}

impl Drawing2D {
    pub fn line(&mut self, a: DVec2, b: DVec2, style: LineStyle, element: Option<ElementId>) {
        if a.distance_squared(b) > 1e-14 {
            self.prims.push(Prim::Line { a, b, style, element, pen: None });
        }
    }

    /// A line with its own pen (view graphics still override what they set).
    pub fn line_pen(&mut self, a: DVec2, b: DVec2, style: LineStyle, pen: Option<Pen>, element: Option<ElementId>) {
        if a.distance_squared(b) > 1e-14 {
            self.prims.push(Prim::Line { a, b, style, element, pen });
        }
    }

    pub fn polyline(&mut self, pts: &[DVec2], closed: bool, style: LineStyle, element: Option<ElementId>) {
        for w in pts.windows(2) {
            self.line(w[0], w[1], style, element);
        }
        if closed && pts.len() > 2 {
            self.line(pts[pts.len() - 1], pts[0], style, element);
        }
    }

    /// [`Self::polyline`] with its own pen.
    pub fn polyline_pen(
        &mut self,
        pts: &[DVec2],
        closed: bool,
        style: LineStyle,
        pen: Option<Pen>,
        element: Option<ElementId>,
    ) {
        for w in pts.windows(2) {
            self.line_pen(w[0], w[1], style, pen, element);
        }
        if closed && pts.len() > 2 {
            self.line_pen(pts[pts.len() - 1], pts[0], style, pen, element);
        }
    }

    pub fn fill(&mut self, pts: Vec<DVec2>, color: [u8; 4], element: Option<ElementId>) {
        if pts.len() >= 3 {
            self.prims.push(Prim::Fill { pts, color, element, cut: false });
        }
    }

    /// A fill of a cut region.
    pub fn cut_fill(&mut self, pts: Vec<DVec2>, color: [u8; 4], element: Option<ElementId>) {
        if pts.len() >= 3 {
            self.prims.push(Prim::Fill { pts, color, element, cut: true });
        }
    }

    pub fn text(&mut self, pos: DVec2, text: impl Into<String>, size: f64) {
        self.text_at(pos, text, size, Anchor::LeftBottom, 0.0);
    }

    pub fn text_at(&mut self, pos: DVec2, text: impl Into<String>, size: f64, anchor: Anchor, angle: f64) {
        self.prims.push(Prim::Text { pos, text: text.into(), size, anchor, angle, element: None, color: None });
    }

    /// Text that annotates `element` (a tag, a dimension value…).
    pub fn tag_text(
        &mut self,
        pos: DVec2,
        text: impl Into<String>,
        size: f64,
        anchor: Anchor,
        angle: f64,
        element: Option<ElementId>,
    ) {
        self.prims.push(Prim::Text { pos, text: text.into(), size, anchor, angle, element, color: None });
    }

    /// [`Self::tag_text`] in its own `color` (`None`: the annotation ink).
    pub fn colored_text(
        &mut self,
        pos: DVec2,
        text: impl Into<String>,
        size: f64,
        (anchor, angle): (Anchor, f64),
        element: Option<ElementId>,
        color: Option<[u8; 3]>,
    ) {
        self.prims.push(Prim::Text { pos, text: text.into(), size, anchor, angle, element, color });
    }

    /// Circular arc approximated with line segments.
    pub fn arc(&mut self, center: DVec2, radius: f64, a0: f64, a1: f64, style: LineStyle, element: Option<ElementId>) {
        let steps = 16;
        let pts: Vec<DVec2> = (0..=steps)
            .map(|i| {
                let a = a0 + (a1 - a0) * i as f64 / steps as f64;
                center + DVec2::new(a.cos(), a.sin()) * radius
            })
            .collect();
        self.polyline(&pts, false, style, element);
    }

    /// A circular arc in a dashed `style` at `1:scale`, its dashes laid
    /// along the whole arc (each a few chords, the gaps left empty), so the
    /// pattern shows however short the chords of [`Self::arc`] would be.
    #[allow(clippy::too_many_arguments)]
    pub fn dashed_arc(
        &mut self,
        center: DVec2,
        radius: f64,
        (a0, a1): (f64, f64),
        style: LineStyle,
        element: Option<ElementId>,
        scale: u32,
    ) {
        let k = f64::from(scale.max(1)) / 1000.0;
        let dashes: Vec<f64> = style.dashes().iter().map(|d| d * k).collect();
        let length = radius * (a1 - a0).abs();
        if dashes.len() < 2
            || dashes.iter().any(|d| *d <= 0.0)
            || !length.is_finite()
            || length <= 0.0
            || length / k > 1e5
        {
            self.arc(center, radius, a0, a1, style, element);
            return;
        }
        let at = |s: f64| {
            let a = a0 + (a1 - a0) * (s / length).clamp(0.0, 1.0);
            center + DVec2::new(a.cos(), a.sin()) * radius
        };
        let (mut s, mut i) = (0.0, 0);
        while s < length {
            let step = dashes[i % dashes.len()];
            if i % 2 == 0 {
                let end = (s + step).min(length);
                let n = ((end - s) / (radius * 0.1).max(1e-6)).ceil().clamp(1.0, 8.0) as usize;
                let pts: Vec<DVec2> = (0..=n).map(|j| at(s + (end - s) * j as f64 / n as f64)).collect();
                self.polyline(&pts, false, style, element);
            }
            s += step;
            i += 1;
        }
    }

    /// Bounding box of geometry (text anchors included).
    pub fn bounds(&self) -> Option<(DVec2, DVec2)> {
        let mut it = self.prims.iter().flat_map(|p| match p {
            Prim::Line { a, b, .. } => vec![*a, *b],
            Prim::Fill { pts, .. } => pts.clone(),
            Prim::Text { pos, .. } => vec![*pos],
            Prim::Image { origin, u, v, .. } => Prim::image_corners(*origin, *u, *v).to_vec(),
        });
        let first = it.next()?;
        Some(it.fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p))))
    }

    /// The extents to fit the view to: [`bounds`](Self::bounds), or only the
    /// elements' geometry and the overlay with `fit_elements` (all of it
    /// when there is none).
    pub fn fit_bounds(&self) -> Option<(DVec2, DVec2)> {
        if !self.fit_elements {
            return self.bounds();
        }
        let overlay = self.prims.len().saturating_sub(self.overlay);
        let fitted = |(i, p): &(usize, &Prim)| p.element().is_some() || *i >= overlay;
        let mut it = self.prims.iter().enumerate().filter(fitted).flat_map(|(_, p)| match p {
            Prim::Line { a, b, .. } => vec![*a, *b],
            Prim::Fill { pts, .. } => pts.clone(),
            Prim::Text { pos, .. } => vec![*pos],
            Prim::Image { origin, u, v, .. } => Prim::image_corners(*origin, *u, *v).to_vec(),
        });
        match it.next() {
            Some(first) => Some(it.fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p)))),
            None => self.bounds(),
        }
    }

    /// Topmost element under `p` within `tol` (view units). Background
    /// elements (imported drawings) are picked only where nothing else is.
    /// The first of [`Self::hits_at`].
    pub fn hit_test(&self, p: DVec2, tol: f64) -> Option<ElementId> {
        self.hits_up_to(p, tol, 1).first().copied()
    }

    /// Every element under `p` within `tol`, in pick order and without
    /// repeats: the lines of the model before those of [`Self::datum`]
    /// elements, each by distance (the nearest first; among lines within a
    /// quarter of `tol` of each other, [`Self::yielding`] ones last), then the fills and
    /// pictures topmost first, then background elements (imported drawings)
    /// the same way. The first is what [`Self::hit_test`] picks; Tab steps
    /// through the others. At most [`MAX_HITS`].
    pub fn hits_at(&self, p: DVec2, tol: f64) -> Vec<ElementId> {
        self.hits_up_to(p, tol, MAX_HITS)
    }

    fn hits_up_to(&self, p: DVec2, tol: f64, limit: usize) -> Vec<ElementId> {
        let mut out = Vec::new();
        match self.index.candidates(&self.prims, p, tol) {
            Some(near) => {
                let prims = || near.iter().filter_map(|i| self.prims.get(*i as usize));
                for background in [false, true] {
                    self.tier_hits(prims(), p, tol, background, limit, &mut out);
                }
            }
            None => {
                for background in [false, true] {
                    self.tier_hits(self.prims.iter(), p, tol, background, limit, &mut out);
                }
            }
        }
        out
    }

    /// The indices of the primitives of `ids`, in drawing order.
    pub fn prims_of(&self, ids: &std::collections::BTreeSet<ElementId>) -> Vec<usize> {
        match self.index.elements(&self.prims) {
            Some(map) => {
                let mut out: Vec<usize> =
                    ids.iter().filter_map(|id| map.get(id)).flatten().map(|i| *i as usize).collect();
                out.sort_unstable();
                out
            }
            None => (0..self.prims.len())
                .filter(|i| self.prims.get(*i).and_then(Prim::element).is_some_and(|e| ids.contains(&e)))
                .collect(),
        }
    }

    /// The element whose line passes within `tol` of `p` (the nearest),
    /// leaving out fills and background elements: what a drag starting at
    /// `p` grabs.
    pub fn hit_test_line(&self, p: DVec2, tol: f64) -> Option<ElementId> {
        match self.index.candidates(&self.prims, p, tol) {
            Some(near) => self.nearest_line(near.iter().filter_map(|i| self.prims.get(*i as usize)), p, tol),
            None => self.nearest_line(self.prims.iter(), p, tol),
        }
    }

    /// The lines of elements passing within `tol` of `p`, with their
    /// element: the model as the view draws it, for snapping. The underlay,
    /// background elements (imported drawings), pattern lines, faded
    /// linework and annotations are left out.
    pub fn lines_near(&self, p: DVec2, tol: f64) -> Vec<(DVec2, DVec2, ElementId)> {
        let near = |i: usize, prim: &Prim| match prim {
            Prim::Line { a, b, style, element: Some(id), .. }
                if i >= self.underlay
                    && !matches!(style, LineStyle::Hatch | LineStyle::Faint | LineStyle::Annotation)
                    && !self.background.contains(id)
                    && cc_geom::polygon::closest_on_segment(*a, *b, p).0.distance(p) <= tol =>
            {
                Some((*a, *b, *id))
            }
            _ => None,
        };
        match self.index.candidates(&self.prims, p, tol) {
            Some(found) => found
                .iter()
                .filter_map(|i| self.prims.get(*i as usize).and_then(|prim| near(*i as usize, prim)))
                .collect(),
            None => self.prims.iter().enumerate().filter_map(|(i, prim)| near(i, prim)).collect(),
        }
    }

    fn nearest_line<'a>(&self, prims: impl Iterator<Item = &'a Prim>, p: DVec2, tol: f64) -> Option<ElementId> {
        let mut best: Option<(f64, ElementId)> = None;
        for prim in prims {
            if let Prim::Line { a, b, element: Some(id), .. } = prim
                && !self.background.contains(id)
            {
                let d = cc_geom::polygon::closest_on_segment(*a, *b, p).0.distance(p);
                if d <= tol && best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, *id));
                }
            }
        }
        best.map(|(_, id)| id)
    }

    /// [`Self::hit_test`] searching every primitive (tests of the index).
    #[cfg(test)]
    pub(crate) fn hit_test_all(&self, p: DVec2, tol: f64) -> Option<ElementId> {
        let mut out = Vec::new();
        for background in [false, true] {
            self.tier_hits(self.prims.iter(), p, tol, background, 1, &mut out);
        }
        out.first().copied()
    }

    /// [`Self::hits_at`] searching every primitive (tests of the index).
    #[cfg(test)]
    pub(crate) fn hits_at_all(&self, p: DVec2, tol: f64) -> Vec<ElementId> {
        let mut out = Vec::new();
        for background in [false, true] {
            self.tier_hits(self.prims.iter(), p, tol, background, MAX_HITS, &mut out);
        }
        out
    }

    /// [`Self::hit_test_line`] searching every primitive (tests of the index).
    #[cfg(test)]
    pub(crate) fn nearest_line_all(&self, p: DVec2, tol: f64) -> Option<ElementId> {
        self.nearest_line(self.prims.iter(), p, tol)
    }

    /// The picks among `prims` (in drawing order) in one tier, foreground
    /// elements or background ones, added to `out` (no repeats) until it
    /// holds `limit`: lines by distance (drawing order among equal ones),
    /// then fills and pictures, the last drawn first.
    fn tier_hits<'a>(
        &self,
        prims: impl DoubleEndedIterator<Item = &'a Prim> + Clone,
        p: DVec2,
        tol: f64,
        background: bool,
        limit: usize,
        out: &mut Vec<ElementId>,
    ) {
        let tier = |id: &ElementId| self.background.contains(id) == background;
        let add = |id: ElementId, out: &mut Vec<ElementId>| {
            if !out.contains(&id) {
                out.push(id);
            }
            out.len() >= limit
        };
        if out.len() >= limit {
            return;
        }
        // Lines first: they are thin and drawn over fills.
        let mut lines: Vec<(f64, ElementId)> = Vec::new();
        for prim in prims.clone() {
            if let Prim::Line { a, b, element: Some(id), .. } = prim
                && tier(id)
            {
                let (q, _) = cc_geom::polygon::closest_on_segment(*a, *b, p);
                let d = q.distance(p);
                if d <= tol {
                    lines.push((d, *id));
                }
            }
        }
        // Model lines before datum ones; then by distance in steps of a quarter of
        // `tol`, an outline lying on another's (a floor's under a wall's) after it.
        // A stable sort: among lines as near, the first drawn comes first.
        let step = (tol * 0.25).max(f64::EPSILON);
        let key = |(d, id): &(f64, ElementId)| {
            let near = if d.is_finite() { (d / step).floor() as i64 } else { i64::MAX };
            (self.datum.contains(id), near, self.yielding.contains(id))
        };
        lines.sort_by(|a, b| key(a).cmp(&key(b)).then(a.0.total_cmp(&b.0)));
        for (_, id) in lines {
            if add(id, out) {
                return;
            }
        }
        for prim in prims.rev() {
            let hit = match prim {
                // Fully transparent fills only show selection (room areas); they are not picked.
                Prim::Fill { pts, element: Some(id), color, .. }
                    if color[3] > 0 && tier(id) && cc_geom::polygon::contains(pts, p) =>
                {
                    Some(*id)
                }
                Prim::Image { origin, u, v, element: Some(id), .. }
                    if tier(id) && cc_geom::polygon::contains(&Prim::image_corners(*origin, *u, *v), p) =>
                {
                    Some(*id)
                }
                _ => None,
            };
            if let Some(id) = hit
                && add(id, out)
            {
                return;
            }
        }
    }

    /// Elements taken by the rectangle `lo`..`hi`: with `fully` (a window)
    /// those whose lines, fills and texts all lie inside it, otherwise (a
    /// crossing) those that any of them reaches into: a line crossing it, a
    /// fill overlapping it, a text anchored in it. A crossing leaves out
    /// background elements (imported drawings), which lie under everything.
    pub fn elements_in(&self, lo: DVec2, hi: DVec2, fully: bool) -> Vec<ElementId> {
        use cc_geom::polygon::{contains, segment_meets_rect};
        use std::collections::BTreeMap;
        let (lo, hi) = (lo.min(hi), lo.max(hi));
        let inside = |p: &DVec2| p.x >= lo.x && p.y >= lo.y && p.x <= hi.x && p.y <= hi.y;
        let center = (lo + hi) * 0.5;
        // Per element: (reaches in, lies wholly inside).
        let mut state: BTreeMap<ElementId, (bool, bool)> = BTreeMap::new();
        for prim in &self.prims {
            let (id, reaches, within) = match prim {
                Prim::Line { a, b, element: Some(id), .. } => {
                    (*id, segment_meets_rect(*a, *b, lo, hi), inside(a) && inside(b))
                }
                Prim::Fill { pts, color, element: Some(id), .. } => {
                    let edges = pts.iter().zip(pts.iter().cycle().skip(1));
                    let crosses = edges.take(pts.len()).any(|(a, b)| segment_meets_rect(*a, *b, lo, hi));
                    // A fill around the whole rectangle, unless it only shows
                    // when selected (room areas): dragging inside a room
                    // would otherwise always take it.
                    let around = color[3] > 0 && pts.len() >= 3 && contains(pts, center);
                    (*id, crosses || around || pts.iter().any(inside), !pts.is_empty() && pts.iter().all(inside))
                }
                Prim::Text { pos, element: Some(id), .. } => (*id, inside(pos), inside(pos)),
                Prim::Image { origin, u, v, element: Some(id), .. } => {
                    let c = Prim::image_corners(*origin, *u, *v);
                    let crosses = (0..4).any(|i| segment_meets_rect(c[i], c[(i + 1) % 4], lo, hi));
                    (*id, crosses || contains(&c, center) || c.iter().any(inside), c.iter().all(inside))
                }
                _ => continue,
            };
            let s = state.entry(id).or_insert((false, true));
            s.0 |= reaches;
            s.1 &= within;
        }
        state
            .into_iter()
            .filter(|(id, (any, all))| if fully { *all } else { *any && !self.background.contains(id) })
            .map(|(id, _)| id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> DVec2 {
        DVec2::new(x, y)
    }

    /// A long wall line, a floor fill, a room area (transparent fill), a
    /// text note and an imported drawing's line.
    fn sample() -> Drawing2D {
        let mut d = Drawing2D::default();
        d.line(v(0.0, 0.0), v(10.0, 0.0), LineStyle::Cut, Some(ElementId(1)));
        let square = vec![v(20.0, 0.0), v(30.0, 0.0), v(30.0, 10.0), v(20.0, 10.0)];
        d.fill(square.clone(), [200, 200, 200, 255], Some(ElementId(2)));
        d.fill(square.iter().map(|p| *p + v(20.0, 0.0)).collect(), [0, 0, 0, 0], Some(ElementId(3)));
        d.prims.push(Prim::Text {
            pos: v(5.0, 5.0),
            text: "Nota".into(),
            size: 2.5,
            anchor: Anchor::LeftBottom,
            angle: 0.0,
            element: Some(ElementId(4)),
            color: None,
        });
        d.line(v(-50.0, -50.0), v(100.0, 50.0), LineStyle::Thin, Some(ElementId(5)));
        d.background.insert(ElementId(5));
        d
    }

    /// Issue #332: the pattern lines of a hatch are not snapped to; the
    /// outline they fill, annotations and lines of no element still are.
    #[test]
    fn hatch_lines_are_not_snap_lines() {
        let mut d = Drawing2D::default();
        d.line(v(-9.0, 0.0), v(-8.0, 0.0), LineStyle::Cut, None);
        d.underlay = 1;
        d.line(v(0.0, 0.0), v(4.0, 0.0), LineStyle::Cut, Some(ElementId(1)));
        d.line(v(1.0, 0.0), v(2.0, 1.0), LineStyle::Hatch, Some(ElementId(1)));
        d.line(v(0.0, 2.0), v(4.0, 2.0), LineStyle::Annotation, None);
        assert_eq!(d.snap_lines(), [(v(0.0, 0.0), v(4.0, 0.0)), (v(0.0, 2.0), v(4.0, 2.0))]);
    }

    fn ids(list: Vec<ElementId>) -> Vec<u64> {
        list.into_iter().map(|e| e.0).collect()
    }

    /// Issue #93: a crossing that cuts a line between its ends, or lies
    /// inside a floor, takes the element; it used to look at ends only.
    #[test]
    fn a_crossing_takes_what_it_cuts_or_lies_in() {
        let d = sample();
        // Across the middle of the wall line, no end inside.
        assert_eq!(ids(d.elements_in(v(4.0, -1.0), v(6.0, 1.0), false)), [1]);
        // Inside the floor, touching none of its edges.
        assert_eq!(ids(d.elements_in(v(24.0, 4.0), v(26.0, 6.0), false)), [2]);
        // Inside a room area: rooms show only when selected, so not taken.
        assert!(d.elements_in(v(44.0, 4.0), v(46.0, 6.0), false).is_empty());
        // Around a text note's anchor.
        assert_eq!(ids(d.elements_in(v(4.0, 4.0), v(6.0, 6.0), false)), [4]);
        // The imported drawing's line passes through all of these, but a crossing leaves it out.
        assert!(!ids(d.elements_in(v(-60.0, -60.0), v(110.0, 60.0), false)).contains(&5));
    }

    #[test]
    fn a_window_takes_only_what_lies_wholly_inside() {
        let d = sample();
        assert!(d.elements_in(v(4.0, -1.0), v(6.0, 1.0), true).is_empty());
        assert_eq!(ids(d.elements_in(v(-1.0, -1.0), v(11.0, 6.0), true)), [1, 4]);
        // A window around everything takes the imported drawing too.
        assert_eq!(ids(d.elements_in(v(-60.0, -60.0), v(110.0, 60.0), true)), [1, 2, 3, 4, 5]);
    }

    /// A drag grabs an element by its lines only, never by a fill or an imported drawing.
    #[test]
    fn drags_grab_lines() {
        let d = sample();
        assert_eq!(d.hit_test_line(v(5.0, 0.05), 0.1), Some(ElementId(1)));
        assert_eq!(d.hit_test_line(v(25.0, 5.0), 0.1), None, "inside the floor");
        assert_eq!(d.hit_test(v(25.0, 5.0), 0.1), Some(ElementId(2)), "a click still picks it");
        assert_eq!(d.hit_test_line(v(25.0, 12.5), 0.1), None, "on the imported line");
    }

    /// Issue #406: every element under the point in pick order, once each:
    /// the lines by distance, the fills topmost first (a room area, fully
    /// transparent, never), the imported drawing last.
    #[test]
    fn hits_list_everything_under_the_point_in_pick_order() {
        let mut d = Drawing2D::default();
        let square = |lo: f64, hi: f64| vec![v(lo, lo), v(hi, lo), v(hi, hi), v(lo, hi)];
        d.line(v(-50.0, -50.0), v(100.0, 50.0), LineStyle::Thin, Some(ElementId(15)));
        d.background.insert(ElementId(15));
        d.fill(square(0.0, 10.0), [200, 200, 200, 255], Some(ElementId(10)));
        d.fill(square(5.0, 15.0), [150, 150, 150, 255], Some(ElementId(11)));
        d.fill(square(-5.0, 20.0), [0, 0, 0, 0], Some(ElementId(14)));
        d.line(v(0.0, 7.8), v(20.0, 7.8), LineStyle::Cut, Some(ElementId(11)));
        d.line(v(0.0, 7.5), v(20.0, 7.5), LineStyle::Cut, Some(ElementId(13)));
        d.line(v(0.0, 7.0), v(20.0, 7.0), LineStyle::Cut, Some(ElementId(12)));
        // A second line of the nearest element, farther: no repeat.
        d.line(v(0.0, 6.5), v(20.0, 6.5), LineStyle::Cut, Some(ElementId(12)));
        let p = v(7.0, 7.2);
        // The imported line passes y = 7.2 at x = 64.4, far from p: give it a point near p too.
        d.line(v(6.0, 7.0), v(8.0, 7.0), LineStyle::Thin, Some(ElementId(15)));
        assert_eq!(ids(d.hits_at(p, 1.0)), [12, 13, 11, 10, 15]);
        assert_eq!(d.hit_test(p, 1.0), Some(ElementId(12)));
        assert_eq!(d.hits_at(p, 1.0), d.hits_at_all(p, 1.0));
        // Inside the fills only, away from the lines: the topmost first.
        assert_eq!(ids(d.hits_at(v(7.0, 9.5), 0.1)), [11, 10]);
        assert_eq!(d.hit_test(v(7.0, 9.5), 0.1), Some(ElementId(11)));
        // Only in the room area: nothing.
        assert!(d.hits_at(v(-2.0, -2.0), 0.1).is_empty());
        assert_eq!(d.hit_test(v(-2.0, -2.0), 0.1), None);
        // Only on the imported drawing: it is picked, as it is alone.
        assert_eq!(ids(d.hits_at(v(-50.0, -50.0), 0.1)), [15]);
        // Equally near lines keep their drawing order.
        let mut e = Drawing2D::default();
        e.line(v(0.0, 1.0), v(10.0, 1.0), LineStyle::Cut, Some(ElementId(2)));
        e.line(v(0.0, -1.0), v(10.0, -1.0), LineStyle::Cut, Some(ElementId(1)));
        assert_eq!(ids(e.hits_at(v(5.0, 0.0), 1.5)), [2, 1]);
        assert_eq!(e.hit_test(v(5.0, 0.0), 1.5), Some(ElementId(2)));
    }

    /// The list is bounded.
    /// A wall's face on a grid line and on a floor's edge: the wall is
    /// picked first even nearer the grid, and before the floor where their
    /// lines meet; the grid is still picked where no model line is near.
    #[test]
    fn model_lines_come_before_datum_and_outlines_yield() {
        let (wall, grid, floor) = (ElementId(1), ElementId(2), ElementId(3));
        let mut d = Drawing2D::default();
        // The floor's edge is drawn first, on the wall's face.
        d.line(v(0.0, 0.0), v(10.0, 0.0), LineStyle::Thin, Some(floor));
        d.line(v(0.0, 0.0), v(10.0, 0.0), LineStyle::Cut, Some(wall));
        d.line(v(0.0, 0.1), v(10.0, 0.1), LineStyle::Thin, Some(grid));
        d.datum.insert(grid);
        d.yielding.insert(floor);
        let tol = 0.25;
        assert_eq!(d.hits_at(v(5.0, 0.09), tol), vec![wall, floor, grid], "on the grid, near the wall");
        assert_eq!(d.hits_at_all(v(5.0, 0.0), tol), vec![wall, floor, grid]);
        assert_eq!(d.hit_test(v(5.0, 0.3), tol), Some(grid), "only the grid within reach");
        // Without the ranks, the nearest line and the first drawn win as before.
        d.datum.clear();
        d.yielding.clear();
        assert_eq!(d.hit_test(v(5.0, 0.09), tol), Some(grid));
        assert_eq!(d.hit_test(v(5.0, 0.0), tol), Some(floor));
    }

    #[test]
    fn hits_are_bounded() {
        let mut d = Drawing2D::default();
        for i in 0..(MAX_HITS as u64 + 10) {
            d.fill(vec![v(-1.0, -1.0), v(1.0, -1.0), v(1.0, 1.0), v(-1.0, 1.0)], [9, 9, 9, 255], Some(ElementId(i)));
        }
        let hits = d.hits_at(v(0.0, 0.0), 0.1);
        assert_eq!(hits.len(), MAX_HITS);
        assert_eq!(hits.first(), Some(&ElementId(MAX_HITS as u64 + 9)), "the last drawn first");
    }

    /// Tags are picked within their box, the last drawn first.
    #[test]
    fn tags_are_picked_in_their_box() {
        let mut d = Drawing2D::default();
        let tag = |id: u64, x: f64| TagHandle {
            element: ElementId(id),
            center: v(x, 0.0),
            half: v(1.0, 0.5),
            rotation: 0.0,
            offset: DVec2::ZERO,
        };
        d.tags = vec![tag(1, 0.0), tag(2, 1.5)];
        assert_eq!(d.tag_at(v(-0.9, 0.4)).map(|t| t.element.0), Some(1));
        assert_eq!(d.tag_at(v(0.8, 0.0)).map(|t| t.element.0), Some(2), "both boxes: the one on top");
        assert_eq!(d.tag_at(v(0.0, 0.6)), None);
        // A tag turned a quarter turn is picked in its turned box.
        d.tags = vec![TagHandle { rotation: std::f64::consts::FRAC_PI_2, ..tag(3, 0.0) }];
        assert_eq!(d.tag_at(v(0.0, 0.9)).map(|t| t.element.0), Some(3));
        assert_eq!(d.tag_at(v(0.9, 0.0)), None);
        let grip = d.tags[0].rotate_grip(0.5);
        assert!((grip - v(0.0, 1.5)).length() < 1e-12, "{grip:?}");
    }
}
