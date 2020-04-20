#!/usr/bin/env python

import subprocess, sys, time
from Xlib import X, XK, Xatom, error, display

class YazgooWM:

    def string_to_keycode(self, dpy, key):
        return dpy.keysym_to_keycode(XK.string_to_keysym(key))

    def keycode_to_char(self, dpy, key):
        return dpy.lookup_string(dpy.keycode_to_keysym(key, 0))

    def wm_action(self, dpy, key, conf):
        char = self.keycode_to_char(dpy, key)
        return conf["wm_actions"][char] if char in conf["wm_actions"] else None

    def geometries_bsp(self, i, window_count, left, top, width, height, vertical=1):
        if window_count == 0:
            return []
        if window_count == 1:
            return [[left, top, width, height]]
        if (i + vertical) % 2 == 0:
            return [[left, top, width, height / 2]] + self.geometries_bsp(i + 1, window_count - 1, left, top + height / 2, width, height / 2)
        return [[left, top, width / 2, height]] + self.geometries_bsp(i + 1, window_count - 1, left + width / 2, top, width / 2, height)

    def resize_workspace_windows(self, windows_by_workspaces, current_workspace, dpy, conf, float_windows, layout, foci_by_workspace):
        windows = []
        for window in windows_by_workspaces[current_workspace]:
            if not window in float_windows:
                windows.append(window)
        count = len(windows)
        if count == 0:
            return
        geos = []
        if layout in ('BSPV', 'BSPH'):
            geos = self.geometries_bsp(0, count, 0, 0, dpy.screen(
            ).width_in_pixels, dpy.screen().height_in_pixels, 1 if layout == 'BSPV' else 0)
        elif layout == 'Monocle':
            count = 1
            windows = [windows[foci_by_workspace[current_workspace]]]
            geos = self.geometries_bsp(0, 1, 0, 0, dpy.screen(
            ).width_in_pixels, dpy.screen().height_in_pixels)
        for i in range(count):
            geo = geos[i]
            windows[i].configure(x=geo[0], y=geo[1], width=geo[2] -
                                 2 * conf["border"]["width"], height=geo[3] - 2 * conf["border"]["width"])

    def configure_window_border(self, windows_by_workspaces, current_workspace, foci_by_workspace, conf, border_colors, border_kind, stack_mode):
        window = windows_by_workspaces[current_workspace][foci_by_workspace[current_workspace]]
        window.configure(border_width=conf["border"]["width"])
        window.change_attributes(None, border_pixel=border_colors[border_kind])
        window.configure(stack_mode=stack_mode)
        return window

    def get_wm_class(self, event):
        # window attributes are not directly available, this is a hack to wait for them
        for _ in range(5):
            try:
                return event.window.get_wm_class()
            except error.BadWindow:
                time.sleep(0.05)
        return None


    def get_window_type(self, dpy, event):
        window_type = None
        try:
            window_type = event.window.get_full_property(
                dpy.intern_atom('_NET_WM_WINDOW_TYPE', True), Xatom.ATOM)
        except error.BadAtom:
            pass
        return window_type

    def enable_event_listening(self, dpy, conf):
        for key in conf["wm_actions"].keys() + conf["custom_actions"].keys() + conf["workspaces"]:
            dpy.screen().root.grab_key(self.string_to_keycode(dpy, key),
                                       conf["meta"], 1, X.GrabModeAsync, X.GrabModeAsync)
            if key in conf["workspaces"]:
                dpy.screen().root.grab_key(self.string_to_keycode(dpy, key), conf["meta"] | X.ShiftMask, 1,
                                           X.GrabModeAsync, X.GrabModeAsync)
        for button in [1, 3]:
            dpy.screen().root.grab_button(button, conf["meta"], 1, X.ButtonPressMask | X.ButtonReleaseMask | X.PointerMotionMask,
                                          X.GrabModeAsync, X.GrabModeAsync, X.NONE, X.NONE)
        dpy.screen().root.change_attributes(event_mask=X.SubstructureNotifyMask)

    def __init__(self, conf):
        self.dpy = display.Display()
        self.layouts = ['BSPV', 'Monocle', 'BSPH']
        self.current_workspace = conf["workspaces"][0]
        self.float_windows = []
        self.windows_by_workspaces = {workspace: [] for workspace in conf["workspaces"]}
        self.layouts_by_workspaces = {workspace: 0 for workspace in conf["workspaces"]}
        self.foci_by_workspace = {workspace: 0 for workspace in conf["workspaces"]}
        self.mouse_move_start = None
        self.border_colors = {name : self.dpy.screen().default_colormap.alloc_named_color(conf["border"][name]).pixel for name in ["focus", "normal"]}
        self.auto_float_types = [self.dpy.intern_atom('_NET_WM_WINDOW_TYPE_' + typ.upper()) for typ in conf["float_types"]]

        self.enable_event_listening(self.dpy, conf)
        self.conf = conf

    def setup_new_window(self, event):
        event.window.configure(border_width=self.conf["border"]["width"])
        wm_class = self.get_wm_class(event)
        if wm_class is None:
            return
        window_type = self.get_window_type(self.dpy, event)
        self.windows_by_workspaces[self.current_workspace].append(event.window)
        float_window = ((window_type is not None and window_type.value[0] in self.auto_float_types) or (
            wm_class is not None and wm_class[0] in self.conf["float_classes"]))
        self.foci_by_workspace[self.current_workspace] = len(self.windows_by_workspaces[self.current_workspace]) - 1
        if float_window:
            self.float_windows.append(event.window)
        else:
            self.resize_workspace_windows(self.windows_by_workspaces, self.current_workspace, self.dpy, self.conf, self.float_windows,
                                     self.layouts[self.layouts_by_workspaces[self.current_workspace]], self.foci_by_workspace)

    def destroy_window(self, event):
        for workspace, workspace_windows in self.windows_by_workspaces.items():
            if event.window in workspace_windows:
                if event.window in self.float_windows:
                    self.float_windows.remove(event.window)
                workspace_windows.remove(event.window)
                self.foci_by_workspace[workspace] = 0
                self.resize_workspace_windows(self.windows_by_workspaces, workspace, self.dpy, self.conf, self.float_windows,
                                         self.layouts[self.layouts_by_workspaces[workspace]], self.foci_by_workspace)

    def change_workspace(self, event):
        if event.state & X.ShiftMask and len(self.windows_by_workspaces[self.current_workspace]) > 0:
            window = self.windows_by_workspaces[self.current_workspace][self.foci_by_workspace[self.current_workspace]]
            self.windows_by_workspaces[self.current_workspace].remove(window)
            destination_workspace = self.keycode_to_char(self.dpy, event.detail)
            self.windows_by_workspaces[destination_workspace].append(window)
            for workspace in (self.current_workspace, destination_workspace):
                self.resize_workspace_windows(self.windows_by_workspaces, workspace, self.dpy, self.conf, self.float_windows,
                    self.layouts[self.layouts_by_workspaces[workspace]], self.foci_by_workspace)
        for window in self.windows_by_workspaces[self.current_workspace]:
            window.unmap()
        self.current_workspace = self.keycode_to_char(self.dpy, event.detail)
        for window in self.windows_by_workspaces[self.current_workspace]:
            window.map()
        if len(self.windows_by_workspaces[self.current_workspace]) > 0:
            window = self.windows_by_workspaces[self.current_workspace][self.foci_by_workspace[self.current_workspace]]
            window.set_input_focus(X.RevertToParent, 0)

    def switch_window(self):
        window_count = len(self.windows_by_workspaces[self.current_workspace])
        if window_count > 0:
            self.configure_window_border(self.windows_by_workspaces, self.current_workspace,
                                    self.foci_by_workspace, self.conf, self.border_colors, "normal", X.Below)
            self.foci_by_workspace[self.current_workspace] += 1
            self.foci_by_workspace[self.current_workspace] = self.foci_by_workspace[self.current_workspace] % window_count
            window = self.configure_window_border(
                self.windows_by_workspaces, self.current_workspace, self.foci_by_workspace, self.conf, self.border_colors, "focus", X.Above)
            window.set_input_focus(X.RevertToParent, 0)
            self.dpy.sync()

    def change_layout(self):
        self.layouts_by_workspaces[self.current_workspace] = (
            self.layouts_by_workspaces[self.current_workspace] + 1) % len(self.layouts)
        self.resize_workspace_windows(self.windows_by_workspaces, self.current_workspace, self.dpy, self.conf, self.float_windows,
                                 self.layouts[self.layouts_by_workspaces[self.current_workspace]], self.foci_by_workspace)

    def close_window(self):
        window_count = len(self.windows_by_workspaces[self.current_workspace])
        if window_count > 0:
            window = self.windows_by_workspaces[self.current_workspace][self.foci_by_workspace[self.current_workspace]]
            self.windows_by_workspaces[self.current_workspace].remove(window)
            window.destroy()

    def resize_window(self, event):
        xdiff = event.root_x - self.mouse_move_start.root_x
        ydiff = event.root_y - self.mouse_move_start.root_y
        self.mouse_move_start.child.configure(
            x=self.attr.x + (self.mouse_move_start.detail == 1 and xdiff or 0),
            y=self.attr.y + (self.mouse_move_start.detail == 1 and ydiff or 0),
            width=max(1, self.attr.width +
                      (self.mouse_move_start.detail == 3 and xdiff or 0)),
            height=max(1, self.attr.height + (self.mouse_move_start.detail == 3 and ydiff or 0)))

    def run(self):
        while True:
            event = self.dpy.next_event()
            if event.type == X.MapNotify and not event.window in self.windows_by_workspaces[self.current_workspace]:
                self.setup_new_window(event)
            elif event.type == X.DestroyNotify:
                self.destroy_window(event)
            elif event.type == X.KeyPress and self.keycode_to_char(self.dpy, event.detail) in self.windows_by_workspaces.keys():
                self.change_workspace(event)
            elif event.type == X.KeyRelease and self.wm_action(self.dpy, event.detail, self.conf) == 'switch_window':
                self.switch_window()
            elif event.type == X.KeyRelease and self.keycode_to_char(self.dpy, event.detail) in self.conf["custom_actions"].keys():
                self.conf["custom_actions"][self.keycode_to_char(self.dpy, event.detail)]()
            elif event.type == X.KeyRelease and self.wm_action(self.dpy, event.detail, self.conf) == 'change_layout':
                self.change_layout()
            elif event.type == X.KeyRelease and self.wm_action(self.dpy, event.detail, self.conf) == 'close_window':
                self.close_window()
            elif event.type == X.KeyPress and event.child != X.NONE:
                event.child.configure(stack_mode=X.Above)
            elif event.type == X.ButtonPress and event.child != X.NONE:
                self.attr = event.child.get_geometry()
                self.mouse_move_start = event
            elif event.type == X.MotionNotify and self.mouse_move_start:
                self.resize_window(event)
            elif event.type == X.ButtonRelease:
                self.mouse_move_start = None
YazgooWM(
    conf = {
        "meta" : X.Mod4Mask if sys.argv[1] == "mod4" else X.Mod1Mask,
        "border": {"width": 2, "focus": "#906cff", "normal": "black"},
        "workspaces": ["a", "u", "i", "o", "p"],
        "custom_actions": {"r": lambda: subprocess.call(["rofi", "-show", "run"]), "t": lambda: subprocess.call(["kitty"]), "q": lambda: sys.exit(1)},
        "wm_actions": {" ": 'switch_window', "w": 'close_window', "f": 'change_layout'},
        "float_classes": ('screenkey', 'audacious', 'Download', 'dropbox', 'file_progress', 'file-roller', 'gimp',
                          'Komodo_confirm_repl', 'Komodo_find2', 'pidgin', 'skype', 'Transmission', 'Update', 'Xephyr', 'obs', 'zoom'),
        "float_types": ('notification', 'toolbar', 'splash', 'dialog'),
        }
        ).run()
