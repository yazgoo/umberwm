use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use xcb::ModMask;
pub use xcb::{
    MOD_MASK_1, MOD_MASK_2, MOD_MASK_3, MOD_MASK_4, MOD_MASK_5, MOD_MASK_CONTROL, MOD_MASK_SHIFT,
};
use xmodmap_pke_umberwm::XmodmapPke;

#[derive(Eq, PartialEq, Hash, Debug, Clone, Deserialize, Serialize)]
pub struct Keybind {
    pub mod_mask: ModMask,
    pub key: String,
}

impl Keybind {
    pub fn new<M, K>(mod_mask: M, key: K) -> Self
    where
        M: Into<ModMask>,
        K: Into<String>,
    {
        Keybind {
            mod_mask: mod_mask.into(),
            key: key.into(),
        }
    }
}

#[derive(Debug)]
pub struct NormalHints {
    pub min_width: u32,
    pub min_height: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub width_inc: u32,
    pub height_inc: u32,
    pub min_aspect: (u32, u32),
    pub max_aspect: (u32, u32),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub enum Events {
    OnChangeWorkspace,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Actions {
    SwitchWindow,
    SerializeAndQuit,
    CloseWindow,
    ChangeLayout,
    ToggleGap,
    Quit,
}

pub enum Meta {
    Mod1,
    Mod4,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Layout {
    Bspv,
    Monocle,
    Bsph,
}

pub type Window = u32;

pub type Key = String;

pub type WorkspaceName = Key;

pub type CustomAction = Box<dyn Fn()>;

pub type Color = u32;

#[derive(Clone)]
pub struct Geometry(pub u32, pub u32, pub u32, pub u32);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowBorder {
    pub width: u32,
    pub focus_color: Color,
    pub normal_color: Color,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub layout: Layout,
    pub windows: Vec<Window>,
    pub focus: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisplayBorder {
    pub left: u32,
    pub right: u32,
    pub bottom: u32,
    pub top: u32,
    pub gap: u32,
}

pub type DisplayId = usize;

pub type OnChangeWorkspace = Option<Box<dyn Fn(WorkspaceName, DisplayId)>>;

pub struct EventsCallbacks {
    pub on_change_workspace: OnChangeWorkspace,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SerializableConf {
    /// modifier key which will be used for changing workspaces
    pub meta: ModMask,
    /// describes the borders of a window
    pub border: WindowBorder,
    /// describes displays borders / gaps
    pub display_borders: Vec<DisplayBorder>,
    /// name the workspaces with the key to press with meta, splitted by display
    pub workspaces_names: Vec<Vec<WorkspaceName>>,
    /// assign keys to pre-defined actions
    pub wm_actions: HashMap<Keybind, Actions>,
    /// will ignore windows with this wm_class
    pub ignore_classes: Vec<String>,
    /// will not resize windows wm_class
    pub float_classes: Vec<String>,
    /// will not resize and display on top windows with this wm_class
    pub overlay_classes: Vec<String>,
    /// will stick these window classes to theses workspace on window open
    pub sticky_classes: HashMap<String, WorkspaceName>,
    /// should we enable gaps (as defined in border) on startup
    pub with_gap: bool,
    /// run commands on given keys
    pub custom_commands: HashMap<Keybind, Vec<String>>,
    /// callback commands to be called on events
    pub command_callbacks: HashMap<Events, Vec<String>>,
}

pub struct Conf {
    pub serializable: SerializableConf,
    pub custom_actions: HashMap<Keybind, CustomAction>,
    pub events_callbacks: EventsCallbacks,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableState {
    pub float_windows: Vec<Window>,
    pub overlay_windows: Vec<Window>,
    pub workspaces: HashMap<WorkspaceName, Workspace>,
    pub current_workspace: WorkspaceName,
}

#[derive(Clone)]
pub struct MouseMoveStart {
    pub root_x: i16,
    pub root_y: i16,
    pub child: Window,
    pub detail: u8,
}

pub struct UmberWm {
    pub conf: Conf,
    pub current_workspace: WorkspaceName,
    pub float_windows: Vec<Window>,
    pub overlay_windows: Vec<Window>,
    pub workspaces: HashMap<WorkspaceName, Workspace>,
    pub conn: xcb::Connection,
    pub mouse_move_start: Option<MouseMoveStart>,
    pub button_press_geometry: Option<Geometry>,
    pub xmodmap_pke: XmodmapPke,
    pub displays_geometries: Vec<Geometry>,
    pub randr_base: u8,
    pub previous_display: DisplayId,
}
