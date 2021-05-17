use std::fmt::Display;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Workspace not found")]
    WorkspaceNotFound,
    #[error("Action not found")]
    ActionNotFound,
    #[error("No screens found")]
    NoScreensFound,
    #[error("Window not found")]
    WindowNotFound,
    #[error("Failed to get window manager class")]
    FailedToGetWmClass,
    #[error("No mouse move start")]
    NoMouseMoveStart,
    #[error("No butten press geometry")]
    NoButtonPressGeometry,
    #[error("Failed to deserialize from JSON: {0}")]
    FailedToDeserializeFromJson(String),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    RegexError(#[from] regex::Error),
    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    XcbGenericError(#[from] xcb::GenericError),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub trait LogError<T> {
    /// Converts from `Result<T, E>` to `Option<T>`, logging errors.
    ///
    /// Converts `self` into an `Option<T>`, consuming `self`, and logging the error, if any, to
    /// `STDERR`.
    fn log(self) -> Option<T>;
}

impl<T, E: Display> LogError<T> for Result<T, E> {
    fn log(self) -> Option<T> {
        match self {
            Ok(x) => Some(x),
            Err(e) => {
                eprintln!("{}", e);
                None
            }
        }
    }
}
