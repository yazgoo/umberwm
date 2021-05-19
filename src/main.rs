use std::env;
use std::path;
use std::process::Command;
use std::thread;
use umberwm::{
    umberwm, Actions, Conf, CustomAction, DisplayBorder, EventsCallbacks, Keybind, WindowBorder,
    MOD_MASK_1, MOD_MASK_4, MOD_MASK_CONTROL, MOD_MASK_SHIFT,
};

fn main() {
    let args: Vec<String> = env::args().collect();

    let meta = if args.len() > 1 && args[1] == "mod4" {
        MOD_MASK_4
    } else {
        MOD_MASK_1
    };

    let conf = Conf {
        // The mod key that is used to switch between workspaces
        meta,
        // Borders defining space the WM wont tile windows to (useful when using task bars)
        display_borders: vec![
            DisplayBorder {
                left: 0,
                right: 0,
                top: 0,
                bottom: 0,
                // Gap between windows (if `with_gap` is set to `true`)
                gap: 10,
            },
            DisplayBorder {
                left: 0,
                right: 0,
                top: 0,
                bottom: 0,
                gap: 10,
            },
        ],
        border: WindowBorder {
            width: 1,
            focus_color: 0x906cff,
            normal_color: 0x000000,
        },
        // Key names of the workspaces (must be a name in `xmodmap -pke`)
        // Each Vec defines the workspaces for a single display. You should have as many Vecs as
        // you have displays.
        workspaces_names: vec![
            // Map workspaces 1-5 to display 1
            (1..=5).map(|i| i.to_string()).collect(),
            // Map workspaces 6-9 to display 2
            (6..=9).map(|i| i.to_string()).collect(),
        ],
        // The keys for keybindings must be named as they are named in `xmodmap -pke`.

        // User defined actions
        custom_actions: vec![
            (
                Keybind::new(meta, "r"),
                Box::new(|| {
                    thread::spawn(move || {
                        // Launch rofi
                        let _ = Command::new("rofi").arg("-show").arg("run").status();
                    });
                }) as CustomAction,
            ),
            (
                Keybind::new(meta | MOD_MASK_SHIFT, "Return"),
                Box::new(|| {
                    thread::spawn(move || {
                        // Launch a terminal (alacritty)
                        let _ = Command::new("alacritty").status();
                    });
                }),
            ),
            (
                Keybind::new(meta | MOD_MASK_CONTROL, "l"),
                Box::new(|| {
                    thread::spawn(move || {
                        // Lock the screen (requires lxlock)
                        let _ = Command::new("lxlock");
                    });
                }),
            ),
            (
                Keybind::new(meta | MOD_MASK_CONTROL, "q"),
                // Quit UmberWM
                Box::new(|| std::process::exit(0)),
            ),
        ]
        .into_iter()
        .collect(),
        wm_actions: vec![
            // Window manager actions
            (Keybind::new(meta, "space"), Actions::SwitchWindow),
            (Keybind::new(meta, "w"), Actions::CloseWindow),
            (Keybind::new(meta, "f"), Actions::ChangeLayout),
            (Keybind::new(meta, "g"), Actions::ToggleGap),
            (
                // Restart UmberWM (if configured to do so - see README.md for details)
                Keybind::new(meta | MOD_MASK_CONTROL, "r"),
                Actions::SerializeAndQuit,
            ),
        ]
        .into_iter()
        .collect(),
        // Won't tile windows with this WM_CLASS
        ignore_classes: vec!["xscreensaver", "Discover-overlay"]
            .into_iter()
            .map(|x| x.to_string())
            .collect(),
        float_classes: vec![
            "confirm",
            "dialog",
            "error",
            "splash",
            "toolbar",
            "screenkey",
            "audacious",
            "Download",
            "dropbox",
            "file_progress",
            "file-roller",
            "Komodo_confirm_repl",
            "Komodo_find2",
            "pidgin",
            "skype",
            "Transmission",
            "Update",
            "Xephyr",
            "obs",
            "rofi",
            "xscreensaver",
            "quickmarks",
            "discover-overlay",
            "Discover-overlay",
        ]
        .into_iter()
        .map(|x| x.to_string())
        .collect(),
        events_callbacks: EventsCallbacks {
            // Custom callback will be called when we change a workspace.
            on_change_workspace: Some(Box::new(|workspace, display| {
                // This defines a custom wallpaper for each workspace. They must be located in
                // `~/Pictures/wallpapers` and be named `umberwm_<workspace_name>.jpg`.
                thread::spawn(move || {
                    // Set the wallpaper using nitrogen
                    let background_path = format!(
                        "{}/Pictures/wallpapers/umberwm_{}.jpg",
                        env::var("HOME").unwrap(),
                        workspace
                    );
                    if path::Path::new(&background_path).exists() {
                        let _ = Command::new("nitrogen")
                            .arg("--set-scaled")
                            .arg(format!("--head={}", display))
                            .arg(background_path)
                            .status();
                    }
                });
            })),
        },

        // Defines if there are gaps between windows (assuming `gap` is not 0 in `display_borders`)
        with_gap: false,
    };
    umberwm(conf).run();
}
