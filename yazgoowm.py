#!/usr/bin/env python
# based on tinywm

from Xlib.display import Display
from Xlib import X, XK, Xatom, error
import subprocess
import sys
import time


# <configuration>
float_classes = ( 'screenkey' 'audacious', 'Download', 'dropbox', 'file_progress', 'file-roller', 'gimp', 'ThisWindowMustFloat', 'Komodo_confirm_repl', 'Komodo_find2', 'pidgin', 'skype', 'Transmission', 'Update', 'Xephyr', 'obs', 'zoom')
meta = X.Mod4Mask if sys.argv[1] == "mod4" else X.Mod1Mask
border = {"width": 2, "focus": "#906cff", "normal": "black"}
keycode_to_char = {38: "a", 39: "u", 40: "i", 41: "o", 42: "p"}
workspaces = ["a", "u", "i", "o", "p"]
windows_by_workspaces = {"a": [], "u": [], "i": [], "o": [], "p": []}
layouts_by_workspaces = {"a": 0, "u": 0, "i": 0, "o": 0, "p": 0}
#</configuration>

layouts = ['BSPV', 'BSPH', 'Monocle']
current_workspace = workspaces[0]
float_windows = []

dpy = Display()
def string_to_keycode(dpy, key):
    return dpy.keysym_to_keycode(XK.string_to_keysym(key))

# space
dpy.screen().root.grab_key(65, meta, 1,
        X.GrabModeAsync, X.GrabModeAsync)
for key in ["r", "f", "w", "t", "q"]:
    dpy.screen().root.grab_key(string_to_keycode(dpy, key), meta, 1,
            X.GrabModeAsync, X.GrabModeAsync)
dpy.screen().root.grab_button(1, meta, 1, X.ButtonPressMask|X.ButtonReleaseMask|X.PointerMotionMask,
        X.GrabModeAsync, X.GrabModeAsync, X.NONE, X.NONE)
dpy.screen().root.grab_button(3, meta, 1, X.ButtonPressMask|X.ButtonReleaseMask|X.PointerMotionMask,
        X.GrabModeAsync, X.GrabModeAsync, X.NONE, X.NONE)
dpy.screen().root.change_attributes(event_mask=X.SubstructureNotifyMask)
for workspace in windows_by_workspaces.keys():
    dpy.screen().root.grab_key(string_to_keycode(dpy, workspace[0]), meta, 1, X.GrabModeAsync, X.GrabModeAsync)
    dpy.screen().root.grab_key(string_to_keycode(dpy, workspace[0]), meta|X.ShiftMask, 1,
            X.GrabModeAsync, X.GrabModeAsync)

start = None
current_focus = 0
colormap = dpy.screen().default_colormap
red = colormap.alloc_named_color(border["focus"]).pixel
black = colormap.alloc_named_color(border["normal"]).pixel

_NET_WM_STATE_MODAL = dpy.intern_atom(name='_NET_WM_STATE_MODAL', only_if_exists=0)
_NET_WM_WINDOW_TYPE = dpy.intern_atom('_NET_WM_WINDOW_TYPE', True)

_NET_WM_WINDOW_TYPE_NOTIFICATION = dpy.intern_atom('_NET_WM_WINDOW_TYPE_NOTIFICATION')
_NET_WM_WINDOW_TYPE_TOOLBAR = dpy.intern_atom('_NET_WM_WINDOW_TYPE_TOOLBAR')
_NET_WM_WINDOW_TYPE_SPLASH = dpy.intern_atom('_NET_WM_WINDOW_TYPE_SPLASH')
_NET_WM_WINDOW_TYPE_DIALOG = dpy.intern_atom('_NET_WM_WINDOW_TYPE_DIALOG')
auto_float_types = [_NET_WM_WINDOW_TYPE_NOTIFICATION, _NET_WM_WINDOW_TYPE_TOOLBAR, _NET_WM_WINDOW_TYPE_SPLASH, _NET_WM_WINDOW_TYPE_DIALOG]

def geometries_bsp(i, n, x, y, width, height, vertical = 1):
    if n == 0:
        return []
    elif n == 1:
        return [[x, y, width, height]]
    elif (i + vertical) % 2 == 0:
        return [[x, y, width, height / 2]] + geometries_bsp(i + 1, n - 1, x, y + height / 2, width, height / 2)
    else:
        return [[x, y, width / 2, height]] + geometries_bsp(i + 1, n - 1, x + width / 2, y, width / 2, height)

def resize_workspace_windows(windows_by_workspaces, current_workspace, dpy, border, float_windows, layout, current_focus):
    windows = []
    for window in windows_by_workspaces[current_workspace]:
        if not window in float_windows:
            windows.append(window)
    count = len(windows)
    geos = []
    if layout == 'BSPV' or layout == 'BSPH':
        geos = geometries_bsp(0, count, 0, 0, dpy.screen().width_in_pixels, dpy.screen().height_in_pixels, 1 if layout == 'BSPV' else 0)
    elif layout == 'Monocle':
        count = 1
        windows = [windows[current_focus]]
        geos = geometries_bsp(0, 1, 0, 0, dpy.screen().width_in_pixels, dpy.screen().height_in_pixels)
    for i in range(count):
        geo = geos[i]
        windows[i].configure(x = geo[0], y = geo[1], width = geo[2] - 2 * border["width"], height = geo[3] - 2 * border["width"])

while 1:
    ev = dpy.next_event()
    if ev.type == X.MapNotify and not ev.window in windows_by_workspaces[current_workspace]:
        ev.window.configure(border_width = border["width"])
        wm_class = ""
        found_window=False
        for i in range(5):
            try:
                wm_class = ev.window.get_wm_class()
                found_window=True
                break
            except error.BadWindow:
                print("got BadWindow, waiting..")
                time.sleep(0.05)
        float_window=False
        if not found_window:
            continue
        window_type = None
        try:
            window_type = ev.window.get_full_property(_NET_WM_WINDOW_TYPE, Xatom.ATOM)
        except error.BadAtom:
            pass
        print("UGUU")
        print(window_type)
        print(wm_class)
        if window_type != None:
            if window_type.value[0] in auto_float_types:
                float_window=True
        if wm_class != None and wm_class[0] in float_classes:
            float_window=True
        windows_by_workspaces[current_workspace].append(ev.window)
        if not float_window:
            resize_workspace_windows(windows_by_workspaces, current_workspace, dpy, border, float_windows,
            layouts[layouts_by_workspaces[current_workspace]], current_focus)
        else:
            float_windows.append(ev.window)
    if ev.type == X.DestroyNotify:
        for workspace, workspace_windows in windows_by_workspaces.items():
            if ev.window in workspace_windows:
                if ev.window in float_windows:
                    float_windows.remove(ev.window)
                workspace_windows.remove(ev.window)
                resize_workspace_windows(windows_by_workspaces, workspace, dpy, border, float_windows,
            layouts[layouts_by_workspaces[current_workspace]], current_focus)
                if current_workspace == workspace:
                    current_focus = 0
    elif ev.type == X.KeyPress and ev.detail in keycode_to_char.keys() and keycode_to_char[ev.detail] in windows_by_workspaces.keys():
        if ev.state & X.ShiftMask:
            if len(windows_by_workspaces[current_workspace]) > 0:
                window = windows_by_workspaces[current_workspace][current_focus]
                windows_by_workspaces[current_workspace].remove(window)
                destination_workspace = keycode_to_char[ev.detail]
                windows_by_workspaces[destination_workspace].append(window)
        for window in windows_by_workspaces[current_workspace]:
            window.unmap()
        current_workspace = keycode_to_char[ev.detail]
        for window in windows_by_workspaces[current_workspace]:
            window.map()
        current_focus = 0
    elif ev.type == X.KeyRelease and ev.detail == 65:
        window_count = len(windows_by_workspaces[current_workspace])
        if window_count > 0:
            window = windows_by_workspaces[current_workspace][current_focus]
            window.configure(border_width = border["width"])
            window.change_attributes(None,border_pixel=black)
            window.configure(stack_mode = X.Below)
            current_focus += 1
            current_focus = current_focus % window_count
            window = windows_by_workspaces[current_workspace][current_focus]
            window.set_input_focus(X.RevertToParent, 0)
            window.configure(border_width = border["width"])
            window.change_attributes(None,border_pixel=red)
            window.configure(stack_mode = X.Above)
            dpy.sync()
    elif ev.type == X.KeyRelease and ev.detail == 46:
        subprocess.call(["rofi", "-show", "run"])
    elif ev.type == X.KeyRelease and ev.detail == 44:
        subprocess.call(["kitty"])
    elif ev.type == X.KeyRelease and ev.detail == 58:
        sys.exit(1)
    elif ev.type == X.KeyRelease and ev.detail == 61:
        layouts_by_workspaces[current_workspace] = (layouts_by_workspaces[current_workspace] + 1) % len(layouts) 
        resize_workspace_windows(windows_by_workspaces, current_workspace, dpy, border, float_windows,
            layouts[layouts_by_workspaces[current_workspace]], current_focus)
    elif ev.type == X.KeyRelease and ev.detail == 35:
        window_count = len(windows_by_workspaces[current_workspace])
        if window_count > 0:
            window = windows_by_workspaces[current_workspace][current_focus]
            windows_by_workspaces[current_workspace].remove(window)
            window.destroy()
    elif ev.type == X.KeyPress and ev.child != X.NONE:
        ev.child.configure(stack_mode = X.Above)
    elif ev.type == X.KeyPress and ev.child != X.NONE:
        ev.child.configure(stack_mode = X.Above)
    elif ev.type == X.ButtonPress and ev.child != X.NONE:
        attr = ev.child.get_geometry()
        start = ev
    elif ev.type == X.MotionNotify and start:
        xdiff = ev.root_x - start.root_x
        ydiff = ev.root_y - start.root_y
        start.child.configure(
            x = attr.x + (start.detail == 1 and xdiff or 0),
            y = attr.y + (start.detail == 1 and ydiff or 0),
            width = max(1, attr.width + (start.detail == 3 and xdiff or 0)),
            height = max(1, attr.height + (start.detail == 3 and ydiff or 0)))
    elif ev.type == X.ButtonRelease:
        start = None
