//! Site Location (Manage → Project): a window with an OpenStreetMap map, an
//! address search, a marker fixed at the center and the square of map data
//! to bring in. The map moves under the marker; OK stores the location,
//! elevation, true north and address (`site_set`); *Import Context* also
//! downloads the map data of the square (Overpass) and runs
//! `site_context_import`. Map tiles, the search and the elevation come
//! from public services in background threads (never in tests); tiles are
//! kept in memory while the window is open.
//!
//! Usage policies followed: an identifying User-Agent, at most two tile
//! connections, one search per second and only on request (no
//! autocomplete), and the OpenStreetMap attribution on the map.

use crate::dialog::{Dialog, session};
use crate::state::AppState;
use crate::t;
use crate::theme::PALETTE;
use cc_core::site::SiteLocation;
use cc_engine::commands::site::{SiteContextImport, SiteSet};
use cc_engine::osm::{self, Place};
use egui::{Color32, Context, Rect, Sense, Stroke, Ui, Vec2, pos2, vec2};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TILE_URL: &str = "https://tile.openstreetmap.org";
const SEARCH_URL: &str = "https://nominatim.openstreetmap.org";
const ELEVATION_URL: &str = "https://api.open-meteo.com/v1/elevation";
const OVERPASS_URL: &str = "https://overpass-api.de/api/interpreter";
const COPYRIGHT_URL: &str = "https://www.openstreetmap.org/copyright";
/// Pixels of a map tile.
const TILE: f64 = 256.0;
const MIN_ZOOM: f64 = 2.0;
const MAX_ZOOM: f64 = 19.0;
/// Most tiles kept as textures; past it the cache starts over.
const MAX_TILES: usize = 400;
/// Largest tile read, bytes.
const MAX_TILE_BYTES: usize = 512 * 1024;
/// Largest search or elevation answer read, bytes.
const MAX_ANSWER: usize = 2 * 1024 * 1024;
/// Web Mercator stops here.
const MAX_LAT: f64 = 85.051_128_78;
/// Range of the square's half side, meters.
const EXTENT_RANGE: std::ops::RangeInclusive<f64> = 25.0..=500.0;

type Job<T> = Receiver<Result<T, String>>;

/// The open Site Location window.
pub struct SiteDialog {
    /// The location being edited (the map's center).
    site: SiteLocation,
    zoom: f64,
    query: String,
    results: Vec<Place>,
    last_request: Option<Instant>,
    search: Option<Job<Vec<Place>>>,
    reverse: Option<Job<Option<Place>>>,
    elevation: Option<Job<f64>>,
    context: Option<Job<String>>,
    /// Half the side of the square of map data, meters.
    extent: f64,
    masses: bool,
    view: Option<cc_core::ViewId>,
    status: String,
    tiles: Tiles,
}

/// Open the window on the project's location (else a view of the whole country map).
pub fn open(state: &mut AppState) {
    let (site, zoom) = match &state.project().info.site {
        Some(s) => (s.clone(), 17.0),
        None => (SiteLocation::at(-15.7939, -47.8828), 4.0),
    };
    let view = state.active_view.filter(|v| {
        matches!(state.project().view(*v).map(|v| &v.kind), Some(cc_core::ViewKind::FloorPlan { mep: None, .. }))
    });
    state.site = Some(SiteDialog {
        site,
        zoom,
        query: String::new(),
        results: Vec::new(),
        last_request: None,
        search: None,
        reverse: None,
        elevation: None,
        context: None,
        extent: 100.0,
        masses: true,
        view,
        status: String::new(),
        tiles: Tiles::default(),
    });
}

pub fn show(ctx: &Context, state: &mut AppState) {
    let Some(mut d) = state.site.take() else { return };
    session::begin(state);
    d.poll(state);
    let mut ok = 0;
    let mut import = false;
    let response = Dialog::new("site-location", t!("dlg-site-location")).width(920.0).show(
        ctx,
        |ui| body(ui, state, &mut d, &mut import),
        |f| {
            ok = f.primary(t!("action-ok"));
            f.secondary(t!("action-cancel"));
        },
    );
    if import {
        d.import(ctx, state);
    }
    if response.clicked == Some(ok) && !d.store(state) {
        // The location was refused: keep the window open with the message.
        state.site = Some(d);
        return;
    }
    if crate::dialogs::ok_cancel(state, &response, ok) {
        return;
    }
    state.site = Some(d);
}

fn body(ui: &mut Ui, state: &mut AppState, d: &mut SiteDialog, import: &mut bool) {
    // Search row.
    ui.horizontal(|ui| {
        let r = ui.add(
            egui::TextEdit::singleline(&mut d.query)
                .hint_text(t!("site-search-hint"))
                .desired_width(ui.available_width() - 110.0),
        );
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let busy = d.search.is_some();
        let clicked = ui.add_enabled(!busy && d.can_request(), egui::Button::new(t!("site-search"))).clicked();
        if (clicked || enter) && !busy && d.can_request() && !d.query.trim().is_empty() {
            d.start_search(ui.ctx());
        }
        if busy {
            ui.spinner();
        }
    });
    if !d.results.is_empty() {
        let mut picked = None;
        egui::ScrollArea::vertical().id_salt("site-results").max_height(96.0).show(ui, |ui| {
            for (i, place) in d.results.iter().enumerate() {
                if ui.selectable_label(false, &place.name).clicked() {
                    picked = Some(i);
                }
            }
        });
        if let Some(place) = picked.and_then(|i| d.results.get(i).cloned()) {
            d.take_place(&place, true);
            d.results.clear();
        }
    }
    ui.add_space(6.0);
    ui.horizontal_top(|ui| {
        map(ui, d);
        ui.add_space(12.0);
        ui.vertical(|ui| form(ui, state, d, import));
    });
    if !d.status.is_empty() {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(&d.status).color(PALETTE.text_muted).size(12.0));
    }
}

fn row_label(ui: &mut Ui, label: String) {
    ui.label(egui::RichText::new(label).color(PALETTE.text_muted));
}

fn form(ui: &mut Ui, state: &mut AppState, d: &mut SiteDialog, import: &mut bool) {
    egui::Grid::new("site-form").num_columns(2).spacing([10.0, 6.0]).min_col_width(96.0).show(ui, |ui| {
        row_label(ui, t!("site-latitude"));
        ui.add(
            egui::DragValue::new(&mut d.site.latitude)
                .range(-MAX_LAT..=MAX_LAT)
                .speed(1e-5)
                .max_decimals(7)
                .suffix("°"),
        );
        ui.end_row();
        row_label(ui, t!("site-longitude"));
        ui.add(
            egui::DragValue::new(&mut d.site.longitude).range(-180.0..=180.0).speed(1e-5).max_decimals(7).suffix("°"),
        );
        ui.end_row();
        row_label(ui, t!("site-elevation"));
        ui.horizontal(|ui| {
            ui.add(
                crate::properties::length_drag(&mut d.site.elevation).range(cc_core::site::ELEVATION_RANGE).speed(0.1),
            );
            let busy = d.elevation.is_some();
            if ui
                .add_enabled(!busy, egui::Button::new(t!("site-elevation-fetch")).small())
                .on_hover_text(t!("site-elevation-tip"))
                .clicked()
            {
                d.start_elevation(ui.ctx());
            }
            if busy {
                ui.spinner();
            }
        });
        ui.end_row();
        row_label(ui, t!("site-true-north"));
        ui.add(
            egui::DragValue::new(&mut d.site.true_north).range(-180.0..=180.0).speed(0.5).max_decimals(2).suffix("°"),
        )
        .on_hover_text(t!("site-true-north-tip"));
        ui.end_row();
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t!("site-address")).strong());
        let busy = d.reverse.is_some();
        if ui
            .add_enabled(!busy && d.can_request(), egui::Button::new(t!("site-address-here")).small())
            .on_hover_text(t!("site-address-here-tip"))
            .clicked()
        {
            d.start_reverse(ui.ctx());
        }
        if busy {
            ui.spinner();
        }
    });
    egui::Grid::new("site-address").num_columns(2).spacing([10.0, 4.0]).min_col_width(96.0).show(ui, |ui| {
        let parts = [
            ("street", t!("site-addr-street")),
            ("number", t!("site-addr-number")),
            ("district", t!("site-addr-district")),
            ("city", t!("site-addr-city")),
            ("region", t!("site-addr-region")),
            ("postcode", t!("site-addr-postcode")),
            ("country", t!("site-addr-country")),
        ];
        for (key, label) in parts {
            row_label(ui, label);
            if let Some(part) = d.site.address.part_mut(key) {
                ui.add(egui::TextEdit::singleline(part).desired_width(220.0));
            }
            ui.end_row();
        }
    });
    ui.add_space(8.0);
    ui.separator();
    ui.label(egui::RichText::new(t!("site-context")).strong());
    egui::Grid::new("site-context").num_columns(2).spacing([10.0, 6.0]).min_col_width(96.0).show(ui, |ui| {
        row_label(ui, t!("site-extent"));
        let mut side = d.extent * 2.0;
        let range = (EXTENT_RANGE.start() * 2.0)..=(EXTENT_RANGE.end() * 2.0);
        let slider = egui::Slider::new(&mut side, range)
            .step_by(10.0)
            .custom_formatter(|v, _| cc_views::format::length(v))
            .custom_parser(cc_views::format::parse_length);
        if ui.add(slider).changed() {
            d.extent = side / 2.0;
        }
        ui.end_row();
        row_label(ui, t!("site-view"));
        let plans: Vec<(cc_core::ViewId, String)> = state
            .project()
            .views
            .values()
            .filter(|v| matches!(v.kind, cc_core::ViewKind::FloorPlan { mep: None, .. }))
            .map(|v| (v.id, v.name.clone()))
            .collect();
        if d.view.is_none_or(|v| !plans.iter().any(|(id, _)| *id == v)) {
            d.view = plans.first().map(|(id, _)| *id);
        }
        let shown = plans.iter().find(|(id, _)| Some(*id) == d.view).map(|(_, n)| n.clone()).unwrap_or_default();
        egui::ComboBox::from_id_salt("site-view").selected_text(shown).width(200.0).show_ui(ui, |ui| {
            for (id, name) in &plans {
                ui.selectable_value(&mut d.view, Some(*id), name);
            }
        });
        ui.end_row();
    });
    ui.checkbox(&mut d.masses, t!("site-masses")).on_hover_text(t!("site-masses-tip"));
    ui.horizontal(|ui| {
        let busy = d.context.is_some();
        if ui
            .add_enabled(!busy && d.view.is_some(), egui::Button::new(t!("site-import")))
            .on_hover_text(t!("site-import-tip"))
            .clicked()
        {
            *import = true;
        }
        if busy {
            ui.spinner();
        }
    });
}

// --- The map ---

/// Web Mercator: world pixels of `lat`, `lon` at zoom 0 (one 256-pixel tile).
fn world(lat: f64, lon: f64) -> (f64, f64) {
    let lat = lat.clamp(-MAX_LAT, MAX_LAT).to_radians();
    let x = (lon + 180.0) / 360.0;
    let y = (1.0 - (lat.tan() + 1.0 / lat.cos()).ln() / std::f64::consts::PI) / 2.0;
    (x * TILE, y * TILE)
}

/// Latitude and longitude of world pixels at zoom 0.
fn unworld(x: f64, y: f64) -> (f64, f64) {
    let lon = x / TILE * 360.0 - 180.0;
    let n = std::f64::consts::PI * (1.0 - 2.0 * y / TILE);
    (n.sinh().atan().to_degrees().clamp(-MAX_LAT, MAX_LAT), lon)
}

fn map(ui: &mut Ui, d: &mut SiteDialog) {
    let size = vec2(560.0, 470.0);
    let (rect, r) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let scale = 2f64.powf(d.zoom);
    let center = rect.center();
    let (cx, cy) = world(d.site.latitude, d.site.longitude);
    let (cx, cy) = (cx * scale, cy * scale);
    // Drag moves the map under the marker; the wheel zooms about the center.
    // Two fingers on a trackpad move the map like a drag (see `gestures`).
    let gesture = if r.hovered() { Some(crate::gestures::read(ui)) } else { None };
    let fingers = gesture.map_or(egui::Vec2::ZERO, |g| g.pan);
    if r.dragged() || fingers != egui::Vec2::ZERO {
        let delta = if r.dragged() { r.drag_delta() } else { fingers };
        let (lat, lon) = unworld((cx - delta.x as f64) / scale, (cy - delta.y as f64) / scale);
        d.site.latitude = lat;
        d.site.longitude = ((lon + 180.0).rem_euclid(360.0)) - 180.0;
    }
    if let Some(g) = gesture {
        // One zoom level per 240 points of wheel scroll, or per doubling of a pinch.
        let levels = g.zoom(std::f64::consts::LN_2 / 240.0).log2();
        if levels.abs() > 1e-9 {
            d.zoom = (d.zoom + levels).clamp(MIN_ZOOM, MAX_ZOOM);
        }
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::from_gray(224));

    let z = d.zoom.round().clamp(MIN_ZOOM, MAX_ZOOM) as u8;
    let tile_px = TILE * 2f64.powf(d.zoom - f64::from(z));
    let n = 1u32 << z;
    let to_screen = |wx: f64, wy: f64| pos2(center.x + (wx - cx) as f32, center.y + (wy - cy) as f32);
    let (left, top) = (cx - f64::from(size.x) / 2.0, cy - f64::from(size.y) / 2.0);
    let (x0, y0) = ((left / tile_px).floor() as i64, (top / tile_px).floor() as i64);
    let (x1, y1) =
        (((left + f64::from(size.x)) / tile_px).floor() as i64, ((top + f64::from(size.y)) / tile_px).floor() as i64);
    d.tiles.receive(ui.ctx());
    for ty in y0.max(0)..=y1.min(i64::from(n) - 1) {
        for tx in x0..=x1 {
            let wrapped = tx.rem_euclid(i64::from(n)) as u32;
            let min = to_screen(tx as f64 * tile_px, ty as f64 * tile_px);
            let tile_rect = Rect::from_min_size(min, Vec2::splat(tile_px as f32));
            if let Some(tex) = d.tiles.get(ui.ctx(), (z, wrapped, ty as u32)) {
                painter.image(tex, tile_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
        }
    }

    // The square of map data, turned with true north.
    let e = d.extent;
    let corners = [(-e, -e), (e, -e), (e, e), (-e, e)].map(|(x, y)| {
        let (lat, lon) = d.site.from_project(glam::DVec2::new(x, y));
        let (wx, wy) = world(lat, lon);
        to_screen(wx * scale, wy * scale)
    });
    let frame = Color32::from_rgb(220, 60, 40);
    painter.add(egui::Shape::convex_polygon(corners.to_vec(), frame.gamma_multiply(0.08), Stroke::new(2.0, frame)));
    // Project north, from the center.
    let north = d.site.north();
    let up = (north.x as f32, -north.y as f32);
    let tip = center + vec2(up.0, up.1) * 46.0;
    painter.arrow(center, tip - center, Stroke::new(1.5, frame));
    // The marker.
    painter.circle(center, 6.0, frame, Stroke::new(2.0, Color32::WHITE));
    painter.line_segment([center - vec2(14.0, 0.0), center - vec2(8.0, 0.0)], Stroke::new(1.5, frame));
    painter.line_segment([center + vec2(8.0, 0.0), center + vec2(14.0, 0.0)], Stroke::new(1.5, frame));

    // Attribution (required) and the scale.
    let mpp = 156_543.033_92 * d.site.latitude.to_radians().cos() / scale;
    let (bar_m, bar_text) = scale_bar(mpp * 100.0, cc_views::format::imperial());
    let bar_px = (bar_m / mpp) as f32;
    let base = rect.left_bottom() + vec2(10.0, -10.0);
    painter.line_segment([base, base + vec2(bar_px, 0.0)], Stroke::new(3.0, Color32::from_gray(40)));
    painter.text(
        base + vec2(0.0, -4.0),
        egui::Align2::LEFT_BOTTOM,
        bar_text,
        egui::FontId::proportional(11.0),
        Color32::from_gray(30),
    );
    let attribution = Rect::from_min_size(rect.right_bottom() - vec2(176.0, 18.0), vec2(176.0, 18.0));
    painter.rect_filled(attribution, 0.0, Color32::from_white_alpha(200));
    ui.put(
        attribution,
        egui::Hyperlink::from_label_and_url(egui::RichText::new(osm::ATTRIBUTION).size(10.5), COPYRIGHT_URL),
    );
    if d.tiles.failed > 0 && d.tiles.ready == 0 {
        painter.text(
            center + vec2(0.0, 40.0),
            egui::Align2::CENTER_TOP,
            t!("site-map-offline"),
            egui::FontId::proportional(12.0),
            Color32::from_gray(60),
        );
    }
}

/// 1, 2 or 5 times a power of ten, at most `max` meters.
fn nice_length(max: f64) -> f64 {
    if !(max.is_finite() && max > 0.0) {
        return 1.0;
    }
    let p = 10f64.powf(max.log10().floor());
    [5.0, 2.0, 1.0].into_iter().map(|k| k * p).find(|v| *v <= max).unwrap_or(p)
}

fn format_length(m: f64) -> String {
    if m >= 1000.0 { format!("{} km", m / 1000.0) } else { format!("{m} m") }
}

/// Feet in a mile.
const MILE_FEET: f64 = 5280.0;

/// The scale bar for at most `max` meters: its length in meters and its
/// text, in meters and kilometers, or in feet and miles when `imperial`.
fn scale_bar(max: f64, imperial: bool) -> (f64, String) {
    if !imperial {
        let m = nice_length(max);
        return (m, format_length(m));
    }
    let foot = cc_core::units::FOOT;
    let feet = max / foot;
    if feet >= MILE_FEET {
        let miles = nice_length(feet / MILE_FEET);
        return (miles * MILE_FEET * foot, format!("{miles} mi"));
    }
    let feet = nice_length(feet);
    (feet * foot, format!("{feet} ft"))
}

// --- Tiles ---

type TileKey = (u8, u32, u32);

enum TileState {
    Loading,
    Ready(egui::TextureHandle),
    Failed,
}

/// Map tiles as textures, fetched by two background threads.
#[derive(Default)]
struct Tiles {
    cache: HashMap<TileKey, TileState>,
    requests: Option<Sender<TileKey>>,
    answers: Option<Receiver<(TileKey, Result<cc_views::raster::Rgba, String>)>>,
    ready: usize,
    failed: usize,
}

impl Tiles {
    fn start(&mut self, ctx: &Context) {
        let (req_tx, req_rx) = std::sync::mpsc::channel::<TileKey>();
        let (ans_tx, ans_rx) = std::sync::mpsc::channel();
        let req_rx = Arc::new(Mutex::new(req_rx));
        for i in 0..2 {
            let req_rx = req_rx.clone();
            let ans_tx = ans_tx.clone();
            let ctx = ctx.clone();
            let started = std::thread::Builder::new().name(format!("map-tiles-{i}")).spawn(move || {
                loop {
                    let next = match req_rx.lock() {
                        Ok(rx) => rx.recv(),
                        Err(_) => return,
                    };
                    let Ok((z, x, y)) = next else { return };
                    let result = get(&format!("{TILE_URL}/{z}/{x}/{y}.png"), MAX_TILE_BYTES).and_then(|bytes| {
                        let image = cc_views::raster::probe(bytes).ok_or("not a picture")?;
                        cc_views::raster::decode(&image, 256).ok_or_else(|| "not a picture".to_string())
                    });
                    if ans_tx.send(((z, x, y), result)).is_err() {
                        return;
                    }
                    ctx.request_repaint();
                }
            });
            if let Err(e) = started {
                log::warn!("could not start the map tiles thread: {e}");
            }
        }
        self.requests = Some(req_tx);
        self.answers = Some(ans_rx);
    }

    fn receive(&mut self, ctx: &Context) {
        let Some(rx) = &self.answers else { return };
        while let Ok((key, result)) = rx.try_recv() {
            let state = match result {
                Ok(rgba) => {
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [rgba.width as usize, rgba.height as usize],
                        &rgba.pixels,
                    );
                    self.ready += 1;
                    let name = format!("tile-{}-{}-{}", key.0, key.1, key.2);
                    TileState::Ready(ctx.load_texture(name, image, egui::TextureOptions::LINEAR))
                }
                Err(e) => {
                    log::debug!("map tile {key:?}: {e}");
                    self.failed += 1;
                    TileState::Failed
                }
            };
            self.cache.insert(key, state);
        }
    }

    /// The texture of a tile, asking for it when it is not there yet.
    fn get(&mut self, ctx: &Context, key: TileKey) -> Option<egui::TextureId> {
        if self.cache.len() > MAX_TILES {
            self.cache.retain(|_, s| matches!(s, TileState::Loading));
        }
        match self.cache.get(&key) {
            Some(TileState::Ready(tex)) => Some(tex.id()),
            Some(_) => None,
            None => {
                if self.requests.is_none() {
                    self.start(ctx);
                }
                if let Some(tx) = &self.requests
                    && tx.send(key).is_ok()
                {
                    self.cache.insert(key, TileState::Loading);
                }
                None
            }
        }
    }
}

// --- Requests ---

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        // No request, and no redirect, leaves https.
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(90)))
        .user_agent(format!(
            "ConstructCraft/{} (+https://github.com/francisreboucas/constructcraft)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .into()
}

fn offline() -> bool {
    cfg!(test) || crate::update::disabled_by_env()
}

/// GET an https `url`, at most `limit` bytes.
fn get(url: &str, limit: usize) -> Result<Vec<u8>, String> {
    if offline() {
        return Err("offline".into());
    }
    let mut response = agent().get(url).call().map_err(|e| e.to_string())?;
    response.body_mut().with_config().limit(limit as u64).read_to_vec().map_err(|e| e.to_string())
}

fn get_text(url: &str, limit: usize) -> Result<String, String> {
    String::from_utf8(get(url, limit)?).map_err(|e| e.to_string())
}

/// How many times an Overpass query is sent while the server is busy.
const OVERPASS_TRIES: u32 = 4;
/// The wait before the first retry; it doubles after each one.
const OVERPASS_FIRST_WAIT: Duration = Duration::from_secs(3);

/// POST an Overpass query, the answer as text (on a worker thread: it
/// waits between retries).
fn overpass(query: &str) -> Result<String, String> {
    if offline() {
        return Err("offline".into());
    }
    let body = format!("data={}", encode(query));
    let send = || -> Result<String, ureq::Error> {
        let mut response = agent()
            .post(OVERPASS_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send(body.as_bytes())?;
        response.body_mut().with_config().limit(osm::MAX_JSON as u64).read_to_string()
    };
    with_retries(send, std::thread::sleep)
}

/// Statuses with which the public Overpass server says it is busy (at
/// capacity, rate limited, timed out), asking clients to retry later.
fn server_busy(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

/// Send with `send` until it succeeds, retrying while the server is busy
/// with waits that double (`wait` sleeps), at most [`OVERPASS_TRIES`] times.
fn with_retries(
    mut send: impl FnMut() -> Result<String, ureq::Error>,
    mut wait: impl FnMut(Duration),
) -> Result<String, String> {
    let mut pause = OVERPASS_FIRST_WAIT;
    for attempt in 1..=OVERPASS_TRIES {
        match send() {
            Ok(text) => return Ok(text),
            Err(ureq::Error::StatusCode(status)) if server_busy(status) => {
                if attempt == OVERPASS_TRIES {
                    return Err(t!("site-server-busy", "status" => status));
                }
                wait(pause);
                pause *= 2;
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Err(t!("site-server-busy", "status" => 0))
}

/// Percent-encode a query value.
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn spawn<T: Send + 'static>(
    ctx: &Context,
    name: &str,
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Job<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    let ctx = ctx.clone();
    let started = std::thread::Builder::new().name(name.into()).spawn(move || {
        let _ = tx.send(work());
        ctx.request_repaint();
    });
    if let Err(e) = started {
        log::warn!("could not start {name}: {e}");
    }
    rx
}

/// The result of a finished job (`Some(Err)` when its thread went away).
fn take<T>(job: &mut Option<Job<T>>) -> Option<Result<T, String>> {
    let rx = job.as_ref()?;
    let out = match rx.try_recv() {
        Ok(v) => v,
        Err(TryRecvError::Empty) => return None,
        Err(TryRecvError::Disconnected) => Err("stopped".into()),
    };
    *job = None;
    Some(out)
}

impl SiteDialog {
    /// One search request per second at most.
    fn can_request(&self) -> bool {
        self.last_request.is_none_or(|t| t.elapsed() >= Duration::from_secs(1))
    }

    fn start_search(&mut self, ctx: &Context) {
        self.last_request = Some(Instant::now());
        let lang = cc_engine::i18n::language().code();
        let url = format!(
            "{SEARCH_URL}/search?format=jsonv2&addressdetails=1&limit=6&accept-language={}&q={}",
            encode(lang),
            encode(self.query.trim())
        );
        self.status.clear();
        self.search = Some(spawn(ctx, "site-search", move || osm::read_places(&get_text(&url, MAX_ANSWER)?)));
    }

    fn start_reverse(&mut self, ctx: &Context) {
        self.last_request = Some(Instant::now());
        let lang = cc_engine::i18n::language().code();
        let url = format!(
            "{SEARCH_URL}/reverse?format=jsonv2&addressdetails=1&zoom=18&accept-language={}&lat={:.7}&lon={:.7}",
            encode(lang),
            self.site.latitude,
            self.site.longitude
        );
        self.reverse = Some(spawn(ctx, "site-reverse", move || osm::read_reverse(&get_text(&url, MAX_ANSWER)?)));
    }

    fn start_elevation(&mut self, ctx: &Context) {
        let url = format!("{ELEVATION_URL}?latitude={:.6}&longitude={:.6}", self.site.latitude, self.site.longitude);
        self.elevation = Some(spawn(ctx, "site-elevation", move || osm::read_elevation(&get_text(&url, MAX_ANSWER)?)));
    }

    /// Center on a found place and take its address (and name, when `name`).
    fn take_place(&mut self, place: &Place, name: bool) {
        self.site.latitude = place.latitude.clamp(-MAX_LAT, MAX_LAT);
        self.site.longitude = place.longitude;
        if !place.address.is_empty() {
            self.site.address = place.address.clone();
        }
        if name {
            self.site.place = place.name.clone();
            self.zoom = self.zoom.max(17.0);
        }
    }

    /// Collect finished requests.
    fn poll(&mut self, state: &mut AppState) {
        match take(&mut self.search) {
            Some(Ok(places)) if places.is_empty() => self.status = t!("site-search-none"),
            Some(Ok(places)) => {
                if let [only] = places.as_slice() {
                    self.take_place(only, true);
                    self.results.clear();
                } else {
                    self.results = places;
                }
            }
            Some(Err(e)) => self.status = t!("site-request-failed", "error" => e),
            None => {}
        }
        match take(&mut self.reverse) {
            Some(Ok(Some(place))) => {
                self.site.address = place.address.clone();
                self.site.place = place.name;
            }
            Some(Ok(None)) => self.status = t!("site-search-none"),
            Some(Err(e)) => self.status = t!("site-request-failed", "error" => e),
            None => {}
        }
        match take(&mut self.elevation) {
            Some(Ok(z)) => {
                self.site.elevation = (z * 100.0).round() / 100.0;
                self.status = t!("site-elevation-done");
            }
            Some(Err(e)) => self.status = t!("site-request-failed", "error" => e),
            None => {}
        }
        match take(&mut self.context) {
            Some(Ok(data)) => {
                let command = SiteContextImport {
                    data: Some(data),
                    path: None,
                    view: self.view.map(|v| cc_engine::Ref::Id(v.0)),
                    extent: Some(self.extent),
                    masses: Some(self.masses),
                    level_height: None,
                    default_levels: None,
                    text_height: None,
                };
                match state.session.run(command) {
                    Ok(out) => {
                        let masses = out.get("masses").and_then(|m| m.as_array()).map_or(0, Vec::len);
                        self.status = t!("site-import-done", "masses" => masses.to_string());
                    }
                    Err(e) => self.status = e.message,
                }
            }
            Some(Err(e)) => self.status = t!("site-request-failed", "error" => e),
            None => {}
        }
    }

    /// The location as a `site_set` command.
    fn command(&self) -> SiteSet {
        let address: serde_json::Map<String, serde_json::Value> =
            self.site.address.parts().iter().map(|(k, v)| (k.to_string(), serde_json::json!(v))).collect();
        SiteSet {
            latitude: Some(self.site.latitude),
            longitude: Some(self.site.longitude),
            elevation: Some(self.site.elevation),
            true_north: Some(self.site.true_north),
            place: Some(self.site.place.clone()),
            address: (!self.site.address.is_empty()).then_some(address),
            project_address: None,
            // The dialog does not edit the offset: the stored one stays.
            utc_offset: None,
            clear: None,
        }
    }

    /// Store the location when it changed; false (with the message) when refused.
    fn store(&mut self, state: &mut AppState) -> bool {
        if state.project().info.site.as_ref() == Some(&self.site) {
            return true;
        }
        match state.session.run(self.command()) {
            Ok(_) => true,
            Err(e) => {
                self.status = e.message;
                false
            }
        }
    }

    /// Store the location, then download the map data of the square.
    fn import(&mut self, ctx: &Context, state: &mut AppState) {
        if !self.store(state) {
            return;
        }
        self.status = t!("site-import-loading");
        let query = osm::overpass_query(&self.site, self.extent);
        self.context = Some(spawn(ctx, "site-context", move || overpass(&query)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mercator_round_trip() {
        for (lat, lon) in [(-3.7319, -38.5267), (51.5, -0.12), (0.0, 0.0), (80.0, 179.0)] {
            let (x, y) = world(lat, lon);
            let (a, b) = unworld(x, y);
            assert!((a - lat).abs() < 1e-9 && (b - lon).abs() < 1e-9);
        }
        // The equator and the prime meridian cross at the middle of the world.
        assert_eq!(world(0.0, 0.0), (128.0, 128.0));
    }

    #[test]
    fn scale_bar_lengths() {
        assert_eq!(nice_length(130.0), 100.0);
        assert_eq!(nice_length(260.0), 200.0);
        assert_eq!(nice_length(7.0), 5.0);
        assert_eq!(format_length(2000.0), "2 km");
        assert_eq!(scale_bar(130.0, false), (100.0, "100 m".to_string()));
        // 130 m is 426 ft: a 200 ft bar; 20 km is 12.4 miles: a 10 mi bar.
        let (m, text) = scale_bar(130.0, true);
        assert!((m - 200.0 * cc_core::units::FOOT).abs() < 1e-9 && text == "200 ft", "{text}");
        let (m, text) = scale_bar(20_000.0, true);
        assert!((m - 10.0 * 5280.0 * cc_core::units::FOOT).abs() < 1e-6 && text == "10 mi", "{text}");
    }

    #[test]
    fn query_values_are_encoded() {
        assert_eq!(encode("Rua A, 10 ç"), "Rua%20A%2C%2010%20%C3%A7");
    }

    #[test]
    fn tests_never_reach_the_network() {
        assert_eq!(get("https://tile.openstreetmap.org/0/0/0.png", 10), Err("offline".into()));
        assert_eq!(overpass("[out:json];"), Err("offline".into()));
    }

    /// The public Overpass server answers 500, 502 or 504 when it is at
    /// capacity; the query is sent again with doubling waits, and other
    /// errors are not retried.
    #[test]
    fn a_busy_map_server_is_asked_again() {
        let answers = |codes: Vec<Option<u16>>| {
            let mut codes = codes.into_iter();
            let mut waits = Vec::new();
            let mut sent = 0;
            let result = with_retries(
                || {
                    sent += 1;
                    match codes.next().flatten() {
                        Some(code) => Err(ureq::Error::StatusCode(code)),
                        None => Ok("{}".to_string()),
                    }
                },
                |d| waits.push(d.as_secs()),
            );
            (result, sent, waits)
        };
        // Busy twice, then the data.
        assert_eq!(answers(vec![Some(502), Some(500), None]), (Ok("{}".into()), 3, vec![3, 6]));
        // Busy every time: a message that says so, after every try.
        let (result, sent, waits) = answers(vec![Some(504); 4]);
        assert_eq!((sent, waits), (OVERPASS_TRIES, vec![3, 6, 12]));
        assert_eq!(result, Err(t!("site-server-busy", "status" => 504)));
        // A bad query is not retried.
        assert_eq!(answers(vec![Some(400)]), (Err("http status: 400".into()), 1, vec![]));
    }
}
