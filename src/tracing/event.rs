use std::fmt::Debug;

use chrono::{DateTime, Local};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Metadata};

#[derive(Debug, Clone)]
pub struct CollectedEvent {
    pub target: String,
    pub level: Level,
    pub message: String,
    pub time: DateTime<Local>,
}

impl CollectedEvent {
    pub fn new(event: &Event, meta: &Metadata) -> Option<Self> {
        let mut message = MessageVisitor(None);
        event.record(&mut message);

        message.0.map(|message| CollectedEvent {
            level: meta.level().to_owned(),
            time: Local::now(),
            target: meta.target().to_owned(),
            message,
        })
    }
}

struct MessageVisitor(Option<String>);

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            let old_message = self.0.replace(format!("{:?}", value));
            debug_assert!(old_message.is_none());
        }
    }
}
