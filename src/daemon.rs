use std::{
    env, fs,
    io::{self, ErrorKind, Write},
    os::unix::{
        fs::FileTypeExt,
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

const SOCKET_NAME: &str = "qmk-layer-indicator.sock";
const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone)]
pub struct LayerPublisher {
    sender: Sender<u8>,
}

impl LayerPublisher {
    pub fn publish(&self, layer: u8) {
        let _ = self.sender.send(layer);
    }
}

pub struct LayerSocket {
    path: PathBuf,
    publisher: LayerPublisher,
    shutdown_sender: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl LayerSocket {
    pub fn start() -> io::Result<Self> {
        let path = socket_path();
        remove_existing_socket(&path)?;
        let listener = UnixListener::bind(&path)?;
        listener.set_nonblocking(true)?;

        let (layer_sender, layer_receiver) = mpsc::channel();
        let (shutdown_sender, shutdown_receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("layer-socket".into())
            .spawn(move || run_server(listener, layer_receiver, shutdown_receiver))?;

        Ok(Self {
            path,
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

impl Drop for LayerSocket {
    fn drop(&mut self) {
        let _ = self.shutdown_sender.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = fs::remove_file(&self.path);
    }
}

fn run_server(
    listener: UnixListener,
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

fn accept_clients(listener: &UnixListener, clients: &mut Vec<UnixStream>, current_layer: u8) {
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                if stream.set_nonblocking(true).is_ok() && write_layer(&mut stream, current_layer) {
                    clients.push(stream);
                }
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => return,
            Err(error) => {
                eprintln!("Layer socket accept error: {error}");
                return;
            }
        }
    }
}

fn broadcast_layer(clients: &mut Vec<UnixStream>, layer: u8) {
    clients.retain_mut(|client| write_layer(client, layer));
}

fn write_layer(client: &mut UnixStream, layer: u8) -> bool {
    let message = format!("LAYER:{layer}\n");
    matches!(client.write(message.as_bytes()), Ok(length) if length == message.len())
}

fn socket_path() -> PathBuf {
    if let Some(path) = env::var_os("QMK_LAYER_INDICATOR_SOCKET") {
        return path.into();
    }

    env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join(SOCKET_NAME)
}

fn remove_existing_socket(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_socket() => match UnixStream::connect(path) {
            Ok(_) => Err(io::Error::new(
                ErrorKind::AddrInUse,
                format!("layer socket is already in use at {}", path.display()),
            )),
            Err(error) if error.kind() == ErrorKind::ConnectionRefused => fs::remove_file(path),
            Err(error) => Err(error),
        },
        Ok(_) => Err(io::Error::new(
            ErrorKind::AlreadyExists,
            format!("refusing to replace non-socket path {}", path.display()),
        )),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
