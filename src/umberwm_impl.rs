use crate::error::{Error, LogError, Result};
use crate::geometries::geometries_bsp;
mod helpers;
use crate::keycode;
use crate::model::*;
mod resize;
use crate::serializable_state::UMBERWM_STATE;
use helpers::{
    change_workspace, get_atom_property, get_display_border, get_displays_geometries,
    get_str_property, is_firefox_drag_n_drop_initialization_window, run_command,
    window_types_from_list,
};
use resize::{resize_bsp, resize_monocle};
use ron::ser::to_string;
use std::cmp::max;
use std::fs::File;
use std::io::prelude::*;
use xcb::randr;
use xcb::xproto;

impl UmberWm {
    fn resize_workspace_windows(&mut self, workspace: &Workspace, mut display: usize) {
        let mut non_float_windows = workspace.windows.clone();
        non_float_windows.retain(|w| !self.float_windows.contains(w));
        let count = non_float_windows.len();
        if count == 0 || self.displays_geometries.is_empty() {
            return;
        }
        if display >= self.displays_geometries.len() {
            display = self.displays_geometries.len() - 1;
        }
        let display_border = get_display_border(&self.conf.serializable.display_borders, display);
        let display_geometry = self.displays_geometries.get(display).unwrap();
        let width = display_geometry.2 as u32 - display_border.right - display_border.left;
        let height = display_geometry.3 as u32 - display_border.top - display_border.bottom;
        let left = display_geometry.0 + display_border.left;
        let top = display_geometry.1 + display_border.top;
        let gap = if self.conf.serializable.with_gap {
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
            Layout::Bspv | Layout::Bsph => resize_bsp(
                &self.conn,
                self.conf.serializable.border.width,
                non_float_windows,
                geos,
                gap,
            ),
            Layout::Monocle => resize_monocle(
                &self.conn,
                self.conf.serializable.border.width,
                workspace,
                geos,
                gap,
            ),
        }
        for (i, window) in workspace.windows.iter().enumerate() {
            self.focus_unfocus(window, i == workspace.focus).log();
        }
        for overlay_window in &self.overlay_windows {
            xcb::configure_window(
                &self.conn,
                *overlay_window,
                &[(xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE)],
            );
        }
    }

    pub fn init(&mut self) {
        self.displays_geometries = get_displays_geometries(&self.conn).unwrap();
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
        .log();
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
                self.conf.serializable.meta as u16,
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
        let custom_actions_keys: Vec<&Keybind> = self.conf.custom_actions.keys().collect();
        for list in vec![
            custom_actions_keys,
            self.conf.serializable.custom_commands.keys().collect(),
        ] {
            for keybind in list {
                keycode::key_to_keycode(&self.xmodmap_pke, &keybind.key).map(|keycode| {
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
    }

    fn grab_wm_action_keys(&self, screen: &xcb::Screen) {
        for keybind in self.conf.serializable.wm_actions.keys() {
            keycode::key_to_keycode(&self.xmodmap_pke, &keybind.key).map(|keycode| {
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
        for mod_mask in &[
            self.conf.serializable.meta,
            self.conf.serializable.meta | xcb::MOD_MASK_SHIFT,
        ] {
            for workspace_name_in_display in &self.conf.serializable.workspaces_names {
                for workspace_name in workspace_name_in_display {
                    keycode::key_to_keycode(&self.xmodmap_pke, workspace_name).map(|keycode| {
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

    fn focus_unfocus(&mut self, window: &xcb::Window, do_focus: bool) -> Result<()> {
        let mut border_focus = false;
        if do_focus {
            xcb::set_input_focus(&self.conn, xcb::INPUT_FOCUS_PARENT as u8, *window, 0);
            let workspace = self
                .workspaces
                .get_mut(&self.current_workspace)
                .ok_or(Error::WorkspaceNotFound)?;
            if let Some(i) = workspace.windows.iter().position(|x| x == window) {
                workspace.focus = i;
            }
            border_focus = !((workspace.windows.len() == 1 || workspace.layout == Layout::Monocle)
                && self.displays_geometries.len() == 1);
            let net_active_window = xcb::intern_atom(&self.conn, false, "_NET_ACTIVE_WINDOW")
                .get_reply()?
                .atom();
            let setup = self.conn.get_setup();
            let root = setup.roots().next().ok_or(Error::NoScreensFound)?.root();
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
                    self.conf.serializable.border.focus_color
                } else {
                    self.conf.serializable.border.normal_color
                },
            )],
        );
        Ok(())
    }

    fn serialize_and_quit(&mut self) -> Result<()> {
        let mut file = File::create(UMBERWM_STATE)?;
        let string = to_string(&SerializableState {
            float_windows: self.float_windows.clone(),
            overlay_windows: self.overlay_windows.clone(),
            workspaces: self.workspaces.clone(),
            current_workspace: self.current_workspace.clone(),
        })?;
        file.write_all(string.as_bytes())?;
        std::process::exit(123);
    }

    fn run_wm_action(&mut self, keybind: &Keybind) -> Result<()> {
        let workspaces_names_by_display = self.conf.serializable.workspaces_names.clone();
        let action = self
            .conf
            .serializable
            .wm_actions
            .get(keybind)
            .ok_or(Error::ActionNotFound)?;
        let workspace = self
            .workspaces
            .get_mut(&self.current_workspace)
            .ok_or(Error::WorkspaceNotFound)?;
        match action {
            Actions::CloseWindow => {
                let window = workspace
                    .windows
                    .get(workspace.focus)
                    .ok_or(Error::WindowNotFound)?;
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
                self.serialize_and_quit().log();
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
                self.conf.serializable.with_gap = !self.conf.serializable.with_gap;
            }
            Actions::Quit => std::process::exit(0),
        };
        for (display, workspaces_names) in workspaces_names_by_display.iter().enumerate() {
            if workspaces_names.contains(&self.current_workspace) {
                let workspace = self
                    .workspaces
                    .get(&self.current_workspace)
                    .ok_or(Error::WorkspaceNotFound)?
                    .clone();
                self.resize_workspace_windows(&workspace, display);
            }
        }
        Ok(())
    }

    fn setup_new_window(&mut self, window: u32) -> Result<()> {
        for workspace in self.workspaces.values() {
            for workspace_window in &workspace.windows {
                if &window == workspace_window {
                    // The window already exist in a workspace
                    return Ok(());
                }
            }
        }
        let wm_class =
            get_str_property(&self.conn, window, "WM_CLASS").ok_or(Error::FailedToGetWmClass)?;
        let window_type = get_atom_property(&self.conn, window, "_NET_WM_WINDOW_TYPE")?;
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
                "dnd".to_string(),
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
        if !wm_class.is_empty()
            && self
                .conf
                .serializable
                .overlay_classes
                .contains(&wm_class[0].to_string())
            && !self.overlay_windows.contains(&window)
        {
            self.overlay_windows.push(window);
            return Ok(());
        }
        if window_types.contains(&window_type)
            || "_KDE_NET_WM_WINDOW_TYPE_OVERRIDE"
                == xcb::get_atom_name(&self.conn, window_type)
                    .get_reply()?
                    .name()
        {
            return Ok(());
        }
        let mut target_workspace = self.current_workspace.clone();
        if !wm_class.is_empty() {
            if is_firefox_drag_n_drop_initialization_window(&self.conn, window, &wm_class)? {
                return Ok(());
            }
            for item in &wm_class {
                if item == &"xscreensaver"
                    || self
                        .conf
                        .serializable
                        .ignore_classes
                        .contains(&item.to_string())
                {
                    return Ok(());
                }
                if let Some(sticky_workspace) =
                    self.conf.serializable.sticky_classes.get(&item.to_string())
                {
                    target_workspace = sticky_workspace.clone();
                }
            }
        }
        if let Some(workspace) = self.workspaces.get_mut(&target_workspace) {
            if !workspace.windows.contains(&window) {
                if !wm_class.is_empty()
                    && self
                        .conf
                        .serializable
                        .float_classes
                        .contains(&wm_class[0].to_string())
                    && !self.float_windows.contains(&window)
                {
                    self.float_windows.push(window);
                }
                workspace.windows.push(window);
                workspace.focus = workspace.windows.len() - 1;
                let workspace2 = workspace.clone();
                let workspaces_names_by_display = self.conf.serializable.workspaces_names.clone();
                for (display, workspaces_names) in workspaces_names_by_display.iter().enumerate() {
                    if workspaces_names.contains(&self.current_workspace)
                        && self.current_workspace == target_workspace
                    {
                        self.resize_workspace_windows(&workspace2, display);
                    }
                }
            }
        }
        if self.current_workspace == target_workspace {
            xcb::change_window_attributes(
                &self.conn,
                window,
                &[(
                    xcb::CW_EVENT_MASK,
                    xcb::EVENT_MASK_ENTER_WINDOW | xcb::EVENT_MASK_LEAVE_WINDOW,
                )],
            );
        } else {
            xcb::unmap_window(&self.conn, window);
        }
        Ok(())
    }

    fn resize_window(&mut self, event: &xcb::MotionNotifyEvent) -> Result<()> {
        let mouse_move_start = self
            .mouse_move_start
            .clone()
            .ok_or(Error::NoMouseMoveStart)?;
        let attr = self
            .button_press_geometry
            .clone()
            .ok_or(Error::NoButtonPressGeometry)?;
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
        self.overlay_windows.retain(|&x| x != window);
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
        let workspaces_names_by_display = self.conf.serializable.workspaces_names.clone();
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
                    self.setup_new_window(map_notify.window()).log();
                }
                if r == self.randr_base + randr::NOTIFY {
                    self.displays_geometries = get_displays_geometries(&self.conn).unwrap();
                }
                if r == xcb::DESTROY_NOTIFY as u8 {
                    let map_notify: &xcb::DestroyNotifyEvent = unsafe { xcb::cast_event(&event) };
                    self.destroy_window(map_notify.window());
                } else if r == xcb::BUTTON_PRESS as u8 {
                    let event: &xcb::ButtonPressEvent = unsafe { xcb::cast_event(&event) };
                    self.handle_button_press(event);
                } else if r == xcb::MOTION_NOTIFY as u8 {
                    let event: &xcb::MotionNotifyEvent = unsafe { xcb::cast_event(&event) };
                    self.resize_window(event).log();
                } else if r == xcb::LEAVE_NOTIFY as u8 {
                    let event: &xcb::LeaveNotifyEvent = unsafe { xcb::cast_event(&event) };
                    self.focus_unfocus(&event.event(), false).log();
                } else if r == xcb::ENTER_NOTIFY as u8 {
                    let event: &xcb::EnterNotifyEvent = unsafe { xcb::cast_event(&event) };
                    self.focus_unfocus(&event.event(), true).log();
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
        if let Some(key) = &keycode::keycode_to_key(&self.xmodmap_pke, keycode) {
            let keybind = Keybind::new(mod_mask, key);

            self.handle_workspace_change(&keybind);

            if self.conf.serializable.wm_actions.contains_key(&keybind) {
                self.run_wm_action(&keybind).log();
            } else if self.conf.custom_actions.contains_key(&keybind) {
                if let Some(action) = self.conf.custom_actions.get(&keybind) {
                    action();
                }
            } else if self
                .conf
                .serializable
                .custom_commands
                .contains_key(&keybind)
            {
                run_command(self.conf.serializable.custom_commands.get(&keybind));
            }
        }
    }

    fn handle_workspace_change(&mut self, keybind: &Keybind) {
        let workspaces_names_by_display = self.conf.serializable.workspaces_names.clone();
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
                        .ok_or(Error::WorkspaceNotFound)
                        .log()
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
                    run_command(
                        self.conf
                            .serializable
                            .command_callbacks
                            .get(&Events::OnChangeWorkspace),
                    );
                }
            }
        }
    }
}
