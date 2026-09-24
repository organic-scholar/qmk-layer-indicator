use std::{error::Error, sync::Arc};

mod config;
mod daemon;
mod headless;
mod icons;
mod qmk;
mod tray;
mod ui;

fn main() -> Result<(), Box<dyn Error>> {
    let settings = config::Settings::load();
    if settings.headless() {
        return headless::run();
    }

    #[cfg(target_os = "linux")]
    if let Err(error) = settings.apply_autostart() {
        eprintln!("Could not update start-at-login setting: {error}");
    }

    eframe::run_native(
        "QMK Layer Indicator",
        ui::native_options(&settings),
        Box::new(move |creation_context| {
            egui_extras::install_image_loaders(&creation_context.egui_ctx);
            let indicator = ui::LayerIndicator::new(creation_context.egui_ctx.clone(), settings);
            let tray_icon = match tray::create(Arc::new({
                let indicator = indicator.clone();
                move |command| indicator.handle_tray_command(command)
            })) {
                Ok(icon) => Some(icon),
                Err(error) => {
                    eprintln!("Could not create system-tray icon: {error}");
                    None
                }
            };
            let layer_socket = match daemon::LayerSocket::start() {
                Ok(socket) => Some(socket),
                Err(error) => {
                    eprintln!("Could not start layer socket: {error}");
                    None
                }
            };
            let publisher = layer_socket.as_ref().map(daemon::LayerSocket::publisher);
            let console_reader = qmk::start_console_reader({
                let indicator = indicator.clone();
                move |event| {
                    publish_layer_event(publisher.as_ref(), &event);
                    indicator.handle_qmk_event(event);
                }
            });
            let config_watcher = match config::watch_config({
                let indicator = indicator.clone();
                move || indicator.reload_configuration()
            }) {
                Ok(watcher) => Some(watcher),
                Err(error) => {
                    eprintln!("Could not watch the configuration file: {error}");
                    None
                }
            };
            Ok(Box::new(ui::EguiApp::new(
                indicator,
                console_reader,
                tray_icon,
                config_watcher,
                layer_socket,
            )))
        }),
    )?;
    Ok(())
}

fn publish_layer_event(publisher: Option<&daemon::LayerPublisher>, event: &qmk::QmkEvent) {
    let Some(publisher) = publisher else {
        return;
    };

    match event {
        qmk::QmkEvent::LayerChanged(layer) => publisher.publish(*layer),
        qmk::QmkEvent::DeviceDisconnected { .. } => publisher.publish(0),
        qmk::QmkEvent::DeviceConnected { .. } => {}
    }
}
