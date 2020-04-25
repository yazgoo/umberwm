extern crate xcb;

use std::collections::HashMap;
use std::process::Command;

enum Actions {
    SwitchWindow, CloseWindow, ChangeLayout,
}

enum Layout {
    BSPV, Monocle, BSPH
}

type Window = u32;

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
    wm_actions: HashMap<Key, Actions>,
    float_classes: Vec<String>,
    auto_float_types: Vec<String>,
}

struct YazgooWM {
    conf: Conf,
    current_workspace: WorkspaceName,
    float_windows: Vec<Window>,
    workspaces: HashMap<WorkspaceName, Workspace>,
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

impl YazgooWM {

    fn init(&mut self) {
        let setup = self.conn.get_setup();
        let screen = setup.roots().nth(0).unwrap();
        for mod_mask in vec![xcb::MOD_MASK_1, xcb::MOD_MASK_1 | xcb::MOD_MASK_SHIFT] {
            for workspace_name in &self.conf.workspaces_names {
                xcb::grab_key(&self.conn, false, screen.root(), mod_mask as u16, key_to_keycode(workspace_name).unwrap(), xcb::GRAB_MODE_ASYNC as u8, xcb::GRAB_MODE_ASYNC as u8);
            }
            for custom_action_key in self.conf.custom_actions.keys() {
                xcb::grab_key(&self.conn, false, screen.root(), mod_mask as u16, key_to_keycode(custom_action_key).unwrap(), xcb::GRAB_MODE_ASYNC as u8, xcb::GRAB_MODE_ASYNC as u8);
            }
        }
        xcb::change_window_attributes(&self.conn, screen.root(), &[(xcb::CW_EVENT_MASK, xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY as u32)]);
        self.conn.flush();
    }

    fn change_workspace(&mut self, workspace: WorkspaceName, move_window: bool) {
        println!("change worspace");
        match self.workspaces.get(&self.current_workspace) {
            Some(previous_workspace) => {
                for window in &previous_workspace.windows {
                    println!("unmap");
                    xcb::unmap_window(&self.conn, *window);
                }
            }
            None => {}
        };
        self.current_workspace = workspace;
        match self.workspaces.get(&self.current_workspace) {
            Some(previous_workspace) => {
                for window in &previous_workspace.windows {
                    println!("map");
                    xcb::map_window(&self.conn, *window);
                }
            }
            None => {}
        };
    }

    fn setup_new_window(&mut self, window: u32) {
        println!("setup new window");
        match self.workspaces.get_mut(&self.current_workspace) {
            Some(workspace) => {
                println!("push window");
                workspace.windows.push(window);
            },
            None => {
                println!("current workspace not found");
            },
        }
    }

    fn destroy_window(&mut self, window: u32) {
        for (_, workspace) in &mut self.workspaces {
            if workspace.windows.contains(&window) {
                workspace.windows.retain(|&x| x != window);
            }
        }
    }

    fn run(&mut self) {
        loop {
            println!("in loop");
            match self.conn.wait_for_event() {
                Some(event) => {
                    println!("event");
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
                        println!("{:?}", key_press.detail());
                        match &keycode_to_key(keycode) {
                            Some(key) => {
                                println!("{:?}", key);
                                if self.conf.workspaces_names.contains(key) {
                                    self.change_workspace(*key, (key_press.state() as u32 ) & xcb::MOD_MASK_SHIFT != 0)
                                }
                                else if self.conf.wm_actions.contains_key(&key) {
                                }
                                else if self.conf.custom_actions.contains_key(&key) {
                                    match self.conf.custom_actions.get(&key) {
                                        Some(action) => {
                                            println!("run action {}", &key);
                                            action()
                                        },
                                        None => {},
                                    }
                                }
                            },
                            None => {
                                println!("error decoding key");
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

    let conf = Conf {
        meta: String::from("mod1"),
        border: Border {
            width: 2,
            focus_color: String::from("#906cff"),
            normal_color: String::from("black"),
        },
        workspaces_names: vec![ 'a', 'u', 'i', 'o', 'p' ],
        custom_actions: custom_actions,
        wm_actions: wm_actions,
        float_classes: vec![],
        auto_float_types: vec![],
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
        conn: conn,
    };

    wm.init();

    wm.run();


    Ok(())
}
