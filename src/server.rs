use std::{
    collections::BTreeMap,
    io::{self, ErrorKind},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use tungstenite::{Message, WebSocket, accept};

use crate::config::Config;

pub const DEFAULT_WEBSOCKET_PORT: u16 = 51_837;

const POLL_INTERVAL: Duration = Duration::from_millis(100);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);

#[derive(Clone)]
pub struct LayerPublisher {
    sender: Sender<ServerEvent>,
}

impl LayerPublisher {
    pub fn publish(&self, layer: u8) {
        let _ = self.sender.send(ServerEvent::Layer(layer));
    }

    pub fn update_aliases(&self, aliases: BTreeMap<u8, String>) {
        let _ = self.sender.send(ServerEvent::Aliases(aliases));
    }
}

enum ServerEvent {
    Layer(u8),
    Aliases(BTreeMap<u8, String>),
}

pub struct LayerServer {
    publisher: LayerPublisher,
    shutdown_sender: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl LayerServer {
    pub fn start(config: &Config) -> io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", DEFAULT_WEBSOCKET_PORT))?;
        listener.set_nonblocking(true)?;

        let (layer_sender, layer_receiver) = mpsc::channel();
        let (shutdown_sender, shutdown_receiver) = mpsc::channel();
        let aliases = config.layer_aliases().clone();
        let thread = thread::Builder::new()
            .name("layer-websocket".into())
            .spawn(move || run_server(listener, layer_receiver, shutdown_receiver, aliases))?;

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
    layer_receiver: Receiver<ServerEvent>,
    shutdown_receiver: Receiver<()>,
    mut aliases: BTreeMap<u8, String>,
) {
    let mut clients = Vec::new();
    let mut current_layer = 0;
    let mut last_broadcast = Instant::now();

    loop {
        if shutdown_receiver.try_recv().is_ok() {
            return;
        }
        accept_clients(&listener, &mut clients, current_layer, &aliases);

        match layer_receiver.recv_timeout(POLL_INTERVAL) {
            Ok(event) => {
                match event {
                    ServerEvent::Layer(layer) => current_layer = layer,
                    ServerEvent::Aliases(updated) => aliases = updated,
                }
                broadcast_layer(&mut clients, current_layer, &aliases);
                last_broadcast = Instant::now();
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if last_broadcast.elapsed() >= HEARTBEAT_INTERVAL {
                    broadcast_heartbeat(&mut clients);
                    last_broadcast = Instant::now();
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn accept_clients(
    listener: &TcpListener,
    clients: &mut Vec<WebSocket<TcpStream>>,
    layer: u8,
    aliases: &BTreeMap<u8, String>,
) {
    loop {
        match listener.accept() {
            Ok((stream, _)) => match accept(stream) {
                Ok(mut client) => {
                    if write_layer(&mut client, layer, aliases) {
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

fn broadcast_layer(
    clients: &mut Vec<WebSocket<TcpStream>>,
    layer: u8,
    aliases: &BTreeMap<u8, String>,
) {
    clients.retain_mut(|client| write_layer(client, layer, aliases));
}

fn broadcast_heartbeat(clients: &mut Vec<WebSocket<TcpStream>>) {
    clients.retain_mut(|client| {
        client
            .send(Message::text(r#"{"type":"heartbeat"}"#))
            .is_ok()
    });
}

fn write_layer(
    client: &mut WebSocket<TcpStream>,
    layer: u8,
    aliases: &BTreeMap<u8, String>,
) -> bool {
    client
        .send(Message::text(layer_message(layer, aliases)))
        .is_ok()
}

fn layer_message(layer: u8, aliases: &BTreeMap<u8, String>) -> String {
    serde_json::json!({
        "layer": layer,
        "alias": aliases.get(&layer).map(String::as_str).unwrap_or(""),
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_message_escapes_configured_alias_and_defaults_to_empty() {
        let aliases = BTreeMap::from([(1, "A\"B".to_owned())]);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&layer_message(1, &aliases)).unwrap(),
            serde_json::json!({"layer": 1, "alias": "A\"B"})
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&layer_message(0, &aliases)).unwrap(),
            serde_json::json!({"layer": 0, "alias": ""})
        );
    }
}
