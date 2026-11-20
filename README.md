# Neverlose Launcher

A native Windows launcher for Neverlose. The entire client - UI and backend - is
written in Rust. The interface is rendered with [GPUI](https://gpui.rs/), the
GPU-accelerated UI framework built by the Zed team (the same framework used by
Discord's Rust desktop rewrite).

The original Svelte/Tauri frontend has been replaced by a single native binary.
The launcher keeps the same visual design, bundled Museo Sans typography,
Neverlose style theming, profile popout, release/nightly picker, changelog
viewer and launch progress screen.

## Building

Requirements:

- Rust 1.85 or newer (edition 2021)
- Windows 10/11 with the MSVC toolchain
- A DirectX 11 compatible GPU

```powershell
cargo run
```

Release build:

```powershell
cargo build --release
```

The executable is written to `target\release\neverlose-launcher.exe`.

## Project layout

| Path | Purpose |
| --- | --- |
| `src/main.rs` | GPUI application bootstrap, embedded assets and fonts |
| `src/app.rs` | Launcher state, async backend work and interactions |
| `src/views.rs` | All GPUI views (shell, subscriptions, details, profile, launch) |
| `src/theme.rs` | Neverlose `.style` decoding and the colour palette |
| `src/downloader.rs` | GitHub release metadata, asset verification and launching |
| `src/steam.rs` | Steam library discovery |
| `src/avatar.rs` | Circular avatar processing and GPUI image conversion |
| `assets/` | Fonts, icons, images and built-in styles, compiled into the exe |

## Notes

- Fonts are bundled into the executable; no webview or runtime assets are needed.
- The launcher reads the active style, profile and configs from `nl_cloud`,
  exactly as the previous client did.
- The Windows icon and version metadata are embedded by `build.rs`.
