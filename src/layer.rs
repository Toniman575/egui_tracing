use std::fmt::Debug;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_queue::ArrayQueue;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id};
use tracing::{Event, Level, Metadata, Subscriber};
#[cfg(feature = "log")]
use tracing_log::NormalizeEvent;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

pub struct EguiTracingLayer(Arc<Inner>);

pub struct Inner {
    pub queue: ArrayQueue<CollectedTracings>,
    pub timer: Instant,
}

impl EguiTracingLayer {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self(inner)
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

        if let Some(event) =
            CollectedEvent::new(event, meta, Instant::now().duration_since(self.0.timer))
        {
            self.0.queue.force_push(CollectedTracings::Event(event));
        }
    }

    fn on_new_span(&self, attributes: &Attributes<'_>, id: &Id, _ctx: Context<'_, S>) {
        self.0
            .queue
            .force_push(CollectedTracings::NewSpan(NewSpan::new(
                attributes,
                id.to_owned(),
            )));
    }

    fn on_enter(&self, id: &Id, _ctx: Context<'_, S>) {
        self.0
            .queue
            .force_push(CollectedTracings::EnterSpan(EnterSpan::new(
                id.to_owned(),
                Instant::now(),
            )));
    }

    fn on_exit(&self, id: &Id, _ctx: Context<'_, S>) {
        self.0
            .queue
            .force_push(CollectedTracings::ExitSpan(ExitSpan::new(
                id.to_owned(),
                Instant::now(),
            )));
    }

    fn on_close(&self, id: Id, _ctx: Context<'_, S>) {
        self.0
            .queue
            .force_push(CollectedTracings::ClosedSpan(ClosedSpan::new(id)));
    }
}

pub enum CollectedTracings {
    Event(CollectedEvent),
    NewSpan(NewSpan),
    EnterSpan(EnterSpan),
    ExitSpan(ExitSpan),
    ClosedSpan(ClosedSpan),
}

#[derive(Debug, Clone)]
pub struct NewSpan {
    pub id: Id,
    pub name: String,
    pub target: String,
    pub level: Level,
}

impl NewSpan {
    pub fn new(attributes: &Attributes<'_>, id: Id) -> Self {
        let metadata = attributes.metadata();

        Self {
            id,
            name: metadata.name().to_owned(),
            target: metadata.target().to_owned(),
            level: metadata.level().to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnterSpan {
    pub id: Id,
    pub time: Instant,
}

impl EnterSpan {
    pub fn new(id: Id, time: Instant) -> Self {
        Self { id, time }
    }
}

#[derive(Debug, Clone)]
pub struct ExitSpan {
    pub id: Id,
    pub time: Instant,
}

impl ExitSpan {
    pub fn new(id: Id, time: Instant) -> Self {
        Self { id, time }
    }
}

#[derive(Debug, Clone)]
pub struct ClosedSpan {
    pub id: Id,
}

impl ClosedSpan {
    pub fn new(id: Id) -> Self {
        Self { id }
    }
}

#[derive(Debug, Clone)]
pub struct CollectedEvent {
    pub target: String,
    pub level: Level,
    pub message: String,
    pub time: Duration,
}

impl CollectedEvent {
    pub fn new(event: &Event, meta: &Metadata, time: Duration) -> Option<Self> {
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
