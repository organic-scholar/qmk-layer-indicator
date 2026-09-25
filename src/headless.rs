use std::{error::Error, sync::mpsc, thread};

use crate::{daemon, qmk};

struct Services {
    _server: daemon::LayerServer,
    _console_reader: qmk::ConsoleReader,
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let server = daemon::LayerServer::start()?;
    let publisher = server.publisher();
    let console_reader = qmk::start_console_reader(move |event| {
        publish_layer_event(&publisher, &event);
    });
    let _services = Services {
        _server: server,
        _console_reader: console_reader,
    };
    let (shutdown_sender, shutdown_receiver) = mpsc::channel();
    install_shutdown_signal_handler(move || {
        let _ = shutdown_sender.send(());
    })?;
    shutdown_receiver.recv()?;
    Ok(())
}

fn publish_layer_event(publisher: &daemon::LayerPublisher, event: &qmk::QmkEvent) {
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
