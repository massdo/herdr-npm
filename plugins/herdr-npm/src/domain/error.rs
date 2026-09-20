use std::fmt;
use std::io;
use std::path::PathBuf;

/// Application-level error. Transport whose effect is unknown is `Uncertain`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    OriginMissing,
    SnapshotUnreadable {
        detail: String,
    },
    OriginChanged,
    NoWorkingTarget,
    SeveralSidebars,
    CannotDetermineProjectDir,
    NoPackageJson,
    InvalidPackageJson,
    CannotReadPackageJson {
        path: PathBuf,
    },
    ScriptsNotObjectOfStrings,
    NoScripts,
    TerminalTooSmall,
    LaunchNotConfirmed {
        tab_id: Option<String>,
    },
    Herdr {
        method: String,
        code: String,
        message: String,
    },
    Uncertain {
        method: String,
        message: String,
    },
    Io {
        message: String,
    },
}

impl AppError {
    pub fn herdr(
        method: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Herdr {
            method: method.into(),
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn uncertain(method: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Uncertain {
            method: method.into(),
            message: message.into(),
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Uncertain { .. } => 2,
            _ => 1,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OriginMissing => write!(f, "origin workspace, tab or pane is missing"),
            Self::SnapshotUnreadable { detail } => {
                write!(f, "pane snapshot cannot be interpreted: {detail}")
            }
            Self::OriginChanged => {
                write!(
                    f,
                    "captured origin pane is no longer present in the pane list"
                )
            }
            Self::NoWorkingTarget => {
                write!(f, "no working pane is available as a split target")
            }
            Self::SeveralSidebars => {
                write!(
                    f,
                    "several herdr-npm sidebars are recognised in the target tab"
                )
            }
            Self::CannotDetermineProjectDir => {
                write!(f, "Cannot determine project directory")
            }
            Self::NoPackageJson => write!(f, "No package.json found"),
            Self::InvalidPackageJson => write!(f, "package.json is not valid JSON"),
            Self::CannotReadPackageJson { path } => {
                write!(f, "Cannot read package.json ({})", path.display())
            }
            Self::ScriptsNotObjectOfStrings => {
                write!(f, "package.json scripts must be an object of strings")
            }
            Self::NoScripts => write!(f, "This package.json has no scripts"),
            Self::TerminalTooSmall => write!(f, "Terminal too small"),
            Self::LaunchNotConfirmed { tab_id } => match tab_id {
                Some(id) => write!(f, "Script launch not confirmed ({id})"),
                None => write!(f, "Script launch not confirmed"),
            },
            Self::Herdr {
                method,
                code,
                message,
            } => write!(f, "{method} failed ({code}): {message}"),
            Self::Uncertain { method, message } => write!(
                f,
                "{method} did not confirm: {message}. Inspect the layout before another attempt."
            ),
            Self::Io { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
        Self::Io {
            message: error.to_string(),
        }
    }
}
