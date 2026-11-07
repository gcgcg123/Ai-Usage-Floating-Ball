mod layout;

pub mod app_state;
pub mod commands;
pub mod config_store;
pub mod credentials;
pub mod provider_manager;
pub mod providers;
pub mod snapshot_store;

use std::sync::Mutex;

use tauri::{
    LogicalSize, Manager, PhysicalPosition, Position, State, WebviewWindow,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
};

use layout::{
    COLLAPSED_LOGICAL_SIZE, Dock, EXPANDED_LOGICAL_HEIGHT, EXPANDED_LOGICAL_WIDTH, Placement,
    Point, Rect, Size, ball_center, choose_placement, collapsed_size, docked_collapsed_position,
    expanded_position,
};

#[derive(Default)]
struct HudLayoutState {
    placement: Mutex<Option<Placement>>,
    dock: Mutex<Dock>,
}

#[allow(dead_code)]
struct TrayState(TrayIcon);

fn monitor_rect(window: &WebviewWindow) -> Result<Rect, String> {
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No monitor found".to_string())?;

    Ok(Rect {
        x: monitor.position().x as f64,
        y: monitor.position().y as f64,
        width: monitor.size().width as f64,
        height: monitor.size().height as f64,
    })
}

fn window_position(window: &WebviewWindow) -> Result<Point, String> {
    let position = window.outer_position().map_err(|error| error.to_string())?;
    Ok(Point::new(position.x as f64, position.y as f64))
}

fn window_size(window: &WebviewWindow) -> Result<Size, String> {
    let size = window.outer_size().map_err(|error| error.to_string())?;
    Ok(Size {
        width: size.width as f64,
        height: size.height as f64,
    })
}

fn set_physical_position(window: &WebviewWindow, position: Point) -> Result<(), String> {
    window
        .set_position(Position::Physical(PhysicalPosition::new(
            position.x.round() as i32,
            position.y.round() as i32,
        )))
        .map_err(|error| error.to_string())
}

fn position_bottom_right(window: &WebviewWindow) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No monitor found".to_string())?;
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let margin = (12.0 * scale).round() as i32;
    let taskbar_reserve = (52.0 * scale).round() as i32;

    let x = monitor.position().x + monitor.size().width as i32 - size.width as i32 - margin;
    let y =
        monitor.position().y + monitor.size().height as i32 - size.height as i32 - taskbar_reserve;

    window
        .set_position(Position::Physical(PhysicalPosition::new(x, y)))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_hud_expanded(
    window: WebviewWindow,
    layout_state: State<'_, HudLayoutState>,
    expanded: bool,
) -> Result<String, String> {
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let monitor = monitor_rect(&window)?;
    let current_position = window_position(&window)?;
    let current_size = window_size(&window)?;

    if expanded {
        let anchor = Point::new(
            current_position.x + current_size.width / 2.0,
            current_position.y + current_size.height / 2.0,
        );
        let placement = choose_placement(anchor, monitor, scale);
        let position = expanded_position(anchor, placement, monitor, scale);

        window
            .set_size(LogicalSize::new(
                EXPANDED_LOGICAL_WIDTH,
                EXPANDED_LOGICAL_HEIGHT,
            ))
            .map_err(|error| error.to_string())?;
        set_physical_position(&window, position)?;

        let mut state = layout_state
            .placement
            .lock()
            .map_err(|error| error.to_string())?;
        *state = Some(placement);
        Ok(placement.as_str().to_string())
    } else {
        let placement = layout_state
            .placement
            .lock()
            .map_err(|error| error.to_string())?
            .unwrap_or(Placement::AboveLeft);
        let anchor = ball_center(current_position, current_size, placement, scale);

        window
            .set_size(LogicalSize::new(
                COLLAPSED_LOGICAL_SIZE,
                COLLAPSED_LOGICAL_SIZE,
            ))
            .map_err(|error| error.to_string())?;

        let collapsed = window_size(&window)?;
        let expected = collapsed_size(scale);
        let dock = *layout_state
            .dock
            .lock()
            .map_err(|error| error.to_string())?;
        let base_position = docked_collapsed_position(anchor, monitor, scale, dock);
        let position = Point::new(
            base_position.x + (collapsed.width - expected.width) / 2.0,
            base_position.y + (collapsed.height - expected.height) / 2.0,
        );
        set_physical_position(&window, position)?;

        let mut state = layout_state
            .placement
            .lock()
            .map_err(|error| error.to_string())?;
        *state = None;
        Ok(placement.as_str().to_string())
    }
}

#[tauri::command]
fn set_hud_dock(
    window: WebviewWindow,
    layout_state: State<'_, HudLayoutState>,
    dock: String,
) -> Result<String, String> {
    let dock = Dock::from_str(&dock).ok_or_else(|| "invalid dock value".to_string())?;
    *layout_state
        .dock
        .lock()
        .map_err(|error| error.to_string())? = dock;

    let collapsed = layout_state
        .placement
        .lock()
        .map_err(|error| error.to_string())?
        .is_none();
    if collapsed {
        let scale = window.scale_factor().map_err(|error| error.to_string())?;
        let monitor = monitor_rect(&window)?;
        let position = window_position(&window)?;
        let size = window_size(&window)?;
        let center = Point::new(
            position.x + size.width / 2.0,
            position.y + size.height / 2.0,
        );
        let target = docked_collapsed_position(center, monitor, scale, dock);
        set_physical_position(&window, target)?;
    }

    Ok(dock.as_str().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(HudLayoutState::default())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let show_item =
                MenuItem::with_id(app, "toggle-main", "顯示/隱藏懸浮球", true, None::<&str>)?;
            let settings_item =
                MenuItem::with_id(app, "open-settings", "開啟 Settings", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "結束", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &show_item,
                    &settings_item,
                    &PredefinedMenuItem::separator(app)?,
                    &quit_item,
                ],
            )?;
            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or_else(|| std::io::Error::other("missing default window icon"))?;
            let tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .tooltip("AI Usage Floating Ball")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "toggle-main" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = if window.is_visible().unwrap_or(false) {
                                window.hide()
                            } else {
                                window.show().and_then(|_| window.set_focus())
                            };
                        }
                    }
                    "open-settings" => {
                        tauri::async_runtime::spawn(commands::open_settings_window(app.clone()));
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = if window.is_visible().unwrap_or(false) {
                                window.hide()
                            } else {
                                window.show().and_then(|_| window.set_focus())
                            };
                        }
                    }
                })
                .build(app)?;
            app.manage(TrayState(tray));

            let state = app_state::AppState::load(app.handle()).map_err(std::io::Error::other)?;
            app.manage(state);
            app.state::<app_state::AppState>().manager.start();
            if let Some(window) = app.get_webview_window("main") {
                position_bottom_right(&window)?;
                window.show()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_hud_expanded,
            set_hud_dock,
            commands::provider_catalog,
            commands::list_provider_profiles,
            commands::get_provider_profile,
            commands::save_provider_profile,
            commands::delete_provider_profile,
            commands::select_provider_profile,
            commands::provider_has_credential,
            commands::move_provider_profile,
            commands::test_provider_profile,
            commands::refresh_provider_profile,
            commands::get_usage_state,
            commands::refresh_all_providers,
            commands::pause_monitoring,
            commands::resume_monitoring,
            commands::open_settings_window,
            commands::get_autostart,
            commands::set_autostart
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
