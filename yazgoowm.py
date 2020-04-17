# based on tinywm

from Xlib.display import Display
from Xlib import X, XK
import subprocess

dpy = Display()

print(X.Mod1Mask)
current_workspace = "a"
keycode_to_char = {38: "a", 39: "u", 40: "i", 41: "o", 42: "p"}
workspaces = ["a", "u", "i", "o", "p"]
windows_by_workspaces = {"a": [], "u": [], "i": [], "o": [], "p": []}
# space
dpy.screen().root.grab_key(65, X.Mod1Mask, 1,
        X.GrabModeAsync, X.GrabModeAsync)
dpy.screen().root.grab_key(dpy.keysym_to_keycode(XK.string_to_keysym("r")), X.Mod1Mask, 1,
        X.GrabModeAsync, X.GrabModeAsync)
dpy.screen().root.grab_key(dpy.keysym_to_keycode(XK.string_to_keysym("f")), X.Mod1Mask, 1,
        X.GrabModeAsync, X.GrabModeAsync)
dpy.screen().root.grab_key(dpy.keysym_to_keycode(XK.string_to_keysym("w")), X.Mod1Mask, 1,
        X.GrabModeAsync, X.GrabModeAsync)
dpy.screen().root.grab_button(1, X.Mod1Mask, 1, X.ButtonPressMask|X.ButtonReleaseMask|X.PointerMotionMask,
        X.GrabModeAsync, X.GrabModeAsync, X.NONE, X.NONE)
dpy.screen().root.grab_button(3, X.Mod1Mask, 1, X.ButtonPressMask|X.ButtonReleaseMask|X.PointerMotionMask,
        X.GrabModeAsync, X.GrabModeAsync, X.NONE, X.NONE)
dpy.screen().root.change_attributes(event_mask=X.SubstructureNotifyMask)
for workspace in windows_by_workspaces.keys():
    dpy.screen().root.grab_key(dpy.keysym_to_keycode(XK.string_to_keysym(workspace[0])), X.Mod1Mask, 1,
            X.GrabModeAsync, X.GrabModeAsync)
    dpy.screen().root.grab_key(dpy.keysym_to_keycode(XK.string_to_keysym(workspace[0])), X.Mod1Mask|X.ShiftMask, 1,
            X.GrabModeAsync, X.GrabModeAsync)

start = None
current_focus = 0
colormap = dpy.screen().default_colormap
red = colormap.alloc_named_color("red").pixel
black = colormap.alloc_named_color("black").pixel
while 1:
    print("lol")
    ev = dpy.next_event()
    print(ev)
    if ev.type == X.MapNotify and not ev.window in windows_by_workspaces[current_workspace]:

        windows_by_workspaces[current_workspace].append(ev.window)
        count = len(windows_by_workspaces[current_workspace])
        if count == 1:
            ev.window.configure(x=0, y=0, width=dpy.screen().width_in_pixels / 2, height=dpy.screen().height_in_pixels)
        else:
            ev.window.configure(x=dpy.screen().width_in_pixels / 2, y=0, width=dpy.screen().width_in_pixels / 2, height=dpy.screen().height_in_pixels)
        print(windows_by_workspaces)
    if ev.type == X.DestroyNotify:
        for _, workspace_windows in windows_by_workspaces.items():
            if ev.window in workspace_windows:
                workspace_windows.remove(ev.window)
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
        print("UGUU => switch to " + current_workspace)
        current_focus = 0
    elif ev.type == X.KeyRelease and ev.detail == 65:
        window_count = len(windows_by_workspaces[current_workspace])
        if window_count > 0:
            window = windows_by_workspaces[current_workspace][current_focus]
            window.change_attributes(None,border_pixel=black, border_width = 5 )
            current_focus += 1
            current_focus = current_focus % window_count
            window = windows_by_workspaces[current_workspace][current_focus]
            window.set_input_focus(X.RevertToParent, 0)
            window.change_attributes(None,border_pixel=red, border_width = 5)

    elif ev.type == X.KeyRelease and ev.detail == 46:
        subprocess.call(["rofi", "-show", "run"])
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
