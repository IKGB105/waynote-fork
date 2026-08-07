//! App-managed runtime preferences in `$XDG_STATE_HOME/waynote/runtime.toml`.
//!
//! Distinct from `config.toml` (the user's hand-editable file, which Waynote never
//! rewrites): this holds UI toggles the app/tray flip at runtime — "confirm before
//! delete" and "font scale" (tray +/- buttons). A missing value falls back to the
//! `config.toml` default, so the user can still set the initial preference there.

use serde::{Deserialize, Serialize};

use super::paths::Paths;
use super::store::atomic_write;

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct RuntimePrefs {
    /// `None` = not set yet → caller uses the config default.
    confirm_delete: Option<bool>,
    /// `None` = not set yet → caller uses `config.font_scale`. Set by the tray
    /// +/- buttons (`Controller::adjust_font_scale`).
    font_scale: Option<f64>,
}

fn load(paths: &Paths) -> RuntimePrefs {
    std::fs::read_to_string(paths.runtime_prefs_file())
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(paths: &Paths, prefs: &RuntimePrefs) -> std::io::Result<()> {
    let path = paths.runtime_prefs_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(prefs)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    atomic_write(&path, text.as_bytes())
}

/// The persisted "confirm before delete" preference, or `default` when unset.
pub fn load_confirm_delete(paths: &Paths, default: bool) -> bool {
    load(paths).confirm_delete.unwrap_or(default)
}

/// Persist the "confirm before delete" preference atomically.
pub fn save_confirm_delete(paths: &Paths, value: bool) -> std::io::Result<()> {
    let mut prefs = load(paths);
    prefs.confirm_delete = Some(value);
    save(paths, &prefs)
}

/// The persisted font-scale preference, or `default` (from `config.font_scale`)
/// when unset.
pub fn load_font_scale(paths: &Paths, default: f64) -> f64 {
    load(paths).font_scale.unwrap_or(default)
}

/// Persist the font-scale preference atomically.
pub fn save_font_scale(paths: &Paths, value: f64) -> std::io::Result<()> {
    let mut prefs = load(paths);
    prefs.font_scale = Some(value);
    save(paths, &prefs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths_in(dir: &std::path::Path) -> Paths {
        let base = dir.to_str().unwrap().to_string();
        Paths::from_env(|k| match k {
            "XDG_STATE_HOME" => Some(base.clone()),
            _ => None,
        })
    }

    #[test]
    fn missing_file_returns_the_given_default() {
        let dir = tempfile::tempdir().unwrap();
        let paths = paths_in(dir.path());
        assert!(load_confirm_delete(&paths, true));
        assert!(!load_confirm_delete(&paths, false));
    }

    #[test]
    fn saved_value_overrides_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let paths = paths_in(dir.path());
        save_confirm_delete(&paths, false).unwrap();
        assert!(!load_confirm_delete(&paths, true));
        save_confirm_delete(&paths, true).unwrap();
        assert!(load_confirm_delete(&paths, false));
    }

    #[test]
    fn font_scale_missing_file_returns_the_given_default() {
        let dir = tempfile::tempdir().unwrap();
        let paths = paths_in(dir.path());
        assert_eq!(load_font_scale(&paths, 1.0), 1.0);
    }

    #[test]
    fn font_scale_saved_value_overrides_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let paths = paths_in(dir.path());
        save_font_scale(&paths, 1.5).unwrap();
        assert_eq!(load_font_scale(&paths, 1.0), 1.5);
    }

    #[test]
    fn confirm_delete_and_font_scale_do_not_clobber_each_other() {
        let dir = tempfile::tempdir().unwrap();
        let paths = paths_in(dir.path());
        save_confirm_delete(&paths, false).unwrap();
        save_font_scale(&paths, 2.0).unwrap();
        assert!(!load_confirm_delete(&paths, true));
        assert_eq!(load_font_scale(&paths, 1.0), 2.0);
    }
}
