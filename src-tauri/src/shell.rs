//! The surface OrbitSwitch lives in: a Liquid Glass popover under the menu bar
//! icon on macOS, a compact window on Windows.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window,
    WindowEvent,
};

pub const LABEL: &str = "main";
/// Open at Login passes this on Windows so the window starts hidden; the
/// macOS popover is hidden until clicked anyway.
#[cfg(not(target_os = "macos"))]
pub const BACKGROUND_ARG: &str = "--background";
pub const SHOWN_EVENT: &str = "orbitswitch://shown";
#[cfg(target_os = "macos")]
const PANEL_WIDTH: f64 = 360.0;
/// A tray click right after the popover lost focus is the same click that
/// dismissed it, not a request to reopen it.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

#[derive(Default)]
pub struct ShellState {
    hidden_at: Mutex<Option<Instant>>,
    /// The menu bar icon's last reported rect, in the physical pixels of the
    /// screen it sits on.
    #[cfg(target_os = "macos")]
    tray_rect: Mutex<Option<tauri::Rect>>,
}

#[cfg(target_os = "macos")]
pub fn remember_tray_rect(app: &AppHandle, rect: tauri::Rect) {
    *app.state::<ShellState>()
        .tray_rect
        .lock()
        .expect("shell state poisoned") = Some(rect);
}

/// Places the popover centered under the menu bar icon, inside the icon's screen.
///
/// The icon rect arrives in the physical pixels of the icon's own screen, while
/// physical window positions are interpreted with the scale of whichever screen
/// the window last sat on, so with mixed-scale displays the two disagree and the
/// window lands off screen. Working in logical points avoids that.
#[cfg(target_os = "macos")]
fn anchor_to_tray(window: &WebviewWindow) {
    let Some(rect) = *window
        .state::<ShellState>()
        .tray_rect
        .lock()
        .expect("shell state poisoned")
    else {
        return;
    };
    let icon_position = rect.position.to_physical::<f64>(1.0);
    let icon_size = rect.size.to_physical::<f64>(1.0);
    let Ok(monitors) = window.available_monitors() else {
        return;
    };
    let screen = monitors.iter().find_map(|monitor| {
        let scale = monitor.scale_factor();
        let origin = monitor.position().to_logical::<f64>(scale);
        let size = monitor.size().to_logical::<f64>(scale);
        let (x, y) = (icon_position.x / scale, icon_position.y / scale);
        (x >= origin.x && x < origin.x + size.width && y >= origin.y && y < origin.y + size.height)
            .then_some((scale, origin, size))
    });
    let Some((scale, origin, size)) = screen else {
        return;
    };
    let Ok(window_size) = window.outer_size() else {
        return;
    };
    let window_size = window_size.to_logical::<f64>(window.scale_factor().unwrap_or(scale));
    let x = (icon_position.x + icon_size.width / 2.0) / scale - window_size.width / 2.0;
    let y = (icon_position.y + icon_size.height) / scale;
    let x = x.clamp(
        origin.x,
        (origin.x + size.width - window_size.width).max(origin.x),
    );
    let y = y.clamp(
        origin.y,
        (origin.y + size.height - window_size.height).max(origin.y),
    );
    let _ = window.set_position(tauri::LogicalPosition::new(x, y));
}

pub fn create(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("OrbitSwitch");

    #[cfg(target_os = "macos")]
    let builder = {
        builder
            .inner_size(PANEL_WIDTH, 480.0)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            // Liquid Glass supplies the surface edge; avoid a second window shadow.
            .shadow(false)
            .visible_on_all_workspaces(true)
            .accept_first_mouse(true)
    };

    #[cfg(not(target_os = "macos"))]
    let builder = builder
        .inner_size(400.0, 640.0)
        .min_inner_size(360.0, 480.0)
        .center()
        .visible(!std::env::args().any(|arg| arg == BACKGROUND_ARG));

    let window = builder.build()?;
    #[cfg(target_os = "macos")]
    {
        let glass_window = window.clone();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        window.with_webview(move |webview| {
            // Tauri supplies a live WKWebView (an NSView subclass) on the main thread.
            let content = unsafe { &*webview.inner().cast::<objc2_app_kit::NSView>() };
            let result = window_vibrancy::apply_liquid_glass(
                &glass_window,
                window_vibrancy::LiquidGlassOptions::default()
                    .radius(18.0)
                    .content_view(content),
            );
            let _ = sender.send(result);
        })?;
        receiver
            .recv()
            .map_err(std::io::Error::other)?
            .map_err(std::io::Error::other)?;
    }
    Ok(window)
}

pub fn toggle(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    if window.is_visible().unwrap_or(false) && window.is_focused().unwrap_or(false) {
        hide(&window);
        return;
    }
    let just_hidden = app
        .state::<ShellState>()
        .hidden_at
        .lock()
        .expect("shell state poisoned")
        .is_some_and(|at| at.elapsed() < REOPEN_GUARD);
    if !just_hidden {
        show(app);
    }
}

pub fn show(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    #[cfg(target_os = "macos")]
    anchor_to_tray(&window);
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
    let _ = app.emit(SHOWN_EVENT, ());
}

fn hide<R: tauri::Runtime>(window: &impl Manager<R>) {
    if let Some(window) = window.get_webview_window(LABEL) {
        let _ = window.hide();
        *window
            .state::<ShellState>()
            .hidden_at
            .lock()
            .expect("shell state poisoned") = Some(Instant::now());
    }
}

pub fn on_window_event(window: &Window, event: &WindowEvent) {
    match event {
        // The popover behaves like a menu: it goes away when focus leaves.
        #[cfg(target_os = "macos")]
        WindowEvent::Focused(false) => hide(window),
        // Closing the Windows window keeps OrbitSwitch running in the tray.
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            hide(window);
        }
        _ => {}
    }
}

/// Fits the macOS popover to its content and keeps it anchored to the icon.
#[cfg(target_os = "macos")]
pub fn resize(app: &AppHandle, height: f64) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.set_size(tauri::LogicalSize::new(
            PANEL_WIDTH,
            height.clamp(200.0, 720.0),
        ));
        if window.is_visible().unwrap_or(false) {
            anchor_to_tray(&window);
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn resize(_app: &AppHandle, _height: f64) {}
