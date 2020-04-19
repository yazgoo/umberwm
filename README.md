yazgoowm is a minimalistic X window manager based on tinywm, inspired by qtile, and tailored for me (hence the name).

# design goals and features

  - kiss: only window management (no taskbar, ...), complex stuff should be done using other programs (rofi, ...)
  - configuration as code (like qtile, dwm)
  - tiled by default (Binary space partitioning)
  - supports workspaces
  - single file, ~200 LoC

# using it

put in your .xinitrc

```shell
exec /path/to/yazgoowm.py mod4
```
