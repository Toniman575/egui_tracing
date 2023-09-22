use std::fmt::Debug;
use std::sync::Arc;

use crossbeam_queue::ArrayQueue;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Metadata, Subscriber};
#[cfg(feature = "log")]
use tracing_log::NormalizeEvent;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

use crate::Timer;

pub struct EguiTracingLayer<T: Timer>(Arc<Inner<T>>);

pub struct Inner<T: Timer> {
    pub queue: ArrayQueue<CollectedEvent<T>>,
    pub timer: T,
}

impl<T: Timer> EguiTracingLayer<T> {
    pub(crate) fn new(inner: Arc<Inner<T>>) -> Self {
        Self(inner)
    }
}

impl<S, T: 'static + Timer> Layer<S> for EguiTracingLayer<T>
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

        if let Some(event) = CollectedEvent::new(event, meta, self.0.timer.time()) {
            self.0.queue.force_push(event);
        }
    }
}

#[derive(Debug, Clone)]
pub struct CollectedEvent<T: Timer> {
    pub target: String,
    pub level: Level,
    pub message: String,
    pub time: T::Time,
}

impl<T: Timer> CollectedEvent<T> {
    pub fn new(event: &Event, meta: &Metadata, time: T::Time) -> Option<Self> {
        let mut message = MessageVisitor(None);
        event.record(&mut message);

        message.0.map(|message| CollectedEvent {
            level: meta.level().to_owned(),
            time,
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
