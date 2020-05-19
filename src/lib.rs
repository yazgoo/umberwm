extern crate xcb;
extern crate regex;

use std::collections::HashMap;
use xcb::xproto;
use xcb::randr;
use std::error::Error;
use std::cmp::max;
use regex::Regex;
use std::process::Command;

type XmodmapPke = HashMap<u8, Vec<String>>;

fn xmodmap_pke() -> Result<XmodmapPke, Box<dyn Error>> {
    let output = Command::new("xmodmap").arg("-pke").output()?;
    let pattern = Regex::new(r"(\d+) = (.*)")?;
    let lines = String::from_utf8(output.stdout)?
        .lines()
        .filter_map(|line| pattern.captures(line))
        .map(|cap| (cap[1].parse().unwrap(), cap[2].split(" ").map(|s| s.to_string()).collect()))
        .collect::<HashMap<u8, Vec<String>>>();
    Ok(lines)
}

pub enum Actions {
    SwitchWindow, CloseWindow, ChangeLayout,
}

pub enum Meta {
    Mod1, Mod4
}

#[derive(Clone, Debug)]
enum Layout {
    BSPV,
    Monocle,
    BSPH
}

type Window = u32;

pub type Key = String;

type WorkspaceName = Key;

pub type CustomAction = Box<dyn Fn() -> ()>;

type Color = u32;

#[derive(Clone)]
pub struct Geometry(u32, u32, u32, u32);

pub struct WindowBorder {
    pub width: u32,
    pub focus_color: Color,
    pub normal_color: Color,
}

#[derive(Clone, Debug)]
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

pub type OnChangeWorkspace = Option<Box<dyn Fn(WorkspaceName, DisplayId) -> ()>>;

pub struct EventsCallbacks {
    pub on_change_workspace: OnChangeWorkspace,
}

pub struct Conf {
    pub meta: Meta,
    pub border: WindowBorder,
    pub display_borders: Vec<DisplayBorder>,
    pub workspaces_names: Vec<Vec<WorkspaceName>>,
    pub custom_actions: HashMap<Key, CustomAction>,
    pub wm_actions: HashMap<Key, Actions>,
    pub float_classes: Vec<String>,
    pub auto_float_types: Vec<String>,
    pub events_callbacks: EventsCallbacks,
}

#[derive(Clone)]
struct MouseMoveStart {
    root_x: i16,
    root_y: i16,
    child: Window,
    detail: u8,
}

pub struct UmberWM {
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

fn keycode_to_key(xmodmap_pke: &XmodmapPke, keycode: u8) -> Option<Key> {
    match xmodmap_pke.get(&keycode) {
        Some(x) => {
            if x.len() > 0 {
                Some(x[0].to_string())
            }
            else {
                None
            }
        },
        None => None
    }
}

fn key_to_keycode(xmodmap_pke: &XmodmapPke, key: &Key) -> Option<u8> {
    for (keycode, symbols) in xmodmap_pke.into_iter() {
        if symbols.contains(&key) {
            return Some(*keycode);
        }
    }
    None
}


fn unmap_workspace_windows(conn: &xcb::Connection, windows: &mut Vec<Window>, focus: usize, move_window: bool, same_display: bool) -> Option<Window> {
    let mut window_to_move = None;
    for (i, window) in windows.iter().enumerate() {
        if move_window && i == focus {
            window_to_move = Some(*window);
        }
        else {
            if same_display {
                xcb::unmap_window(conn, *window);
            }
        }
    }
    window_to_move
}

fn change_workspace(conn: &xcb::Connection, workspaces: &mut HashMap<WorkspaceName, Workspace>, previous_workspace: WorkspaceName, next_workspace: WorkspaceName, move_window: bool, same_display: bool) -> Result<WorkspaceName, Box<dyn Error>> {
    let workspace = workspaces.get_mut(&previous_workspace).ok_or("workspace not found")?;
    let window_to_move = unmap_workspace_windows(conn, &mut workspace.windows, workspace.focus, move_window, same_display);
    match window_to_move {
        Some(w) => {
            workspace.windows.retain( |x| *x != w );
            if workspace.windows.len() > 0 {
                workspace.focus = workspace.windows.len() - 1;
            }
            else {
                workspace.focus = 0;
            }
        },
        None => {},
    };
    let workspace = workspaces.get_mut(&next_workspace).ok_or("workspace not found")?;
    for window in &workspace.windows {
        xcb::map_window(conn, *window);
    }
    window_to_move.map( 
        |w| { 
            workspace.windows.push(w);
            workspace.focus = workspace.windows.len() - 1;
        }
    );
    Ok(next_workspace)
}

    fn geometries_bsp(i: usize, window_count: usize, left: u32, top: u32, width: u32, height: u32, vertical: usize) -> Vec<Geometry> {
        if window_count == 0 {
            vec![]
        }
        else if window_count == 1 {
            vec![Geometry(left, top, width, height)]
        }
        else if i % 2 == vertical {
            let mut res = vec![Geometry(left, top, width, height / 2)];
            res.append(
                &mut geometries_bsp(i + 1, window_count - 1, left, top + height / 2, width, height / 2, vertical));
            res
        }
        else {
            let mut res = vec![Geometry(left, top, width / 2, height)];
            res.append(
                &mut geometries_bsp(i + 1, window_count - 1, left + width / 2, top, width / 2, height, vertical));
            res
        }
    }


    fn window_types_from_list(conn: &xcb::Connection, types_names: &Vec<String>) -> Vec<xcb::Atom> {
        types_names.into_iter().map(|x| {
            let name = format!("_NET_WM_WINDOW_TYPE_{}", x.to_uppercase());
            let res = xcb::intern_atom(&conn, true, name.as_str()).get_reply().map(|x| x.atom());
            res.ok()
        }
        ).flatten().collect()
    }

impl UmberWM {

    pub fn get_displays_geometries(&mut self) -> Result<Vec<Geometry>, Box<dyn Error>> {
        let conn = &self.conn;
        let setup = self.conn.get_setup();
        let screen = setup.roots().nth(0).unwrap();
        let window_dummy = conn.generate_id();
        xcb::create_window(&conn, 0, window_dummy, screen.root(), 0, 0, 1, 1, 0, 0, 0, &[]);
        let screen_res_cookie =
            randr::get_screen_resources(&conn, window_dummy);
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
                if i != 0 { println!(""); }
                    println!("CRTC[{}] INFO:", i);
                    println!(" x-off\t: {}", reply.x());
                    println!(" y-off\t: {}", reply.y());
                    println!(" width\t: {}", reply.width());
                    println!(" height\t: {}", reply.height());
                    result.push(Geometry(reply.x() as u32, reply.y() as u32, reply.width() as u32, reply.height() as u32))
                }
            }
        }
        Ok(result)
    }


    fn get_display_border(&mut self, display: usize)-> DisplayBorder {
        let display_border_i = if display >= self.conf.display_borders.len() {
            self.conf.display_borders.len() - 1
        }
        else {
            display
        };
        self.conf.display_borders[display_border_i].clone()
    }


    fn resize_workspace_windows(&mut self, workspace: &Workspace, mut display: usize) {
        let mut non_float_windows = workspace.windows.clone();
        non_float_windows.retain(|w| !self.float_windows.contains(&w));
        let count = non_float_windows.len();
        if count == 0 || self.displays_geometries.len() <= 0 {
            return
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
        let geos = match workspace.layout {
            Layout::BSPV => {
                geometries_bsp(0, count, left, top, width, height, 1)},
            Layout::BSPH => {
                geometries_bsp(0, count, left, top, width, height, 0)},
            Layout::Monocle => {
                geometries_bsp(0, 1, left, top, width, height, 1)},
        };
        match workspace.layout {
            Layout::BSPV | Layout::BSPH => {
                for (i, geo) in geos.iter().enumerate() {
                    match non_float_windows.get(i) {
                        Some(window) => {xcb::configure_window(&self.conn, *window, &[
                            (xcb::CONFIG_WINDOW_X as u16, geo.0 + display_border.gap),
                            (xcb::CONFIG_WINDOW_Y as u16, geo.1 + display_border.gap),
                            (xcb::CONFIG_WINDOW_WIDTH as u16, geo.2.saturating_sub(2 * self.conf.border.width + 2 * display_border.gap)),
                            (xcb::CONFIG_WINDOW_HEIGHT as u16, geo.3.saturating_sub(2 * self.conf.border.width + 2 * display_border.gap)),
                            (xcb::CONFIG_WINDOW_BORDER_WIDTH as u16, self.conf.border.width),
                        ]
                        );
                        },
                        None => {}
                    }
                }
            }
            Layout::Monocle => {
                match workspace.windows.get(workspace.focus) {
                    Some(window) => {xcb::configure_window(&self.conn, *window, &[
                        (xcb::CONFIG_WINDOW_X as u16, geos[0].0 + display_border.gap),
                        (xcb::CONFIG_WINDOW_Y as u16, geos[0].1 + display_border.gap),
                        (xcb::CONFIG_WINDOW_WIDTH as u16, geos[0].2.saturating_sub(2 * self.conf.border.width + 2 * display_border.gap)),
                        (xcb::CONFIG_WINDOW_HEIGHT as u16, geos[0].3.saturating_sub(2 * self.conf.border.width + 2 * display_border.gap)),
                        (xcb::CONFIG_WINDOW_BORDER_WIDTH as u16, self.conf.border.width),
                        (xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE),
                    ]
                    );
                    },
                    None => {}
                }
            }
        }
        for (i, _) in workspace.windows.iter().enumerate() {
            match workspace.windows.get(i) {
                Some(window) => {
                    let _ = self.focus_unfocus(window, i == workspace.focus);
                },
                None =>{}
            }
        }
    }

    fn init(&mut self) {
        self.displays_geometries = self.get_displays_geometries().unwrap();
        let screen = self.conn.get_setup().roots().nth(0).unwrap();
        let mod_key = match self.conf.meta {
             Meta::Mod4 => xcb::MOD_MASK_4,
             Meta::Mod1 => xcb::MOD_MASK_1
        };
        self.randr_base = self.conn.get_extension_data(&mut randr::id()).unwrap().first_event();
        let _ = randr::select_input(&self.conn, screen.root(), randr::NOTIFY_MASK_CRTC_CHANGE as u16)
            .request_check();
        for mod_mask in vec![mod_key, mod_key | xcb::MOD_MASK_SHIFT] {
            for workspace_name_in_display in &self.conf.workspaces_names {
                for workspace_name in workspace_name_in_display {
                    key_to_keycode(&self.xmodmap_pke, workspace_name).map ( |keycode|
                        xcb::grab_key(&self.conn, false, screen.root(), mod_mask as u16, keycode, xcb::GRAB_MODE_ASYNC as u8, xcb::GRAB_MODE_ASYNC as u8)
                    );
                }
            }
            for custom_action_key in self.conf.custom_actions.keys() {
                key_to_keycode(&self.xmodmap_pke, custom_action_key).map ( |keycode|
                    xcb::grab_key(&self.conn, false, screen.root(), mod_mask as u16, keycode, xcb::GRAB_MODE_ASYNC as u8, xcb::GRAB_MODE_ASYNC as u8)
                );
            }
            for custom_action_key in self.conf.wm_actions.keys() {
                key_to_keycode(&self.xmodmap_pke, custom_action_key).map ( |keycode|
                    xcb::grab_key(&self.conn, false, screen.root(), mod_mask as u16, keycode, xcb::GRAB_MODE_ASYNC as u8, xcb::GRAB_MODE_ASYNC as u8)
                );
            }
        }
        for button in vec![1, 3] {
            xcb::grab_button(&self.conn, false, screen.root(), (xcb::EVENT_MASK_BUTTON_PRESS | xcb::EVENT_MASK_BUTTON_RELEASE | xcb::EVENT_MASK_POINTER_MOTION) as u16, xcb::GRAB_MODE_ASYNC as u8, xcb::GRAB_MODE_ASYNC as u8, xcb::NONE, xcb::NONE, button as u8, mod_key as u16);
        }
        xcb::change_window_attributes(&self.conn, screen.root(), &[(xcb::CW_EVENT_MASK, xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY as u32)]);
        self.conn.flush();
    }

    fn focus_unfocus(&mut self, window: &xcb::Window, do_focus: bool) -> Result<(), Box<dyn Error>> {
        xcb::change_window_attributes(&self.conn, *window, &[
            (xcb::CW_BORDER_PIXEL, 
             if do_focus {
                 self.conf.border.focus_color
             } else {
                 self.conf.border.normal_color
             }
            ),
        ]);
        if do_focus {
            xcb::set_input_focus(&self.conn, xcb::INPUT_FOCUS_PARENT as u8, *window, 0);
            let workspace = self.workspaces.get_mut(&self.current_workspace).ok_or("workspace not found")?;
            workspace.windows.iter().position(|x| x == window).map(|i| workspace.focus = i );

            let net_active_window = xcb::intern_atom(&self.conn, false, "_NET_ACTIVE_WINDOW").get_reply()?.atom();
            let setup = self.conn.get_setup();
            let root = setup.roots().nth(0).ok_or("roots 0 not found")?.root();
            let data = vec![*window];
            xproto::change_property(&self.conn, xcb::PROP_MODE_REPLACE as u8, root, net_active_window, xproto::ATOM_WINDOW, 32, &data[..]);

        }
        Ok(())
    }


    fn run_wm_action(&mut self, key: &Key) -> Result<(), Box<dyn Error>> {
        let workspaces_names_by_display = self.conf.workspaces_names.clone();
        let action = self.conf.wm_actions.get(&key.to_string()).ok_or("action not found")?;
        let workspace = self.workspaces.get_mut(&self.current_workspace).ok_or("workspace not found")?;
        match action {
            Actions::CloseWindow => {
                let window = workspace.windows.get(workspace.focus).ok_or("window not found")?;
                let wm_delete_window = xcb::intern_atom(&self.conn, false, "WM_DELETE_WINDOW").get_reply()?.atom();
                let wm_protocols = xcb::intern_atom(&self.conn, false, "WM_PROTOCOLS").get_reply()?.atom();
                let data = xcb::ClientMessageData::from_data32([
                    wm_delete_window,
                    xcb::CURRENT_TIME,
                    0, 0, 0,
                ]);
                let ev = xcb::ClientMessageEvent::new(32, *window,
                    wm_protocols, data);
                xcb::send_event(&self.conn, false, *window, xcb::EVENT_MASK_NO_EVENT, &ev);
                self.conn.flush();
            },
            Actions::SwitchWindow => {
                if workspace.windows.len() > 0 {
                    workspace.focus = (workspace.focus + 1) % workspace.windows.len();
                }
            },
            Actions::ChangeLayout => {
                workspace.layout = match workspace.layout {
                    Layout::BSPV => Layout::Monocle,
                    Layout::Monocle => Layout::BSPH,
                    Layout::BSPH => Layout::BSPV,
                }
            },
        };
        for (display, workspaces_names) in workspaces_names_by_display.iter().enumerate() {
            if workspaces_names.contains(&self.current_workspace) {
                let workspace = self.workspaces.get(&self.current_workspace).ok_or("workspace not found")?.clone();
                self.resize_workspace_windows(&workspace, display);
            }
        }
        Ok(())
    }



    fn get_str_property(&mut self, window: u32, name: &str) -> Option<String> {
        let _net_wm_window_type = xcb::intern_atom(&self.conn, false, name).get_reply().unwrap().atom();
        let cookie = xcb::get_property(&self.conn, false, window, _net_wm_window_type, xcb::ATOM_ANY, 0, 1024);
        if let Ok(reply) = cookie.get_reply() {
            Some(std::str::from_utf8(reply.value()).unwrap().to_string())
        } else {
            None
        }
    }

    fn get_atom_property(&mut self, id: u32, name: &str) -> Result<u32, Box<dyn Error>> {
        let window: xproto::Window = id;
        let ident = xcb::intern_atom(&self.conn, true, name).get_reply()?.atom();
        let reply = xproto::get_property(&self.conn, false, window, ident, xproto::ATOM_ATOM, 0, 1024).get_reply()?;
        if reply.value_len() <= 0 {
            Ok(42)
        }
        else {
            Ok(reply.value()[0])
        }
    }

    fn setup_new_window(&mut self, window: u32) -> Result<(), Box<dyn Error>> {
        let wm_class = self.get_str_property(window, "WM_CLASS").ok_or("failed getting wm class")?;
        let window_type = self.get_atom_property(window, "_NET_WM_WINDOW_TYPE")?;
        let auto_float_types =  window_types_from_list(&self.conn, &self.conf.auto_float_types);
        if auto_float_types.contains(&window_type) {
            return Ok(())
        }
        let wm_class : Vec<&str> = wm_class.split('\0').collect();
        match self.workspaces.get_mut(&self.current_workspace) {
            Some(workspace) => {
                if !workspace.windows.contains(&window) {
                    if wm_class.len() != 0 && self.conf.float_classes.contains(&wm_class[0].to_string()) && !self.float_windows.contains(&window) { 
                        self.float_windows.push(window);
                    }
                    workspace.windows.push(window);
                    let workspace2 = workspace.clone();
                    let workspaces_names_by_display = self.conf.workspaces_names.clone();
                    for (display, workspaces_names) in workspaces_names_by_display.iter().enumerate() {
                        if workspaces_names.contains(&self.current_workspace) {
                            self.resize_workspace_windows(&workspace2, display);
                        }
                    }
                }
            },
            None => {
            },
        }
        xcb::change_window_attributes(&self.conn, window, &[(xcb::CW_EVENT_MASK, xcb::EVENT_MASK_ENTER_WINDOW | xcb::EVENT_MASK_LEAVE_WINDOW)]);
        Ok(())
    }

    fn resize_window(&mut self, event: &xcb::MotionNotifyEvent) -> Result<(), Box<dyn Error>> {
        let mouse_move_start = self.mouse_move_start.clone().ok_or("no mouse move start")?;
        let attr = self.button_press_geometry.clone().ok_or("no button press geometry")?;
        let xdiff = event.root_x() - mouse_move_start.root_x;
        let ydiff = event.root_y() - mouse_move_start.root_y;
        let x = attr.0 as i32 + if mouse_move_start.detail == 1 { xdiff as i32 } else { 0 };
        let y = attr.1 as i32 + if mouse_move_start.detail == 1 { ydiff as i32 } else { 0 };
        let width = max(1, attr.2 as i32 + if mouse_move_start.detail == 3 { xdiff as i32 } else { 0 });
        let height = max(1, attr.3 as i32 + if mouse_move_start.detail == 3 { ydiff as i32 } else { 0 });
        xcb::configure_window(&self.conn, mouse_move_start.child, &[
                            (xcb::CONFIG_WINDOW_X as u16, x as u32),
                            (xcb::CONFIG_WINDOW_Y as u16, y as u32),
                            (xcb::CONFIG_WINDOW_WIDTH as u16, width as u32),
                            (xcb::CONFIG_WINDOW_HEIGHT as u16, height as u32),
                        ]);
        Ok(())
    }

    fn destroy_window(&mut self, window: u32) {
        self.float_windows.retain(|&x| x != window);
        let mut workspace2 : Option<Workspace> = None;
        for (_, workspace) in &mut self.workspaces {
            if workspace.windows.contains(&window) {
                workspace.windows.retain(|&x| x != window);
                workspace2 = Some(workspace.clone());
                workspace.focus = 0;
            }
        }
        let workspaces_names_by_display = self.conf.workspaces_names.clone();
        let mut dis = 0;
        for (display, workspaces_names) in workspaces_names_by_display.iter().enumerate() {
            if workspaces_names.contains(&self.current_workspace) {
                dis = display;
            }
        }

        workspace2.map(|workspace|self.resize_workspace_windows(&workspace, dis));
    }

    pub fn run(&mut self) {
        loop {
            match self.conn.wait_for_event() {
                Some(event) => {
                    let r = event.response_type();
                    if r == xcb::MAP_NOTIFY as u8 {
                        let map_notify : &xcb::MapNotifyEvent = unsafe {
                            xcb::cast_event(&event)
                        };
                        let _ = self.setup_new_window(map_notify.window());
                    }
                    if r == self.randr_base + randr::NOTIFY {
                        self.displays_geometries = self.get_displays_geometries().unwrap();
                    }
                    if r == xcb::DESTROY_NOTIFY as u8 {
                        let map_notify : &xcb::DestroyNotifyEvent = unsafe {
                            xcb::cast_event(&event)
                        };
                        self.destroy_window(map_notify.window());
                    }
                    else if r == xcb::BUTTON_PRESS as u8 {
                        let event : &xcb::ButtonPressEvent = unsafe {
                            xcb::cast_event(&event)
                        };
                        match xcb::get_geometry(&self.conn, event.child()).get_reply() {
                            Ok(geometry) => {self.button_press_geometry = Some(
                                Geometry(geometry.x() as u32, geometry.y() as u32, geometry.width() as u32, geometry.height() as u32)
                                );},
                            Err(_) => {},
                        }
                        self.mouse_move_start = Some(MouseMoveStart{
                            root_x: event.root_x(),
                            root_y: event.root_y(),
                            child: event.child(),
                            detail: event.detail(),
                        });
                    }
                    else if r == xcb::MOTION_NOTIFY as u8 {
                        let event : &xcb::MotionNotifyEvent = unsafe {
                            xcb::cast_event(&event)
                        };
                        let _ = self.resize_window(event);
                    }
                    else if r == xcb::LEAVE_NOTIFY as u8 {
                        let event : &xcb::LeaveNotifyEvent = unsafe {
                            xcb::cast_event(&event)
                        };
                        let _= self.focus_unfocus(&event.event(), false);
                    }
                    else if r == xcb::ENTER_NOTIFY as u8 {
                        let event : &xcb::EnterNotifyEvent = unsafe {
                            xcb::cast_event(&event)
                        };
                        let _ = self.focus_unfocus(&event.event(), true);
                    }
                    else if r == xcb::BUTTON_RELEASE as u8 {
                        self.mouse_move_start = None;
                    }
                    else if r == xcb::KEY_PRESS as u8 {
                        let key_press : &xcb::KeyPressEvent = unsafe {
                            xcb::cast_event(&event)
                        };
                        let keycode = key_press.detail();
                        match &keycode_to_key(&self.xmodmap_pke, keycode) {
                            Some(key) => {
                                let workspaces_names_by_display = self.conf.workspaces_names.clone();
                                for (display, workspaces_names) in workspaces_names_by_display.iter().enumerate() {
                                    if workspaces_names.contains(key) {
                                        match change_workspace(&self.conn, &mut self.workspaces, self.current_workspace.to_string(), key.to_string(), (key_press.state() as u32 ) & xcb::MOD_MASK_SHIFT != 0, 
                                            workspaces_names.contains(&self.current_workspace) || display >= self.displays_geometries.len() || self.previous_display >= self.displays_geometries.len()) {
                                            Ok(workspace) => { 
                                                self.previous_display = display;
                                                self.current_workspace = workspace;
                                                let workspace = self.workspaces.get(&self.current_workspace).ok_or("workspace not found").unwrap().clone();
                                                self.resize_workspace_windows(&workspace, display);
                                                let actual_display = if display >= self.displays_geometries.len() {
                                                    self.displays_geometries.len() - 1
                                                }
                                                else {
                                                    display
                                                };
                                                self.conf.events_callbacks.on_change_workspace.as_ref().map ( |callback|
                                                    callback(key.to_string(), actual_display)
                                                );
                                            },
                                            Err(_) => {},
                                        };
                                    }
                                }
                                if self.conf.wm_actions.contains_key(&key.to_string()) {
                                    let _ = self.run_wm_action(&key);
                                }
                                else if self.conf.custom_actions.contains_key(&key.to_string()) {
                                    match self.conf.custom_actions.get(&key.to_string()) {
                                        Some(action) => {
                                            action()
                                        },
                                        None => {},
                                    }
                                }
                            },
                            None => {
                            },

                        }
                    }
                },
                None => {}
            }
            self.conn.flush();
        }
    }
}

pub fn umberwm(conf: Conf) -> UmberWM {
    let (conn, _) = xcb::Connection::connect(None).unwrap();
    let conf_workspaces_flatten : Vec<Key> = conf.workspaces_names.clone().into_iter().flatten().collect();
    let workspaces = conf_workspaces_flatten.into_iter().map( |x|
            (x, Workspace {
                layout: Layout::BSPV,
                windows: vec![],
                focus: 0,
        })).into_iter().collect();
    let xmodmap_pke = xmodmap_pke().unwrap();
    let current_workspace = conf.workspaces_names.get(0).unwrap()[0].to_string();
    let mut wm = UmberWM {
        conf: conf,
        current_workspace: current_workspace,
        float_windows: vec![],
        workspaces: workspaces,
        conn: conn,
        button_press_geometry: None,
        mouse_move_start: None,
        xmodmap_pke: xmodmap_pke,
        displays_geometries: Vec::new(),
        randr_base: 0,
        previous_display: 0,
    };
    wm.init();
    wm
}
