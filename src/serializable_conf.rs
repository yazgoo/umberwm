use crate::error::Result;
use crate::model::*;
use ron::de::from_str;
use ron::ser::{to_string_pretty, PrettyConfig};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

const UMBERWM_CONF: &str = "umberwm.ron";

pub fn umberwm_conf() -> String {
    format!(
        "{}/{}",
        dirs::config_dir().unwrap().to_str().unwrap(),
        UMBERWM_CONF
    )
}

impl SerializableConf {
    pub fn save(&self) -> Result<()> {
        let path = umberwm_conf();
        let mut file = File::create(path.clone())?;
        let conf = PrettyConfig::new();
        let string = to_string_pretty(&self, conf)?;
        file.write_all(string.as_bytes())?;
        println!("generated configuration in {}", path);
        Ok(())
    }
    pub fn load() -> Result<Self, anyhow::Error> {
        let mut file = File::open(umberwm_conf())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(from_str(contents.as_str())?)
    }

    pub fn exists() -> bool {
        Path::new(&umberwm_conf()).exists()
    }
}
