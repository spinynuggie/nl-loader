use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use chrono::{DateTime, Local};
use gpui::{Context, FocusHandle, PathPromptOptions, RenderImage, SharedString, Timer, Window};
use tokio::runtime::Runtime;

use crate::{
    avatar,
    downloader::{self, LauncherGitMetadata, LauncherVersion},
    steam::{self, InstalledGames},
    theme::{self, ConfigEntry, Palette},
    ui::parse_changelog,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Boot,
    Launcher,
    Details,
    ClosingDetails,
    Launching,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    Release,
    Nightly,
}

impl Branch {
    pub fn label(self) -> &'static str {
        match self {
            Branch::Release => "Release",
            Branch::Nightly => "Nightly",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Game {
    Cs2Legacy,
    CsgoStandalone,
}

impl Game {
    pub fn label(self) -> &'static str {
        match self {
            Game::Cs2Legacy => "CS:GO (cs2-csgo_legacy)",
            Game::CsgoStandalone => "CS:GO Standalone",
        }
    }

    pub fn appid(self) -> i32 {
        match self {
            Game::Cs2Legacy => 730,
            Game::CsgoStandalone => 4_465_480,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HoverTrack {
    from: f32,
    target: f32,
    started: Instant,
    duration_ms: f32,
}

impl HoverTrack {
    fn sample(&self, now: Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.started).as_secs_f32() * 1000.0;
        let raw = (elapsed / self.duration_ms.max(1.0)).clamp(0.0, 1.0);
        let eased = if raw < 0.5 {
            2.0 * raw * raw
        } else {
            let x = -2.0 * raw + 2.0;
            1.0 - x * x / 2.0
        };
        self.from + (self.target - self.from) * eased
    }

    fn is_active(&self, now: Instant) -> bool {
        let elapsed = now.saturating_duration_since(self.started).as_secs_f32() * 1000.0;
        elapsed < self.duration_ms && (self.target - self.from).abs() > 0.001
    }
}

pub struct LauncherApp {
    pub(crate) view: View,

    pub(crate) game: Game,
    pub(crate) installed: InstalledGames,
    pub(crate) metadata: LauncherGitMetadata,
    pub(crate) selected_version: String,
    pub(crate) branch: Branch,

    pub(crate) branch_open: bool,
    pub(crate) version_open: bool,
    pub(crate) config_open: bool,
    pub(crate) profile_open: bool,
    pub(crate) profile_closing: bool,
    pub(crate) profile_close_gen: u64,

    pub(crate) configs: Vec<ConfigEntry>,
    pub(crate) selected_config_id: Option<i32>,

    pub(crate) username: String,
    pub(crate) profile_name: String,
    pub(crate) profile_cursor: usize,
    pub(crate) profile_focus: FocusHandle,
    pub(crate) profile_saving: bool,
    pub(crate) profile_error: String,

    pub(crate) avatar: Option<Arc<RenderImage>>,
    pub(crate) avatar_before_edit: Option<Arc<RenderImage>>,
    pub(crate) avatar_loading: bool,
    pub(crate) pending_avatar_bytes: Option<Vec<u8>>,

    pub(crate) launch_pending: bool,
    pub(crate) launch_error: String,
    pub(crate) launch_error_closing: bool,
    pub(crate) launch_error_gen: u64,
    pub(crate) progress: f32,
    pub(crate) launch_generation: u64,

    pub(crate) theme: Palette,
    pub(crate) hover_tracks: HashMap<SharedString, HoverTrack>,
}

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .thread_name("neverlose-backend")
            .build()
            .expect("failed to create backend runtime")
    })
}

impl LauncherApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let profile_focus = cx.focus_handle();
        let mut app = Self {
            view: View::Boot,
            game: Game::CsgoStandalone,
            installed: InstalledGames::default(),
            metadata: LauncherGitMetadata {
                releases: Vec::new(),
                nightlies: Vec::new(),
            },
            selected_version: String::new(),
            branch: Branch::Release,
            branch_open: false,
            version_open: false,
            config_open: false,
            profile_open: false,
            profile_closing: false,
            profile_close_gen: 0,
            configs: Vec::new(),
            selected_config_id: None,
            username: "Player".to_string(),
            profile_name: "Player".to_string(),
            profile_cursor: "Player".len(),
            profile_focus,
            profile_saving: false,
            profile_error: String::new(),
            avatar: None,
            avatar_before_edit: None,
            avatar_loading: false,
            pending_avatar_bytes: None,
            launch_pending: false,
            launch_error: String::new(),
            launch_error_closing: false,
            launch_error_gen: 0,
            progress: 0.0,
            launch_generation: 0,
            theme: Palette::default(),
            hover_tracks: HashMap::new(),
        };
        cx.observe_keystrokes(|this, event, _window, cx| {
            if this.profile_open {
                this.profile_keystroke(&event.keystroke, cx);
            } else if event.keystroke.key == "escape" && this.any_menu_open() {
                this.close_menus(cx);
                cx.notify();
            }
        })
        .detach();
        app.begin_loading(cx);
        app
    }

    fn begin_loading(&mut self, cx: &mut Context<Self>) {
        self.load_theme(cx);
        self.load_settings(cx);
        self.load_metadata(cx);
        self.detect_games(cx);

        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(1000)).await;
            this.update(cx, |this, cx| {
                this.view = View::Launcher;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn load_theme(&self, cx: &mut Context<Self>) {
        let task = runtime().spawn(theme::load_launcher_theme());
        cx.spawn(async move |this, cx| {
            let Ok(Ok(theme)) = task.await else {
                return;
            };
            this.update(cx, |this, cx| {
                this.theme = theme.palette;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn load_settings(&self, cx: &mut Context<Self>) {
        let task = runtime().spawn(theme::read_launcher_settings());
        cx.spawn(async move |this, cx| {
            let Ok(Ok(settings)) = task.await else {
                return;
            };
            this.update(cx, |this, cx| {
                this.apply_settings(settings);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn load_metadata(&self, cx: &mut Context<Self>) {
        let task = runtime().spawn(downloader::load_git_metadata());
        cx.spawn(async move |this, cx| {
            let metadata = match task.await {
                Ok(Ok(metadata)) => metadata,
                _ => unavailable_metadata("Could not load GitHub release metadata."),
            };
            this.update(cx, |this, cx| {
                this.selected_version = metadata
                    .releases
                    .first()
                    .or_else(|| metadata.nightlies.first())
                    .map(|version| version.tag.clone())
                    .unwrap_or_default();
                this.metadata = metadata;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn detect_games(&self, cx: &mut Context<Self>) {
        let task = runtime().spawn_blocking(steam::detect_installed_games);
        cx.spawn(async move |this, cx| {
            let Ok(installed) = task.await else {
                return;
            };
            this.update(cx, |this, cx| {
                this.installed = installed;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn apply_settings(&mut self, settings: theme::LauncherSettings) {
        self.username = if settings.username.trim().is_empty() {
            "Player".to_string()
        } else {
            settings.username
        };
        self.profile_name = self.username.clone();
        self.profile_cursor = self.profile_name.len();
        self.avatar = settings
            .avatar_bytes
            .as_deref()
            .and_then(avatar::render_image);
        self.avatar_before_edit = self.avatar.clone();
        self.configs = settings.configs;
        self.selected_config_id = settings
            .selected_config_id
            .or_else(|| self.configs.first().map(|config| config.entry_id));
    }

    pub fn versions(&self) -> &[LauncherVersion] {
        match self.branch {
            Branch::Release => &self.metadata.releases,
            Branch::Nightly => &self.metadata.nightlies,
        }
    }

    pub fn selected_version_data(&self) -> Option<&LauncherVersion> {
        let versions = self.versions();
        versions
            .iter()
            .find(|version| version.tag == self.selected_version)
            .or_else(|| versions.first())
    }

    pub fn selected_version_label(&self) -> String {
        if self.selected_version.is_empty() {
            "No builds".to_string()
        } else {
            self.selected_version.clone()
        }
    }

    pub fn selected_branch_label(&self) -> String {
        format!("{} {}", self.branch.label(), self.selected_version_label())
    }

    pub fn selected_config_name(&self) -> String {
        self.configs
            .iter()
            .find(|config| Some(config.entry_id) == self.selected_config_id)
            .map(|config| config.name.clone())
            .unwrap_or_else(|| "Loading...".to_string())
    }

    pub fn updated_at_label(&self) -> String {
        let value = self
            .selected_version_data()
            .map(|version| version.updated_at.as_str())
            .unwrap_or("");
        format_git_date(value)
    }

    pub fn selected_release_url(&self) -> String {
        self.selected_version_data()
            .map(|version| version.url.clone())
            .unwrap_or_default()
    }

    pub fn changelog_entries(&self) -> Vec<String> {
        parse_changelog(
            self.selected_version_data()
                .map(|version| version.changelog.as_str()),
        )
    }

    pub fn any_menu_open(&self) -> bool {
        self.branch_open || self.version_open || self.config_open || self.profile_open
    }

    pub fn set_hover(
        &mut self,
        key: impl Into<SharedString>,
        hovered: bool,
        duration_ms: f32,
        cx: &mut Context<Self>,
    ) {
        let key = key.into();
        let target = if hovered { 1.0 } else { 0.0 };
        let now = Instant::now();
        let current = self
            .hover_tracks
            .get(&key)
            .map(|track| track.sample(now))
            .unwrap_or(0.0);
        if (current - target).abs() < 0.001
            && self
                .hover_tracks
                .get(&key)
                .is_some_and(|track| (track.target - target).abs() < 0.001)
        {
            return;
        }
        self.hover_tracks.insert(
            key,
            HoverTrack {
                from: current,
                target,
                started: now,
                duration_ms,
            },
        );
        cx.notify();
    }

    pub fn hover_t(&self, key: &str) -> f32 {
        self.hover_tracks
            .get(key)
            .map(|track| track.sample(Instant::now()))
            .unwrap_or(0.0)
    }

    pub fn step_hover_animations(&mut self, window: &mut Window) {
        let now = Instant::now();
        let mut any_active = false;
        self.hover_tracks.retain(|_, track| {
            if track.is_active(now) {
                any_active = true;
                true
            } else {
                track.target > 0.001
            }
        });
        if any_active {
            window.request_animation_frame();
        }
    }

    pub fn clear_hover_prefix(&mut self, prefix: &str) {
        self.hover_tracks.retain(|key, _| !key.starts_with(prefix));
    }

    pub fn close_menus(&mut self, cx: &mut Context<Self>) {
        self.branch_open = false;
        self.version_open = false;
        self.config_open = false;
        self.clear_hover_prefix("menu:");

        if self.profile_open {
            if self.pending_avatar_bytes.is_some() {
                self.avatar = self.avatar_before_edit.clone();
                self.pending_avatar_bytes = None;
            }

            let name_changed = self.profile_name.trim() != self.username
                && !self.profile_name.trim().is_empty();
            self.request_close_profile(cx);
            if name_changed {
                self.save_profile(cx);
            }
        }
    }

    fn request_close_profile(&mut self, cx: &mut Context<Self>) {
        if !self.profile_open || self.profile_closing {
            return;
        }
        self.profile_closing = true;
        self.profile_close_gen += 1;
        let gen = self.profile_close_gen;
        cx.notify();
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(220)).await;
            this.update(cx, |this, cx| {
                if this.profile_close_gen == gen {
                    this.profile_open = false;
                    this.profile_closing = false;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    pub fn open_details(&mut self, game: Game, cx: &mut Context<Self>) {
        self.game = game;
        self.view = View::Details;
        self.launch_pending = false;
        self.launch_error.clear();
        self.launch_error_closing = false;
        self.launch_generation = self.launch_generation.wrapping_add(1);
        self.close_menus(cx);
        cx.notify();
    }

    pub fn close_details(&mut self, cx: &mut Context<Self>) {
        self.launch_pending = false;
        self.progress = 0.0;
        self.branch_open = false;
        self.version_open = false;
        self.config_open = false;
        self.launch_generation = self.launch_generation.wrapping_add(1);

        if self.view == View::Details {
            self.view = View::ClosingDetails;
            cx.notify();
            cx.spawn(async move |this, cx| {
                Timer::after(Duration::from_millis(240)).await;
                this.update(cx, |this, cx| {
                    if this.view == View::ClosingDetails {
                        this.view = View::Launcher;
                        cx.notify();
                    }
                })
                .ok();
            })
            .detach();
        } else if self.view != View::Launching {
            self.view = View::Launcher;
            cx.notify();
        }
    }

    pub fn toggle_branch(&mut self, cx: &mut Context<Self>) {
        self.version_open = false;
        self.config_open = false;
        self.branch_open = !self.branch_open;
        cx.notify();
    }

    pub fn select_branch(&mut self, branch: Branch, cx: &mut Context<Self>) {
        self.branch = branch;
        self.selected_version = match branch {
            Branch::Release => self.metadata.releases.first(),
            Branch::Nightly => self.metadata.nightlies.first(),
        }
        .map(|version| version.tag.clone())
        .unwrap_or_default();
        self.branch_open = false;
        self.clear_hover_prefix("menu:");
        cx.notify();
    }

    pub fn toggle_version(&mut self, cx: &mut Context<Self>) {
        self.config_open = false;
        self.branch_open = false;
        self.version_open = !self.version_open;
        if !self.version_open {
            self.clear_hover_prefix("menu:");
        }
        cx.notify();
    }

    pub fn select_version(&mut self, version: String, cx: &mut Context<Self>) {
        self.selected_version = version;
        self.version_open = false;
        self.clear_hover_prefix("menu:");
        cx.notify();
    }

    pub fn toggle_config(&mut self, cx: &mut Context<Self>) {
        self.branch_open = false;
        self.version_open = false;
        self.config_open = !self.config_open;
        if !self.config_open {
            self.clear_hover_prefix("menu:");
        }
        cx.notify();
    }

    pub fn select_config(&mut self, entry_id: i32, cx: &mut Context<Self>) {
        self.selected_config_id = Some(entry_id);
        self.config_open = false;
        self.clear_hover_prefix("menu:");
        cx.notify();
    }

    pub fn open_profile(&mut self, cx: &mut Context<Self>) {
        self.profile_name = self.username.clone();
        self.profile_cursor = self.profile_name.len();
        self.avatar_before_edit = self.avatar.clone();
        self.pending_avatar_bytes = None;
        self.profile_error.clear();
        self.branch_open = false;
        self.version_open = false;
        self.config_open = false;
        if self.profile_open && !self.profile_closing {
            self.request_close_profile(cx);
        } else {
            self.profile_close_gen += 1;
            self.profile_closing = false;
            self.profile_open = true;
        }
        cx.notify();
    }

    pub fn profile_keystroke(&mut self, keystroke: &gpui::Keystroke, cx: &mut Context<Self>) {
        let key = keystroke.key.as_str();
        let modifiers = keystroke.modifiers;

        if modifiers.control && key == "v" {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                let text = text.replace(['\r', '\n'], " ");
                self.insert_profile_text(&text);
            }
            cx.notify();
            return;
        }

        match key {
            "backspace" => {
                if let Some(previous) = previous_boundary(&self.profile_name, self.profile_cursor) {
                    self.profile_name.replace_range(previous..self.profile_cursor, "");
                    self.profile_cursor = previous;
                }
            }
            "delete" => {
                if let Some(next) = next_boundary(&self.profile_name, self.profile_cursor) {
                    self.profile_name.replace_range(self.profile_cursor..next, "");
                }
            }
            "left" => {
                if let Some(previous) = previous_boundary(&self.profile_name, self.profile_cursor) {
                    self.profile_cursor = previous;
                }
            }
            "right" => {
                if let Some(next) = next_boundary(&self.profile_name, self.profile_cursor) {
                    self.profile_cursor = next;
                }
            }
            "home" => self.profile_cursor = 0,
            "end" => self.profile_cursor = self.profile_name.len(),
            "enter" => {
                if !self.profile_saving && self.profile_name.trim() != self.username {
                    self.save_profile(cx);
                }
            }
            "escape" => {
                self.profile_name = self.username.clone();
                self.profile_cursor = self.profile_name.len();
                self.profile_error.clear();
            }
            _ => {
                if !modifiers.control && !modifiers.alt && !modifiers.platform {
                    if let Some(key_char) = keystroke.key_char.as_deref() {
                        if !key_char.is_empty() && !key_char.chars().any(char::is_control) {
                            self.insert_profile_text(key_char);
                        }
                    }
                }
            }
        }
        cx.notify();
    }

    fn insert_profile_text(&mut self, value: &str) {
        if self.profile_name.chars().count() + value.chars().count() > 32 {
            return;
        }
        self.profile_name.insert_str(self.profile_cursor, value);
        self.profile_cursor += value.len();
        self.profile_error.clear();
    }

    pub fn choose_avatar(&mut self, cx: &mut Context<Self>) {
        if self.avatar_loading || self.profile_saving {
            return;
        }
        self.avatar_loading = true;
        cx.notify();

        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose a profile image".into()),
        });

        cx.spawn(async move |this, cx| {
            let selected = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };

            let Some(path) = selected else {
                this.update(cx, |this, cx| {
                    this.avatar_loading = false;
                    cx.notify();
                })
                .ok();
                return;
            };

            let processed = runtime()
                .spawn_blocking(move || avatar::circular_avatar_png(&path))
                .await;

            this.update(cx, |this, cx| {
                this.avatar_loading = false;
                match processed {
                    Ok(Ok(bytes)) => {
                        if let Some(image) = avatar::render_image(&bytes) {
                            this.avatar = Some(image);
                            this.pending_avatar_bytes = Some(bytes);
                            this.profile_error.clear();
                        } else {
                            this.profile_error = "Could not prepare profile image.".to_string();
                        }
                    }
                    Ok(Err(error)) => this.profile_error = error,
                    Err(error) => this.profile_error = error.to_string(),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn cancel_avatar_edit(&mut self, cx: &mut Context<Self>) {
        self.avatar = self.avatar_before_edit.clone();
        self.pending_avatar_bytes = None;
        self.profile_error.clear();
        cx.notify();
    }

    pub fn save_profile(&mut self, cx: &mut Context<Self>) {
        if self.profile_saving {
            return;
        }
        let name = self.profile_name.trim().to_string();
        if name.is_empty() {
            self.profile_error = "Enter a profile name.".to_string();
            cx.notify();
            return;
        }

        self.profile_saving = true;
        self.profile_error.clear();
        cx.notify();

        let avatar_bytes = self.pending_avatar_bytes.clone();
        let task = runtime().spawn(theme::save_launcher_profile(name, avatar_bytes.clone()));

        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.profile_saving = false;
                match result {
                    Ok(Ok(settings)) => {
                        this.apply_settings(settings);
                        this.pending_avatar_bytes = None;
                        this.avatar_before_edit = this.avatar.clone();
                    }
                    Ok(Err(error)) => {
                        this.profile_error = error.to_string();
                        if avatar_bytes.is_some() {
                            this.pending_avatar_bytes = avatar_bytes;
                        }
                    }
                    Err(error) => this.profile_error = error.to_string(),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn launch(&mut self, cx: &mut Context<Self>) {
        if self.launch_pending {
            return;
        }

        self.branch_open = false;
        self.version_open = false;
        self.config_open = false;
        self.launch_pending = true;
        self.launch_error.clear();
        self.launch_error_closing = false;
        self.progress = 0.0;
        self.launch_generation = self.launch_generation.wrapping_add(1);
        let generation = self.launch_generation;
        let version = self.selected_version.clone();
        let config_id = self.selected_config_id;
        let appid = self.game.appid();
        cx.notify();

        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(230)).await;
            let should_launch = this
                .update(cx, |this, _cx| {
                    this.launch_generation == generation
                        && this.launch_pending
                        && this.view == View::Details
                })
                .unwrap_or(false);
            if !should_launch {
                return;
            }

            this.update(cx, |this, cx| {
                this.view = View::Launching;
                this.start_launch_progress(version, config_id, appid, cx);
            })
            .ok();
        })
        .detach();
    }

    fn start_launch_progress(
        &mut self,
        version: String,
        config_id: Option<i32>,
        appid: i32,
        cx: &mut Context<Self>,
    ) {
        let generation = self.launch_generation;
        let backend = runtime().spawn(async move {
            downloader::download_and_launch_version(version, config_id, appid).await
        });

        cx.spawn(async move |this, cx| {
            let started = Instant::now();
            loop {
                if backend.is_finished() {
                    break;
                }
                let elapsed = started.elapsed().as_secs_f32();
                let progress = (18.0 + (elapsed / 3.2) * 74.0).min(92.0);
                this.update(cx, |this, cx| {
                    if this.launch_generation == generation && this.view == View::Launching {
                        this.progress = progress;
                        cx.notify();
                    }
                })
                .ok();
                Timer::after(Duration::from_millis(33)).await;
            }

            let result = backend.await;
            let success = matches!(result, Ok(Ok(())));
            this.update(cx, |this, cx| {
                if this.launch_generation != generation {
                    return;
                }
                match result {
                    Ok(Ok(())) => {
                        this.progress = 100.0;
                    }
                    Ok(Err(error)) => {
                        this.launch_pending = false;
                        this.launch_error = error.to_string();
                        this.progress = 0.0;
                        this.view = View::Details;
                        let _ = downloader::kill_background_processes();
                    }
                    Err(error) => {
                        this.launch_pending = false;
                        this.launch_error = error.to_string();
                        this.progress = 0.0;
                        this.view = View::Details;
                        let _ = downloader::kill_background_processes();
                    }
                }
                cx.notify();
            })
            .ok();

            if success {
                Timer::after(Duration::from_millis(1000)).await;
                cx.update(|cx| cx.quit()).ok();
            }
        })
        .detach();
    }

    pub fn dismiss_launch_error(&mut self, cx: &mut Context<Self>) {
        if self.launch_error.is_empty() || self.launch_error_closing {
            return;
        }
        self.launch_error_closing = true;
        self.launch_error_gen += 1;
        let gen = self.launch_error_gen;
        cx.notify();
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(160)).await;
            this.update(cx, |this, cx| {
                if this.launch_error_gen == gen {
                    this.launch_error.clear();
                    this.launch_error_closing = false;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }
}

fn unavailable_metadata(message: &str) -> LauncherGitMetadata {
    LauncherGitMetadata {
        releases: vec![LauncherVersion {
            tag: "Unavailable".to_string(),
            name: "Unavailable".to_string(),
            changelog: message.to_string(),
            updated_at: String::new(),
            url: String::new(),
            assets: Vec::new(),
        }],
        nightlies: Vec::new(),
    }
}

fn format_git_date(value: &str) -> String {
    if value.is_empty() {
        return "Unknown".to_string();
    }

    match DateTime::parse_from_rfc3339(value) {
        Ok(date) => date
            .with_timezone(&Local)
            .format("%d.%m.%Y %H:%M")
            .to_string(),
        Err(_) => value.to_string(),
    }
}

fn previous_boundary(value: &str, cursor: usize) -> Option<usize> {
    value[..cursor].char_indices().last().map(|(index, _)| index)
}

fn next_boundary(value: &str, cursor: usize) -> Option<usize> {
    value[cursor..]
        .char_indices()
        .nth(1)
        .map(|(index, _)| cursor + index)
        .or_else(|| {
            if cursor < value.len() {
                Some(value.len())
            } else {
                None
            }
        })
}



