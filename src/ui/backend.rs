use super::layer_indicator::LayerIndicatorView;

const INDICATOR_SIZE: i32 = 104;
#[cfg(target_os = "linux")]
const BOTTOM_MARGIN: i32 = 50;

/// Extra clearance so the indicator sits above the macOS Dock.
#[cfg(target_os = "macos")]
const MACOS_BOTTOM_MARGIN: i32 = 120;

pub fn configure_backend() -> Result<(), slint::PlatformError> {
    #[cfg(target_os = "linux")]
    {
        use slint::winit_030::{
            SlintEvent,
            winit::{
                event_loop::EventLoop,
                platform::x11::{EventLoopBuilderExtX11, WindowAttributesExtX11},
            },
        };

        let initial_position = initial_bottom_center_position();
        let mut event_loop_builder = EventLoop::<SlintEvent>::with_user_event();
        event_loop_builder.with_x11();

        return slint::BackendSelector::new()
            .backend_name("winit".into())
            .renderer_name("software".into())
            .with_winit_event_loop_builder(event_loop_builder)
            .with_winit_window_attributes_hook(move |attributes| {
                let attributes = attributes.with_override_redirect(true);
                match initial_position {
                    Some(position) => attributes.with_position(position),
                    None => attributes,
                }
            })
            .select();
    }

    #[cfg(target_os = "macos")]
    {
        use slint::winit_030::winit::platform::macos::WindowAttributesExtMacOS;

        return slint::BackendSelector::new()
            .backend_name("winit".into())
            .renderer_name("femtovg".into())
            .with_winit_window_attributes_hook(|attributes| {
                attributes
                    .with_transparent(true)
                    .with_titlebar_hidden(true)
                    .with_title_hidden(true)
                    .with_fullsize_content_view(true)
                    .with_has_shadow(false)
            })
            .select();
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select()
}

pub(super) fn set_mouse_passthrough(window: &LayerIndicatorView, enabled: bool) {
    #[cfg(target_os = "linux")]
    {
        use slint::ComponentHandle;
        use slint::winit_030::WinitWindowAccessor;

        let _ = window.window().with_winit_window(|window| {
            if let Err(error) = window.set_cursor_hittest(!enabled) {
                eprintln!("Could not update layer-indicator mouse hit testing: {error}");
            }
        });
    }
}

/// Places the indicator at the bottom center of the primary monitor.
///
/// On Linux the initial position is applied through the X11 window attributes,
/// but macOS only exposes monitor geometry once the native window exists, so the
/// placement is (re)applied whenever the indicator updates.
pub(super) fn position_bottom_center(window: &LayerIndicatorView) {
    #[cfg(target_os = "macos")]
    {
        use slint::ComponentHandle;
        use slint::winit_030::WinitWindowAccessor;
        use slint::winit_030::winit::dpi::PhysicalPosition;

        let _ = window.window().with_winit_window(|winit_window| {
            let Some(monitor) = winit_window
                .current_monitor()
                .or_else(|| winit_window.primary_monitor())
            else {
                return;
            };

            let scale = winit_window.scale_factor();
            let indicator = (INDICATOR_SIZE as f64 * scale).round() as i32;
            let margin = (MACOS_BOTTOM_MARGIN as f64 * scale).round() as i32;
            let monitor_position = monitor.position();
            let monitor_size = monitor.size();

            winit_window.set_outer_position(PhysicalPosition::new(
                monitor_position.x + (monitor_size.width as i32 - indicator) / 2,
                monitor_position.y + monitor_size.height as i32 - indicator - margin,
            ));
        });
    }

    #[cfg(not(target_os = "macos"))]
    let _ = window;
}

#[cfg(target_os = "linux")]
fn initial_bottom_center_position() -> Option<slint::winit_030::winit::dpi::PhysicalPosition<i32>> {
    use x11rb::{connection::Connection, protocol::randr::ConnectionExt as _};

    let (connection, screen_number) = x11rb::connect(None).ok()?;
    let screen = connection.setup().roots.get(screen_number)?;
    let monitors = connection
        .randr_get_monitors(screen.root, true)
        .ok()?
        .reply()
        .ok()?
        .monitors;
    let monitor = monitors
        .iter()
        .find(|monitor| monitor.primary)
        .or_else(|| monitors.first())?;

    Some(slint::winit_030::winit::dpi::PhysicalPosition::new(
        monitor.x as i32 + (monitor.width as i32 - INDICATOR_SIZE) / 2,
        monitor.y as i32 + monitor.height as i32 - INDICATOR_SIZE - BOTTOM_MARGIN,
    ))
}
