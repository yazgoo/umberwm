# umberwm :ram:

Minimalistic X window manager based on tinywm, inspired by qtile.

![UmberWM Screenshot](screenshot.jpg)

video introduction [on LBRY](https://open.lbry.com/@goo:c/umberwm:e?r=FKWhS2Vay3CVr66qMZD98HdsLQ2LN7za) or [on youtube](https://youtu.be/5XdFNEq69N0).

# prerequisites

you should have xmodmap installed

# design goals and features

  - kiss: only window management (no taskbar, system tray, ...), complex stuff should be done using other programs (rofi, ...)
  - configuration as code (like qtile, dwm, xmonad)
  - tiled by default (Binary space partitioning)
  - supports workspaces
  - supports multiple displays
  - single file (~600 LoC)

# using it

umberwm is used/configured in rust, here is how to use it:

1. install rust and cargo https://doc.rust-lang.org/cargo/getting-started/installation.html
2. clone template project (__:warning: it is a different repository__): `git clone https://github.com/yazgoo/myumberwm`
3. edit src/main.rs (see comments for more details)
4. run `cargo build`, binary is available in target/debug/myumerwm

add the following to your .xinitrc :

```shell
exec /path/to/myumberwm mod4
```
