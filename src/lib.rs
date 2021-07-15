mod error;

use error::{Error, Result};
pub mod model;
use crate::model::*;
use xmodmap_pke_umberwm::xmodmap_pke;
mod geometries;
mod keycode;
mod serializable_conf;
mod serializable_state;
use serializable_state::load_serializable_state;
mod umberwm;
use std::collections::HashMap;

pub fn umberwm_from_conf() -> Result<UmberWm> {
    let res: SerializableConf =
        SerializableConf::load().map_err(|_| Error::FailedToDeserializeFromJson("".to_string()))?;
    Ok(umberwm(Conf {
        serializable: res,
        custom_actions: HashMap::new(),
        events_callbacks: EventsCallbacks {
            on_change_workspace: None,
        },
    }))
}

pub fn umberwm(conf: Conf) -> UmberWm {
    let (conn, _) = xcb::Connection::connect(None).unwrap();
    let serializable_state = load_serializable_state(&conf).unwrap();
    let xmodmap_pke_res = xmodmap_pke(&conn).unwrap();
    let mut wm = UmberWm {
        conf,
        current_workspace: serializable_state.current_workspace,
        float_windows: serializable_state.float_windows,
        overlay_windows: serializable_state.overlay_windows,
        workspaces: serializable_state.workspaces,
        conn,
        button_press_geometry: None,
        mouse_move_start: None,
        xmodmap_pke: xmodmap_pke_res,
        displays_geometries: Vec::new(),
        randr_base: 0,
        previous_display: 0,
    };
    wm.init();
    wm
}
