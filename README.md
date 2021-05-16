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
4. Run `cargo build --release`. The binary will be available in `target/release/umerwm`.
5. Configure your system to launch `umberwm`:
    + If you launch your window managers with `startx`, add the following to your `.xinitrc`:

        ```sh
        exec /path/to/myumberwm mod4
        ```

    + If you use a display manager (such as `GDM`, `SDDM` or `LightDM`), you will need a file named
      `umberwm.desktop` in `/usr/share/xsessions`. That file should look like this:

        ```ini
        [Desktop Entry]
        Encoding=UTF-8
        Name=UmberWM
        Comment=Rusty window manager
        Exec=/path/to/umberwm
        Type=XSession
        ```

## Using it as a dependency

If you don't want to modify the source code, you can create your own rust project and add `umberwm`
as a `cargo` dependency.

`Cargo.toml`:
```toml
# ...
[dependencies]
umberwm = "0.0.19"
```

<<<<<<< HEAD
You can then supply your own `main.rs` rather than editing the existing one. It is advised that you
use `main.rs` from this repository as your starting point.

[lbry]: https://open.lbry.com/@goo:c/umberwm:e?r=FKWhS2Vay3CVr66qMZD98HdsLQ2LN7za
[yt]: https://youtu.be/5XdFNEq69N0
[install-rust]: https://doc.rust-lang.org/cargo/getting-started/installation.html
=======
## hot reloading

Hot reloading allows to restart umberwm while keeping its state (i.e. keeping track of windows and their relative workspaces).
This is quite useful when you want to update your configuration.
You can add hot reload in your `.xinitrc` by running umberwm in a loop via:

```bash
while true; do
  echo starting umberwm...
  /path/to/umberwm mod4
  [ $? -ne 123 ] && break
done
```

in your `wm_actions`, add

```rust
("d".to_string(), Actions::SerializeAndQuit),
```

In this example, when pushing 'mod4 + d', umberwm will serialize a state under `.umberwm_state` and exit with return code `123`.
Then the `xinitrc` code will detect code `123`, causing a restart, umberwm will detect the serialized state, load it at startup and delete it.
>>>>>>> master
