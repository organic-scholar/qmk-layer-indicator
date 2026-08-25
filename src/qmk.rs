use std::{
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use hidapi::{HidApi, HidDevice};

const CONSOLE_HID_USAGE_PAGE: u16 = 0xFF31;
const CONSOLE_HID_USAGE: u16 = 0x74;
const CONSOLE_READ_TIMEOUT_MS: i32 = 100;
const CONSOLE_RECONNECT_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, PartialEq, Eq)]
pub enum QmkEvent {
    DeviceConnected { name: String },
    DeviceDisconnected { reason: String },
    LayerChanged(u8),
}

struct QmkConsole {
    device: HidDevice,
    name: String,
}

impl QmkConsole {
    fn read_timeout(&self, report: &mut [u8], timeout_ms: i32) -> Result<usize, String> {
        self.device
            .read_timeout(report, timeout_ms)
            .map_err(|error| error.to_string())
    }
}

pub struct ConsoleReader {
    stop_sender: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for ConsoleReader {
    fn drop(&mut self) {
        let _ = self.stop_sender.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn start_console_reader(on_event: impl Fn(QmkEvent) + Send + 'static) -> ConsoleReader {
    let (stop_sender, stop_receiver) = mpsc::channel();
    let thread = thread::Builder::new()
        .name("qmk-console-reader".into())
        .spawn(move || run_console_reader(stop_receiver, on_event))
        .expect("failed to start QMK console reader thread");

    ConsoleReader {
        stop_sender,
        thread: Some(thread),
    }
}

fn connect_console(api: &mut HidApi) -> Result<Option<QmkConsole>, String> {
    api.refresh_devices().map_err(|error| error.to_string())?;

    for device in api.device_list() {
        if device.usage_page() == CONSOLE_HID_USAGE_PAGE && device.usage() == CONSOLE_HID_USAGE {
            let device_handle = device.open_device(api).map_err(|error| {
                format!(
                    "found {:04x}:{:04x}, but opening its console interface failed: {error}",
                    device.vendor_id(),
                    device.product_id(),
                )
            })?;

            return Ok(Some(QmkConsole {
                device: device_handle,
                name: device.product_string().unwrap_or("QMK keyboard").into(),
            }));
        }
    }

    Ok(None)
}

fn run_console_reader(stop_receiver: Receiver<()>, on_event: impl Fn(QmkEvent)) {
    loop {
        let mut api = match HidApi::new() {
            Ok(api) => api,
            Err(error) => {
                eprintln!("Could not initialize HID API: {error}");
                if wait_for_stop(&stop_receiver) {
                    return;
                }
                continue;
            }
        };
        let console = match connect_console(&mut api) {
            Ok(Some(console)) => {
                on_event(QmkEvent::DeviceConnected {
                    name: console.name.clone(),
                });
                console
            }
            Ok(None) => {
                if wait_for_stop(&stop_receiver) {
                    return;
                }
                continue;
            }
            Err(error) => {
                eprintln!("Could not connect to QMK console HID: {error}");
                if wait_for_stop(&stop_receiver) {
                    return;
                }
                continue;
            }
        };

        if !read_console(console, &stop_receiver, &on_event) {
            return;
        }
        if wait_for_stop(&stop_receiver) {
            return;
        }
    }
}

fn read_console(
    console: QmkConsole,
    stop_receiver: &Receiver<()>,
    on_event: &impl Fn(QmkEvent),
) -> bool {
    let mut report = [0_u8; 64];
    let mut parser = ConsoleParser::default();

    loop {
        if stop_receiver.try_recv().is_ok() {
            return false;
        }

        match console.read_timeout(&mut report, CONSOLE_READ_TIMEOUT_MS) {
            Ok(0) => {}
            Ok(length) => {
                let message = String::from_utf8_lossy(&report[..length]);
                let message = message.trim_end_matches('\0');
                #[cfg(debug_assertions)]
                eprint!("{message}");
                for event in parser.process(message) {
                    on_event(event);
                }
            }
            Err(error) => {
                eprint!("device disconnected: {error}");
                on_event(QmkEvent::DeviceDisconnected { reason: error });
                return true;
            }
        }
    }
}

fn wait_for_stop(stop_receiver: &Receiver<()>) -> bool {
    matches!(
        stop_receiver.recv_timeout(CONSOLE_RECONNECT_INTERVAL),
        Ok(()) | Err(RecvTimeoutError::Disconnected)
    )
}

#[derive(Default)]
struct ConsoleParser {
    line_buffer: String,
}

impl ConsoleParser {
    fn process(&mut self, message: &str) -> Vec<QmkEvent> {
        self.line_buffer.push_str(message);
        let mut events = Vec::new();

        while let Some(line_end) = self.line_buffer.find('\n') {
            let line: String = self.line_buffer.drain(..=line_end).collect();
            if let Some(layer) = line
                .trim()
                .strip_prefix("LAYER:")
                .and_then(|value| value.trim().parse::<u8>().ok())
            {
                events.push(QmkEvent::LayerChanged(layer));
            }
        }

        if self.line_buffer.len() > 256 {
            self.line_buffer.clear();
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::{ConsoleParser, QmkEvent};

    #[test]
    fn parses_a_layer_line() {
        let mut parser = ConsoleParser::default();
        assert_eq!(parser.process("LAYER:2\n"), vec![QmkEvent::LayerChanged(2)]);
    }

    #[test]
    fn buffers_a_split_layer_line() {
        let mut parser = ConsoleParser::default();
        assert!(parser.process("LAYER:").is_empty());
        assert_eq!(parser.process("12\n"), vec![QmkEvent::LayerChanged(12)]);
    }

    #[test]
    fn ignores_malformed_console_lines() {
        let mut parser = ConsoleParser::default();
        assert!(parser.process("debug\nLAYER:not-a-number\n").is_empty());
    }

    #[test]
    fn parses_multiple_layer_lines_in_one_report() {
        let mut parser = ConsoleParser::default();
        assert_eq!(
            parser.process("LAYER:1\nLAYER:3\n"),
            vec![QmkEvent::LayerChanged(1), QmkEvent::LayerChanged(3)],
        );
    }
}
