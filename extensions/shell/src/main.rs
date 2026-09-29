use std::{
    env,
    fs::{File, OpenOptions},
    io::{self, Write},
    thread,
    time::Duration,
};

use tungstenite::{Message, WebSocket, connect, stream::MaybeTlsStream};

const DEFAULT_URL: &str = "ws://127.0.0.1:51837";
const RETRY_DELAY: Duration = Duration::from_secs(1);

fn main() {
    if let Err(error) = run() {
        eprintln!("qmk-layer-indicator-shell: {error}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let url = env::var("QMK_LAYER_INDICATOR_URL").unwrap_or_else(|_| DEFAULT_URL.to_owned());
    let mut terminal = OpenOptions::new().write(true).open("/dev/tty")?;
    let parent = match env::args().nth(1) {
        Some(pid) => pid
            .parse::<libc::pid_t>()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?,
        None => unsafe { libc::getppid() },
    };

    // A shell may enable `stty tostop`; allow this background listener to
    // write cursor escapes to its own controlling terminal in that case.
    if unsafe { libc::signal(libc::SIGTTOU, libc::SIG_IGN) } == libc::SIG_ERR {
        return Err(io::Error::last_os_error());
    }

    while unsafe { libc::getppid() } == parent {
        if let Ok((mut socket, _)) = connect(url.as_str()) {
            if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
                stream.set_read_timeout(Some(RETRY_DELAY))?;
            }
            read_layers(&mut socket, &mut terminal, parent)?;
        }
        thread::sleep(RETRY_DELAY);
    }
    Ok(())
}

fn read_layers(
    socket: &mut WebSocket<MaybeTlsStream<std::net::TcpStream>>,
    terminal: &mut File,
    parent: libc::pid_t,
) -> io::Result<()> {
    while unsafe { libc::getppid() } == parent {
        match socket.read() {
            Ok(Message::Text(text)) => {
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(&text)
                    && let Some(layer) = event.get("layer").and_then(serde_json::Value::as_u64)
                {
                    let style = match layer {
                        1 => 2, // steady block
                        2 => 4, // steady underline
                        _ => 6, // steady bar
                    };
                    write!(terminal, "\x1b[{style} q")?;
                    terminal.flush()?;
                }
            }
            Ok(Message::Ping(payload)) => {
                if socket.send(Message::Pong(payload)).is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) => break,
            Err(tungstenite::Error::Io(ref error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) => {}
            Err(_) => break,
            _ => {}
        }
    }
    Ok(())
}
