# umberwm :ram:

[![Discord](https://img.shields.io/badge/discord--blue?logo=discord)](https://discord.gg/F684Y8rYwZ)

Minimalistic X window manager based on tinywm, inspired by qtile.

![UmberWM Screenshot](screenshot.jpg)

Video introduction [on LBRY][lbry] or [on youtube][yt].

# Design goals and features

  - Kiss: only window management (no taskbar, system tray, ...), complex stuff should be done using
    other programs (`rofi`, ...)
  - Configuration as code (like `qtile`, `dwm`, `xmonad`)
  - Tiled by default (Binary space partitioning)
  - Supports workspaces
  - Supports multiple displays
  - Single file (~1000 LoC), not counting configuration (`main.rs`)

# Prerequisites

You should have xmodmap installed.

You will need XCB bindings with the randr extension.

Ubuntu: `sudo apt install libxcb-randr0-dev`.

# Using it

`UmberWM` is used/configured in rust, here is how to use it:

1. [Install rust and cargo][install-rust]
2. Clone this project: `git clone https://github.com/yazgoo/umberwm`
    + Note: if you don't want to modify the source code, you can instead add `umberwm` as a
      dependency to your own project. See [using it as a dependency](#using-it-as-a-dependency).
3. Edit `src/main.rs`.
4. Edit `umberwm-start` if desired. Here you can launch any programs you need to before launching
   `umberwm`.
5. Run `cargo build --release`. The binary will be available in `target/release/umberwm`.
6. Optionally, run `./install.py`. This will do three things:
    1. Symlink `target/release/umberwm` to `/usr/bin`.
    2. Symlink `umberwm-start` to `/usr/bin`.
    3. Copy `umberwm.desktop` to `/usr/share/xsessions`. This will allow display managers such as
       `GDM` to find `umberwm` and allow you to launch it.
7. If you do not use a display manager, you will need to add the following to your `.xinitrc`:

    ```sh
    exec umberwm-start
    ```

## Using it as a dependency

If you don't want to modify the source code, you can create your own rust project and add `umberwm`
as a `cargo` dependency.

`Cargo.toml`:
```toml
# ...
[dependencies]
umberwm = "0.0.21"
```

You can then supply your own `main.rs` rather than editing the existing one. It is advised that you
use `main.rs` from this repository as your starting point.

See [yazgoo/myumberwm](https://github.com/yazgoo/myumberwm) for an example.

Note that you will have to manually set up `umberwm-start` and `umberwm.desktop` if you wish to use
them.

## Hot reloading

Hot reloading allows to restart `umberwm` while keeping its state (i.e. keeping track of windows and
their relative workspaces).
This is quite useful when you want to update your configuration.

In `wm_actions:`, the action `Actions::SerializeAndQuit` will serialize all of its windows and then
quit with exit code `123`.
The `umberwm-start` script checks for the exit code `123` and reruns `umberwm`, thereby facilitating
a smooth restart.

[lbry]: https://open.lbry.com/@goo:c/umberwm:e?r=FKWhS2Vay3CVr66qMZD98HdsLQ2LN7za
[yt]: https://youtu.be/5XdFNEq69N0
[install-rust]: https://doc.rust-lang.org/cargo/getting-started/installation.html
