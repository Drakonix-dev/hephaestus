use std::{
    collections::VecDeque,
    fmt,
    sync::{Arc, Mutex},
};

use tracing::{
    Event, Id, Level, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Record},
};
use tracing_subscriber::{
    Layer,
    filter::LevelFilter,
    layer::{Context, SubscriberExt},
    registry,
};

pub const DEFAULT_CAPACITY: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordKind {
    Event,
    Span,
}

#[derive(Clone, Debug)]
pub enum FieldValue {
    Bool(bool),
    Debug(String),
    F64(f64),
    I64(i64),
    Str(String),
    U64(u64),
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::Bool(v) => write!(f, "{v}"),
            FieldValue::Debug(v) => write!(f, "{v}"),
            FieldValue::F64(v) => write!(f, "{v}"),
            FieldValue::I64(v) => write!(f, "{v}"),
            FieldValue::Str(v) => write!(f, "{v}"),
            FieldValue::U64(v) => write!(f, "{v}"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CapturedRecord {
    pub kind: RecordKind,
    pub level: Level,
    pub name: String,
    pub span_id: Option<u64>,
    pub target: String,
    pub thread_name: Option<String>,
    fields: Vec<(String, FieldValue)>,
}

impl CapturedRecord {
    pub fn field(&self, key: &str) -> Option<&FieldValue> {
        self.fields
            .iter()
            .find(|(name, _)| name.as_str() == key)
            .map(|(_, value)| value)
    }

    pub fn field_bool(&self, key: &str) -> Option<bool> {
        match self.field(key)? {
            FieldValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn field_i64(&self, key: &str) -> Option<i64> {
        match self.field(key)? {
            FieldValue::I64(v) => Some(*v),
            FieldValue::U64(v) => i64::try_from(*v).ok(),
            _ => None,
        }
    }

    pub fn field_str(&self, key: &str) -> Option<&str> {
        match self.field(key)? {
            FieldValue::Debug(v) | FieldValue::Str(v) => Some(v),
            _ => None,
        }
    }

    pub fn field_u64(&self, key: &str) -> Option<u64> {
        match self.field(key)? {
            FieldValue::I64(v) => u64::try_from(*v).ok(),
            FieldValue::U64(v) => Some(*v),
            _ => None,
        }
    }

    pub fn has_field(&self, key: &str) -> bool {
        self.field(key).is_some()
    }
}

#[derive(Default)]
struct FieldVisitor(Vec<(String, FieldValue)>);

impl Visit for FieldVisitor {
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.0
            .push((field.name().to_owned(), FieldValue::Bool(value)));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0.push((
            field.name().to_owned(),
            FieldValue::Debug(format!("{value:?}")),
        ));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0
            .push((field.name().to_owned(), FieldValue::F64(value)));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0
            .push((field.name().to_owned(), FieldValue::I64(value)));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0
            .push((field.name().to_owned(), FieldValue::Str(value.to_owned())));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0
            .push((field.name().to_owned(), FieldValue::U64(value)));
    }
}

struct RingBuffer {
    capacity: usize,
    records: VecDeque<CapturedRecord>,
}

impl RingBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            records: VecDeque::new(),
        }
    }

    fn push(&mut self, record: CapturedRecord) {
        while self.records.len() >= self.capacity {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }
}

#[derive(Clone)]
pub struct CaptureHandle {
    buffer: Arc<Mutex<RingBuffer>>,
}

impl CaptureHandle {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(RingBuffer::new(capacity))),
        }
    }

    pub fn clear(&self) {
        self.buffer.lock().unwrap().records.clear();
    }

    pub fn count(&self, target: &str) -> usize {
        self.buffer
            .lock()
            .unwrap()
            .records
            .iter()
            .filter(|record| record.target == target)
            .count()
    }

    pub fn find(
        &self,
        mut predicate: impl FnMut(&CapturedRecord) -> bool,
    ) -> Option<CapturedRecord> {
        self.buffer
            .lock()
            .unwrap()
            .records
            .iter()
            .find(|record| predicate(record))
            .cloned()
    }

    pub fn records(&self) -> Vec<CapturedRecord> {
        self.buffer
            .lock()
            .unwrap()
            .records
            .iter()
            .cloned()
            .collect()
    }

    pub(crate) fn layer(&self) -> CaptureLayer {
        CaptureLayer {
            buffer: self.buffer.clone(),
        }
    }
}

pub(crate) struct CaptureLayer {
    buffer: Arc<Mutex<RingBuffer>>,
}

impl CaptureLayer {
    fn record(
        &self,
        kind: RecordKind,
        span_id: Option<u64>,
        metadata: &tracing::Metadata<'_>,
        visitor: FieldVisitor,
    ) {
        self.buffer.lock().unwrap().push(CapturedRecord {
            kind,
            level: *metadata.level(),
            name: metadata.name().to_owned(),
            span_id,
            target: metadata.target().to_owned(),
            thread_name: std::thread::current().name().map(str::to_owned),
            fields: visitor.0,
        });
    }
}

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        self.record(RecordKind::Event, None, event.metadata(), visitor);
    }

    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        attrs.record(&mut visitor);
        self.record(
            RecordKind::Span,
            Some(id.into_u64()),
            attrs.metadata(),
            visitor,
        );
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        values.record(&mut visitor);

        let span_id = id.into_u64();
        let mut buffer = self.buffer.lock().unwrap();
        let Some(record) = buffer
            .records
            .iter_mut()
            .rev()
            .find(|record| record.span_id == Some(span_id))
        else {
            return;
        };

        for (key, value) in visitor.0 {
            match record.fields.iter_mut().find(|(name, _)| *name == key) {
                Some(existing) => existing.1 = value,
                None => record.fields.push((key, value)),
            }
        }
    }
}

pub struct CaptureGuard {
    handle: CaptureHandle,
    _default: tracing::subscriber::DefaultGuard,
}

impl CaptureGuard {
    pub fn count(&self, target: &str) -> usize {
        self.handle.count(target)
    }

    pub fn find(&self, predicate: impl FnMut(&CapturedRecord) -> bool) -> Option<CapturedRecord> {
        self.handle.find(predicate)
    }

    pub fn handle(&self) -> CaptureHandle {
        self.handle.clone()
    }

    pub fn records(&self) -> Vec<CapturedRecord> {
        self.handle.records()
    }
}

pub fn capture() -> CaptureGuard {
    capture_with_capacity(DEFAULT_CAPACITY)
}

pub fn capture_with_capacity(capacity: usize) -> CaptureGuard {
    let handle = CaptureHandle::new(capacity);
    let subscriber = registry().with(handle.layer().with_filter(LevelFilter::TRACE));
    let _default = tracing::subscriber::set_default(subscriber);
    CaptureGuard { handle, _default }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_event_fields_by_target() {
        let capture = capture();
        tracing::info!(target: "heph::test", answer = 42_u64, "hello");

        let record = capture
            .find(|record| record.target == "heph::test")
            .expect("event should be captured");
        assert_eq!(record.kind, RecordKind::Event);
        assert_eq!(record.field_u64("answer"), Some(42));
        assert_eq!(capture.count("heph::test"), 1);
    }

    #[test]
    fn captures_deferred_span_field_recorded_after_creation() {
        let capture = capture();
        let span = tracing::info_span!(target: "heph::test", "work", draws = tracing::field::Empty);
        span.record("draws", 7_u64);

        let record = capture
            .find(|record| record.kind == RecordKind::Span && record.name == "work")
            .expect("span should be captured");
        assert_eq!(record.field_u64("draws"), Some(7));
    }

    #[test]
    fn captures_events_emitted_from_other_threads() {
        let handle = CaptureHandle::new(64);
        let subscriber = registry().with(handle.layer().with_filter(LevelFilter::TRACE));
        let dispatch = tracing::Dispatch::new(subscriber);
        let thread_dispatch = dispatch.clone();

        std::thread::Builder::new()
            .name("worker".to_owned())
            .spawn(move || {
                tracing::dispatcher::with_default(&thread_dispatch, || {
                    tracing::info!(target: "heph::test", "from worker");
                });
            })
            .unwrap()
            .join()
            .unwrap();

        let record = handle
            .find(|record| record.target == "heph::test")
            .expect("cross-thread event should be captured");
        assert_eq!(record.thread_name.as_deref(), Some("worker"));
    }

    #[test]
    fn ring_buffer_drops_oldest_beyond_capacity() {
        let capture = capture_with_capacity(2);
        for index in 0..5_u64 {
            tracing::info!(target: "heph::test", index, "tick");
        }

        let records = capture.records();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].field_u64("index"), Some(3));
        assert_eq!(records[1].field_u64("index"), Some(4));
    }
}
