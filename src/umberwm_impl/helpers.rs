use crate::error::{Error, LogError, Result};
use crate::model::*;
use std::collections::HashMap;
use std::process::Command;
use std::thread;
use xcb::randr;
use xcb::xproto;

pub fn run_command(list: Option<&Vec<String>>) {
    if let Some(args) = list {
        if let Some(head) = args.first() {
            let tail: Vec<String> = args[1..].to_vec();
            let headc = head.clone();
            thread::spawn(move || {
                let _ = Command::new(headc).args(tail).status();
            });
        }
    }
}

pub fn get_displays_geometries(conn: &xcb::Connection) -> Result<Vec<Geometry>> {
    let setup = conn.get_setup();
    let screen = setup.roots().next().unwrap();
    let window_dummy = conn.generate_id();
    xcb::create_window(
        &conn,
        0,
        window_dummy,
        screen.root(),
        0,
        0,
        1,
        1,
        0,
        0,
        0,
        &[],
    );
    let screen_res_cookie = randr::get_screen_resources(&conn, window_dummy);
    let screen_res_reply = screen_res_cookie.get_reply().unwrap();
    let crtcs = screen_res_reply.crtcs();

    let mut crtc_cookies = Vec::with_capacity(crtcs.len());
    for crtc in crtcs {
        crtc_cookies.push(randr::get_crtc_info(&conn, *crtc, 0));
    }

    let mut result = Vec::new();
    for (i, crtc_cookie) in crtc_cookies.into_iter().enumerate() {
        if let Ok(reply) = crtc_cookie.get_reply() {
            if reply.width() > 0 {
                if i != 0 {
                    println!();
                }
                println!("CRTC[{}] INFO:", i);
                println!(" x-off\t: {}", reply.x());
                println!(" y-off\t: {}", reply.y());
                println!(" width\t: {}", reply.width());
                println!(" height\t: {}", reply.height());
                result.push(Geometry(
                    reply.x() as u32,
                    reply.y() as u32,
                    reply.width() as u32,
                    reply.height() as u32,
                ))
            }
        }
    }
    Ok(result)
}

pub fn get_str_property(conn: &xcb::Connection, window: u32, name: &str) -> Option<String> {
    let _net_wm_window_type = xcb::intern_atom(conn, false, name)
        .get_reply()
        .unwrap()
        .atom();
    let cookie = xcb::get_property(
        conn,
        false,
        window,
        _net_wm_window_type,
        xcb::ATOM_ANY,
        0,
        1024,
    );
    if let Ok(reply) = cookie.get_reply() {
        Some(std::str::from_utf8(reply.value()).unwrap().to_string())
    } else {
        None
    }
}

pub fn get_wm_normal_hints(conn: &xcb::Connection, id: u32) -> Result<Option<NormalHints>> {
    let window: xproto::Window = id;
    let ident = xcb::intern_atom(conn, true, "WM_NORMAL_HINTS")
        .get_reply()?
        .atom();
    let reply =
        xproto::get_property(conn, false, window, ident, xproto::ATOM_ANY, 0, 1024).get_reply()?;
    if reply.value_len() >= 15 {
        let value = reply.value();
        let hints = NormalHints {
            min_width: value[5],
            min_height: value[6],
            max_width: value[7],
            max_height: value[8],
            width_inc: value[9],
            height_inc: value[10],
            min_aspect: (value[11], value[12]),
            max_aspect: (value[13], value[14]),
        };
        Ok(Some(hints))
    } else {
        Ok(None)
    }
}

pub fn get_atom_property(conn: &xcb::Connection, id: u32, name: &str) -> Result<u32> {
    let window: xproto::Window = id;
    let ident = xcb::intern_atom(conn, true, name).get_reply()?.atom();
    let reply =
        xproto::get_property(conn, false, window, ident, xproto::ATOM_ATOM, 0, 1024).get_reply()?;
    if reply.value_len() == 0 {
        Ok(42)
    } else {
        Ok(reply.value()[0])
    }
}

pub fn is_firefox_drag_n_drop_initialization_window(
    conn: &xcb::Connection,
    id: u32,
    wm_class: &[&str],
) -> Result<bool> {
    if wm_class.len() >= 2 && wm_class[0] == "firefox" && wm_class[1] == "firefox" {
        if let Some(hints) = get_wm_normal_hints(conn, id)? {
            return Ok(hints.max_height == 0 && hints.max_width == 0);
        }
    }
    Ok(false)
}

pub fn window_types_from_list(conn: &xcb::Connection, types_names: &[String]) -> Vec<xcb::Atom> {
    types_names
        .iter()
        .map(|x| {
            let name = format!("_NET_WM_WINDOW_TYPE_{}", x.to_uppercase());
            let res = xcb::intern_atom(&conn, true, name.as_str())
                .get_reply()
                .map(|x| x.atom());
            res.log()
        })
        .flatten()
        .collect()
}

fn unmap_workspace_windows(
    conn: &xcb::Connection,
    windows: &mut Vec<Window>,
    focus: usize,
    move_window: bool,
    same_display: bool,
) -> Option<Window> {
    let mut window_to_move = None;
    for (i, window) in windows.iter().enumerate() {
        if move_window && i == focus {
            window_to_move = Some(*window);
        } else if same_display {
            xcb::unmap_window(conn, *window);
        }
    }
    window_to_move
}

pub fn change_workspace(
    conn: &xcb::Connection,
    workspaces: &mut HashMap<WorkspaceName, Workspace>,
    previous_workspace: WorkspaceName,
    next_workspace: WorkspaceName,
    move_window: bool,
    same_display: bool,
) -> Result<WorkspaceName> {
    let workspace = workspaces
        .get_mut(&previous_workspace)
        .ok_or(Error::WorkspaceNotFound)?;
    let window_to_move = unmap_workspace_windows(
        conn,
        &mut workspace.windows,
        workspace.focus,
        move_window,
        same_display,
    );
    if let Some(w) = window_to_move {
        workspace.windows.retain(|x| *x != w);
        if !workspace.windows.is_empty() {
            workspace.focus = workspace.windows.len() - 1;
        } else {
            workspace.focus = 0;
        }
    };
    let workspace = workspaces
        .get_mut(&next_workspace)
        .ok_or(Error::WorkspaceNotFound)?;
    for window in &workspace.windows {
        xcb::map_window(conn, *window);
    }
    if let Some(w) = window_to_move {
        workspace.windows.push(w);
        workspace.focus = workspace.windows.len() - 1;
    }
    Ok(next_workspace)
}

/// Returns the display border for the display requested, or for the last display if the index
/// is out of range.
pub fn get_display_border(display_borders: &[DisplayBorder], display: usize) -> DisplayBorder {
    let i = std::cmp::min(display_borders.len() - 1, display);
    display_borders[i].clone()
}
