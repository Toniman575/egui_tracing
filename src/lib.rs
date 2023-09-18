#![warn(clippy::all, clippy::cargo)]

mod time;
pub mod tracing;
pub mod ui;

pub use self::tracing::EguiTracing;
pub use self::ui::Logs;
