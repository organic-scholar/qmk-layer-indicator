use std::{
    collections::BTreeMap,
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Duration,
};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;

const CONFIG_RELOAD_DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    layer_aliases: BTreeMap<u8, String>,
}

impl Config {
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

    pub fn layer_aliases(&self) -> &BTreeMap<u8, String> {
        &self.layer_aliases
    }
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
