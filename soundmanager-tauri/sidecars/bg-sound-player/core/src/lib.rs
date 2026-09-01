//! sound-manager-core: domain + platform logic shared by the Tauri app and the
//! bg-sound-player sidecar. Port of ORelio/Sound-Manager v3.5.0 (CDDL-1.0).

pub mod domain;
pub mod platform;
pub mod archive;
pub mod audio;
pub mod catalog;
pub mod downloader;
pub mod errors;

pub use errors::{CoreError, CoreResult};
