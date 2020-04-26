extern crate xcb;

use std::collections::HashMap;
use std::process::Command;
use xcb::xproto;
use std::error::Error;
use std::env;

enum Actions {
    SwitchWindow, CloseWindow, ChangeLayout,
}

enum Layout {
    BSPV,
    Monocle,
    BSPH
}

type Window = u32;

type Key = char;

type WorkspaceName = Key;

type CustomAction = Box<dyn Fn() -> ()>;

type Color = u32;

struct Geometry(u32, u32, u32, u32);

struct Border {
    width: u32,
    focus_color: Color,
    normal_color: Color,
}

struct Workspace {
    layout: Layout,
    windows: Vec<Window>,
    focus: usize,
}

struct Conf {
    meta: u32,
    border: Border,
    workspaces_names: Vec<WorkspaceName>,
    custom_actions: HashMap<Key, CustomAction>,
    wm_actions: HashMap<Key, Actions>,
    float_classes: Vec<String>,
    auto_float_types: Vec<String>,
}

struct YazgooWM {
    conf: Conf,
    current_workspace: WorkspaceName,
    float_windows: Vec<Window>,
    workspaces: HashMap<WorkspaceName, Workspace>,
    auto_float_types: Vec<xcb::Atom>,
    conn: xcb::Connection,
}

fn keycode_to_key(keycode: u8) -> Option<Key> {
    let mut translator = HashMap::new();
    translator.insert(38, 'a');
    translator.insert(39, 'u');
    translator.insert(40, 'i');
    translator.insert(27, 'o');
    translator.insert(26, 'p');
    translator.insert(65, ' ');
    translator.insert(61, 'f');
    translator.insert(46, 'r');
    translator.insert(35, 'w');
    translator.insert(44, 't');
    translator.insert(58, 'q');
    match translator.get(&keycode) {
        Some(x) => Some(*x),
        None => None
    }
}

fn key_to_keycode(key: &Key) -> Option<u8> {
    let mut translator = HashMap::new();
    translator.insert('a', 38);
    translator.insert('u', 39);
    translator.insert('i', 40);
    translator.insert('o', 27);
    translator.insert('p', 26);
    translator.insert(' ', 65);
    translator.insert('f', 61);
    translator.insert('r', 46);
    translator.insert('w', 35);
    translator.insert('t', 44);
    translator.insert('q', 58);
    match translator.get(key) {
        Some(x) => Some(*x),
        None => None
    }
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

    fn resize_workspace_windows(conn: &xcb::Connection,workspace: &Workspace, border: &Border, float_windows: &Vec<u32>) {
        let mut non_float_windows = workspace.windows.clone();
        non_float_windows.retain(|w| float_windows.contains(&w));
        let count = non_float_windows.len();
        let screen = conn.get_setup().roots().nth(0).unwrap();
        let geos = match workspace.layout {
            Layout::BSPV => {
                geometries_bsp(0, count, 0, 0, screen.width_in_pixels() as u32, screen.height_in_pixels() as u32, 1)},
            Layout::BSPH => {
                geometries_bsp(0, count, 0, 0, screen.width_in_pixels() as u32, screen.height_in_pixels() as u32, 0)},
            Layout::Monocle => {
                geometries_bsp(0, count, 0, 0, screen.width_in_pixels() as u32, screen.height_in_pixels() as u32, 1)},
        };
        for (i, geo) in geos.iter().enumerate() {
            match non_float_windows.get(i) {
                Some(window) => {xcb::configure_window(&conn, *window, &[
                        (xcb::CONFIG_WINDOW_X as u16, geo.0),
                        (xcb::CONFIG_WINDOW_Y as u16, geo.1),
                        (xcb::CONFIG_WINDOW_WIDTH as u16, geo.2 - 2 * border.width),
                        (xcb::CONFIG_WINDOW_HEIGHT as u16, geo.3 - 2 * border.width),
                        (xcb::CONFIG_WINDOW_BORDER_WIDTH as u16, border.width),
                ]
                    );
                },
                None => {}
            }
        }
        for (i, geo) in workspace.windows.iter().enumerate() {
            match workspace.windows.get(i) {
                Some(window) => {
                    xcb::change_window_attributes(&conn, *window, &[
                        (xcb::CW_BORDER_PIXEL, 
                         if i == workspace.focus {
                             border.focus_color
                         } else {
                             border.normal_color
                         }
                        ),
                    ]);
                    if i == workspace.focus {
                        xcb::set_input_focus(&conn, xcb::INPUT_FOCUS_PARENT as u8, *window, 0);
                    }
                },
                None =>{}
            }
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


impl YazgooWM {

    fn init(&mut self) {
        let screen = self.conn.get_setup().roots().nth(0).unwrap();
        let mod_key = self.conf.meta;
        for mod_mask in vec![mod_key, mod_key | xcb::MOD_MASK_SHIFT] {
            for workspace_name in &self.conf.workspaces_names {
                xcb::grab_key(&self.conn, false, screen.root(), mod_mask as u16, key_to_keycode(workspace_name).unwrap(), xcb::GRAB_MODE_ASYNC as u8, xcb::GRAB_MODE_ASYNC as u8);
            }
            for custom_action_key in self.conf.custom_actions.keys() {
                xcb::grab_key(&self.conn, false, screen.root(), mod_mask as u16, key_to_keycode(custom_action_key).unwrap(), xcb::GRAB_MODE_ASYNC as u8, xcb::GRAB_MODE_ASYNC as u8);
            }
            for custom_action_key in self.conf.wm_actions.keys() {
                xcb::grab_key(&self.conn, false, screen.root(), mod_mask as u16, key_to_keycode(custom_action_key).unwrap(), xcb::GRAB_MODE_ASYNC as u8, xcb::GRAB_MODE_ASYNC as u8);
            }
        }
        xcb::change_window_attributes(&self.conn, screen.root(), &[(xcb::CW_EVENT_MASK, xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY as u32)]);
        self.conn.flush();
    }

    fn run_wm_action(&mut self, key: &Key) -> Result<(), Box<dyn Error>> {
        let action = self.conf.wm_actions.get(&key).ok_or("action not found")?;
        let workspace = self.workspaces.get_mut(&self.current_workspace).ok_or("workspace not found")?;
        match action {
            Actions::CloseWindow => {
                let window = workspace.windows.get(workspace.focus).ok_or("window not found")?;
                xcb::destroy_window(&self.conn, *window);
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
        resize_workspace_windows(&self.conn, &workspace, &self.conf.border, &self.float_windows);
        Ok(())
    }

    fn change_workspace(&mut self, workspace: WorkspaceName, move_window: bool) {
        match self.workspaces.get(&self.current_workspace) {
            Some(previous_workspace) => {
                for window in &previous_workspace.windows {
                    xcb::unmap_window(&self.conn, *window);
                }
            }
            None => {}
        };
        self.current_workspace = workspace;
        match self.workspaces.get(&self.current_workspace) {
            Some(previous_workspace) => {
                for window in &previous_workspace.windows {
                    xcb::map_window(&self.conn, *window);
                }
            }
            None => {}
        };
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

    fn get_atom_property(&mut self, id: u32, name: &str) -> Result<u32, Box<Error>> {
        let window: xproto::Window = id;
        let ident = xcb::intern_atom(&self.conn, true, "_NET_WM_WINDOW_TYPE").get_reply()?.atom();
        let reply = xproto::get_property(&self.conn, false, window, ident, xproto::ATOM_ATOM, 0, 1024).get_reply()?;
        Ok(0)
    }

    fn setup_new_window(&mut self, window: u32) {
        let wm_class = self.get_str_property(window, "WM_CLASS").unwrap();
        let window_type = self.get_atom_property(window, "_NET_WM_WINDOW_TYPE").unwrap();
        if self.auto_float_types.contains(&window_type) {
            return
        }
        let wm_class : Vec<&str> = wm_class.split('\0').collect();
        match self.workspaces.get_mut(&self.current_workspace) {
            Some(workspace) => {
                if !workspace.windows.contains(&window) {
                    if wm_class.len() != 0 && ! self.conf.float_classes.contains(&wm_class[0].to_string()) && !self.float_windows.contains(&window) { 
                        self.float_windows.push(window);
                    }
                    workspace.windows.push(window);
                    resize_workspace_windows(&self.conn, &workspace, &self.conf.border, &self.float_windows);
                }
            },
            None => {
            },
        }
    }

    fn destroy_window(&mut self, window: u32) {
        if self.float_windows.contains(&window) {
            self.float_windows.retain(|&x| x != window);
        }
        for (_, workspace) in &mut self.workspaces {
            if workspace.windows.contains(&window) {
                workspace.windows.retain(|&x| x != window);
                resize_workspace_windows(&self.conn, &workspace, &self.conf.border, &self.float_windows);
                workspace.focus = 0;
            }
        }
    }

    fn run(&mut self) {
        loop {
            match self.conn.wait_for_event() {
                Some(event) => {
                    let r = event.response_type();
                    if r == xcb::MAP_NOTIFY as u8 {
                        let map_notify : &xcb::MapNotifyEvent = unsafe {
                            xcb::cast_event(&event)
                        };
                        self.setup_new_window(map_notify.window());
                    }
                    if r == xcb::DESTROY_NOTIFY as u8 {
                        let map_notify : &xcb::DestroyNotifyEvent = unsafe {
                            xcb::cast_event(&event)
                        };
                        self.destroy_window(map_notify.window());
                    }
                    else if r == xcb::KEY_PRESS as u8 {
                        let key_press : &xcb::KeyPressEvent = unsafe {
                            xcb::cast_event(&event)
                        };
                        let keycode = key_press.detail();
                        match &keycode_to_key(keycode) {
                            Some(key) => {
                                if self.conf.workspaces_names.contains(key) {
                                    self.change_workspace(*key, (key_press.state() as u32 ) & xcb::MOD_MASK_SHIFT != 0)
                                }
                                else if self.conf.wm_actions.contains_key(&key) {
                                    self.run_wm_action(&key);
                                }
                                else if self.conf.custom_actions.contains_key(&key) {
                                    match self.conf.custom_actions.get(&key) {
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

fn main() -> Result<(), ()> {
    let mut wm_actions = HashMap::new();
    wm_actions.insert(' ', Actions::SwitchWindow);
    wm_actions.insert('w', Actions::CloseWindow);
    wm_actions.insert('f', Actions::ChangeLayout);
    let mut custom_actions : HashMap<Key, CustomAction> = HashMap::new();
    custom_actions.insert('r', Box::new(|| { Command::new("rofi").arg("-show").arg("run").spawn();}));
    custom_actions.insert('t', Box::new(|| { Command::new("kitty").spawn();}));
    custom_actions.insert('q', Box::new(|| std::process::exit(0)));

    let auto_float_types : Vec<String> = vec!["notification", "toolbar", "splash", "dialog", "popup_menu", "utility", "tooltip"].into_iter().map( |x|
            x.to_string()
        ).collect();


    let args: Vec<String> = env::args().collect();
    let conf = Conf {
        meta: if args[0] == "mod4".to_string() { xcb::MOD_MASK_4 } else { xcb::MOD_MASK_1 },
        border: Border {
            width: 2,
            focus_color: 0x906cff,
            normal_color: 0x000000,
        },
        workspaces_names: vec![ 'a', 'u', 'i', 'o', 'p' ],
        custom_actions: custom_actions,
        wm_actions: wm_actions,
        float_classes: vec!["screenkey", "audacious", "Download", "dropbox", "file_progress", "file-roller", "gimp",
                          "Komodo_confirm_repl", "Komodo_find2", "pidgin", "skype", "Transmission", "Update", "Xephyr", "obs"].into_iter().map( |x|
            x.to_string()
        ).collect(),
        auto_float_types: auto_float_types.clone(),
    };

    let (conn, _) = xcb::Connection::connect(None).unwrap();
    let workspaces = conf.workspaces_names.clone().into_iter().map( |x|
            (x, Workspace {
                layout: Layout::BSPV,
                windows: vec![],
                focus: 0,
        })).into_iter().collect();
    let mut wm = YazgooWM {
        conf: conf,
        current_workspace: 'a',
        float_windows: vec![],
        workspaces: workspaces,
        auto_float_types: window_types_from_list(&conn, &auto_float_types),
        conn: conn,
    };

    wm.init();

    wm.run();


    Ok(())
}
