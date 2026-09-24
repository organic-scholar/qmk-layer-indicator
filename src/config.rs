use std::{
    collections::BTreeMap,
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Duration,
};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};

const EDITABLE_LAYER_COUNT: u8 = 7;
const CONFIG_RELOAD_DEBOUNCE: Duration = Duration::from_millis(150);
const DEFAULT_INDICATOR_SIZE: u16 = 64;
const DEFAULT_MARGIN: u16 = 120;

#[derive(Debug, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    headless: bool,
    #[serde(default)]
    indicator_shape: IndicatorShape,
    #[serde(default)]
    position: IndicatorPosition,
    #[serde(default = "default_indicator_size")]
    size: u16,
    #[serde(default = "default_margin")]
    margin: u16,
    #[serde(default = "default_layer_aliases")]
    layer_aliases: BTreeMap<u8, String>,
    #[serde(default)]
    layer_icons: BTreeMap<u8, String>,
    #[cfg(target_os = "linux")]
    #[serde(default)]
    start_at_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            headless: false,
            indicator_shape: IndicatorShape::default(),
            position: IndicatorPosition::default(),
            size: default_indicator_size(),
            margin: default_margin(),
            layer_aliases: default_layer_aliases(),
            layer_icons: BTreeMap::new(),
            #[cfg(target_os = "linux")]
            start_at_login: false,
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

    pub fn try_load() -> Result<Self, Box<dyn Error>> {
        let path = config_path()?;
        let contents = fs::read_to_string(&path)?;
        Ok(toml::from_str(&contents)?)
    }

    pub fn layer_alias(&self, layer: u8) -> Option<&str> {
        self.layer_aliases
            .get(&layer)
            .filter(|alias| !alias.is_empty())
            .map(String::as_str)
    }

    pub fn headless(&self) -> bool {
        self.headless
    }

    pub fn indicator_shape(&self) -> IndicatorShape {
        self.indicator_shape
    }

    pub fn position(&self) -> IndicatorPosition {
        self.position
    }

    pub fn indicator_size(&self) -> f32 {
        self.size.clamp(32, 128) as f32
    }

    pub fn margin(&self) -> f32 {
        self.margin.min(300) as f32
    }

    pub fn layer_icon(&self, layer: u8) -> Option<&str> {
        let icon = self.layer_icons.get(&layer)?.as_str();
        crate::icons::contains(icon).then_some(icon)
    }

    #[cfg(target_os = "linux")]
    pub fn apply_autostart(&self) -> Result<(), Box<dyn Error>> {
        let path = autostart_path()?;

        if !self.start_at_login {
            match fs::remove_file(&path) {
                Ok(()) => return Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error.into()),
            }
        }

        let executable = env::current_exe()?;
        let executable = executable
            .to_str()
            .ok_or("application path is not valid UTF-8")?;
        let executable = escape_desktop_exec_argument(executable);
        let directory = path
            .parent()
            .expect("autostart path has a parent directory");
        fs::create_dir_all(directory)?;
        fs::write(
            path,
            format!(
                "[Desktop Entry]\nType=Application\nName=QMK Layer Indicator\nExec={executable}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
            ),
        )?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IndicatorShape {
    #[default]
    Circle,
    Squircle,
    RoundedRectangle,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IndicatorPosition {
    Top,
    #[default]
    Bottom,
}

const fn default_indicator_size() -> u16 {
    DEFAULT_INDICATOR_SIZE
}

const fn default_margin() -> u16 {
    DEFAULT_MARGIN
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
    stop_sender: mpsc::Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        let _ = self.stop_sender.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
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

    let (change_sender, change_receiver) = mpsc::channel();
    let (stop_sender, stop_receiver) = mpsc::channel();
    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<Event>| match result {
            Ok(event) if event_touches_config(&event, &path) => {
                let _ = change_sender.send(());
            }
            Ok(_) => {}
            Err(error) => eprintln!("Config watch error: {error}"),
        })?;
    watcher.watch(&directory, RecursiveMode::NonRecursive)?;

    let thread = thread::Builder::new()
        .name("config-watcher".into())
        .spawn(move || {
            loop {
                if stop_receiver.try_recv().is_ok() {
                    return;
                }

                if change_receiver
                    .recv_timeout(CONFIG_RELOAD_DEBOUNCE)
                    .is_err()
                {
                    continue;
                }

                while change_receiver.recv_timeout(CONFIG_RELOAD_DEBOUNCE).is_ok() {}
                if stop_receiver.try_recv().is_ok() {
                    return;
                }
                on_change();
            }
        })?;

    Ok(ConfigWatcher {
        _watcher: watcher,
        stop_sender,
        thread: Some(thread),
    })
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
    Ok(config_base_directory()?
        .join("qmk-layer-indicator")
        .join("config.toml"))
}

fn config_base_directory() -> Result<PathBuf, Box<dyn Error>> {
    #[cfg(target_os = "linux")]
    let base_directory = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));

    #[cfg(target_os = "macos")]
    let base_directory =
        env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"));

    let base_directory = base_directory.ok_or("could not determine the configuration directory")?;
    Ok(base_directory)
}

#[cfg(target_os = "linux")]
fn autostart_path() -> Result<PathBuf, Box<dyn Error>> {
    Ok(config_base_directory()?
        .join("autostart")
        .join("qmk-layer-indicator.desktop"))
}

#[cfg(target_os = "linux")]
fn escape_desktop_exec_argument(argument: &str) -> String {
    format!(
        "\"{}\"",
        argument
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('`', "\\`")
            .replace('$', "\\$")
    )
}

fn default_layer_aliases() -> BTreeMap<u8, String> {
    (1..=EDITABLE_LAYER_COUNT)
        .map(|layer| (layer, String::new()))
        .collect()
}
