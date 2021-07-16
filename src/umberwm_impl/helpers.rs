use crate::error::{LogError, Result};
use crate::model::*;
use xcb::randr;
use xcb::xproto;

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
