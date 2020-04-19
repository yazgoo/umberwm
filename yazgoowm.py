#!/usr/bin/env python

"""see README.md for more information"""

import subprocess
import sys
import time
from Xlib import X, XK, Xatom, error, display

def main():
    """runs the window manager, see README.md for more information"""
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

    def string_to_keycode(dpy, key):
        return dpy.keysym_to_keycode(XK.string_to_keysym(key))


    def keycode_to_char(dpy, key):
        return dpy.lookup_string(dpy.keycode_to_keysym(key, 0))


    def wm_action(dpy, key, conf):
        char = keycode_to_char(dpy, key)
        return conf["wm_actions"][char] if char in conf["wm_actions"] else None

    def geometries_bsp(i, window_count, left, top, width, height, vertical=1):
        if window_count == 0:
            return []
        if window_count == 1:
            return [[left, top, width, height]]
        if (i + vertical) % 2 == 0:
            return [[left, top, width, height / 2]] + geometries_bsp(i + 1, window_count - 1, left, top + height / 2, width, height / 2)
        return [[left, top, width / 2, height]] + geometries_bsp(i + 1, window_count - 1, left + width / 2, top, width / 2, height)


    def resize_workspace_windows(windows_by_workspaces, current_workspace, dpy, conf, float_windows, layout, current_focus):
        windows = []
        for window in windows_by_workspaces[current_workspace]:
            if not window in float_windows:
                windows.append(window)
        count = len(windows)
        if count == 0:
            return
        geos = []
        if layout in ('BSPV', 'BSPH'):
            geos = geometries_bsp(0, count, 0, 0, dpy.screen(
            ).width_in_pixels, dpy.screen().height_in_pixels, 1 if layout == 'BSPV' else 0)
        elif layout == 'Monocle':
            count = 1
            windows = [windows[current_focus]]
            geos = geometries_bsp(0, 1, 0, 0, dpy.screen(
            ).width_in_pixels, dpy.screen().height_in_pixels)
        for i in range(count):
            geo = geos[i]
            windows[i].configure(x=geo[0], y=geo[1], width=geo[2] -
                                 2 * conf["border"]["width"], height=geo[3] - 2 * conf["border"]["width"])


    def configure_window_border(windows_by_workspaces, current_workspace, current_focus, conf, border_colors, border_kind, stack_mode):
        window = windows_by_workspaces[current_workspace][current_focus]
        window.configure(border_width=conf["border"]["width"])
        window.change_attributes(None, border_pixel=border_colors[border_kind])
        window.configure(stack_mode=stack_mode)
        return window


    def get_wm_class(event):
        # window attributes are not directly available, this is a hack to wait for them
        for _ in range(5):
            try:
                return event.window.get_wm_class()
            except error.BadWindow:
                time.sleep(0.05)
        return None


    def get_window_type(event):
        window_type = None
        try:
            window_type = event.window.get_full_property(
                dpy.intern_atom('_NET_WM_WINDOW_TYPE', True), Xatom.ATOM)
        except error.BadAtom:
            pass
        return window_type

    def enable_event_listening(dpy, conf):
        for key in conf["wm_actions"].keys() + conf["custom_actions"].keys() + conf["workspaces"]:
            dpy.screen().root.grab_key(string_to_keycode(dpy, key),
                                       conf["meta"], 1, X.GrabModeAsync, X.GrabModeAsync)
            if key in conf["workspaces"]:
                dpy.screen().root.grab_key(string_to_keycode(dpy, key), conf["meta"] | X.ShiftMask, 1,
                                           X.GrabModeAsync, X.GrabModeAsync)
        for button in [1, 3]:
            dpy.screen().root.grab_button(button, conf["meta"], 1, X.ButtonPressMask | X.ButtonReleaseMask | X.PointerMotionMask,
                                          X.GrabModeAsync, X.GrabModeAsync, X.NONE, X.NONE)
        dpy.screen().root.change_attributes(event_mask=X.SubstructureNotifyMask)

    dpy = display.Display()
    layouts = ['BSPV', 'Monocle', 'BSPH']
    current_workspace = conf["workspaces"][0]
    float_windows = []
    windows_by_workspaces = {workspace: [] for workspace in conf["workspaces"]}
    layouts_by_workspaces = {workspace: 0 for workspace in conf["workspaces"]}
    mouse_move_start = None
    current_focus = 0
    border_colors = {name : dpy.screen().default_colormap.alloc_named_color(conf["border"][name]).pixel for name in ["focus", "normal"]}
    auto_float_types = [dpy.intern_atom('_NET_WM_WINDOW_TYPE_' + typ.upper()) for typ in conf["float_types"]]

    enable_event_listening(dpy, conf)

    while True:
        event = dpy.next_event()
        if event.type == X.MapNotify and not event.window in windows_by_workspaces[current_workspace]:
            event.window.configure(border_width=conf["border"]["width"])
            wm_class = get_wm_class(event)
            if wm_class is None:
                continue
            window_type = get_window_type(event)
            windows_by_workspaces[current_workspace].append(event.window)
            float_window = ((window_type is not None and window_type.value[0] in auto_float_types) or (
                wm_class is not None and wm_class[0] in conf["float_classes"]))
            if float_window:
                float_windows.append(event.window)
            else:
                resize_workspace_windows(windows_by_workspaces, current_workspace, dpy, conf, float_windows,
                                         layouts[layouts_by_workspaces[current_workspace]], current_focus)
        if event.type == X.DestroyNotify:
            for workspace, workspace_windows in windows_by_workspaces.items():
                if event.window in workspace_windows:
                    if event.window in float_windows:
                        float_windows.remove(event.window)
                    workspace_windows.remove(event.window)
                    resize_workspace_windows(windows_by_workspaces, workspace, dpy, conf, float_windows,
                                             layouts[layouts_by_workspaces[current_workspace]], current_focus)
                    if current_workspace == workspace:
                        current_focus = 0
        elif event.type == X.KeyPress and keycode_to_char(dpy, event.detail) in windows_by_workspaces.keys():
            if event.state & X.ShiftMask and len(windows_by_workspaces[current_workspace]) > 0:
                window = windows_by_workspaces[current_workspace][current_focus]
                windows_by_workspaces[current_workspace].remove(window)
                destination_workspace = keycode_to_char(dpy, event.detail)
                windows_by_workspaces[destination_workspace].append(window)
            for window in windows_by_workspaces[current_workspace]:
                window.unmap()
            current_workspace = keycode_to_char(dpy, event.detail)
            for window in windows_by_workspaces[current_workspace]:
                window.map()
            current_focus = 0
        elif event.type == X.KeyRelease and wm_action(dpy, event.detail, conf) == 'switch_window':
            window_count = len(windows_by_workspaces[current_workspace])
            if window_count > 0:
                configure_window_border(windows_by_workspaces, current_workspace,
                                        current_focus, conf, border_colors, "normal", X.Below)
                current_focus += 1
                current_focus = current_focus % window_count
                window = configure_window_border(
                    windows_by_workspaces, current_workspace, current_focus, conf, border_colors, "focus", X.Above)
                window.set_input_focus(X.RevertToParent, 0)
                dpy.sync()
        elif event.type == X.KeyRelease and keycode_to_char(dpy, event.detail) in conf["custom_actions"].keys():
            conf["custom_actions"][keycode_to_char(dpy, event.detail)]()
        elif event.type == X.KeyRelease and wm_action(dpy, event.detail, conf) == 'change_layout':
            layouts_by_workspaces[current_workspace] = (
                layouts_by_workspaces[current_workspace] + 1) % len(layouts)
            resize_workspace_windows(windows_by_workspaces, current_workspace, dpy, conf, float_windows,
                                     layouts[layouts_by_workspaces[current_workspace]], current_focus)
        elif event.type == X.KeyRelease and wm_action(dpy, event.detail, conf) == 'close_window':
            window_count = len(windows_by_workspaces[current_workspace])
            if window_count > 0:
                window = windows_by_workspaces[current_workspace][current_focus]
                windows_by_workspaces[current_workspace].remove(window)
                window.destroy()
        elif event.type == X.KeyPress and event.child != X.NONE:
            event.child.configure(stack_mode=X.Above)
        elif event.type == X.ButtonPress and event.child != X.NONE:
            attr = event.child.get_geometry()
            mouse_move_start = event
        elif event.type == X.MotionNotify and mouse_move_start:
            xdiff = event.root_x - mouse_move_start.root_x
            ydiff = event.root_y - mouse_move_start.root_y
            mouse_move_start.child.configure(
                x=attr.x + (mouse_move_start.detail == 1 and xdiff or 0),
                y=attr.y + (mouse_move_start.detail == 1 and ydiff or 0),
                width=max(1, attr.width +
                          (mouse_move_start.detail == 3 and xdiff or 0)),
                height=max(1, attr.height + (mouse_move_start.detail == 3 and ydiff or 0)))
        elif event.type == X.ButtonRelease:
            mouse_move_start = None
main()
