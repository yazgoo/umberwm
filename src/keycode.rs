use crate::model::Key;
use xmodmap_pke_umberwm::XmodmapPke;

pub fn keycode_to_key(xmodmap_pke: &XmodmapPke, keycode: u8) -> Option<Key> {
    if let Some(x) = xmodmap_pke.get(&keycode) {
        if !x.is_empty() {
            return Some(x[0].to_string());
        }
    }
    None
}

pub fn key_to_keycode(xmodmap_pke: &XmodmapPke, key: &str) -> Option<u8> {
    for (keycode, symbols) in xmodmap_pke.iter() {
        if symbols.contains(&key.to_string()) {
            return Some(*keycode);
        }
    }
    None
}
