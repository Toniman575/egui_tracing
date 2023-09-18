pub mod event;

use std::sync::Arc;

use crossbeam_queue::ArrayQueue;
use ringbuffer::{AllocRingBuffer, RingBuffer};
use tracing::{Event, Subscriber};
#[cfg(feature = "log")]
use tracing_log::NormalizeEvent;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

pub use self::event::CollectedEvent;

#[derive(Debug)]
pub struct EguiTracing {
    queue: Arc<ArrayQueue<CollectedEvent>>,
    events: AllocRingBuffer<CollectedEvent>,
}

pub struct EguiTracingLayer {
    queue: Arc<ArrayQueue<CollectedEvent>>,
}

impl EguiTracing {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: Arc::new(ArrayQueue::new(capacity)),
            events: AllocRingBuffer::new(capacity),
        }
    }

    pub fn events(&mut self) -> impl '_ + Iterator<Item = &CollectedEvent> {
        while let Some(event) = self.queue.pop() {
            self.events.push(event);
        }

        self.events.iter()
    }

    pub fn layer(&self) -> EguiTracingLayer {
        EguiTracingLayer {
            queue: Arc::clone(&self.queue),
        }
    }
}

impl Default for EguiTracing {
    fn default() -> Self {
        EguiTracing::new(50000)
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
