use std::fs;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use winreg::RegKey;

#[derive(Debug, Clone, Copy, Default)]
pub struct InstalledGames {
    pub cs2_legacy_branch: bool,
    pub csgo_standalone: bool,
}

pub fn detect_installed_games() -> InstalledGames {
    InstalledGames {
        cs2_legacy_branch: find_game_install_path("Counter-Strike Global Offensive").is_some(),
        csgo_standalone: find_game_install_path("csgo legacy").is_some(),
    }
}

#[cfg(windows)]
pub fn get_steam_install_path() -> Option<PathBuf> {
    let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey("Software\\Valve\\Steam") {
        if let Ok(steam_path) = key.get_value::<String, _>("SteamPath") {
            return Some(PathBuf::from(steam_path));
        }
    }
    let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey("SOFTWARE\\Wow6432Node\\Valve\\Steam") {
        if let Ok(steam_path) = key.get_value::<String, _>("InstallPath") {
            return Some(PathBuf::from(steam_path));
        }
    }
    if let Ok(key) = hklm.open_subkey("SOFTWARE\\Valve\\Steam") {
        if let Ok(steam_path) = key.get_value::<String, _>("InstallPath") {
            return Some(PathBuf::from(steam_path));
        }
    }
    None
}

#[cfg(not(windows))]
pub fn get_steam_install_path() -> Option<PathBuf> {
    None
}

pub fn find_game_install_path(game_name: &str) -> Option<PathBuf> {
    if let Ok(cur) = std::env::current_dir() {
        let exe_check = cur.join("csgo.exe");
        if exe_check.exists() {
            return Some(cur);
        }
    }

    let steam_path = get_steam_install_path()?;

    let check_path = |lib: &Path, name: &str| -> Option<PathBuf> {
        let full_path = lib.join("steamapps").join("common").join(name);
        let exe_check = full_path.join("csgo.exe");
        if exe_check.exists() {
            Some(full_path)
        } else {
            None
        }
    };

    let check_all_libs = |name: &str| -> Option<PathBuf> {
        if let Some(path) = check_path(&steam_path, name) {
            return Some(path);
        }

        let vdf_path = steam_path.join("steamapps").join("libraryfolders.vdf");
        if let Ok(content) = fs::read_to_string(&vdf_path) {
            if let Some(library_paths) = extract_library_paths(&content) {
                for lib_path in library_paths {
                    if let Some(path) = check_path(&lib_path, name) {
                        return Some(path);
                    }
                }
            }
        }
        None
    };

    if let Some(path) = check_all_libs(game_name) {
        return Some(path);
    }

    if game_name == "Counter-Strike Global Offensive" {
        if let Some(path) = check_all_libs("Counter-Strike Global Offensive 730") {
            return Some(path);
        }
    }

    None
}

fn extract_library_paths(content: &str) -> Option<Vec<PathBuf>> {
    let vdf = keyvalues_parser::parse(content)
        .map(keyvalues_parser::Vdf::from)
        .ok()?;
    let mut paths = Vec::new();

    if let keyvalues_parser::Value::Obj(root_obj) = &vdf.value {
        for (_key, values) in root_obj.iter() {
            for val in values {
                if let keyvalues_parser::Value::Obj(folder_obj) = val {
                    for (folder_key, folder_values) in folder_obj.iter() {
                        if folder_key.eq_ignore_ascii_case("path") {
                            for path_val in folder_values {
                                if let keyvalues_parser::Value::Str(path_str) = path_val {
                                    let clean_path = path_str.replace("\\\\", "\\");
                                    paths.push(PathBuf::from(clean_path));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if paths.is_empty() {
        None
    } else {
        Some(paths)
    }
}
