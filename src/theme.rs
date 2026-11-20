use std::path::{Path, PathBuf};

use base64::Engine as _;
use gpui::{Hsla, Rgba};
use rmpv::Value;
use serde::Deserialize;

use crate::error::LauncherError;
use crate::steam;

const STYLE_ALPHA: &str = "177670";
const STYLE_BLUE: &str = "177671";
const STYLE_GREEN: &str = "177676";
const STYLE_RED: &str = "177687";
const STYLE_SELECTION_KEY_1: &str = "2090499946";
const STYLE_SELECTION_KEY_2: &str = "993947594";
const STYLE_SELECTION_KEY_3: &str = "277698370";

const BUILTIN_STYLE_BLUE: &str = include_str!("../assets/styles/Blue.style");
const BUILTIN_STYLE_BLACK: &str = include_str!("../assets/styles/Black.style");
const BUILTIN_STYLE_LIGHT: &str = include_str!("../assets/styles/Light.style");

#[derive(Debug, Deserialize)]
pub struct CloudState {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub type7_blob: Option<String>,
    #[serde(default)]
    pub last_loaded_config_id: Option<i32>,
    pub last_loaded_style_id: Option<i32>,
    pub log: Vec<LogEntry>,
}

#[derive(Debug, Deserialize)]
pub struct LogEntry {
    pub entry_id: i32,
    pub entry_type: String,
    pub name: String,
    #[serde(default)]
    pub deleted_at: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

impl ThemeColor {
    pub fn hsla(self) -> Hsla {
        Rgba {
            r: self.r as f32 / 255.0,
            g: self.g as f32 / 255.0,
            b: self.b as f32 / 255.0,
            a: self.a.clamp(0.0, 1.0),
        }
        .into()
    }

    pub fn with_alpha(self, alpha: f32) -> Self {
        Self {
            a: alpha.clamp(0.0, 1.0),
            ..self
        }
    }
}

#[derive(Debug, Clone)]
pub struct LauncherTheme {
    pub palette: Palette,
}

/// Resolved launcher palette from the active Neverlose style.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub text: Hsla,
    pub active_text: Hsla,
    pub small_text: Hsla,
    pub logo: Hsla,
    pub popup_bg: Hsla,
    pub main_bg_opaque: Hsla,
    pub border: Hsla,
    pub frame_bg: Hsla,
    pub text_preview: Hsla,
    pub spinner: Hsla,
    pub block_bg: Hsla,
    pub block_bg_opaque: Hsla,
    pub button: Hsla,
    pub button_active: Hsla,
    pub button_active_text: Hsla,
    pub link: Hsla,
    pub link_active: Hsla,
    pub selection: Hsla,
    pub separator: Hsla,
    pub shadow: Hsla,
    pub shadow_soft: Hsla,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            text: rgba(255, 255, 255, 224),
            active_text: rgb(255, 255, 255),
            small_text: rgba(255, 255, 255, 147),
            logo: rgb(255, 255, 255),
            popup_bg: rgba(7, 12, 19, 166),
            main_bg_opaque: rgb(1, 3, 6),
            border: rgba(255, 255, 255, 19),
            frame_bg: rgb(3, 8, 15),
            text_preview: rgba(247, 245, 255, 204),
            spinner: rgb(170, 182, 255),
            block_bg: rgb(3, 8, 15),
            block_bg_opaque: rgb(3, 8, 15),
            button: rgb(63, 102, 245),
            button_active: rgb(72, 108, 238),
            button_active_text: rgba(255, 255, 255, 230),
            link: rgb(98, 107, 230),
            link_active: rgb(255, 255, 255),
            selection: rgba(16, 31, 49, 199),
            separator: rgba(255, 255, 255, 11),
            shadow: rgba(0, 0, 0, 133),
            shadow_soft: rgba(0, 0, 0, 122),
        }
    }
}

impl Palette {
    pub fn from_colors(colors: &[ThemeColor]) -> Self {
        let defaults = Self::default();
        Self {
            text: pick(colors, 0, defaults.text),
            active_text: pick(colors, 2, defaults.active_text),
            small_text: pick(colors, 3, defaults.small_text),
            logo: pick(colors, 5, defaults.logo),
            popup_bg: pick(colors, 8, defaults.popup_bg),
            main_bg_opaque: pick_opaque(colors, 9, defaults.main_bg_opaque),
            border: pick(colors, 11, defaults.border),
            frame_bg: pick(colors, 12, defaults.frame_bg),
            text_preview: pick(colors, 14, defaults.text_preview),
            spinner: pick(colors, 40, defaults.spinner),
            block_bg: pick(colors, 41, defaults.block_bg),
            block_bg_opaque: pick_opaque(colors, 41, defaults.block_bg_opaque),
            button: pick(colors, 28, defaults.button),
            button_active: pick(colors, 29, defaults.button_active),
            button_active_text: pick(colors, 30, defaults.button_active_text),
            link: pick(colors, 31, defaults.link),
            link_active: pick(colors, 32, defaults.link_active),
            selection: pick(colors, 34, defaults.selection),
            separator: pick(colors, 35, defaults.separator),
            shadow: pick(colors, 39, defaults.shadow),
            shadow_soft: pick_alpha(colors, 39, defaults.shadow_soft, 0.48),
        }
    }

    pub fn text_hover(&self) -> Hsla {
        mix(self.text, self.active_text, 0.5)
    }

    pub fn strong_border(&self) -> Hsla {
        mix(self.border, self.text, 0.14)
    }

    pub fn button_half(&self) -> Hsla {
        self.button_active.opacity(0.5)
    }

    /// Opaque popup surface (`Hsla::opacity` multiplies alpha, so force `a = 1.0`).
    pub fn menu_bg(&self) -> Hsla {
        let popup: Rgba = self.popup_bg.into();
        let solid = Hsla { a: 1.0, ..popup.into() };
        mix(self.main_bg_opaque, solid, 0.8)
    }

    pub fn menu_border(&self) -> Hsla {
        self.button_active.opacity(0.16)
    }

    pub fn white_border(&self) -> Hsla {
        rgba(255, 255, 255, 19)
    }
}

pub fn mix(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let a: Rgba = a.into();
    let b: Rgba = b.into();
    Rgba {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
    .into()
}

fn rgb(r: u8, g: u8, b: u8) -> Hsla {
    ThemeColor { r, g, b, a: 1.0 }.hsla()
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Hsla {
    ThemeColor {
        r,
        g,
        b,
        a: a as f32 / 255.0,
    }
    .hsla()
}

fn pick(colors: &[ThemeColor], index: usize, fallback: Hsla) -> Hsla {
    colors
        .get(index)
        .map(|color| color.hsla())
        .unwrap_or(fallback)
}

fn pick_opaque(colors: &[ThemeColor], index: usize, fallback: Hsla) -> Hsla {
    colors
        .get(index)
        .map(|color| color.with_alpha(1.0).hsla())
        .unwrap_or(fallback)
}

fn pick_alpha(colors: &[ThemeColor], index: usize, fallback: Hsla, alpha: f32) -> Hsla {
    colors
        .get(index)
        .map(|color| color.with_alpha(alpha).hsla())
        .unwrap_or(fallback)
}

pub async fn load_launcher_theme() -> Result<LauncherTheme, LauncherError> {
    let cloud = nl_cloud_path()?;
    let state_path = cloud.join("state.json");
    let state_text = tokio::fs::read_to_string(&state_path)
        .await
        .map_err(|error| LauncherError::Io(format!("failed to read {}: {error}", state_path.display())))?;
    let state: CloudState = serde_json::from_str(&state_text)
        .map_err(|error| LauncherError::SerdeJson(format!("failed to parse {}: {error}", state_path.display())))?;

    let style_id = state
        .type7_blob
        .as_deref()
        .and_then(extract_style_id_from_type7)
        .or(state.last_loaded_style_id);

    let Some(style_id) = style_id else {
        return Ok(default_theme());
    };

    if let Some((_, style_text)) = builtin_style(style_id) {
        let colors = decode_style_colors(style_text)?;
        return Ok(LauncherTheme {
            palette: Palette::from_colors(&colors),
        });
    }

    let Some(entry) = state.log.iter().find(|entry| {
        entry.entry_id == style_id && entry.entry_type == "Style" && entry.deleted_at.is_none()
    }) else {
        return Ok(default_theme());
    };

    let style_path = cloud.join("styles").join(format!(
        "{}_{}.style",
        entry.entry_id,
        sanitize_filename(&entry.name)
    ));
    let style_text = tokio::fs::read_to_string(&style_path)
        .await
        .map_err(|error| LauncherError::Io(format!("failed to read {}: {error}", style_path.display())))?;
    let colors = decode_style_colors(&style_text)?;

    Ok(LauncherTheme {
        palette: Palette::from_colors(&colors),
    })
}

pub fn default_theme() -> LauncherTheme {
    LauncherTheme {
        palette: Palette::default(),
    }
}

pub fn nl_cloud_path() -> Result<PathBuf, LauncherError> {
    if let Ok(path) = std::env::var("NL_CLOUD_PATH") {
        return Ok(PathBuf::from(path));
    }

    launcher_cloud_dir()
}

pub fn launcher_cloud_dir() -> Result<PathBuf, LauncherError> {
    if let Some(game_dir) = steam::find_game_install_path("Counter-Strike Global Offensive") {
        let cloud_path = game_dir.join("nl_cloud");
        if cloud_path.exists() {
            return Ok(cloud_path);
        }
    }
    if let Some(game_dir) = steam::find_game_install_path("csgo legacy") {
        let cloud_path = game_dir.join("nl_cloud");
        if cloud_path.exists() {
            return Ok(cloud_path);
        }
    }
    if let Some(game_dir) = steam::find_game_install_path("Counter-Strike Global Offensive") {
        return Ok(game_dir.join("nl_cloud"));
    }
    if let Some(game_dir) = steam::find_game_install_path("csgo legacy") {
        return Ok(game_dir.join("nl_cloud"));
    }

    let appdata = std::env::var("APPDATA")
        .map_err(|error| LauncherError::System(format!("APPDATA is not available: {error}")))?;
    Ok(Path::new(&appdata).join("neverlose").join("nl_cloud"))
}

fn builtin_style(style_id: i32) -> Option<(&'static str, &'static str)> {
    match style_id {
        0 => Some(("Blue", BUILTIN_STYLE_BLUE)),
        1 => Some(("Black", BUILTIN_STYLE_BLACK)),
        2 => Some(("Light", BUILTIN_STYLE_LIGHT)),
        _ => None,
    }
}

fn extract_style_id_from_type7(blob_b64: &str) -> Option<i32> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(blob_b64.trim())
        .ok()?;
    let value = rmpv::decode::read_value(&mut &bytes[..]).ok()?;
    let selected = map_get(&value, STYLE_SELECTION_KEY_1)
        .and_then(|value| map_get(value, STYLE_SELECTION_KEY_2))
        .and_then(|value| map_get(value, STYLE_SELECTION_KEY_3))?;

    selected
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
}

fn map_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let Value::Map(entries) = value else {
        return None;
    };

    entries.iter().find_map(|(entry_key, entry_value)| {
        if entry_key.as_str()? == key {
            Some(entry_value)
        } else {
            None
        }
    })
}

fn decode_style_colors(style_text: &str) -> Result<Vec<ThemeColor>, LauncherError> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(style_text.trim())?;
    let value = rmpv::decode::read_value(&mut &bytes[..])?;
    let Value::Array(items) = value else {
        return Err(LauncherError::Validation(
            "style payload was not an array".to_string(),
        ));
    };

    items.iter().map(decode_style_color).collect()
}

fn decode_style_color(value: &Value) -> Result<ThemeColor, LauncherError> {
    let Value::Map(entries) = value else {
        return Err(LauncherError::Validation(
            "style color entry was not a map".to_string(),
        ));
    };

    let alpha = style_channel(entries, STYLE_ALPHA)?;
    let red = style_channel(entries, STYLE_RED)?;
    let green = style_channel(entries, STYLE_GREEN)?;
    let blue = style_channel(entries, STYLE_BLUE)?;

    Ok(ThemeColor {
        r: channel_to_u8(red),
        g: channel_to_u8(green),
        b: channel_to_u8(blue),
        a: alpha.clamp(0.0, 1.0) as f32,
    })
}

fn style_channel(entries: &[(Value, Value)], key: &str) -> Result<f64, LauncherError> {
    entries
        .iter()
        .find_map(|(entry_key, value)| {
            let entry_key = entry_key.as_str()?;
            if entry_key == key {
                value.as_f64()
            } else {
                None
            }
        })
        .ok_or_else(|| LauncherError::Validation(format!("style color entry was missing channel {key}")))
}

fn channel_to_u8(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub fn sanitize_filename(name: &str) -> String {
    let value: String = name
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect();
    let value = value.trim_end_matches(|character: char| {
        character == '.' || character == ' ' || character.is_control()
    });
    if value.is_empty() {
        "unnamed".to_string()
    } else {
        value.to_string()
    }
}

pub fn sanitize_path_segment(name: &str) -> String {
    let value = sanitize_filename(name);
    if value == "." || value == ".." {
        "version".to_string()
    } else {
        value
    }
}

#[derive(Debug)]
pub struct LauncherSettings {
    pub username: String,
    pub avatar_bytes: Option<Vec<u8>>,
    pub selected_config_id: Option<i32>,
    pub configs: Vec<ConfigEntry>,
}

#[derive(Debug, Clone)]
pub struct ConfigEntry {
    pub entry_id: i32,
    pub name: String,
}

pub async fn read_launcher_settings() -> Result<LauncherSettings, LauncherError> {
    let cloud = nl_cloud_path()?;
    let state_path = cloud.join("state.json");
    let state_text = tokio::fs::read_to_string(&state_path)
        .await
        .map_err(|error| LauncherError::Io(format!("failed to read {}: {error}", state_path.display())))?;
    let state: CloudState = serde_json::from_str(&state_text)
        .map_err(|error| LauncherError::SerdeJson(format!("failed to parse {}: {error}", state_path.display())))?;

    let configs = state
        .log
        .iter()
        .filter(|entry| entry.entry_type == "Config" && entry.deleted_at.is_none())
        .map(|entry| ConfigEntry {
            entry_id: entry.entry_id,
            name: entry.name.clone(),
        })
        .collect::<Vec<_>>();

    Ok(LauncherSettings {
        username: state.username,
        avatar_bytes: load_avatar_bytes(&cloud).await?,
        selected_config_id: state.last_loaded_config_id,
        configs,
    })
}

async fn load_avatar_bytes(cloud: &Path) -> Result<Option<Vec<u8>>, LauncherError> {
    let avatar_path = cloud.join("avatar.png");
    if !avatar_path.exists() {
        return Ok(None);
    }

    let bytes = tokio::fs::read(&avatar_path)
        .await
        .map_err(|error| LauncherError::Io(format!("failed to read {}: {error}", avatar_path.display())))?;
    Ok(Some(bytes))
}

pub async fn save_launcher_profile(
    username: String,
    avatar_bytes: Option<Vec<u8>>,
) -> Result<LauncherSettings, LauncherError> {
    let username = username.trim();
    if username.is_empty() {
        return Err(LauncherError::Validation(
            "profile name cannot be empty".to_string(),
        ));
    }

    let cloud = nl_cloud_path()?;
    tokio::fs::create_dir_all(&cloud)
        .await
        .map_err(|error| LauncherError::Io(format!("failed to create {}: {error}", cloud.display())))?;

    let state_path = cloud.join("state.json");
    let state_text = tokio::fs::read_to_string(&state_path)
        .await
        .map_err(|error| LauncherError::Io(format!("failed to read {}: {error}", state_path.display())))?;
    let mut state_value: serde_json::Value = serde_json::from_str(&state_text)
        .map_err(|error| LauncherError::SerdeJson(format!("failed to parse {}: {error}", state_path.display())))?;
    let Some(state_object) = state_value.as_object_mut() else {
        return Err(LauncherError::Validation(
            "state.json root was not an object".to_string(),
        ));
    };
    state_object.insert(
        "username".to_string(),
        serde_json::Value::String(username.to_string()),
    );
    let next_state = serde_json::to_string_pretty(&state_value)
        .map_err(|error| LauncherError::SerdeJson(format!("failed to serialize {}: {error}", state_path.display())))?;
    tokio::fs::write(&state_path, next_state)
        .await
        .map_err(|error| LauncherError::Io(format!("failed to write {}: {error}", state_path.display())))?;

    if let Some(bytes) = avatar_bytes {
        if bytes.is_empty() {
            return Err(LauncherError::Validation(
                "profile image was empty".to_string(),
            ));
        }
        let avatar_path = cloud.join("avatar.png");
        tokio::fs::write(&avatar_path, bytes)
            .await
            .map_err(|error| LauncherError::Io(format!("failed to write {}: {error}", avatar_path.display())))?;
    }

    read_launcher_settings().await
}
