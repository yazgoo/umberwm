extern crate regex;
extern crate xcb;

use miniserde::{json, Deserialize, Serialize};
use regex::Regex;
use std::cmp::max;
use std::collections::HashMap;
use std::error::Error;
use std::fs::remove_file;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::process::Command;
use xcb::randr;
use xcb::xproto;
use xcb::ModMask;
pub use xcb::{
    MOD_MASK_1, MOD_MASK_2, MOD_MASK_3, MOD_MASK_4, MOD_MASK_5, MOD_MASK_CONTROL, MOD_MASK_SHIFT,
};

type XmodmapPke = HashMap<u8, Vec<String>>;

#[derive(Eq, PartialEq, Hash)]
pub struct Keybind {
    pub key: String,
    pub mod_mask: ModMask,
}

impl Keybind {
    pub fn new<S, U>(key: S, mod_mask: U) -> Self
    where
        S: Into<String>,
        U: Into<ModMask>,
    {
        Keybind {
            key: key.into(),
            mod_mask: mod_mask.into(),
        }
    }
}

pub enum Actions {
    SwitchWindow,
    SerializeAndQuit,
    CloseWindow,
    ChangeLayout,
    ToggleGap,
}

pub enum Meta {
    Mod1,
    Mod4,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum Layout {
    Bspv,
    Monocle,
    Bsph,
}

type Window = u32;

pub type Key = String;

type WorkspaceName = Key;

pub type CustomAction = Box<dyn Fn()>;

type Color = u32;

#[derive(Clone)]
pub struct Geometry(u32, u32, u32, u32);

pub struct WindowBorder {
    pub width: u32,
    pub focus_color: Color,
    pub normal_color: Color,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Workspace {
    layout: Layout,
    windows: Vec<Window>,
    focus: usize,
}

#[derive(Clone, Debug)]
pub struct DisplayBorder {
    pub left: u32,
    pub right: u32,
    pub bottom: u32,
    pub top: u32,
    pub gap: u32,
}

type DisplayId = usize;

pub type OnChangeWorkspace = Option<Box<dyn Fn(WorkspaceName, DisplayId)>>;

pub struct EventsCallbacks {
    pub on_change_workspace: OnChangeWorkspace,
}

pub struct Conf {
    pub meta: ModMask,
    pub border: WindowBorder,
    pub display_borders: Vec<DisplayBorder>,
    pub workspaces_names: Vec<Vec<WorkspaceName>>,
    pub custom_actions: HashMap<Keybind, CustomAction>,
    pub wm_actions: HashMap<Keybind, Actions>,
    pub ignore_classes: Vec<String>,
    pub float_classes: Vec<String>,
    pub events_callbacks: EventsCallbacks,
    pub with_gap: bool,
}

#[derive(Clone)]
struct MouseMoveStart {
    root_x: i16,
    root_y: i16,
    child: Window,
    detail: u8,
}

const UMBERWM_STATE: &str = ".umberwm_state";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableState {
    float_windows: Vec<Window>,
    workspaces: HashMap<WorkspaceName, Workspace>,
    current_workspace: WorkspaceName,
}

pub struct UmberWm {
    conf: Conf,
    current_workspace: WorkspaceName,
    float_windows: Vec<Window>,
    workspaces: HashMap<WorkspaceName, Workspace>,
    conn: xcb::Connection,
    mouse_move_start: Option<MouseMoveStart>,
    button_press_geometry: Option<Geometry>,
    xmodmap_pke: XmodmapPke,
    displays_geometries: Vec<Geometry>,
    randr_base: u8,
    previous_display: DisplayId,
}

fn xmodmap_pke() -> Result<XmodmapPke, Box<dyn Error>> {
    let output = Command::new("xmodmap").arg("-pke").output()?;
    let pattern = Regex::new(r"(\d+) = (.*)")?;
    let lines = String::from_utf8(output.stdout)?
        .lines()
        .filter_map(|line| pattern.captures(line))
        .map(|cap| {
            (
                cap[1].parse().unwrap(),
                cap[2].split(' ').map(|s| s.to_string()).collect(),
            )
        })
        .collect::<HashMap<u8, Vec<String>>>();
    Ok(lines)
}

fn keycode_to_key(xmodmap_pke: &XmodmapPke, keycode: u8) -> Option<Key> {
    if let Some(x) = xmodmap_pke.get(&keycode) {
        if !x.is_empty() {
            return Some(x[0].to_string());
        }
    }
    None
}

fn key_to_keycode(xmodmap_pke: &XmodmapPke, key: &str) -> Option<u8> {
    for (keycode, symbols) in xmodmap_pke.iter() {
        if symbols.contains(&key.to_string()) {
            return Some(*keycode);
        }
    }
    None
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

fn change_workspace(
    conn: &xcb::Connection,
    workspaces: &mut HashMap<WorkspaceName, Workspace>,
    previous_workspace: WorkspaceName,
    next_workspace: WorkspaceName,
    move_window: bool,
    same_display: bool,
) -> Result<WorkspaceName, Box<dyn Error>> {
    let workspace = workspaces
        .get_mut(&previous_workspace)
        .ok_or("workspace not found")?;
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
        .ok_or("workspace not found")?;
    for window in &workspace.windows {
        xcb::map_window(conn, *window);
    }
    if let Some(w) = window_to_move {
        workspace.windows.push(w);
        workspace.focus = workspace.windows.len() - 1;
    }
    Ok(next_workspace)
}

fn geometries_bsp(
    i: usize,
    window_count: usize,
    left: u32,
    top: u32,
    width: u32,
    height: u32,
    vertical: usize,
) -> Vec<Geometry> {
    if window_count == 0 {
        vec![]
    } else if window_count == 1 {
        vec![Geometry(left, top, width, height)]
    } else if i % 2 == vertical {
        let mut res = vec![Geometry(left, top, width, height / 2)];
        res.append(&mut geometries_bsp(
            i + 1,
            window_count - 1,
            left,
            top + height / 2,
            width,
            height / 2,
            vertical,
        ));
        res
    } else {
        let mut res = vec![Geometry(left, top, width / 2, height)];
        res.append(&mut geometries_bsp(
            i + 1,
            window_count - 1,
            left + width / 2,
            top,
            width / 2,
            height,
            vertical,
        ));
        res
    }
}

fn window_types_from_list(conn: &xcb::Connection, types_names: &[String]) -> Vec<xcb::Atom> {
    types_names
        .iter()
        .map(|x| {
            let name = format!("_NET_WM_WINDOW_TYPE_{}", x.to_uppercase());
            let res = xcb::intern_atom(&conn, true, name.as_str())
                .get_reply()
                .map(|x| x.atom());
            res.ok()
        })
        .flatten()
        .collect()
}

impl UmberWm {
    pub fn get_displays_geometries(&mut self) -> Result<Vec<Geometry>, Box<dyn Error>> {
        let conn = &self.conn;
        let setup = self.conn.get_setup();
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

    /// Returns the display border for the display requested, or for the last display if the index
    /// is out of range.
    fn get_display_border(&mut self, display: usize) -> DisplayBorder {
        let i = std::cmp::min(self.conf.display_borders.len() - 1, display);
        self.conf.display_borders[i].clone()
    }

    fn resize_workspace_windows(&mut self, workspace: &Workspace, mut display: usize) {
        let mut non_float_windows = workspace.windows.clone();
        non_float_windows.retain(|w| !self.float_windows.contains(&w));
        let count = non_float_windows.len();
        if count == 0 || self.displays_geometries.is_empty() {
            return;
        }
        if display >= self.displays_geometries.len() {
            display = self.displays_geometries.len() - 1;
        }
        let display_border = self.get_display_border(display);
        let display_geometry = self.displays_geometries.get(display).unwrap();
        let width = display_geometry.2 as u32 - display_border.right - display_border.left;
        let height = display_geometry.3 as u32 - display_border.top - display_border.bottom;
        let left = display_geometry.0 + display_border.left;
        let top = display_geometry.1 + display_border.top;
        let gap = if self.conf.with_gap {
            display_border.gap
        } else {
            0
        };
        let geos = match workspace.layout {
            Layout::Bspv => geometries_bsp(0, count, left, top, width, height, 1),
            Layout::Bsph => geometries_bsp(0, count, left, top, width, height, 0),
            Layout::Monocle => geometries_bsp(0, 1, left, top, width, height, 1),
        };
        match workspace.layout {
            Layout::Bspv | Layout::Bsph => self.resize_bsp(non_float_windows, geos, gap),
            Layout::Monocle => self.resize_monocle(workspace, geos, gap),
        }
        for (i, window) in workspace.windows.iter().enumerate() {
            self.focus_unfocus(window, i == workspace.focus).ok();
        }
    }

    fn resize_bsp(&self, non_float_windows: Vec<u32>, geos: Vec<Geometry>, gap: u32) {
        for (window, geo) in non_float_windows.iter().zip(geos.iter()) {
            xcb::configure_window(
                &self.conn,
                *window,
                &[
                    (xcb::CONFIG_WINDOW_X as u16, geo.0 + gap),
                    (xcb::CONFIG_WINDOW_Y as u16, geo.1 + gap),
                    (
                        xcb::CONFIG_WINDOW_WIDTH as u16,
                        geo.2.saturating_sub(2 * self.conf.border.width + 2 * gap),
                    ),
                    (
                        xcb::CONFIG_WINDOW_HEIGHT as u16,
                        geo.3.saturating_sub(2 * self.conf.border.width + 2 * gap),
                    ),
                    (
                        xcb::CONFIG_WINDOW_BORDER_WIDTH as u16,
                        self.conf.border.width,
                    ),
                ],
            );
        }
    }

    fn resize_monocle(&self, workspace: &Workspace, geos: Vec<Geometry>, gap: u32) {
        if let Some(window) = workspace.windows.get(workspace.focus) {
            xcb::configure_window(
                &self.conn,
                *window,
                &[
                    (xcb::CONFIG_WINDOW_X as u16, geos[0].0 + gap),
                    (xcb::CONFIG_WINDOW_Y as u16, geos[0].1 + gap),
                    (
                        xcb::CONFIG_WINDOW_WIDTH as u16,
                        geos[0]
                            .2
                            .saturating_sub(2 * self.conf.border.width + 2 * gap),
                    ),
                    (
                        xcb::CONFIG_WINDOW_HEIGHT as u16,
                        geos[0]
                            .3
                            .saturating_sub(2 * self.conf.border.width + 2 * gap),
                    ),
                    (
                        xcb::CONFIG_WINDOW_BORDER_WIDTH as u16,
                        self.conf.border.width,
                    ),
                    (xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE),
                ],
            );
        }
    }

    fn init(&mut self) {
        self.displays_geometries = self.get_displays_geometries().unwrap();
        let screen = self.conn.get_setup().roots().next().unwrap();
        self.randr_base = self
            .conn
            .get_extension_data(&mut randr::id())
            .unwrap()
            .first_event();
        randr::select_input(
            &self.conn,
            screen.root(),
            randr::NOTIFY_MASK_CRTC_CHANGE as u16,
        )
        .request_check()
        .ok();
        self.grab_custom_action_keys(&screen);
        self.grab_wm_action_keys(&screen);
        self.grab_workspace_keys(&screen);
        for button in &[1_u8, 3_u8] {
            xcb::grab_button(
                &self.conn,
                false,
                screen.root(),
                (xcb::EVENT_MASK_BUTTON_PRESS
                    | xcb::EVENT_MASK_BUTTON_RELEASE
                    | xcb::EVENT_MASK_POINTER_MOTION) as u16,
                xcb::GRAB_MODE_ASYNC as u8,
                xcb::GRAB_MODE_ASYNC as u8,
                xcb::NONE,
                xcb::NONE,
                *button,
                self.conf.meta as u16,
            );
        }
        xcb::change_window_attributes(
            &self.conn,
            screen.root(),
            &[(
                xcb::CW_EVENT_MASK,
                xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY as u32,
            )],
        );
        self.conn.flush();
    }

    fn grab_custom_action_keys(&self, screen: &xcb::Screen) {
        for keybind in self.conf.custom_actions.keys() {
            key_to_keycode(&self.xmodmap_pke, &keybind.key).map(|keycode| {
                xcb::grab_key(
                    &self.conn,
                    false,
                    screen.root(),
                    keybind.mod_mask as u16,
                    keycode,
                    xcb::GRAB_MODE_ASYNC as u8,
                    xcb::GRAB_MODE_ASYNC as u8,
                )
            });
        }
    }

    fn grab_wm_action_keys(&self, screen: &xcb::Screen) {
        for keybind in self.conf.wm_actions.keys() {
            key_to_keycode(&self.xmodmap_pke, &keybind.key).map(|keycode| {
                xcb::grab_key(
                    &self.conn,
                    false,
                    screen.root(),
                    keybind.mod_mask as u16,
                    keycode,
                    xcb::GRAB_MODE_ASYNC as u8,
                    xcb::GRAB_MODE_ASYNC as u8,
                )
            });
        }
    }

    fn grab_workspace_keys(&self, screen: &xcb::Screen) {
        for mod_mask in &[self.conf.meta, self.conf.meta | xcb::MOD_MASK_SHIFT] {
            for workspace_name_in_display in &self.conf.workspaces_names {
                for workspace_name in workspace_name_in_display {
                    key_to_keycode(&self.xmodmap_pke, workspace_name).map(|keycode| {
                        xcb::grab_key(
                            &self.conn,
                            false,
                            screen.root(),
                            *mod_mask as u16,
                            keycode,
                            xcb::GRAB_MODE_ASYNC as u8,
                            xcb::GRAB_MODE_ASYNC as u8,
                        )
                    });
                }
            }
        }
    }

    fn focus_unfocus(
        &mut self,
        window: &xcb::Window,
        do_focus: bool,
    ) -> Result<(), Box<dyn Error>> {
        let mut border_focus = false;
        if do_focus {
            xcb::set_input_focus(&self.conn, xcb::INPUT_FOCUS_PARENT as u8, *window, 0);
            let workspace = self
                .workspaces
                .get_mut(&self.current_workspace)
                .ok_or("workspace not found")?;
            if let Some(i) = workspace.windows.iter().position(|x| x == window) {
                workspace.focus = i;
            }
            border_focus = !((workspace.windows.len() == 1 || workspace.layout == Layout::Monocle)
                && self.displays_geometries.len() == 1);
            let net_active_window = xcb::intern_atom(&self.conn, false, "_NET_ACTIVE_WINDOW")
                .get_reply()?
                .atom();
            let setup = self.conn.get_setup();
            let root = setup.roots().next().ok_or("roots 0 not found")?.root();
            let data = vec![*window];
            xproto::change_property(
                &self.conn,
                xcb::PROP_MODE_REPLACE as u8,
                root,
                net_active_window,
                xproto::ATOM_WINDOW,
                32,
                &data[..],
            );
        }
        xcb::change_window_attributes(
            &self.conn,
            *window,
            &[(
                xcb::CW_BORDER_PIXEL,
                if border_focus {
                    self.conf.border.focus_color
                } else {
                    self.conf.border.normal_color
                },
            )],
        );
        Ok(())
    }

    fn serialize_and_quit(&mut self) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(UMBERWM_STATE)?;
        let string = json::to_string(&SerializableState {
            float_windows: self.float_windows.clone(),
            workspaces: self.workspaces.clone(),
            current_workspace: self.current_workspace.clone(),
        });
        file.write_all(string.as_bytes())?;
        std::process::exit(123);
    }

    fn run_wm_action(&mut self, keybind: &Keybind) -> Result<(), Box<dyn Error>> {
        let workspaces_names_by_display = self.conf.workspaces_names.clone();
        let action = self
            .conf
            .wm_actions
            .get(keybind)
            .ok_or("action not found")?;
        let workspace = self
            .workspaces
            .get_mut(&self.current_workspace)
            .ok_or("workspace not found")?;
        match action {
            Actions::CloseWindow => {
                let window = workspace
                    .windows
                    .get(workspace.focus)
                    .ok_or("window not found")?;
                let wm_delete_window = xcb::intern_atom(&self.conn, false, "WM_DELETE_WINDOW")
                    .get_reply()?
                    .atom();
                let wm_protocols = xcb::intern_atom(&self.conn, false, "WM_PROTOCOLS")
                    .get_reply()?
                    .atom();
                let data = xcb::ClientMessageData::from_data32([
                    wm_delete_window,
                    xcb::CURRENT_TIME,
                    0,
                    0,
                    0,
                ]);
                let ev = xcb::ClientMessageEvent::new(32, *window, wm_protocols, data);
                xcb::send_event(&self.conn, false, *window, xcb::EVENT_MASK_NO_EVENT, &ev);
                self.conn.flush();
            }
            Actions::SerializeAndQuit => {
                self.serialize_and_quit().ok();
            }
            Actions::SwitchWindow => {
                if !workspace.windows.is_empty() {
                    workspace.focus = (workspace.focus + 1) % workspace.windows.len();
                }
            }
            Actions::ChangeLayout => {
                workspace.layout = match workspace.layout {
                    Layout::Bspv => Layout::Monocle,
                    Layout::Monocle => Layout::Bsph,
                    Layout::Bsph => Layout::Bspv,
                }
            }
            Actions::ToggleGap => {
                self.conf.with_gap = !self.conf.with_gap;
            }
        };
        for (display, workspaces_names) in workspaces_names_by_display.iter().enumerate() {
            if workspaces_names.contains(&self.current_workspace) {
                let workspace = self
                    .workspaces
                    .get(&self.current_workspace)
                    .ok_or("workspace not found")?
                    .clone();
                self.resize_workspace_windows(&workspace, display);
            }
        }
        Ok(())
    }

    fn get_str_property(&mut self, window: u32, name: &str) -> Option<String> {
        let _net_wm_window_type = xcb::intern_atom(&self.conn, false, name)
            .get_reply()
            .unwrap()
            .atom();
        let cookie = xcb::get_property(
            &self.conn,
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

    fn get_atom_property(&mut self, id: u32, name: &str) -> Result<u32, Box<dyn Error>> {
        let window: xproto::Window = id;
        let ident = xcb::intern_atom(&self.conn, true, name).get_reply()?.atom();
        let reply =
            xproto::get_property(&self.conn, false, window, ident, xproto::ATOM_ATOM, 0, 1024)
                .get_reply()?;
        if reply.value_len() == 0 {
            Ok(42)
        } else {
            Ok(reply.value()[0])
        }
    }

    fn setup_new_window(&mut self, window: u32) -> Result<(), Box<dyn Error>> {
        for workspace in self.workspaces.values() {
            for workspace_window in &workspace.windows {
                if &window == workspace_window {
                    // The window already exist in a workspace
                    return Ok(());
                }
            }
        }
        let wm_class = self
            .get_str_property(window, "WM_CLASS")
            .ok_or("failed getting wm class")?;
        let window_type = self.get_atom_property(window, "_NET_WM_WINDOW_TYPE")?;
        let window_types = window_types_from_list(
            &self.conn,
            &vec![
                "menu".to_string(),
                "popup_menu".to_string(),
                "dropdown_menu".to_string(),
                "tooltip".to_string(),
                "utility".to_string(),
                "notification".to_string(),
                "toolbar".to_string(),
                "splash".to_string(),
                "dialog".to_string(),
                "dock".to_string(),
            ],
        );
        let wm_class: Vec<&str> = wm_class.split('\0').collect();
        println!(
            "UGUU: {} {} {}",
            window,
            xcb::get_atom_name(&self.conn, window_type)
                .get_reply()?
                .name(),
            wm_class.join("-")
        );
        if window_types.contains(&window_type)
            || "_KDE_NET_WM_WINDOW_TYPE_OVERRIDE"
                == xcb::get_atom_name(&self.conn, window_type)
                    .get_reply()?
                    .name()
        {
            return Ok(());
        }
        if !wm_class.is_empty() {
            for item in &wm_class {
                if item == &"xscreensaver" || self.conf.ignore_classes.contains(&item.to_string()) {
                    return Ok(());
                }
            }
        }
        if let Some(workspace) = self.workspaces.get_mut(&self.current_workspace) {
            if !workspace.windows.contains(&window) {
                if !wm_class.is_empty()
                    && self.conf.float_classes.contains(&wm_class[0].to_string())
                    && !self.float_windows.contains(&window)
                {
                    self.float_windows.push(window);
                }
                workspace.windows.push(window);
                workspace.focus = workspace.windows.len() - 1;
                let workspace2 = workspace.clone();
                let workspaces_names_by_display = self.conf.workspaces_names.clone();
                for (display, workspaces_names) in workspaces_names_by_display.iter().enumerate() {
                    if workspaces_names.contains(&self.current_workspace) {
                        self.resize_workspace_windows(&workspace2, display);
                    }
                }
            }
        }
        xcb::change_window_attributes(
            &self.conn,
            window,
            &[(
                xcb::CW_EVENT_MASK,
                xcb::EVENT_MASK_ENTER_WINDOW | xcb::EVENT_MASK_LEAVE_WINDOW,
            )],
        );
        Ok(())
    }

    fn resize_window(&mut self, event: &xcb::MotionNotifyEvent) -> Result<(), Box<dyn Error>> {
        let mouse_move_start = self.mouse_move_start.clone().ok_or("no mouse move start")?;
        let attr = self
            .button_press_geometry
            .clone()
            .ok_or("no button press geometry")?;
        let xdiff = event.root_x() - mouse_move_start.root_x;
        let ydiff = event.root_y() - mouse_move_start.root_y;
        let x = attr.0 as i32
            + if mouse_move_start.detail == 1 {
                xdiff as i32
            } else {
                0
            };
        let y = attr.1 as i32
            + if mouse_move_start.detail == 1 {
                ydiff as i32
            } else {
                0
            };
        let width = max(
            1,
            attr.2 as i32
                + if mouse_move_start.detail == 3 {
                    xdiff as i32
                } else {
                    0
                },
        );
        let height = max(
            1,
            attr.3 as i32
                + if mouse_move_start.detail == 3 {
                    ydiff as i32
                } else {
                    0
                },
        );
        xcb::configure_window(
            &self.conn,
            mouse_move_start.child,
            &[
                (xcb::CONFIG_WINDOW_X as u16, x as u32),
                (xcb::CONFIG_WINDOW_Y as u16, y as u32),
                (xcb::CONFIG_WINDOW_WIDTH as u16, width as u32),
                (xcb::CONFIG_WINDOW_HEIGHT as u16, height as u32),
            ],
        );
        Ok(())
    }

    fn destroy_window(&mut self, window: u32) {
        self.float_windows.retain(|&x| x != window);
        let mut workspace2: Option<Workspace> = None;
        for workspace in self.workspaces.values_mut() {
            if workspace.windows.contains(&window) {
                workspace.windows.retain(|&x| x != window);
                if workspace.focus > 0 {
                    workspace.focus -= 1;
                }
                workspace2 = Some(workspace.clone());
            }
        }
        let workspaces_names_by_display = self.conf.workspaces_names.clone();
        let mut dis = 0;
        for (display, workspaces_names) in workspaces_names_by_display.iter().enumerate() {
            if workspaces_names.contains(&self.current_workspace) {
                dis = display;
            }
        }

        if let Some(workspace) = workspace2 {
            workspace
                .windows
                .get(workspace.focus)
                .map(|previous_window| self.focus_unfocus(previous_window, true));
            self.resize_workspace_windows(&workspace, dis);
        }
    }

    pub fn run(&mut self) {
        loop {
            if let Some(event) = self.conn.wait_for_event() {
                let r = event.response_type();
                if r == xcb::MAP_NOTIFY as u8 {
                    let map_notify: &xcb::MapNotifyEvent = unsafe { xcb::cast_event(&event) };
                    self.setup_new_window(map_notify.window()).ok();
                }
                if r == self.randr_base + randr::NOTIFY {
                    self.displays_geometries = self.get_displays_geometries().unwrap();
                }
                if r == xcb::DESTROY_NOTIFY as u8 {
                    let map_notify: &xcb::DestroyNotifyEvent = unsafe { xcb::cast_event(&event) };
                    self.destroy_window(map_notify.window());
                } else if r == xcb::BUTTON_PRESS as u8 {
                    let event: &xcb::ButtonPressEvent = unsafe { xcb::cast_event(&event) };
                    self.handle_button_press(event);
                } else if r == xcb::MOTION_NOTIFY as u8 {
                    let event: &xcb::MotionNotifyEvent = unsafe { xcb::cast_event(&event) };
                    self.resize_window(event).ok();
                } else if r == xcb::LEAVE_NOTIFY as u8 {
                    let event: &xcb::LeaveNotifyEvent = unsafe { xcb::cast_event(&event) };
                    self.focus_unfocus(&event.event(), false).ok();
                } else if r == xcb::ENTER_NOTIFY as u8 {
                    let event: &xcb::EnterNotifyEvent = unsafe { xcb::cast_event(&event) };
                    self.focus_unfocus(&event.event(), true).ok();
                } else if r == xcb::BUTTON_RELEASE as u8 {
                    self.mouse_move_start = None;
                } else if r == xcb::KEY_PRESS as u8 {
                    let event: &xcb::KeyPressEvent = unsafe { xcb::cast_event(&event) };
                    self.handle_key_press(event);
                }
            }
            self.conn.flush();
        }
    }

    fn handle_button_press(&mut self, event: &xcb::ButtonPressEvent) {
        if let Ok(geometry) = xcb::get_geometry(&self.conn, event.child()).get_reply() {
            self.button_press_geometry = Some(Geometry(
                geometry.x() as u32,
                geometry.y() as u32,
                geometry.width() as u32,
                geometry.height() as u32,
            ));
        }
        self.mouse_move_start = Some(MouseMoveStart {
            root_x: event.root_x(),
            root_y: event.root_y(),
            child: event.child(),
            detail: event.detail(),
        });
    }

    fn handle_key_press(&mut self, event: &xcb::KeyPressEvent) {
        let keycode = event.detail();
        let mod_mask = event.state();
        if let Some(key) = &keycode_to_key(&self.xmodmap_pke, keycode) {
            let keybind = Keybind::new(key, mod_mask);

            self.handle_workspace_change(&keybind);

            if self.conf.wm_actions.contains_key(&keybind) {
                self.run_wm_action(&keybind).ok();
            } else if self.conf.custom_actions.contains_key(&keybind) {
                if let Some(action) = self.conf.custom_actions.get(&keybind) {
                    action();
                }
            }
        }
    }

    fn handle_workspace_change(&mut self, keybind: &Keybind) {
        let workspaces_names_by_display = self.conf.workspaces_names.clone();
        for (display, workspaces_names) in workspaces_names_by_display.iter().enumerate() {
            if workspaces_names.contains(&keybind.key) {
                if let Ok(workspace) = change_workspace(
                    &self.conn,
                    &mut self.workspaces,
                    self.current_workspace.to_string(),
                    keybind.key.clone(),
                    keybind.mod_mask & xcb::MOD_MASK_SHIFT != 0,
                    workspaces_names.contains(&self.current_workspace)
                        || display >= self.displays_geometries.len()
                        || self.previous_display >= self.displays_geometries.len(),
                ) {
                    self.previous_display = display;
                    self.current_workspace = workspace;
                    let workspace = self
                        .workspaces
                        .get(&self.current_workspace)
                        .ok_or("workspace not found")
                        .unwrap()
                        .clone();
                    self.resize_workspace_windows(&workspace, display);
                    let actual_display = if display >= self.displays_geometries.len() {
                        self.displays_geometries.len() - 1
                    } else {
                        display
                    };
                    if let Some(callback) = self.conf.events_callbacks.on_change_workspace.as_ref()
                    {
                        callback(keybind.key.clone(), actual_display)
                    }
                }
            }
        }
    }
}

fn load_serializable_state(conf: &Conf) -> Result<SerializableState, Box<dyn Error>> {
    if Path::new(UMBERWM_STATE).exists() {
        let mut file = File::open(UMBERWM_STATE)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let res: SerializableState = json::from_str(contents.as_str())?;
        remove_file(UMBERWM_STATE)?;
        Ok(res)
    } else {
        Ok(SerializableState {
            float_windows: vec![],
            workspaces: conf
                .workspaces_names
                .clone()
                .into_iter()
                .flatten()
                .into_iter()
                .map(|x| {
                    (
                        x,
                        Workspace {
                            layout: Layout::Bspv,
                            windows: vec![],
                            focus: 0,
                        },
                    )
                })
                .into_iter()
                .collect(),
            current_workspace: conf.workspaces_names.get(0).unwrap()[0].to_string(),
        })
    }
}

pub fn umberwm(conf: Conf) -> UmberWm {
    let (conn, _) = xcb::Connection::connect(None).unwrap();
    let serializable_state = load_serializable_state(&conf).unwrap();
    let xmodmap_pke = xmodmap_pke().unwrap();
    let mut wm = UmberWm {
        conf,
        current_workspace: serializable_state.current_workspace,
        float_windows: serializable_state.float_windows,
        workspaces: serializable_state.workspaces,
        conn,
        button_press_geometry: None,
        mouse_move_start: None,
        xmodmap_pke,
        displays_geometries: Vec::new(),
        randr_base: 0,
        previous_display: 0,
    };
    wm.init();
    wm
}
