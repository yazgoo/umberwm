#!/usr/bin/env python

from Xlib import X, XK, Xatom, error, display
import subprocess, sys, time

# <configuration>
meta = X.Mod4Mask if sys.argv[1] == "mod4" else X.Mod1Mask
border = {"width": 2, "focus": "#906cff", "normal": "black"}
workspaces = ["a", "u", "i", "o", "p"]
custom_actions = {"r": lambda : subprocess.call(["rofi", "-show", "run"]), "t": lambda: subprocess.call(["kitty"]), "q": lambda: sys.exit(1)}
wm_actions = {" ": 'switch_window', "w": 'close_window', "f": 'change_layout'}
float_classes = ( 'screenkey' 'audacious', 'Download', 'dropbox', 'file_progress', 'file-roller', 'gimp', 'ThisWindowMustFloat', 'Komodo_confirm_repl', 'Komodo_find2', 'pidgin', 'skype', 'Transmission', 'Update', 'Xephyr', 'obs', 'zoom')
# </configuration>

layouts = ['BSPV', 'Monocle', 'BSPH']
current_workspace = workspaces[0]
float_windows = []
windows_by_workspaces = dict([(workspace, []) for workspace in workspaces])
layouts_by_workspaces = dict([(workspace, 0) for workspace in workspaces])

def string_to_keycode(dpy, key): return dpy.keysym_to_keycode(XK.string_to_keysym(key))
def keycode_to_char(dpy, key): return dpy.lookup_string(dpy.keycode_to_keysym(key, 0))
def wm_action(dpy, key, wm_actions):
    char = keycode_to_char(dpy, key)
    if char in wm_actions:
        return wm_actions[char]
    else:
        return None

dpy = display.Display()
for key in wm_actions.keys() + custom_actions.keys() + workspaces:
    dpy.screen().root.grab_key(string_to_keycode(dpy, key), meta, 1, X.GrabModeAsync, X.GrabModeAsync)
    if key in workspaces:
        dpy.screen().root.grab_key(string_to_keycode(dpy, key), meta|X.ShiftMask, 1,
                X.GrabModeAsync, X.GrabModeAsync)
for button in [1, 3]: dpy.screen().root.grab_button(button, meta, 1, X.ButtonPressMask|X.ButtonReleaseMask|X.PointerMotionMask,
        X.GrabModeAsync, X.GrabModeAsync, X.NONE, X.NONE)
dpy.screen().root.change_attributes(event_mask=X.SubstructureNotifyMask)

start = None
current_focus = 0
border_colors = dict([(name, dpy.screen().default_colormap.alloc_named_color(border[name]).pixel) for name in ["focus", "normal"]])

_NET_WM_STATE_MODAL = dpy.intern_atom(name='_NET_WM_STATE_MODAL', only_if_exists=0)
_NET_WM_WINDOW_TYPE = dpy.intern_atom('_NET_WM_WINDOW_TYPE', True)
auto_float_types = [ dpy.intern_atom('_NET_WM_WINDOW_TYPE_NOTIFICATION'), dpy.intern_atom('_NET_WM_WINDOW_TYPE_TOOLBAR'),
        dpy.intern_atom('_NET_WM_WINDOW_TYPE_SPLASH'), dpy.intern_atom('_NET_WM_WINDOW_TYPE_DIALOG'), ]

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
        # window attributes are not directly available, this is a hack to wait for them
        for i in range(5):
            try:
                wm_class = ev.window.get_wm_class()
                found_window=True
                break
            except error.BadWindow:
                time.sleep(0.05)
        float_window=False
        if not found_window:
            continue
        window_type = None
        try:
            window_type = ev.window.get_full_property(_NET_WM_WINDOW_TYPE, Xatom.ATOM)
        except error.BadAtom:
            pass
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
    elif ev.type == X.KeyPress and keycode_to_char(dpy, ev.detail) in windows_by_workspaces.keys():
        if ev.state & X.ShiftMask:
            if len(windows_by_workspaces[current_workspace]) > 0:
                window = windows_by_workspaces[current_workspace][current_focus]
                windows_by_workspaces[current_workspace].remove(window)
                destination_workspace = keycode_to_char(dpy, ev.detail)
                windows_by_workspaces[destination_workspace].append(window)
        for window in windows_by_workspaces[current_workspace]:
            window.unmap()
        current_workspace = keycode_to_char(dpy, ev.detail)
        for window in windows_by_workspaces[current_workspace]:
            window.map()
        current_focus = 0
    elif ev.type == X.KeyRelease and wm_action(dpy, ev.detail, wm_actions) == 'switch_window':
        window_count = len(windows_by_workspaces[current_workspace])
        if window_count > 0:
            window = windows_by_workspaces[current_workspace][current_focus]
            window.configure(border_width = border["width"])
            window.change_attributes(None,border_pixel=border_colors["normal"])
            window.configure(stack_mode = X.Below)
            current_focus += 1
            current_focus = current_focus % window_count
            window = windows_by_workspaces[current_workspace][current_focus]
            window.set_input_focus(X.RevertToParent, 0)
            window.configure(border_width = border["width"])
            window.change_attributes(None,border_pixel=border_colors["focus"])
            window.configure(stack_mode = X.Above)
            dpy.sync()
    elif ev.type == X.KeyRelease and keycode_to_char(dpy, ev.detail) in [k for k in custom_actions.keys()]:
        custom_actions[keycode_to_char(dpy, ev.detail)]()
    elif ev.type == X.KeyRelease and wm_action(dpy, ev.detail, wm_actions) == 'change_layout':
        layouts_by_workspaces[current_workspace] = (layouts_by_workspaces[current_workspace] + 1) % len(layouts) 
        resize_workspace_windows(windows_by_workspaces, current_workspace, dpy, border, float_windows,
            layouts[layouts_by_workspaces[current_workspace]], current_focus)
    elif ev.type == X.KeyRelease and wm_action(dpy, ev.detail, wm_actions) == 'close_window':
        window_count = len(windows_by_workspaces[current_workspace])
        if window_count > 0:
            window = windows_by_workspaces[current_workspace][current_focus]
            windows_by_workspaces[current_workspace].remove(window)
            window.destroy()
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
