//! Strip footing reinforcement: bars across the footing slab (radial in an
//! arc strip), bars along it (ring bars), and the bars of a beam for the
//! beam on top of a haunched strip (or the whole section of a plain one).
//! Top bars follow the top of the footing: up its slopes and flat between
//! the faces of its beam.

use crate::BeamGeom;
use crate::rebar::{Bar, MAX_BARS, stations};
use cc_core::{Rebar, RebarLayout, StripHaunch};
use glam::DVec3;

/// A strip's section as its bars see it: its width, the footing's thickness
/// at the beam, and the beam on top, when it has one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct StripSection {
    pub width: f64,
    pub thickness: f64,
    pub haunch: Option<StripHaunch>,
}

impl StripSection {
    /// Height of the top of the footing slab above its bottom, `v` across it.
    pub fn top(&self, v: f64) -> f64 {
        let Some(h) = self.haunch else { return self.thickness };
        let (edge, face) = (self.width / 2.0, h.beam_width / 2.0);
        let v = v.abs();
        if v <= face || edge - face < 1e-9 {
            return self.thickness;
        }
        // Down the slope from the beam's face to the edge.
        let t = ((v - face) / (edge - face)).clamp(0.0, 1.0);
        self.thickness + (h.edge_thickness - self.thickness) * t
    }

    /// Half the width of the beam part: the beam on top, or the whole strip.
    pub fn beam_half(&self) -> f64 {
        self.haunch.map_or(self.width, |h| h.beam_width) / 2.0
    }

    /// Whether a beam stands on the footing slab.
    fn has_beam(&self) -> bool {
        self.haunch.is_some_and(|h| h.beam_height > 0.0)
    }
}

/// The bars of the strip's own set `r` (bars `d` thick): `other` is the
/// largest bar of the sets of the same layer that run the other way, which
/// the bars along the strip lie inside. The bars along the strip run `past`
/// its start and end by those lengths instead of stopping the cover short.
pub(crate) fn strip_bars(
    g: &BeamGeom,
    section: &StripSection,
    r: &Rebar,
    d: f64,
    other: f64,
    past: (f64, f64),
) -> Vec<Bar> {
    let cover = r.cover;
    let (u0, u1) = (g.start_ext.0.max(g.start_ext.1) + cover, g.end_ext.0.min(g.end_ext.1) - cover);
    // The outermost bar line across the strip, and the heights of the layers.
    let reach = section.width / 2.0 - cover - d / 2.0;
    if u1 <= u0 || reach <= 0.0 {
        return Vec::new();
    }
    let at = |u: f64, v: f64, y: f64| g.world(u, v).extend(g.z_range.0 + y);
    let low = cover + d / 2.0;
    let under = |v: f64| (section.top(v) - cover - d / 2.0).max(low);
    match r.layout {
        RebarLayout::StripTransverse { top, spacing } => stations(u0, u1, spacing)
            .into_iter()
            .map(|u| {
                let points: Vec<DVec3> = if top {
                    // Down to the bottom layer at each edge, up the slopes, flat between the beam's faces.
                    let face = section.beam_half().min(reach);
                    let mut line = vec![(-reach, low), (-reach, under(reach))];
                    if section.haunch.is_some() && face < reach {
                        line.extend([(-face, under(face)), (face, under(face))]);
                    }
                    line.extend([(reach, under(reach)), (reach, low)]);
                    line.into_iter().map(|(v, y)| at(u, v, y)).collect()
                } else {
                    vec![at(u, -reach, low), at(u, reach, low)]
                };
                Bar::new(points, false)
            })
            .collect(),
        RebarLayout::StripLongitudinal { top, spacing } => {
            // Lines either side of the strip's axis; the top ones leave the beam to its own bars.
            let count = (reach / spacing).floor() as i64;
            let face = section.beam_half();
            let mut bars = Vec::new();
            for (index, k) in (-count..=count).enumerate() {
                let v = k as f64 * spacing;
                if top && section.has_beam() && v.abs() < face - 1e-9 {
                    continue;
                }
                let y = if top { (under(v) - other).max(low) } else { low + other };
                // Length of the bar per meter of the axis: its own radius on an arc.
                let scale = g.arc.as_ref().map_or(1.0, |a| (a.radius_at(v) / a.radius.max(1e-9)).max(1e-9));
                let (u0, u1) = crate::rebar::run_past((u0, u1), cover, past, scale);
                let length = (u1 - u0) * scale;
                let pieces = r.splice.map_or_else(|| vec![(0.0, length)], |s| s.pieces(length, index));
                for (from, to) in pieces {
                    if bars.len() >= MAX_BARS {
                        return bars;
                    }
                    let (a, b) = (u0 + from / scale, u0 + to / scale);
                    bars.push(match &g.arc {
                        Some(arc) => {
                            let n = cc_core::arc::segments((b - a) / arc.radius.max(1e-9), cc_core::arc::ARC_STEP);
                            let points = (0..=n).map(|i| at(a + (b - a) * i as f64 / n as f64, v, y)).collect();
                            Bar { true_length: Some(to - from), ..Bar::new(points, false) }
                        }
                        None => Bar::new(vec![at(a, v, y), at(b, v, y)], false),
                    });
                }
            }
            bars
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_top_of_a_haunched_footing_slopes_from_the_beam_to_the_edge() {
        let haunch = StripHaunch { edge_thickness: 0.2, beam_width: 0.4, beam_height: 0.5 };
        let s = StripSection { width: 1.6, thickness: 0.4, haunch: Some(haunch) };
        assert_eq!((s.top(0.0), s.top(0.2), s.top(-0.2)), (0.4, 0.4, 0.4));
        assert!((s.top(0.8) - 0.2).abs() < 1e-12 && (s.top(-0.5) - 0.3).abs() < 1e-12);
        assert!((s.beam_half() - 0.2).abs() < 1e-12 && s.has_beam());
        // A plain strip is flat, and its beam part is the whole of it.
        let plain = StripSection { width: 1.0, thickness: 0.4, haunch: None };
        assert_eq!((plain.top(0.3), plain.beam_half(), plain.has_beam()), (0.4, 0.5, false));
        // The section's area: the two slopes and the beam through the footing.
        assert!((haunch.area(1.6, 0.4) - (1.2 * 0.3 + 0.4 * 0.9)).abs() < 1e-12);
        assert_eq!(haunch.outline(1.6, 0.4).len(), 8);
        assert!(haunch.fits(1.6, 0.4) && !haunch.fits(0.4, 0.4) && !haunch.fits(1.6, 0.1));
    }
}
