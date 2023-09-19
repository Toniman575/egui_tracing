#![warn(clippy::all, clippy::cargo)]

mod layer;
mod time;
mod ui;

use std::sync::Arc;

use crossbeam_queue::ArrayQueue;
pub use globset;
use globset::{Glob, GlobSet, GlobSetBuilder};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use serde::{Deserialize, Serialize};
use tracing::Level;

use self::layer::CollectedEvent;
pub use self::layer::EguiTracingLayer;

#[derive(Debug)]
pub struct EguiTracing {
    queue: Arc<ArrayQueue<CollectedEvent>>,
    events: AllocRingBuffer<CollectedEvent>,
    globset: Option<GlobSet>,
}

impl EguiTracing {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: Arc::new(ArrayQueue::new(capacity)),
            events: AllocRingBuffer::new(capacity),
            globset: None,
        }
    }

    pub fn layer(&self) -> EguiTracingLayer {
        EguiTracingLayer::new(Arc::clone(&self.queue))
    }

    fn fetch_events(&mut self) {
        while let Some(event) = self.queue.pop() {
            self.events.push(event);
        }
    }

    fn update_globset(&mut self, target_filter: &TargetFilter) {
        let mut glob = GlobSetBuilder::new();

        for target in target_filter.targets.clone() {
            glob.add(target);
        }

        self.globset = Some(glob.build().expect("found invalid `Glob`"));
    }
}

impl Default for EguiTracing {
    fn default() -> Self {
        EguiTracing::new(50000)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct State {
    pub level_filter: LevelFilter,
    pub target_filter: TargetFilter,
}

#[derive(Clone, Copy, Debug, Deserialize, Hash, Serialize)]
pub struct LevelFilter {
    pub trace: bool,
    pub debug: bool,
    pub info: bool,
    pub warn: bool,
    pub error: bool,
}

impl Default for LevelFilter {
    fn default() -> Self {
        Self {
            trace: false,
            debug: true,
            info: true,
            warn: true,
            error: true,
        }
    }
}

impl LevelFilter {
    fn matches(&self, level: Level) -> bool {
        match level {
            Level::TRACE => self.trace,
            Level::DEBUG => self.debug,
            Level::INFO => self.info,
            Level::WARN => self.warn,
            Level::ERROR => self.error,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Hash, Serialize)]
pub struct TargetFilter {
    pub input: String,
    pub targets: Vec<Glob>,
}
