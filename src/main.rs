#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod avatar;
mod downloader;
mod error;
mod steam;
mod theme;
mod ui;
mod views;

use std::borrow::Cow;

use gpui::{
    px, size, App, AppContext, Application, AssetSource, Bounds, SharedString, TitlebarOptions,
    WindowBackgroundAppearance, WindowBounds, WindowOptions,
};

use crate::app::LauncherApp;

struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let bytes: Option<&'static [u8]> = match path {
            "icons/chevron-down.svg" => Some(include_bytes!("../assets/icons/chevron-down.svg")),
            "icons/close.svg" => Some(include_bytes!("../assets/icons/close.svg")),
            "icons/cog.svg" => Some(include_bytes!("../assets/icons/cog.svg")),
            "icons/construction.svg" => Some(include_bytes!("../assets/icons/construction.svg")),
            "icons/git-branch.svg" => Some(include_bytes!("../assets/icons/git-branch.svg")),
            "icons/play.svg" => Some(include_bytes!("../assets/icons/play.svg")),
            "icons/spinner.svg" => Some(include_bytes!("../assets/icons/spinner.svg")),
            "images/csgo.png" => Some(include_bytes!("../assets/images/csgo.png")),
            "images/avatar-placeholder.svg" => {
                Some(include_bytes!("../assets/images/avatar-placeholder.svg"))
            }
            _ => None,
        };

        Ok(bytes.map(Cow::Borrowed))
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

const FONT_DATA: &[&[u8]] = &[
    include_bytes!("../assets/fonts/MuseoSans-100.ttf"),
    include_bytes!("../assets/fonts/MuseoSans-300.ttf"),
    include_bytes!("../assets/fonts/MuseoSans_500.ttf"),
    include_bytes!("../assets/fonts/MuseoSans_700.ttf"),
    include_bytes!("../assets/fonts/MuseoSans_900.ttf"),
];

fn load_fonts(cx: &mut App) {
    let fonts = FONT_DATA
        .iter()
        .map(|bytes| Cow::Borrowed(*bytes))
        .collect();

    if let Err(error) = cx.text_system().add_fonts(fonts) {
        eprintln!("Failed to load bundled fonts: {error}");
    }
}

#[cfg(windows)]
fn disable_nc_shadow(window: &mut gpui::Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_NCRENDERING_POLICY,
    };

    const DWMNCRP_DISABLED: u32 = 1;

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };

    let hwnd = win32.hwnd.get() as windows_sys::Win32::Foundation::HWND;
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY as u32,
            &DWMNCRP_DISABLED as *const u32 as *const _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

#[cfg(not(windows))]
fn disable_nc_shadow(_window: &mut gpui::Window) {}

fn main() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
        load_fonts(cx);

        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(590.), px(460.)), cx);

        let launcher = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Neverlose Launcher".into()),
                        appears_transparent: true,
                        traffic_light_position: None,
                    }),
                    window_background: WindowBackgroundAppearance::Transparent,
                    is_resizable: false,
                    is_minimizable: true,
                    window_min_size: Some(size(px(590.), px(460.))),
                    ..Default::default()
                },
                |_, cx| cx.new(|cx| LauncherApp::new(cx)),
            )
            .expect("failed to open the launcher window");

        launcher
            .update(cx, |_, window, _| disable_nc_shadow(window))
            .ok();

        cx.activate(true);
    });
}
