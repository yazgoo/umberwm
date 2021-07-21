use crate::model::*;

pub fn resize_bsp(
    conn: &xcb::Connection,
    border_width: u32,
    non_float_windows: Vec<u32>,
    geos: Vec<Geometry>,
    gap: u32,
) {
    for (window, geo) in non_float_windows.iter().zip(geos.iter()) {
        xcb::configure_window(
            conn,
            *window,
            &[
                (xcb::CONFIG_WINDOW_X as u16, geo.0 + gap),
                (xcb::CONFIG_WINDOW_Y as u16, geo.1 + gap),
                (
                    xcb::CONFIG_WINDOW_WIDTH as u16,
                    geo.2.saturating_sub(2 * border_width + 2 * gap),
                ),
                (
                    xcb::CONFIG_WINDOW_HEIGHT as u16,
                    geo.3.saturating_sub(2 * border_width + 2 * gap),
                ),
                (xcb::CONFIG_WINDOW_BORDER_WIDTH as u16, border_width),
            ],
        );
    }
}

pub fn resize_monocle(
    conn: &xcb::Connection,
    border_width: u32,
    workspace: &Workspace,
    geos: Vec<Geometry>,
    gap: u32,
) {
    if let Some(window) = workspace.windows.get(workspace.focus) {
        xcb::configure_window(
            conn,
            *window,
            &[
                (xcb::CONFIG_WINDOW_X as u16, geos[0].0 + gap),
                (xcb::CONFIG_WINDOW_Y as u16, geos[0].1 + gap),
                (
                    xcb::CONFIG_WINDOW_WIDTH as u16,
                    geos[0].2.saturating_sub(2 * border_width + 2 * gap),
                ),
                (
                    xcb::CONFIG_WINDOW_HEIGHT as u16,
                    geos[0].3.saturating_sub(2 * border_width + 2 * gap),
                ),
                (xcb::CONFIG_WINDOW_BORDER_WIDTH as u16, border_width),
                (xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE),
            ],
        );
    }
}
