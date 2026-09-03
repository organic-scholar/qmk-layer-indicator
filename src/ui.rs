use std::sync::{Arc, Mutex};

use crate::{
    config::{ConfigWatcher, IndicatorPosition, IndicatorShape, Settings},
    qmk::{ConsoleReader, QmkEvent},
    tray::{TrayCommand, TrayHandle},
};

const INDICATOR_SIZE: f32 = 96.0;
// Larger on macOS to clear the Dock, which is not excluded from `monitor_size`.
#[cfg(target_os = "macos")]
const BOTTOM_MARGIN: f32 = 120.0;
#[cfg(not(target_os = "macos"))]
const BOTTOM_MARGIN: f32 = 120.0;
const TOP_MARGIN: f32 = 120.0;
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
        // Drop the macOS window shadow so the overlay has no border/ghosting.
        .with_has_shadow(false)
        .with_taskbar(false);

    #[cfg(target_os = "linux")]
    let viewport = viewport
        .with_window_type(egui::X11WindowType::Utility)
        .with_override_redirect(true);

    eframe::NativeOptions {
        viewport,
        // Keep the app out of the macOS Dock and app switcher.
        #[cfg(target_os = "macos")]
        event_loop_builder: Some(Box::new(|builder| {
            use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
            builder.with_activation_policy(ActivationPolicy::Accessory);
            // Don't steal focus from the active app when launching.
            builder.with_activate_ignoring_other_apps(false);
        })),
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
        let mut state = IndicatorState {
            settings,
            ..Default::default()
        };
        state.sync_display();
        Self {
            state: Arc::new(Mutex::new(state)),
            context,
        }
    }

    pub fn handle_qmk_event(&self, event: QmkEvent) {
        let mut state = self.state.lock().expect("indicator state lock poisoned");
        match event {
            QmkEvent::DeviceConnected { .. } => return,
            QmkEvent::LayerChanged(layer) if state.active_layer == layer => return,
            QmkEvent::LayerChanged(layer) => state.active_layer = layer,
            QmkEvent::DeviceDisconnected { .. } if state.active_layer == 0 => return,
            QmkEvent::DeviceDisconnected { .. } => state.active_layer = 0,
        }
        state.sync_display();
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
            TrayCommand::Quit => self.context.send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }

    pub fn reload_configuration(&self) {
        let settings = match Settings::try_load() {
            Ok(settings) => settings,
            Err(error) => {
                eprintln!("Could not reload configuration; keeping current settings: {error}");
                return;
            }
        };
        #[cfg(target_os = "linux")]
        if let Err(error) = settings.apply_autostart() {
            eprintln!("Could not update start-at-login setting: {error}");
        }
        let mut state = self.state.lock().expect("indicator state lock poisoned");
        state.settings = settings;
        // Apply a changed position the next time `ui` runs, including while the
        // indicator is already visible.
        state.positioned = false;
        state.sync_display();
        drop(state);
        self.context.request_repaint();
    }
}

#[derive(Default)]
struct IndicatorState {
    active_layer: u8,
    visible: bool,
    positioned: bool,
    shown: bool,
    previous_active: bool,
    label: String,
    icon: Option<String>,
    settings: Settings,
}

impl IndicatorState {
    // Only layers with a non-empty alias are shown; otherwise keep the last
    // content so the fade-out animation still has something to render.
    fn sync_display(&mut self) {
        self.visible = false;
        if let Some(icon) = self.settings.layer_icon(self.active_layer) {
            self.icon = Some(icon.into());
            self.visible = true;
            return;
        }
        if let Some(alias) = self.settings.layer_alias(self.active_layer) {
            self.icon = None;
            self.label = alias.into();
            self.visible = true;
        }
    }

    fn is_shown(&self) -> bool {
        self.visible
    }
}

pub struct EguiApp {
    indicator: LayerIndicator,
    _console_reader: ConsoleReader,
    _tray_icon: Option<TrayHandle>,
    _config_watcher: Option<ConfigWatcher>,
}

impl EguiApp {
    pub fn new(
        indicator: LayerIndicator,
        console_reader: ConsoleReader,
        tray_icon: Option<TrayHandle>,
        config_watcher: Option<ConfigWatcher>,
    ) -> Self {
        Self {
            indicator,
            _console_reader: console_reader,
            _tray_icon: tray_icon,
            _config_watcher: config_watcher,
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
        let active = state.is_shown();

        if active && !state.positioned {
            let monitor_size = ui.ctx().input(|input| input.viewport().monitor_size);
            if let Some(monitor_size) = monitor_size {
                let position = state.settings.position();
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::OuterPosition(indicator_position(
                        monitor_size,
                        position,
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

        if active != state.previous_active {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::MousePassthrough(!active));
            state.previous_active = active;
        }

        let visibility = ui.ctx().animate_bool_with_time_and_easing(
            egui::Id::new("indicator-visibility"),
            active,
            ANIMATION_DURATION,
            egui::emath::easing::cubic_out,
        );
        if visibility == 0.0 {
            return;
        }

        let label = state.label.clone();
        let icon = state.icon.clone();
        let shape = state.settings.indicator_shape();
        drop(state);

        let available = ui.max_rect();
        let scale = 0.9 + (0.1 * visibility);
        let indicator_rect =
            egui::Rect::from_center_size(available.center(), available.size() * scale);
        let hovered = ui.rect_contains_pointer(indicator_rect);
        let (background_alpha, text_alpha) = if hovered { (0.5, 1.0) } else { (0.4, 1.0) };

        let background = egui::Rgba::from_black_alpha(background_alpha * visibility).into();
        match shape {
            IndicatorShape::Squircle => paint_squircle(ui.painter(), indicator_rect, background),
            IndicatorShape::RoundedRectangle => {
                ui.painter().rect_filled(
                    indicator_rect,
                    egui::CornerRadius::same((20.0 * scale).round() as u8),
                    background,
                );
            }
        }
        match icon {
            Some(icon) => {
                let icon_rect = indicator_rect.shrink(24.0 * scale);
                if let Some(image) = crate::icons::image_source(&icon) {
                    ui.put(
                        icon_rect,
                        egui::Image::new(image)
                            .tint(egui::Rgba::from_white_alpha(text_alpha * visibility)),
                    );
                }
            }
            None => {
                ui.painter().text(
                    indicator_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(44.0 * scale),
                    egui::Rgba::from_white_alpha(text_alpha * visibility).into(),
                );
            }
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }
}

fn indicator_position(monitor_size: egui::Vec2, position: IndicatorPosition) -> egui::Pos2 {
    let x = (monitor_size.x - INDICATOR_SIZE) / 2.0;
    let y = match position {
        IndicatorPosition::Top => TOP_MARGIN,
        IndicatorPosition::Bottom => monitor_size.y - INDICATOR_SIZE - BOTTOM_MARGIN,
    };
    egui::pos2(x, y)
}

fn paint_squircle(painter: &egui::Painter, rect: egui::Rect, fill: egui::Color32) {
    const SEGMENTS: usize = 64;
    // A Lamé curve with exponent four closely matches macOS-style continuous corners.
    const EXPONENT: f32 = 4.0;

    let center = rect.center();
    let radius = rect.size() / 2.0;
    let points = (0..SEGMENTS)
        .map(|index| {
            let angle = std::f32::consts::TAU * index as f32 / SEGMENTS as f32;
            let x = angle.cos();
            let y = angle.sin();
            egui::pos2(
                center.x + radius.x * x.signum() * x.abs().powf(2.0 / EXPONENT),
                center.y + radius.y * y.signum() * y.abs().powf(2.0 / EXPONENT),
            )
        })
        .collect();

    painter.add(egui::Shape::convex_polygon(
        points,
        fill,
        egui::Stroke::NONE,
    ));
}

fn application_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .expect("icon-white.png must be a valid PNG")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}
