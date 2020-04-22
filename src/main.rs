extern crate x11_wrapper;

use std::collections::HashMap;

use x11_wrapper::core::display::X11Display;
use x11_wrapper::core::XlibHandle;
use x11::xlib::Window;

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
    workspaceNames: Vec<WorkspaceName>,
    custom_actions: HashMap<Key, CustomAction>,
    wm_actions: HashMap<String, Actions>,
    float_classes: Vec<String>,
    auto_float_types: Vec<String>,
}

struct Yazgoo {
    dpy: X11Display,
    current_workspace: WorkspaceName,
    float_windows: Vec<Window>,
    workspaces: HashMap<WorkspaceName, Workspace>,
}

fn main() -> Result<(), ()> {
    let xlib_handle = XlibHandle::initialize_xlib().unwrap();
    let dpy = xlib_handle.create_display()?;
    
    return Ok(())
}
