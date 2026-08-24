use std::sync::{
    Arc, RwLock,
    atomic::{AtomicU8, Ordering},
};

use slint::ComponentHandle;

use crate::{config::Settings, qmk::QmkEvent, tray::TrayCommand};

slint::include_modules!();

/// Schedules application changes onto Slint's event loop.
///
/// Cloning this handle is inexpensive and lets background services report their
/// results without accessing the native window directly.
#[derive(Clone)]
pub struct LayerIndicator {
    view: slint::Weak<LayerIndicatorView>,
    settings: Arc<RwLock<Settings>>,
    active_layer: Arc<AtomicU8>,
}

impl LayerIndicator {
    pub fn new(view: &LayerIndicatorView, settings: Settings) -> Self {
        let layer_indicator = Self {
            view: view.as_weak(),
            settings: Arc::new(RwLock::new(settings)),
            active_layer: Arc::new(AtomicU8::new(0)),
        };
        layer_indicator.schedule_update(None, Some(false), false);
        return layer_indicator;
    }

    pub fn handle_qmk_event(&self, event: QmkEvent) {
        match event {
            QmkEvent::DeviceConnected { .. } => {}
            QmkEvent::LayerChanged(layer) => self.set_layer(layer),
            QmkEvent::DeviceDisconnected { .. } => self.set_layer(0),
        }
    }

    pub fn handle_tray_command(&self, command: TrayCommand) {
        match command {
            TrayCommand::EditConfiguration => {
                if let Err(error) = crate::config::open_in_default_application() {
                    eprintln!("Could not open configuration: {error}");
                }
            }
            TrayCommand::ReloadConfiguration => {
                *self.settings.write().expect("settings lock poisoned") = Settings::load();
                let layer = self.active_layer.load(Ordering::Relaxed);
                if layer != 0 {
                    self.schedule_update(Some(self.layer_label(layer)), None, false);
                }
            }
            TrayCommand::Quit => self.schedule_update(None, None, true),
        }
    }

    fn set_layer(&self, layer: u8) {
        self.active_layer.store(layer, Ordering::Relaxed);
        if layer == 0 {
            self.schedule_update(None, Some(false), false);
        } else {
            self.schedule_update(Some(self.layer_label(layer)), Some(true), false);
        }
    }

    fn layer_label(&self, layer: u8) -> String {
        self.settings
            .read()
            .expect("settings lock poisoned")
            .layer_label(layer)
    }

    fn schedule_update(&self, label: Option<String>, active: Option<bool>, quit: bool) {
        let window = self.view.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(window) = window.upgrade() {
                super::backend::position_bottom_center(&window);
                if let Some(label) = label {
                    window.set_layer_label(label.into());
                }
                if let Some(active) = active {
                    window.set_active(active);
                    super::backend::set_mouse_passthrough(&window, !active);
                }
                if quit {
                    let _ = window.hide();
                }
            }
        });
    }
}
