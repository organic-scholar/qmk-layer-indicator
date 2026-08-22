use std::sync::Arc;

use slint::ComponentHandle;

mod config;
mod qmk;
mod tray;
mod ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ui::configure_backend()?;

    let view = ui::LayerIndicatorView::new()?;
    let layer_indicator = ui::LayerIndicator::new(&view, config::Settings::load());

    let _console_reader = qmk::start_console_reader({
        let controller = layer_indicator.clone();
        move |event| controller.handle_qmk_event(event)
    });
    let _tray_icon = match tray::create(Arc::new({
        let controller = layer_indicator.clone();
        move |command| controller.handle_tray_command(command)
    })) {
        Ok(icon) => Some(icon),
        Err(error) => {
            eprintln!("Could not create system-tray icon: {error}");
            None
        }
    };
    view.run()?;
    Ok(())
}
