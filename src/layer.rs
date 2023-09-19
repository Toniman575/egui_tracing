use std::fmt::Debug;
use std::sync::Arc;

use chrono::{DateTime, Local};
use crossbeam_queue::ArrayQueue;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Metadata, Subscriber};
#[cfg(feature = "log")]
use tracing_log::NormalizeEvent;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

pub struct EguiTracingLayer {
    queue: Arc<ArrayQueue<CollectedEvent>>,
}

impl EguiTracingLayer {
    pub(crate) fn new(queue: Arc<ArrayQueue<CollectedEvent>>) -> Self {
        Self { queue }
    }
}

impl<S> Layer<S> for EguiTracingLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        #[cfg(feature = "log")]
        let normalized_meta = event.normalized_metadata();
        #[cfg(feature = "log")]
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());
        #[cfg(not(feature = "log"))]
        let meta = event.metadata();

        if let Some(event) = CollectedEvent::new(event, meta) {
            self.queue.force_push(event);
        }
    }
}

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
