use std::{
    collections::BTreeMap,
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};

const EDITABLE_LAYER_COUNT: u8 = 7;

#[derive(Debug, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default = "default_layer_aliases")]
    layer_aliases: BTreeMap<u8, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            layer_aliases: default_layer_aliases(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let Ok(path) = config_path() else {
            return Self::default();
        };
        let Ok(contents) = fs::read_to_string(&path) else {
            return Self::default();
        };

        match toml::from_str(&contents) {
            Ok(settings) => settings,
            Err(error) => {
                eprintln!("Could not read {}: {error}", path.display());
                Self::default()
            }
        }
    }

    pub fn layer_alias(&self, layer: u8) -> Option<String> {
        self.layer_aliases
            .get(&layer)
            .filter(|alias| !alias.is_empty())
            .cloned()
    }
}

pub fn open_in_default_application() -> Result<(), Box<dyn Error>> {
    let path = ensure_config_file()?;

    #[cfg(target_os = "linux")]
    Command::new("xdg-open").arg(path).spawn()?;

    #[cfg(target_os = "macos")]
    Command::new("open").arg(path).spawn()?;

    Ok(())
}

pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
}

pub fn watch_config(
    on_change: impl Fn() + Send + 'static,
) -> Result<ConfigWatcher, Box<dyn Error>> {
    let path = config_path()?;
    let directory = path
        .parent()
        .ok_or("config path has a parent directory")?
        .to_path_buf();
    // Watch the directory so atomic saves (write-temp-then-rename) are caught.
    fs::create_dir_all(&directory)?;

    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<Event>| match result {
            Ok(event) if event_touches_config(&event, &path) => on_change(),
            Ok(_) => {}
            Err(error) => eprintln!("Config watch error: {error}"),
        })?;
    watcher.watch(&directory, RecursiveMode::NonRecursive)?;

    Ok(ConfigWatcher { _watcher: watcher })
}

fn event_touches_config(event: &Event, config_path: &Path) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) && event.paths.iter().any(|path| path == config_path)
}

fn ensure_config_file() -> Result<PathBuf, Box<dyn Error>> {
    let path = config_path()?;
    if path.exists() {
        return Ok(path);
    }

    let directory = path.parent().expect("config path has a parent directory");
    fs::create_dir_all(directory)?;
    fs::write(&path, toml::to_string_pretty(&Settings::default())?)?;
    Ok(path)
}

fn config_path() -> Result<PathBuf, Box<dyn Error>> {
    #[cfg(target_os = "linux")]
    let base_directory = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));

    #[cfg(target_os = "macos")]
    let base_directory =
        env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"));

    let base_directory = base_directory.ok_or("could not determine the configuration directory")?;
    Ok(base_directory
        .join("qmk-layer-indicator")
        .join("config.toml"))
}

fn default_layer_aliases() -> BTreeMap<u8, String> {
    (1..=EDITABLE_LAYER_COUNT)
        .map(|layer| (layer, String::new()))
        .collect()
}
