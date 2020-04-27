a minimalistic X window manager based on tinywm, inspired by qtile.

# Demo

[![Demo](https://img.youtube.com/vi/syz2i6MyOAg/0.jpg)](https://www.youtube.com/watch?v=syz2i6MyOAg)

# prerequisites

you should have xmodmap installed

# design goals and features

  - kiss: only window management (no taskbar, system tray, ...), complex stuff should be done using other programs (rofi, ...)
  - configuration as code (like qtile, dwm)
  - tiled by default (Binary space partitioning)
  - supports workspaces
  - two implementations: python and rust
  - single file (python: ~200 LoC, rust: ~500 LoC)

# using it

You should generate your binary based on the crate (see src/main.rs) for an example.

put in your .xinitrc

```shell
exec /path/to/umberwm mod4
```
