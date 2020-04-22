extern crate xcb;

use std::collections::HashMap;
use xcb::Window;

enum Actions {
    SwitchWindow, CloseWindow, ChangeLayout,
}

enum Layout {
    BSPV, Monocle, BSPH
}

type Key = char;

type WorkspaceName = Key;

type CustomAction = Box<dyn Fn() -> ()>;

type Color = String;

struct Border {
    width: u8,
    focus_color: Color,
    normal_color: Color,
}

struct Workspace {
    layout: Layout,
    windows: Vec<Window>,
    focus: u32,
}

struct Conf {
    meta: String,
    border: Border,
    workspaces_names: Vec<WorkspaceName>,
    custom_actions: HashMap<Key, CustomAction>,
    wm_actions: HashMap<String, Actions>,
    float_classes: Vec<String>,
    auto_float_types: Vec<String>,
}

struct Yazgoo {
    current_workspace: WorkspaceName,
    float_windows: Vec<Window>,
    workspaces: HashMap<WorkspaceName, Workspace>,
}

fn main() -> Result<(), ()> {
    let conf = Conf {
        meta: String::from("mod1"),
        border: Border {
            width: 2,
            focus_color: String::from("#906cff"),
            normal_color: String::from("black"),
        },
        workspaces_names: vec![ 'a', 'u', 'i', 'o', 'p' ],
        custom_actions: HashMap::new(),
        wm_actions: HashMap::new(),
        float_classes: vec![],
        auto_float_types: vec![],
    };
    let (conn, screen_num) = xcb::Connection::connect(None).unwrap();
    let setup = conn.get_setup();
    let screen = setup.roots().nth(0).unwrap();
    xcb::grab_key(&conn, false, screen.root(), xcb::MOD_MASK_ANY as u16, xcb::GRAB_ANY as u8, xcb::GRAB_MODE_ASYNC as u8, xcb::GRAB_MODE_ASYNC as u8);
    conn.flush();
    loop {
        println!("in loop");
        let event = conn.wait_for_event();
        println!("event");
        conn.flush();
    }

    Ok(())
}
