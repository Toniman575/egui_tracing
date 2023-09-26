#![warn(clippy::all, clippy::cargo)]

mod layer;
mod time;
mod ui;

use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_queue::ArrayQueue;
pub use globset;
use globset::{Glob, GlobSet, GlobSetBuilder};
use hashbrown::HashMap;
use layer::{CollectedEvent, CollectedTracings, Inner};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use serde::{Deserialize, Serialize};
use tracing::span::Id;
use tracing::Level;

pub use self::layer::EguiTracingLayer;

struct Span {
    name: String,
    target: String,
    level: Level,
    start: Option<Instant>,
    parent: Option<Id>,
}

#[derive(Debug)]
struct FinishedSpan {
    name: String,
    target: String,
    level: Level,
    start: Duration,
    duration: Duration,
    parent: Option<Id>,
}

pub struct EguiTracing {
    inner: Arc<Inner>,
    events: AllocRingBuffer<CollectedEvent>,
    spans: HashMap<Id, Span>,
    finished_spans: HashMap<Id, FinishedSpan>,
    globset: Option<GlobSet>,
}

impl EguiTracing {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Inner {
                queue: ArrayQueue::new(capacity),
                timer: Instant::now(),
            }),
            events: AllocRingBuffer::new(capacity),
            spans: HashMap::new(),
            finished_spans: HashMap::new(),
            globset: None,
        }
    }
}

impl EguiTracing {
    pub fn new_with_timer(capacity: usize, timer: Instant) -> Self {
        Self {
            inner: Arc::new(Inner {
                queue: ArrayQueue::new(capacity),
                timer,
            }),
            spans: HashMap::new(),
            events: AllocRingBuffer::new(capacity),
            finished_spans: HashMap::new(),
            globset: None,
        }
    }

    pub fn layer(&self) -> EguiTracingLayer {
        EguiTracingLayer::new(Arc::clone(&self.inner))
    }

    fn fetch_tracings(&mut self) {
        while let Some(tracing) = self.inner.queue.pop() {
            match tracing {
                CollectedTracings::Event(event) => {
                    self.events.push(event);
                }
                CollectedTracings::NewSpan(new_span) => {
                    self.spans.insert(
                        new_span.id,
                        Span {
                            name: new_span.name,
                            target: new_span.target,
                            level: new_span.level,
                            start: None,
                            parent: new_span.parent,
                        },
                    );
                }
                CollectedTracings::EnterSpan(enter_span) => {
                    self.spans
                        .get_mut(&enter_span.id)
                        .expect("Span was entered before creating it")
                        .start = Some(enter_span.time);
                }
                CollectedTracings::ExitSpan(exit_span) => {
                    let span = self
                        .spans
                        .get_mut(&exit_span.id)
                        .expect("Span was exited before creating it");

                    let start = span
                        .start
                        .as_ref()
                        .expect("Span was exitted without entering it");

                    self.finished_spans.insert(
                        exit_span.id,
                        FinishedSpan {
                            name: span.name.clone(),
                            target: span.target.clone(),
                            level: span.level,
                            start: start.duration_since(self.inner.timer),
                            duration: exit_span.time.duration_since(*start),
                            parent: span.parent.clone(),
                        },
                    );
                }
                CollectedTracings::ClosedSpan(closed_span) => {
                    assert!(
                        self.spans.remove(&closed_span.id).is_some(),
                        "Span was closed before creating it"
                    );
                }
            }
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
    // TODO: This probably doesn't belong in `State`.
    pub input: String,
    pub targets: Vec<Glob>,
}
