use std::{error::Error, sync::mpsc, thread};

mod config;
mod qmk;
mod server;

fn main() -> Result<(), Box<dyn Error>> {
    let settings = config::Config::load();
    let server = server::LayerServer::start(&settings)?;
    let publisher = server.publisher();
    let _console_reader = qmk::start_console_reader(move |event| {
        publish_layer_event(&publisher, &event);
    });
    let _config_watcher = match config::watch_config({
        let publisher = server.publisher();
        move || match config::Config::try_load() {
            Ok(config) => publisher.update_aliases(config.layer_aliases().clone()),
            Err(error) => {
                eprintln!("Could not reload configuration; keeping current aliases: {error}")
            }
        }
    }) {
        Ok(watcher) => Some(watcher),
        Err(error) => {
            eprintln!("Could not watch the configuration file: {error}");
            None
        }
    };

    let (shutdown_sender, shutdown_receiver) = mpsc::channel();
    install_shutdown_signal_handler(move || {
        let _ = shutdown_sender.send(());
    })?;
    shutdown_receiver.recv()?;
    Ok(())
}

fn publish_layer_event(publisher: &server::LayerPublisher, event: &qmk::QmkEvent) {
    match event {
        qmk::QmkEvent::LayerChanged(layer) => publisher.publish(*layer),
        qmk::QmkEvent::DeviceDisconnected { .. } => publisher.publish(0),
        qmk::QmkEvent::DeviceConnected { .. } => {}
    }
}

fn install_shutdown_signal_handler(
    on_shutdown: impl Fn() + Send + 'static,
) -> Result<(), Box<dyn Error>> {
    use signal_hook::{consts::signal::SIGINT, consts::signal::SIGTERM, iterator::Signals};

    let mut signals = Signals::new([SIGINT, SIGTERM])?;
    thread::spawn(move || {
        if signals.forever().next().is_some() {
            on_shutdown();
        }
    });
    Ok(())
}
