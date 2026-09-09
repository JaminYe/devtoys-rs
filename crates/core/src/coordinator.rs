use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use devtoys_api::RawData;

use crate::clipboard::{ClipboardSource, SystemClipboard};
use crate::detection::{DetectOptions, DetectionEngine, Recommendation};

const CLIPBOARD_INTERVAL: Duration = Duration::from_millis(800);
const RESULT_INTERVAL: Duration = Duration::from_millis(80);
const DETECTION_TIMEOUT: Duration = Duration::from_secs(2);

struct DetectionResult {
    generation: u64,
    completed_at: Instant,
    recommendations: Vec<Recommendation>,
}

// Only time and task execution vary here. Both adapters exercise the same
// coordinator lifecycle through poll, rather than a second scheduling model.
trait DetectionRuntime {
    fn now(&self) -> Instant;
    fn start(
        &self,
        engine: Arc<DetectionEngine>,
        raw: RawData,
        gen: u64,
        cancel: Arc<AtomicBool>,
    ) -> Receiver<DetectionResult>;
}

struct SystemRuntime;

impl DetectionRuntime for SystemRuntime {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn start(
        &self,
        engine: Arc<DetectionEngine>,
        raw: RawData,
        gen: u64,
        cancel: Arc<AtomicBool>,
    ) -> Receiver<DetectionResult> {
        let (tx, rx) = mpsc::channel();
        let (finished_tx, finished_rx) = mpsc::channel();
        let timeout_cancel = cancel.clone();
        std::thread::spawn(move || {
            if finished_rx.recv_timeout(DETECTION_TIMEOUT).is_err() {
                timeout_cancel.store(true, Ordering::Relaxed);
            }
        });
        std::thread::spawn(move || {
            let hits = engine.detect(
                &raw,
                DetectOptions {
                    strict: false,
                    active_tool: None,
                    enabled: true,
                    cancel: cancel.as_ref(),
                },
            );
            let _ = tx.send(DetectionResult {
                generation: gen,
                completed_at: Instant::now(),
                recommendations: hits,
            });
            let _ = finished_tx.send(());
        });
        rx
    }
}

/// Coordinates clipboard monitoring, debounced detection, asynchronous execution,
/// cancellation, generational updates, and tool recommendations.
pub struct DetectionCoordinator {
    engine: Arc<DetectionEngine>,
    clipboard: Box<dyn ClipboardSource>,
    runtime: Box<dyn DetectionRuntime>,
    last_clipboard: Option<RawData>,
    next_clipboard_at: Instant,
    enabled: bool,
    /// Untouched engine output (no active_tool filtering). This is the source of
    /// truth so that leaving a tool page can restore a recommendation without
    /// re-detecting.
    raw_hits: Vec<Recommendation>,
    /// The projected view exposed to callers: `raw_hits` with the current
    /// `active_tool` excluded. Recomputed on every `poll`.
    recommendations: Vec<Recommendation>,
    detect_cancel: Arc<AtomicBool>,
    detect_gen: u64,
    detect_rx: Option<Receiver<DetectionResult>>,
    detect_deadline: Option<Instant>,
}

impl DetectionCoordinator {
    /// Time until the host should poll again; `None` when monitoring is stopped.
    /// Call after `poll` and pass the delay to the host's wakeup mechanism,
    /// including when the clipboard is empty or no detection task is running.
    pub fn next_poll_after(&self) -> Option<Duration> {
        self.enabled.then(|| {
            let now = self.runtime.now();
            let clipboard = self.next_clipboard_at.saturating_duration_since(now);
            if let Some(deadline) = self.detect_deadline {
                clipboard
                    .min(RESULT_INTERVAL)
                    .min(deadline.saturating_duration_since(now))
            } else {
                clipboard
            }
        })
    }

    /// Creates a new `DetectionCoordinator` with the given engine and clipboard source.
    pub fn new(engine: Arc<DetectionEngine>, clipboard: Box<dyn ClipboardSource>) -> Self {
        Self::with_runtime(engine, clipboard, Box::new(SystemRuntime))
    }

    fn with_runtime(
        engine: Arc<DetectionEngine>,
        clipboard: Box<dyn ClipboardSource>,
        runtime: Box<dyn DetectionRuntime>,
    ) -> Self {
        let now = runtime.now();
        Self {
            engine,
            clipboard,
            runtime,
            enabled: false,
            last_clipboard: None,
            next_clipboard_at: now,
            raw_hits: Vec::new(),
            recommendations: Vec::new(),
            detect_cancel: Arc::new(AtomicBool::new(false)),
            detect_gen: 0,
            detect_rx: None,
            detect_deadline: None,
        }
    }

    /// Creates a new `DetectionCoordinator` with the default system clipboard.
    pub fn system(engine: Arc<DetectionEngine>) -> Self {
        Self::new(engine, Box::new(SystemClipboard::new()))
    }

    /// Polls detection state, debouncing clipboard reads and collecting results.
    ///
    /// The coordinator owns the recommendation lifecycle: it caches raw hits and
    /// publishes a projection that excludes the current `active_tool`. This means
    /// entering a tool only hides its recommendation (projection), and leaving it
    /// with an unchanged clipboard restores the recommendation from cached raw
    /// hits without re-detecting.
    ///
    /// - If `!enabled`: cancels pending detections, clears the clipboard cache,
    ///   raw hits and projection, and returns `None`. Re-enabling detects even
    ///   unchanged clipboard content on the next scheduled clipboard read.
    /// - Drains completed detection results from the worker thread. If the
    ///   generation matches the current `detect_gen`, the raw hit cache is updated
    ///   with the untouched engine output. Stale generational results are dropped.
    /// - Recomputes the projection (raw hits excluding `active_tool`) each call.
    /// - If at least 800ms have passed since the last clipboard check, reads the
    ///   clipboard. If content changed, starts a new detection task.
    /// - Returns a reference to the projection if non-empty, otherwise `None`.
    pub fn poll(&mut self, active_tool: Option<&str>, enabled: bool) -> Option<&[Recommendation]> {
        self.enabled = enabled;
        if !enabled {
            self.clear();
            return None;
        }

        let now = self.runtime.now();
        // Judge a completed task by its completion time, not when the host
        // collects it. A delayed host pass must not lose an on-time result.
        if let Some(rx) = self.detect_rx.as_ref() {
            match rx.try_recv() {
                Ok(result) => {
                    if result.generation == self.detect_gen
                        && self
                            .detect_deadline
                            .is_some_and(|deadline| result.completed_at < deadline)
                    {
                        self.raw_hits = result.recommendations;
                    }
                    self.cancel_detection();
                }
                Err(mpsc::TryRecvError::Empty) => {
                    if self.detect_deadline.is_some_and(|deadline| now >= deadline)
                        || self.detect_cancel.load(Ordering::Relaxed)
                    {
                        self.cancel_detection();
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.detect_rx = None;
                    self.detect_deadline = None;
                }
            }
        }

        // Project the cached raw hits onto the current active tool. Because raw
        // hits are never mutated by `active_tool`, leaving a tool page restores
        // the hidden recommendation on the next poll.
        self.recommendations = self
            .raw_hits
            .iter()
            .filter(|hit| active_tool != Some(hit.tool_id.as_str()))
            .cloned()
            .collect();

        // 800ms debounce check for clipboard reading
        if now >= self.next_clipboard_at {
            self.next_clipboard_at = now + CLIPBOARD_INTERVAL;
            if let Some(raw) = self.clipboard.read_raw() {
                if self.last_clipboard.as_ref() != Some(&raw) {
                    self.last_clipboard = Some(raw.clone());
                    self.start_detect(raw);
                }
            } else if self.last_clipboard.is_some() {
                self.last_clipboard = None;
                self.raw_hits.clear();
                self.recommendations.clear();
                self.cancel_detection();
            }
        }

        if self.recommendations.is_empty() {
            None
        } else {
            Some(&self.recommendations)
        }
    }

    /// Spawns a background detection task for the given raw data.
    ///
    /// Cancels any prior task and executes a new generation asynchronously.
    /// The runtime enforces the budget even while the host is not polling;
    /// poll also retires timed-out tasks so a slow detector cannot keep it busy.
    pub fn start_detect(&mut self, raw: RawData) {
        self.detect_cancel.store(true, Ordering::Relaxed);
        self.detect_cancel = Arc::new(AtomicBool::new(false));
        self.detect_gen = self.detect_gen.wrapping_add(1);
        self.detect_deadline = Some(self.runtime.now() + DETECTION_TIMEOUT);
        self.detect_rx = Some(self.runtime.start(
            self.engine.clone(),
            raw,
            self.detect_gen,
            self.detect_cancel.clone(),
        ));
    }

    /// Cancels any in-flight detection and clears cached clipboard, raw hits, and
    /// the projection.
    ///
    /// Resetting `last_clipboard` to `None` means that re-enabling detection after
    /// a `clear()` treats the next read as a change and re-detects fresh.
    pub fn clear(&mut self) {
        self.enabled = false;
        self.cancel_detection();
        self.raw_hits.clear();
        self.recommendations.clear();
        self.last_clipboard = None;
    }

    fn cancel_detection(&mut self) {
        self.detect_cancel.store(true, Ordering::Relaxed);
        self.detect_rx = None;
        self.detect_deadline = None;
    }

    /// Returns the current active recommendations.
    pub fn recommendations(&self) -> &[Recommendation] {
        &self.recommendations
    }

    /// Returns the last processed clipboard data, if any.
    pub fn last_clipboard(&self) -> Option<&RawData> {
        self.last_clipboard.as_ref()
    }

    /// Returns the current generation counter.
    pub fn detect_gen(&self) -> u64 {
        self.detect_gen
    }

    /// Returns whether a background detection task is currently in flight.
    pub fn is_detecting(&self) -> bool {
        self.detect_rx.is_some()
    }
}

impl Drop for DetectionCoordinator {
    fn drop(&mut self) {
        self.cancel_detection();
    }
}

#[cfg(test)]
mod tests;
