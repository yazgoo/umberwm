use std::collections::HashMap;
use std::env;
use umberwm::{
    umberwm, Actions, Conf, DisplayBorder, Events, EventsCallbacks, Keybind, SerializableConf,
    WindowBorder, MOD_MASK_1, MOD_MASK_4, MOD_MASK_CONTROL, MOD_MASK_SHIFT,
};

fn main() {
    if SerializableConf::exists() {
        let serializable = SerializableConf::load().unwrap();
        let conf = Conf {
            serializable,
            events_callbacks: EventsCallbacks {
                on_change_workspace: None,
            },
            // User defined actions
            custom_actions: HashMap::new(),
        };
        umberwm(conf).run();
    } else {
        let args: Vec<String> = env::args().collect();

        let meta = if args.len() > 1 && args[1] == "mod4" {
            MOD_MASK_4
        } else {
            MOD_MASK_1
        };

        let serializable = SerializableConf {
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
            wm_actions: vec![
                // Window manager actions
                (Keybind::new(meta, "space"), Actions::SwitchWindow),
                (Keybind::new(meta, "w"), Actions::CloseWindow),
                (Keybind::new(meta, "f"), Actions::ChangeLayout),
                (Keybind::new(meta, "g"), Actions::ToggleGap),
                (Keybind::new(meta | MOD_MASK_CONTROL, "q"), Actions::Quit),
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
            ]
            .into_iter()
            .map(|x| x.to_string())
            .collect(),
            overlay_classes: vec!["discover-overlay", "Discover-overlay"]
                .into_iter()
                .map(|x| x.to_string())
                .collect(),

            // Defines if there are gaps between windows (assuming `gap` is not 0 in `display_borders`)
            with_gap: false,
            custom_commands: vec![
                (
                    Keybind::new(meta, "r"),
                    vec!["rofi", "-show", "run"]
                        .into_iter()
                        .map(|x| x.to_string())
                        .collect(),
                ),
                (
                    Keybind::new(meta | MOD_MASK_SHIFT, "Return"),
                    vec!["alacritty"]
                        .into_iter()
                        .map(|x| x.to_string())
                        .collect(),
                ),
                (
                    Keybind::new(meta | MOD_MASK_SHIFT, "l"),
                    vec!["lxlock"].into_iter().map(|x| x.to_string()).collect(),
                ),
            ]
            .into_iter()
            .collect(),
            command_callbacks: vec![(
                Events::OnChangeWorkspace,
                vec!["echo", "change workspace"]
                    .into_iter()
                    .map(|x| x.to_string())
                    .collect(),
            )]
            .into_iter()
            .collect(),
        };
        serializable.save().ok();
    }
}
