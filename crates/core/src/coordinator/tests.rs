use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::InMemoryClipboard;

struct Job {
    gen: u64,
    tx: mpsc::Sender<DetectionResult>,
    cancel: Arc<AtomicBool>,
}

struct ManualRuntime {
    now: RefCell<Instant>,
    jobs: RefCell<Vec<Job>>,
}

impl ManualRuntime {
    fn advance(&self, millis: u64) {
        *self.now.borrow_mut() += Duration::from_millis(millis);
    }

    fn complete(&self, job: usize, payload: &str) {
        let jobs = self.jobs.borrow();
        // Sending may fail when a superseded/disabled task has lost its receiver.
        // That is a valid way for the coordinator to reject a late result.
        let _ = jobs[job].tx.send(DetectionResult {
            generation: jobs[job].gen,
            completed_at: *self.now.borrow(),
            recommendations: vec![Recommendation {
                tool_id: "JsonFormatter".into(),
                data_type: "json".into(),
                payload: payload.into(),
            }],
        });
    }
}

impl DetectionRuntime for Rc<ManualRuntime> {
    fn now(&self) -> Instant {
        *self.now.borrow()
    }

    fn start(
        &self,
        _: Arc<DetectionEngine>,
        _: RawData,
        gen: u64,
        cancel: Arc<AtomicBool>,
    ) -> Receiver<DetectionResult> {
        let (tx, rx) = mpsc::channel();
        self.jobs.borrow_mut().push(Job { gen, tx, cancel });
        rx
    }
}

fn setup() -> (DetectionCoordinator, InMemoryClipboard, Rc<ManualRuntime>) {
    let clipboard = InMemoryClipboard::new();
    let runtime = Rc::new(ManualRuntime {
        now: RefCell::new(Instant::now()),
        jobs: RefCell::new(Vec::new()),
    });
    let coordinator = DetectionCoordinator::with_runtime(
        Arc::new(DetectionEngine::new(Vec::new(), &[])),
        Box::new(clipboard.clone()),
        Box::new(runtime.clone()),
    );
    (coordinator, clipboard, runtime)
}

#[test]
fn completion_before_deadline_survives_late_collection() {
    let (mut c, clipboard, runtime) = setup();
    clipboard.set_text("data");
    c.poll(None, true);
    runtime.advance(1950);
    runtime.complete(0, "data");
    runtime.advance(60);
    c.poll(None, true);
    assert_eq!(c.recommendations().len(), 1);
    assert_eq!(c.recommendations()[0].payload, "data");
    assert!(!c.is_detecting());
}

#[test]
fn completion_after_deadline_is_rejected_even_before_timeout_poll() {
    let (mut c, clipboard, runtime) = setup();
    clipboard.set_text("data");
    c.poll(None, true);
    runtime.advance(2001);
    runtime.complete(0, "too late");
    c.poll(None, true);
    assert!(c.recommendations().is_empty());
    assert!(!c.is_detecting());
}

#[test]
fn timeout_stops_waiting_and_rejects_late_result() {
    let (mut c, clipboard, runtime) = setup();
    clipboard.set_text("first");
    c.poll(None, true);
    runtime.advance(1999);
    c.poll(None, true);
    assert!(c.is_detecting());
    runtime.advance(1);
    c.poll(None, true);
    assert!(!c.is_detecting(), "timeout must stop result polling");
    assert!(runtime.jobs.borrow()[0].cancel.load(Ordering::Relaxed));
    assert_eq!(c.next_poll_after(), Some(Duration::from_millis(799)));
    runtime.complete(0, "late");
    c.poll(None, true);
    assert!(c.recommendations().is_empty());
}

#[test]
fn idle_period_deduplicates_content_and_completion_returns_to_idle() {
    let (mut c, clipboard, runtime) = setup();
    c.poll(None, true);
    assert_eq!(c.next_poll_after(), Some(Duration::from_millis(800)));
    clipboard.set_text("one");
    runtime.advance(799);
    c.poll(None, true);
    assert!(!c.is_detecting());
    assert_eq!(c.next_poll_after(), Some(Duration::from_millis(1)));
    runtime.advance(1);
    c.poll(None, true);
    assert!(c.is_detecting());
    assert_eq!(c.next_poll_after(), Some(Duration::from_millis(80)));
    runtime.complete(0, "one");
    c.poll(None, true);
    assert_eq!(c.recommendations()[0].payload, "one");
    assert!(!c.is_detecting());
    assert_eq!(c.next_poll_after(), Some(Duration::from_millis(800)));
    runtime.advance(800);
    c.poll(None, true);
    assert_eq!(c.detect_gen(), 1);
    clipboard.set_text("two");
    runtime.advance(800);
    c.poll(None, true);
    assert_eq!(c.detect_gen(), 2);
}

#[test]
fn late_old_generation_cannot_replace_new_result() {
    let (mut c, clipboard, runtime) = setup();
    clipboard.set_text("old");
    c.poll(None, true);
    clipboard.set_text("new");
    runtime.advance(800);
    c.poll(None, true);
    assert!(runtime.jobs.borrow()[0].cancel.load(Ordering::Relaxed));
    runtime.complete(1, "new");
    c.poll(None, true);
    runtime.complete(0, "old");
    c.poll(None, true);
    assert_eq!(c.recommendations()[0].payload, "new");
}

#[test]
fn disabled_rejects_late_results_and_reenabled_reads_same_content() {
    let (mut c, clipboard, runtime) = setup();
    clipboard.set_text("same");
    c.poll(None, true);
    c.poll(None, false);
    runtime.complete(0, "late");
    c.poll(None, false);
    assert_eq!(c.next_poll_after(), None);
    assert!(!c.is_detecting());
    assert!(c.recommendations().is_empty());
    assert!(c.last_clipboard().is_none());
    c.poll(None, true);
    assert_eq!(c.next_poll_after(), Some(Duration::from_millis(800)));
    runtime.advance(800);
    c.poll(None, true);
    runtime.complete(1, "same");
    c.poll(None, true);
    assert_eq!(c.recommendations()[0].payload, "same");
}

#[test]
fn entering_and_leaving_tool_projects_cached_hits_without_detection() {
    let (mut c, clipboard, runtime) = setup();
    clipboard.set_text("data");
    c.poll(None, true);
    runtime.complete(0, "data");
    c.poll(Some("JsonFormatter"), true);
    assert!(c.recommendations().is_empty());
    c.poll(None, true);
    assert_eq!(c.recommendations()[0].payload, "data");
    assert_eq!(c.detect_gen(), 1);
}

#[test]
fn empty_clipboard_clears_hits_and_cancels_pending_task() {
    let (mut c, clipboard, runtime) = setup();
    clipboard.set_text("old");
    c.poll(None, true);
    runtime.complete(0, "old");
    c.poll(None, true);
    clipboard.set_text("pending");
    runtime.advance(800);
    c.poll(None, true);
    clipboard.clear();
    runtime.advance(800);
    c.poll(None, true);
    assert!(c.recommendations().is_empty());
    assert!(!c.is_detecting());
    assert!(c.last_clipboard().is_none());
    runtime.complete(1, "late");
    c.poll(None, true);
    assert!(c.recommendations().is_empty());
    assert_eq!(c.next_poll_after(), Some(Duration::from_millis(800)));
}

#[test]
fn clipboard_is_not_read_before_scheduled_time() {
    use std::sync::atomic::AtomicUsize;
    struct CountReads(Arc<AtomicUsize>);
    impl ClipboardSource for CountReads {
        fn read_raw(&self) -> Option<RawData> {
            self.0.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
    let (_, _, runtime) = setup();
    let reads = Arc::new(AtomicUsize::new(0));
    let mut c = DetectionCoordinator::with_runtime(
        Arc::new(DetectionEngine::new(Vec::new(), &[])),
        Box::new(CountReads(reads.clone())),
        Box::new(runtime.clone()),
    );
    c.poll(None, true);
    runtime.advance(799);
    c.poll(None, true);
    assert_eq!(reads.load(Ordering::Relaxed), 1);
    runtime.advance(1);
    c.poll(None, true);
    assert_eq!(reads.load(Ordering::Relaxed), 2);
    c.poll(None, false);
    runtime.advance(800);
    c.poll(None, false);
    assert_eq!(reads.load(Ordering::Relaxed), 2);
}
