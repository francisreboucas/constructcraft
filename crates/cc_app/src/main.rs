//! ConstructCraft — open source BIM authoring.

// Release builds on Windows open no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod app;
mod bracing_ui;
mod browser;
mod browser_org_dialog;
mod cad_ui;
mod callout_ui;
mod canvas;
mod clipboard;
mod commands;
mod component_env;
mod component_sketch;
mod connection_ui;
mod curtain_ui;
mod design_options_ui;
mod detail_component_ui;
mod detail_ui;
mod dialog;
mod dialogs;
mod dim_edit;
mod dim_type_ui;
mod engine_edit;
mod fill_patterns_dialog;
mod gestures;
mod gpu;
mod graph_ui;
mod graphics_dialog;
mod grid_frame_ui;
mod home;
mod i18n;
mod icons;
mod jobs;
mod level_mark_ui;
mod library;
mod line_styles_dialog;
mod lps_cabling_ui;
mod material_browser;
mod mcp_bridge;
mod member_host_ui;
mod modify;
mod modify3d;
mod new_component_dialog;
mod new_project_dialog;
mod obj_ui;
mod opening_place;
mod paint;
mod paint_cache;
mod paint_ui;
mod palette;
mod phases_dialog;
mod plugins_ui;
mod preferences;
mod profile_sketch;
mod profiles_dialog;
mod properties;
mod recovery;
mod region_ui;
mod ribbon;
mod schedule_ui;
mod section_mark;
mod sheet;
mod sheet_props;
mod side_panels;
mod site_ui;
mod sketch_mode;
mod snap_geom;
mod stair_ui;
mod state;
mod steel_building_ui;
mod sun_overlay;
mod sun_ui;
mod systems_dialog;
mod table;
mod text_tool;
mod theme;
mod thumbnails;
mod title_block_env;
mod tools;
mod truss_ui;
mod type_dialog;
#[cfg(test)]
mod ui_tests;
mod units_dialog;
mod update;
mod update_install;
mod view3d;
mod view_cube;
mod wall_framing_ui;

fn main() -> eframe::Result<()> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if let Some(args) = mcp_bridge::mcp_args(&raw) {
        std::process::exit(mcp_bridge::run(args));
    }
    // OpenGL drivers report performance hints as warnings on every frame; keep only its errors
    // (`RUST_LOG` still overrides this).
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,wgpu_hal::gles=error")).init();
    // Panics are logged to the crash log (release builds on Windows have no console).
    recovery::install_panic_hook();
    let args = parse_args(raw.into_iter());
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([900.0, 600.0])
        .with_title("ConstructCraft")
        .with_icon(app_icon());
    // The Wayland app id / X11 class names the desktop file (packaging/linux),
    // so desktops show its icon. eframe keeps its settings under the same
    // (lowercased) folder name on Linux, so nothing moves.
    #[cfg(target_os = "linux")]
    let viewport = viewport.with_app_id("constructcraft");
    // The language of a start-up error message (the app then applies the saved preference).
    i18n::set_language(args.language.unwrap_or_else(i18n::Language::from_system));
    // Try the graphics APIs in turn until one shows the window (issue #74).
    let mut last_error = None;
    for api in gpu::startup_plan() {
        gpu::begin(api);
        let options = eframe::NativeOptions {
            viewport: viewport.clone(),
            renderer: eframe::Renderer::Wgpu,
            wgpu_options: gpu::wgpu_options(api),
            ..Default::default()
        };
        let args = args.clone();
        let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            eframe::run_native("ConstructCAD", options, Box::new(|cc| Ok(Box::new(app::ConstructApp::new(cc, args)))))
        }));
        match run {
            Ok(Ok(())) => return Ok(()),
            // eframe returned cleanly before the first frame: try the next API.
            Ok(Err(error)) if !gpu::started() => {
                gpu::failed(api, &error.to_string());
                last_error = Some(error);
            }
            Ok(Err(error)) => {
                recovery::log_event(&format!("The app stopped with an error: {error}"));
                return Err(error);
            }
            // A panic (already in the crash log) before the first frame:
            // restart the app, which tries the next API first.
            Err(_) if !gpu::started() && gpu::restart(api) => return Ok(()),
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }
    let error = last_error.unwrap_or_else(|| eframe::Error::AppCreation("no graphics API to try".into()));
    gpu::report_failure(&error.to_string());
    Err(error)
}

/// Window and taskbar icon.
fn app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(theme::APP_ICON_PNG).unwrap_or_default()
}

/// `constructcraft [project.ccad] [--view NAME]... [--graph FILE.ccgraph] [--lang en|pt-BR|es|fr|de|it|lo|pl|uk-UA|zh-TW|zh-CN|ja]`
///
/// Named views replace the default layout (the first floor plan beside the
/// 3D view): the first on the left, the second on the right, others as tabs.
fn parse_args(args: impl Iterator<Item = String>) -> app::Args {
    let mut out = app::Args::default();
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--view" => out.views.extend(args.next()),
            "--graph" => out.graph = args.next().map(std::path::PathBuf::from),
            "--lang" => out.language = args.next().and_then(|c| i18n::Language::from_code(&c)),
            // An unknown option is not a project path.
            _ if arg.starts_with("--") => log::warn!("unknown option {arg}"),
            _ if out.path.is_none() => out.path = Some(std::path::PathBuf::from(arg)),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod args_tests {
    #[test]
    fn unknown_options_are_not_project_paths() {
        let args = super::parse_args(["--stdio", "house.ccad", "--lang", "en"].into_iter().map(String::from));
        assert_eq!(args.path, Some(std::path::PathBuf::from("house.ccad")));
    }
}
