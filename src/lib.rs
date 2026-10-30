//! Compatibility facade. Implementation lives in independently owned workspace crates.
pub use ti_artifact as artifact;
#[cfg(feature = "mensoria-host")]
pub use ti_host_mensoria as mensoria;
pub use ti_language::*;

pub use ti_project as project;
