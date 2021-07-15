use crate::error::{Error, Result};
use crate::model::*;
use ron::de::from_str;
use std::fs::remove_file;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

pub const UMBERWM_STATE: &str = ".umberwm_state";
pub fn load_serializable_state(conf: &Conf) -> Result<SerializableState> {
    if Path::new(UMBERWM_STATE).exists() {
        let mut file = File::open(UMBERWM_STATE)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let res: SerializableState = from_str(contents.as_str())
            .map_err(|_| Error::FailedToDeserializeFromJson(contents.to_owned()))?;
        remove_file(UMBERWM_STATE)?;
        Ok(res)
    } else {
        Ok(SerializableState {
            float_windows: vec![],
            overlay_windows: vec![],
            workspaces: conf
                .serializable
                .workspaces_names
                .clone()
                .into_iter()
                .flatten()
                .into_iter()
                .map(|x| {
                    (
                        x,
                        Workspace {
                            layout: Layout::Bspv,
                            windows: vec![],
                            focus: 0,
                        },
                    )
                })
                .into_iter()
                .collect(),
            current_workspace: conf.serializable.workspaces_names.get(0).unwrap()[0].to_string(),
        })
    }
}
