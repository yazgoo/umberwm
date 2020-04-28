use std::process::Command;
use umberwm::{Actions, Conf, umberwm, WindowBorder, DisplayBorder, Key, CustomAction, Meta};
use std::env;
use std::collections::HashMap;

fn main() {

    let args: Vec<String> = env::args().collect();

    umberwm(Conf {
        /* the main key used to detect WM events */
        meta: if args.len() > 1 && args[1] == "mod4" { Meta::Mod4 } else { Meta::Mod1 },
        /* borders defining space the WM wont tile windows to (usefull when using task bars) */
        display_border: DisplayBorder {
            left: 0,
            right: 0,
            top: 20,
            bottom: 0,
        },
        border: WindowBorder {
            width: 1,
            /* gap between windows */
            gap: 10,
            focus_color: 0x906cff,
            normal_color: 0x000000,
        },
        /* key names of the workspaces (must be a name in xmodmap -pke) */
        workspaces_names: vec![ "a", "u", "i", "o", "p" ].into_iter().map( |x| x.to_string() ).collect(),
        /* mapping between key names (must be a name in xmodmap -pke) and user-defined actions */
        custom_actions: 
            vec![
            ("r".to_string(), Box::new(|| { let _ = Command::new("rofi").arg("-show").arg("run").spawn();}) as CustomAction),
            ("t".to_string(), Box::new(|| { let _ = Command::new("kitty").spawn();})),
            ("q".to_string(), Box::new(|| std::process::exit(0))),
            ].into_iter().collect::<HashMap<Key, CustomAction>>(),
        /* mapping between key names (must be a name in xmodmap -pke) and window manager specific actions */
        wm_actions: 
            vec![
            ("space".to_string(), Actions::SwitchWindow),
            ("w".to_string(), Actions::CloseWindow),
            ("f".to_string(), Actions::ChangeLayout)].into_iter().collect::<HashMap<Key, Actions>>(),
        /* won't tile windows with this WM_CLASS */
        float_classes: vec!["screenkey", "audacious", "Download", "dropbox", "file_progress", "file-roller", "gimp",
                          "Komodo_confirm_repl", "Komodo_find2", "pidgin", "skype", "Transmission", "Update", "Xephyr", "obs"]
                              .into_iter().map( |x| x.to_string() ).collect(),
        /* will leave alone windows with this _NET_WM_WINDOW_TYPE */
        auto_float_types: vec!["notification", "toolbar", "splash", "dialog", "popup_menu", "utility", "tooltip", "dock"]
            .into_iter().map( |x| x.to_string() ).collect(),
    }).run();

}
