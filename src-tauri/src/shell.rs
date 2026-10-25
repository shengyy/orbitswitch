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
    {
        use tauri_plugin_positioner::{Position, WindowExt};
        let _ = window.move_window_constrained(Position::TrayBottomCenter);
    }
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
    use tauri_plugin_positioner::{Position, WindowExt};
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.set_size(tauri::LogicalSize::new(
            PANEL_WIDTH,
            height.clamp(200.0, 720.0),
        ));
        if window.is_visible().unwrap_or(false) {
            let _ = window.move_window_constrained(Position::TrayBottomCenter);
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn resize(_app: &AppHandle, _height: f64) {}
