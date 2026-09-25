use std::{
    io::{self, ErrorKind},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use tungstenite::{Message, WebSocket, accept};

pub const DEFAULT_WEBSOCKET_PORT: u16 = 51_837;

const POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone)]
pub struct LayerPublisher {
    sender: Sender<u8>,
}

impl LayerPublisher {
    pub fn publish(&self, layer: u8) {
        let _ = self.sender.send(layer);
    }
}

pub struct LayerServer {
    publisher: LayerPublisher,
    shutdown_sender: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl LayerServer {
    pub fn start() -> io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", DEFAULT_WEBSOCKET_PORT))?;
        listener.set_nonblocking(true)?;

        let (layer_sender, layer_receiver) = mpsc::channel();
        let (shutdown_sender, shutdown_receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("layer-websocket".into())
            .spawn(move || run_server(listener, layer_receiver, shutdown_receiver))?;

        Ok(Self {
            publisher: LayerPublisher {
                sender: layer_sender,
            },
            shutdown_sender,
            thread: Some(thread),
        })
    }

    pub fn publisher(&self) -> LayerPublisher {
        self.publisher.clone()
    }
}

impl Drop for LayerServer {
    fn drop(&mut self) {
        let _ = self.shutdown_sender.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run_server(
    listener: TcpListener,
    layer_receiver: Receiver<u8>,
    shutdown_receiver: Receiver<()>,
) {
    let mut clients = Vec::new();
    let mut current_layer = 0;

    loop {
        if shutdown_receiver.try_recv().is_ok() {
            return;
        }
        accept_clients(&listener, &mut clients, current_layer);

        match layer_receiver.recv_timeout(POLL_INTERVAL) {
            Ok(layer) => {
                current_layer = layer;
                broadcast_layer(&mut clients, layer);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn accept_clients(listener: &TcpListener, clients: &mut Vec<WebSocket<TcpStream>>, layer: u8) {
    loop {
        match listener.accept() {
            Ok((stream, _)) => match accept(stream) {
                Ok(mut client) => {
                    if write_layer(&mut client, layer) {
                        clients.push(client);
                    }
                }
                Err(error) => eprintln!("Layer WebSocket handshake error: {error}"),
            },
            Err(error) if error.kind() == ErrorKind::WouldBlock => return,
            Err(error) => {
                eprintln!("Layer WebSocket accept error: {error}");
                return;
            }
        }
    }
}

fn broadcast_layer(clients: &mut Vec<WebSocket<TcpStream>>, layer: u8) {
    clients.retain_mut(|client| write_layer(client, layer));
}

fn write_layer(client: &mut WebSocket<TcpStream>, layer: u8) -> bool {
    client.send(Message::text(format!("LAYER:{layer}"))).is_ok()
}
