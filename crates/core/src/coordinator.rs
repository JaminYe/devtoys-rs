use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use devtoys_api::RawData;

use crate::clipboard::{ClipboardSource, SystemClipboard};
use crate::detection::{DetectOptions, DetectionEngine, Recommendation};

/// Coordinates clipboard monitoring, debounced detection, asynchronous execution,
/// cancellation, generational updates, and tool recommendations.
pub struct DetectionCoordinator {
    engine: Arc<DetectionEngine>,
    clipboard: Box<dyn ClipboardSource>,
    last_clipboard: Option<RawData>,
    last_clipboard_at: Instant,
    /// Untouched engine output (no active_tool filtering). This is the source of
    /// truth so that leaving a tool page can restore a recommendation without
    /// re-detecting.
    raw_hits: Vec<Recommendation>,
    /// The projected view exposed to callers: `raw_hits` with the current
    /// `active_tool` excluded. Recomputed on every `poll`.
    recommendations: Vec<Recommendation>,
    detect_cancel: Arc<AtomicBool>,
    detect_gen: u64,
    detect_rx: Option<Receiver<(u64, Vec<Recommendation>)>>,
}

impl DetectionCoordinator {
    /// Creates a new `DetectionCoordinator` with the given engine and clipboard source.
    pub fn new(engine: Arc<DetectionEngine>, clipboard: Box<dyn ClipboardSource>) -> Self {
        Self {
            engine,
            clipboard,
            last_clipboard: None,
            // Initialize 1 second in the past so the first poll check can trigger immediately.
            last_clipboard_at: Instant::now()
                .checked_sub(Duration::from_millis(1000))
                .unwrap_or_else(Instant::now),
            raw_hits: Vec::new(),
            recommendations: Vec::new(),
            detect_cancel: Arc::new(AtomicBool::new(false)),
            detect_gen: 0,
            detect_rx: None,
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
    /// - If `!enabled`: cancels pending detections, clears the raw hit cache and
    ///   the projection, and returns `None`.
    /// - Drains completed detection results from the worker thread. If the
    ///   generation matches the current `detect_gen`, the raw hit cache is updated
    ///   with the untouched engine output. Stale generational results are dropped.
    /// - Recomputes the projection (raw hits excluding `active_tool`) each call.
    /// - If at least 800ms have passed since the last clipboard check, reads the
    ///   clipboard. If content changed, starts a new detection task.
    /// - Returns a reference to the projection if non-empty, otherwise `None`.
    pub fn poll(&mut self, active_tool: Option<&str>, enabled: bool) -> Option<&[Recommendation]> {
        if !enabled {
            self.detect_cancel.store(true, Ordering::Relaxed);
            self.detect_rx = None;
            self.raw_hits.clear();
            self.recommendations.clear();
            return None;
        }

        // Drain worker thread result if available
        if let Some(rx) = self.detect_rx.as_ref() {
            match rx.try_recv() {
                Ok((gen, hits)) => {
                    if gen == self.detect_gen {
                        self.raw_hits = hits;
                    }
                    self.detect_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.detect_rx = None;
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
        if self.last_clipboard_at.elapsed() >= Duration::from_millis(800) {
            self.last_clipboard_at = Instant::now();
            if let Some(raw) = self.clipboard.read_raw() {
                if self.last_clipboard.as_ref() != Some(&raw) {
                    self.last_clipboard = Some(raw.clone());
                    self.start_detect(raw);
                }
            } else if self.last_clipboard.is_some() {
                self.last_clipboard = None;
                self.raw_hits.clear();
                self.recommendations.clear();
                self.detect_cancel.store(true, Ordering::Relaxed);
                self.detect_rx = None;
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
    /// Cancels any prior in-flight task, increments generation, sets up a 2-second timeout
    /// watchdog thread, and executes detection asynchronously.
    pub fn start_detect(&mut self, raw: RawData) {
        self.detect_cancel.store(true, Ordering::Relaxed);
        let cancel = Arc::new(AtomicBool::new(false));
        self.detect_cancel = cancel.clone();
        self.detect_gen = self.detect_gen.wrapping_add(1);
        let gen = self.detect_gen;
        let engine = self.engine.clone();
        let (tx, rx) = mpsc::channel();
        self.detect_rx = Some(rx);

        // Timeout thread: cancels detection after 2 seconds
        let timeout_cancel = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(2));
            timeout_cancel.store(true, Ordering::Relaxed);
        });

        // Worker thread: runs detection and sends result
        let worker_cancel = cancel;
        std::thread::spawn(move || {
            let hits = engine.detect(
                &raw,
                DetectOptions {
                    strict: false,
                    // active_tool exclusion is owned by the coordinator, so the
                    // engine always returns the full, untouched hit set.
                    active_tool: None,
                    enabled: true,
                    cancel: worker_cancel.as_ref(),
                },
            );
            let _ = tx.send((gen, hits));
        });
    }

    /// Cancels any in-flight detection and clears cached clipboard, raw hits, and
    /// the projection.
    ///
    /// Resetting `last_clipboard` to `None` means that re-enabling detection after
    /// a `clear()` treats the next read as a change and re-detects fresh.
    pub fn clear(&mut self) {
        self.detect_cancel.store(true, Ordering::Relaxed);
        self.detect_rx = None;
        self.raw_hits.clear();
        self.recommendations.clear();
        self.last_clipboard = None;
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
