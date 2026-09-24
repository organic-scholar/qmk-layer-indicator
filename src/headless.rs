use std::{
    error::Error,
    sync::{Arc, mpsc},
    thread,
};

use crate::{daemon, qmk, tray};

struct Services {
    _socket: daemon::LayerSocket,
    _console_reader: qmk::ConsoleReader,
    _tray_icon: tray::TrayHandle,
}

pub fn run() -> Result<(), Box<dyn Error>> {
    run_platform()
}

fn start_services(tray_callback: tray::TrayCallback) -> Result<Services, Box<dyn Error>> {
    let socket = daemon::LayerSocket::start()?;
    let publisher = socket.publisher();
    let console_reader = qmk::start_console_reader(move |event| {
        publish_layer_event(&publisher, &event);
    });
    let tray_icon = tray::create(tray_callback)?;

    Ok(Services {
        _socket: socket,
        _console_reader: console_reader,
        _tray_icon: tray_icon,
    })
}

#[cfg(target_os = "linux")]
fn run_platform() -> Result<(), Box<dyn Error>> {
    let (shutdown_sender, shutdown_receiver) = mpsc::channel();
    let tray_sender = shutdown_sender.clone();
    let _services = start_services(Arc::new(move |command| {
        handle_tray_command(command, &tray_sender);
    }))?;
    install_shutdown_signal_handler(move || {
        let _ = shutdown_sender.send(());
    })?;
    shutdown_receiver.recv()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn run_platform() -> Result<(), Box<dyn Error>> {
    use winit::{
        event::Event,
        event_loop::{ControlFlow, EventLoop},
    };

    let event_loop = EventLoop::<()>::with_user_event().build()?;
    let event_loop_proxy = event_loop.create_proxy();
    let (shutdown_sender, shutdown_receiver) = mpsc::channel();
    let tray_sender = shutdown_sender.clone();
    let tray_proxy = event_loop_proxy.clone();
    let _services = start_services(Arc::new(move |command| {
        handle_tray_command(command, &tray_sender);
        let _ = tray_proxy.send_event(());
    }))?;
    install_shutdown_signal_handler(move || {
        let _ = shutdown_sender.send(());
        let _ = event_loop_proxy.send_event(());
    })?;

    event_loop.run(move |event, event_loop| {
        event_loop.set_control_flow(ControlFlow::Wait);
        if matches!(event, Event::UserEvent(())) && shutdown_receiver.try_recv().is_ok() {
            event_loop.exit();
        }
    })?;
    Ok(())
}

fn handle_tray_command(command: tray::TrayCommand, shutdown_sender: &mpsc::Sender<()>) {
    match command {
        tray::TrayCommand::EditConfiguration => {
            if let Err(error) = crate::config::open_in_default_application() {
                eprintln!("Could not open configuration: {error}");
            }
        }
        tray::TrayCommand::Quit => {
            let _ = shutdown_sender.send(());
        }
    }
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
