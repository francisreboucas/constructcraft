//! Derived geometry for ConstructCraft.
//!
//! [`ModelGeometry::build`] turns a [`cc_core::Project`] into boundary
//! representation [`Solid`]s (planar polygon faces) that feed the 3D renderer,
//! elevations and sections, plus [`WallGeom`] and [`BeamGeom`] data used by plan views.

pub mod analytical;
pub mod bar_marks;
pub mod bar_shape;
mod beam;
mod build;
mod cap_rebar;
pub mod component;
pub mod connection;
pub mod coplanar;
pub mod csg;
pub mod curtain;
mod cut;
pub mod faces;
pub mod fitting_catalog;
pub mod level_mark;
pub mod lightning;
pub mod mass;
pub mod mep;
pub mod mep_net;
mod mesh;
pub mod pad;
pub mod paint;
pub mod pile;
mod pile_rebar;
mod plan_cover;
pub mod polygon;
pub mod profile_sweep;
pub mod ramp;
pub mod rebar;
pub mod region;
pub mod roof_framing;
pub mod roof_join;
pub mod roofs;
mod rooms;
pub mod section;
pub mod sizing;
pub mod skeleton;
mod slab_rebar;
mod solid;
pub mod stair;
pub mod strip;
mod strip_rebar;
pub mod topography;
mod wall;
#[cfg(test)]
mod wall_layers_tests;
#[cfg(test)]
mod wall_profile_tests;
mod wall_rebar;
#[cfg(test)]
mod wall_spline_tests;

pub use beam::{BeamEnd, BeamGeom};
pub use build::{
    BoxPart, FRAME_COLOR, FoundationGeom, HostedPart, ModelGeometry, OpeningSymbol, WorldConnector, column_outline,
    component_connectors, component_parts, hosted_parts, opening_parts, rect_outline,
};
pub use component::{instance_errors, resolve_component, resolve_component_in, resolve_instance, resolve_pieces};
pub use cut::{clip_polygon, face_plane_segments};
pub use mep::{SegmentGeom, TraySection, ValveGeom, segment_geom, valve_geom};
pub use mesh::{MeshPlan, mesh_plan, plan_edges, top_edges};
pub use plan_cover::PlanCover;
pub use ramp::{RampGeom, RampPiece, ramp_geom};
pub use rooms::{BoundaryEdge, RoomArrangement, boundary_edges, outer_boundary, room_boundary};
pub use solid::{Face, Look, Ray, Solid, join_pieces, plane_axes};
pub use stair::{
    Flight, LandingRect, Pieces, StairGeom, StairLanding, StairStep, StepKind, add_railing, railing_length,
    railing_path, railing_pieces, stair_geom,
};
pub use strip::StripArc;
pub use wall::{Band, Column, LayerGap, Lin, Opening, RECESS_MIN_BACK, Recess, WallGeom};
pub use wall_rebar::flipped_vertical_offset;

pub use glam::{DVec2, DVec3};
