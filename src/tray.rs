use std::sync::Arc;

#[derive(Debug)]
pub enum TrayCommand {
    EditConfiguration,
    Quit,
}

pub type TrayCallback = Arc<dyn Fn(TrayCommand) + Send + Sync + 'static>;

#[cfg(target_os = "linux")]
use ksni::{Tray, blocking::TrayMethods, menu::StandardItem};

#[cfg(target_os = "linux")]
pub struct TrayHandle {
    _handle: ksni::blocking::Handle<LinuxTray>,
}

#[cfg(target_os = "linux")]
struct LinuxTray {
    command_callback: TrayCallback,
    icon: ksni::Icon,
}

#[cfg(target_os = "linux")]
impl Tray for LinuxTray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }

    fn title(&self) -> String {
        "QMK Layer Indicator".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![self.icon.clone()]
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Edit configuration…".into(),
                activate: Box::new(|tray: &mut LinuxTray| {
                    (tray.command_callback)(TrayCommand::EditConfiguration);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|tray: &mut LinuxTray| {
                    (tray.command_callback)(TrayCommand::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

#[cfg(target_os = "linux")]
pub fn create(command_callback: TrayCallback) -> Result<TrayHandle, Box<dyn std::error::Error>> {
    let tray = LinuxTray {
        command_callback,
        icon: linux_icon(),
    };
    let handle = tray.spawn()?;

    Ok(TrayHandle { _handle: handle })
}

#[cfg(target_os = "linux")]
fn linux_icon() -> ksni::Icon {
    let image = image::load_from_memory(include_bytes!("../assets/icon-tray.png"))
        .expect("icon-tray.png must be a valid PNG")
        .into_rgba8();
    let (width, height) = image.dimensions();
    let mut data = image.into_raw();
    for pixel in data.chunks_exact_mut(4) {
        pixel.rotate_right(1);
    }

    ksni::Icon {
        width: width as i32,
        height: height as i32,
        data,
    }
}

#[cfg(target_os = "macos")]
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};

#[cfg(target_os = "macos")]
const QUIT_MENU_ID: &str = "quit";

#[cfg(target_os = "macos")]
const EDIT_CONFIGURATION_MENU_ID: &str = "edit-configuration";

#[cfg(target_os = "macos")]
pub struct TrayHandle {
    _icon: TrayIcon,
}

#[cfg(target_os = "macos")]
pub fn create(command_callback: TrayCallback) -> Result<TrayHandle, Box<dyn std::error::Error>> {
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let command = if event.id == EDIT_CONFIGURATION_MENU_ID {
            Some(TrayCommand::EditConfiguration)
        } else if event.id == QUIT_MENU_ID {
            Some(TrayCommand::Quit)
        } else {
            None
        };

        if let Some(command) = command {
            command_callback(command);
        }
    }));

    let menu = Menu::new();
    let edit_configuration_item = MenuItem::with_id(
        EDIT_CONFIGURATION_MENU_ID,
        "Edit configuration…",
        true,
        None,
    );
    let quit_item = MenuItem::with_id(QUIT_MENU_ID, "Quit", true, None);
    menu.append(&edit_configuration_item)?;
    menu.append(&quit_item)?;

    Ok(TrayHandle {
        _icon: TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("QMK Layer Indicator")
            .with_icon(macos_icon()?)
            .with_icon_as_template(true)
            .build()?,
    })
}

#[cfg(target_os = "macos")]
fn macos_icon() -> Result<Icon, tray_icon::BadIcon> {
    let image = image::load_from_memory(include_bytes!("../assets/icon-tray.png"))
        .expect("icon-tray.png must be a valid PNG")
        .into_rgba8();
    let (width, height) = image.dimensions();

    Icon::from_rgba(image.into_raw(), width, height)
}
