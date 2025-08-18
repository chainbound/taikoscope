//! Taikoscope Driver crate root
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cognitive_complexity)]

pub mod driver;
pub mod event_handler;
pub mod event_processing;
pub mod gap_detection;
pub mod monitoring;
pub mod preconf;
pub mod reorg_detection;
mod subscription;
