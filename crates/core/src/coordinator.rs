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
    /// - If `!enabled`: cancels pending detections, clears recommendations, and returns `None`.
    /// - Drains completed detection results from the worker thread. If the generation matches
    ///   the current `detect_gen`, recommendations are updated (filtering out `active_tool`).
    /// - If at least 800ms have passed since the last clipboard check, reads the clipboard.
    ///   If clipboard content has changed, starts a new detection task.
    /// - Returns a reference to current recommendations if any exist.
    pub fn poll(&mut self, active_tool: Option<&str>, enabled: bool) -> Option<&[Recommendation]> {
        if !enabled {
            self.detect_cancel.store(true, Ordering::Relaxed);
            self.detect_rx = None;
            self.recommendations.clear();
            return None;
        }

        // Drain worker thread result if available
        if let Some(rx) = self.detect_rx.as_ref() {
            match rx.try_recv() {
                Ok((gen, hits)) => {
                    if gen == self.detect_gen {
                        self.recommendations = hits
                            .into_iter()
                            .filter(|hit| active_tool != Some(hit.tool_id.as_str()))
                            .collect();
                    }
                    self.detect_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.detect_rx = None;
                }
            }
        }

        // Retain only recommendations that don't match the current active tool
        if let Some(active) = active_tool {
            self.recommendations.retain(|hit| hit.tool_id != active);
        }

        // 800ms debounce check for clipboard reading
        if self.last_clipboard_at.elapsed() >= Duration::from_millis(800) {
            self.last_clipboard_at = Instant::now();
            if let Some(raw) = self.clipboard.read_raw() {
                if self.last_clipboard.as_ref() != Some(&raw) {
                    self.last_clipboard = Some(raw.clone());
                    self.start_detect(raw, active_tool);
                }
            } else if self.last_clipboard.is_some() {
                self.last_clipboard = None;
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
    pub fn start_detect(&mut self, raw: RawData, active_tool: Option<&str>) {
        self.detect_cancel.store(true, Ordering::Relaxed);
        let cancel = Arc::new(AtomicBool::new(false));
        self.detect_cancel = cancel.clone();
        self.detect_gen = self.detect_gen.wrapping_add(1);
        let gen = self.detect_gen;
        let engine = self.engine.clone();
        let active = active_tool.map(ToOwned::to_owned);
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
                    active_tool: active.as_deref(),
                    enabled: true,
                    cancel: worker_cancel.as_ref(),
                },
            );
            let _ = tx.send((gen, hits));
        });
    }

    /// Cancels any in-flight detection and clears cached clipboard and recommendations.
    pub fn clear(&mut self) {
        self.detect_cancel.store(true, Ordering::Relaxed);
        self.detect_rx = None;
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
