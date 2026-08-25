use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    config::Settings,
    qmk::{ConsoleReader, QmkEvent},
    tray::{TrayCommand, TrayHandle},
};

const INDICATOR_SIZE: f32 = 104.0;
const BOTTOM_MARGIN: f32 = 50.0;
const ANIMATION_DURATION: f32 = 0.12;

pub fn native_options() -> eframe::NativeOptions {
    let viewport = egui::ViewportBuilder::default()
        .with_icon(Arc::new(application_icon()))
        .with_inner_size([INDICATOR_SIZE, INDICATOR_SIZE])
        .with_resizable(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_active(false)
        .with_visible(false)
        .with_mouse_passthrough(true)
        .with_taskbar(false);

    #[cfg(target_os = "linux")]
    let viewport = viewport
        .with_window_type(egui::X11WindowType::Utility)
        .with_override_redirect(true);

    eframe::NativeOptions {
        viewport,
        ..Default::default()
    }
}

#[derive(Clone)]
pub struct LayerIndicator {
    state: Arc<Mutex<IndicatorState>>,
    context: egui::Context,
}

impl LayerIndicator {
    pub fn new(context: egui::Context, settings: Settings) -> Self {
        Self {
            state: Arc::new(Mutex::new(IndicatorState {
                settings,
                ..Default::default()
            })),
            context,
        }
    }

    pub fn handle_qmk_event(&self, event: QmkEvent) {
        let mut state = self.state.lock().expect("indicator state lock poisoned");
        match event {
            QmkEvent::DeviceConnected { .. } => return,
            QmkEvent::LayerChanged(layer) => {
                state.active_layer = layer;
                if layer != 0 {
                    state.displayed_layer = layer;
                }
            }
            QmkEvent::DeviceDisconnected { .. } => state.active_layer = 0,
        }
        drop(state);
        self.context.request_repaint();
    }

    pub fn handle_tray_command(&self, command: TrayCommand) {
        match command {
            TrayCommand::EditConfiguration => {
                if let Err(error) = crate::config::open_in_default_application() {
                    eprintln!("Could not open configuration: {error}");
                }
            }
            TrayCommand::ReloadConfiguration => {
                self.state
                    .lock()
                    .expect("indicator state lock poisoned")
                    .settings = Settings::load();
                self.context.request_repaint();
            }
            TrayCommand::Quit => self.context.send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }
}

#[derive(Default)]
struct IndicatorState {
    active_layer: u8,
    displayed_layer: u8,
    positioned: bool,
    shown: bool,
    settings: Settings,
}

pub struct EguiApp {
    indicator: LayerIndicator,
    _console_reader: ConsoleReader,
    _tray_icon: Option<TrayHandle>,
    previous_active: bool,
}

impl EguiApp {
    pub fn new(
        indicator: LayerIndicator,
        console_reader: ConsoleReader,
        tray_icon: Option<TrayHandle>,
    ) -> Self {
        Self {
            indicator,
            _console_reader: console_reader,
            _tray_icon: tray_icon,
            previous_active: false,
        }
    }
}

impl eframe::App for EguiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut state = self
            .indicator
            .state
            .lock()
            .expect("indicator state lock poisoned");
        let active = state.active_layer != 0;

        if active && !state.positioned {
            let monitor_size = ui.ctx().input(|input| input.viewport().monitor_size);
            if let Some(monitor_size) = monitor_size {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                        (monitor_size.x - INDICATOR_SIZE) / 2.0,
                        monitor_size.y - INDICATOR_SIZE - BOTTOM_MARGIN,
                    )));
                state.positioned = true;
                ui.ctx().request_repaint();
                return;
            }
        }

        if active && state.positioned && !state.shown {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(true));
            state.shown = true;
        }

        if active != self.previous_active {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::MousePassthrough(!active));
            self.previous_active = active;
        }

        let visibility = ui.ctx().animate_bool_with_time_and_easing(
            egui::Id::new("indicator-visibility"),
            active,
            ANIMATION_DURATION,
            egui::emath::easing::cubic_out,
        );
        if visibility > 0.0 && visibility < 1.0 {
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
        if visibility == 0.0 {
            return;
        }

        let label = state.settings.layer_label(state.displayed_layer);
        drop(state);

        let available = ui.max_rect();
        let scale = 0.9 + (0.1 * visibility);
        let indicator_rect =
            egui::Rect::from_center_size(available.center(), available.size() * scale);
        let hovered = ui.rect_contains_pointer(indicator_rect);
        let (background_alpha, text_alpha) = if hovered { (0.5, 1.0) } else { (0.25, 0.85) };

        ui.painter().rect_filled(
            indicator_rect,
            egui::CornerRadius::same((22.0 * scale).round() as u8),
            egui::Rgba::from_black_alpha(background_alpha * visibility),
        );
        ui.painter().text(
            indicator_rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(48.0 * scale),
            egui::Rgba::from_white_alpha(text_alpha * visibility).into(),
        );
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }
}

fn application_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../assets/icon-white.png"))
        .expect("icon-white.png must be a valid PNG")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}
