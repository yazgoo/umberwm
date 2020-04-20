yazgoowm is a minimalistic X window manager based on tinywm, inspired by qtile, and tailored for me (hence the name).

<iframe width="1280" height="720" src="https://www.youtube.com/embed/syz2i6MyOAg" frameborder="0" allow="accelerometer; autoplay; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>

# Demo

[![Demo](https://img.youtube.com/vi/syz2i6MyOAg/0.jpg)](https://www.youtube.com/watch?v=syz2i6MyOAg)

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
