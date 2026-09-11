//! Update manager orchestrating version checks, download, and installation.

pub mod client;
pub mod download;
pub mod install;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;

use crate::version::SemVer;
pub use client::{CheckOutcome, ReleasePackage, UpdateAsset};
pub use download::DownloadOutcome;
pub use install::{detect_install_type, InstallType};

#[derive(Clone, Debug, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Latest {
        current_version: String,
    },
    UpdateAvailable(ReleasePackage),
    Downloading {
        package: ReleasePackage,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
    },
    Cancelling {
        package: ReleasePackage,
    },
    CleanupFailed {
        package: ReleasePackage,
        failed_path: PathBuf,
        reason: String,
    },
    Verifying(ReleasePackage),
    ReadyToInstall {
        target_version: String,
        installer_path: PathBuf,
        expected_sha256: String,
    },
    PreparingInstall {
        target_version: String,
    },
    NoRelease,
    Failed {
        reason: String,
        can_retry: bool,
    },
}

impl UpdateStatus {
    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            Self::Checking
                | Self::Downloading { .. }
                | Self::Cancelling { .. }
                | Self::CleanupFailed { .. }
                | Self::Verifying { .. }
                | Self::PreparingInstall { .. }
        )
    }

    #[allow(dead_code)]
    pub fn is_downloading(&self) -> bool {
        matches!(
            self,
            Self::Downloading { .. } | Self::Verifying { .. } | Self::Cancelling { .. }
        )
    }
}

pub enum UpdateMsg {
    Check(CheckOutcome),
    DownloadProgress {
        task_id: u64,
        downloaded: u64,
        total: Option<u64>,
    },
    #[allow(dead_code)]
    Verifying {
        task_id: u64,
    },
    DownloadDone {
        task_id: u64,
        outcome: DownloadOutcome,
    },
    InstallFailed(String),
}
pub type CheckHandler = Arc<dyn Fn(&SemVer, bool) -> CheckOutcome + Send + Sync>;
pub type DownloadHandler = Arc<
    dyn Fn(
            &UpdateAsset,
            Option<&UpdateAsset>,
            &str,
            &Path,
            u64,
            &AtomicBool,
            &dyn Fn(u64, Option<u64>),
        ) -> DownloadOutcome
        + Send
        + Sync,
>;

pub struct UpdateManager {
    status: UpdateStatus,
    sender: Sender<UpdateMsg>,
    receiver: Receiver<UpdateMsg>,
    check_handler: Option<CheckHandler>,
    download_handler: Option<DownloadHandler>,
    cancel_token: Option<Arc<AtomicBool>>,
    active_task_id: Option<u64>,
    next_task_id: u64,
    #[cfg(test)]
    thread_spawner:
        Option<Arc<dyn Fn(Box<dyn FnOnce() + Send>) -> std::io::Result<()> + Send + Sync>>,
}

impl Default for UpdateManager {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            status: UpdateStatus::Idle,
            sender,
            receiver,
            check_handler: None,
            download_handler: None,
            cancel_token: None,
            active_task_id: None,
            next_task_id: 1,
            #[cfg(test)]
            thread_spawner: None,
        }
    }
}

impl UpdateManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn status(&self) -> &UpdateStatus {
        &self.status
    }

    pub fn set_status(&mut self, status: UpdateStatus) {
        self.status = status;
    }

    #[allow(dead_code)]
    pub fn set_check_handler<F>(&mut self, handler: F)
    where
        F: Fn(&SemVer, bool) -> CheckOutcome + Send + Sync + 'static,
    {
        self.check_handler = Some(Arc::new(handler));
    }

    #[allow(dead_code)]
    pub fn set_download_handler<F>(&mut self, handler: F)
    where
        F: Fn(
                &UpdateAsset,
                Option<&UpdateAsset>,
                &str,
                &Path,
                u64,
                &AtomicBool,
                &dyn Fn(u64, Option<u64>),
            ) -> DownloadOutcome
            + Send
            + Sync
            + 'static,
    {
        self.download_handler = Some(Arc::new(handler));
    }

    #[cfg(test)]
    pub(crate) fn set_thread_spawner<F>(&mut self, spawner: F)
    where
        F: Fn(Box<dyn FnOnce() + Send>) -> std::io::Result<()> + Send + Sync + 'static,
    {
        self.thread_spawner = Some(Arc::new(spawner));
    }

    #[cfg(test)]
    pub(crate) fn active_task_id(&self) -> Option<u64> {
        self.active_task_id
    }

    /// Ticks the updater on each egui frame to receive background results.
    /// Returns true if the status changed.
    pub fn tick(&mut self) -> bool {
        let mut changed = false;
        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                UpdateMsg::Check(outcome) => {
                    let new_status = match outcome {
                        CheckOutcome::Latest { current_version } => {
                            UpdateStatus::Latest { current_version }
                        }
                        CheckOutcome::UpdateAvailable(pkg) => UpdateStatus::UpdateAvailable(*pkg),
                        CheckOutcome::NoRelease => UpdateStatus::NoRelease,
                        CheckOutcome::Failed { reason, can_retry } => {
                            UpdateStatus::Failed { reason, can_retry }
                        }
                    };
                    if self.status != new_status {
                        self.status = new_status;
                        changed = true;
                    }
                }
                UpdateMsg::DownloadProgress {
                    task_id,
                    downloaded,
                    total,
                } => {
                    if self.active_task_id == Some(task_id) {
                        if let UpdateStatus::Downloading { ref package, .. } = self.status {
                            self.status = UpdateStatus::Downloading {
                                package: package.clone(),
                                downloaded_bytes: downloaded,
                                total_bytes: total,
                            };
                            changed = true;
                        }
                    }
                }
                UpdateMsg::Verifying { task_id } => {
                    if self.active_task_id == Some(task_id) {
                        if let UpdateStatus::Downloading { ref package, .. } = self.status {
                            self.status = UpdateStatus::Verifying(package.clone());
                            changed = true;
                        }
                    }
                }
                UpdateMsg::DownloadDone { task_id, outcome } => {
                    if self.active_task_id != Some(task_id) {
                        // Late message from an old or cancelled task; ignore to protect current task
                        continue;
                    }
                    self.active_task_id = None;
                    self.cancel_token = None;
                    let new_status = match outcome {
                        DownloadOutcome::Success {
                            installer_path,
                            expected_sha256,
                            target_version,
                        } => UpdateStatus::ReadyToInstall {
                            target_version,
                            installer_path,
                            expected_sha256,
                        },
                        DownloadOutcome::Cancelled => match &self.status {
                            UpdateStatus::Downloading { package, .. }
                            | UpdateStatus::Verifying(package)
                            | UpdateStatus::Cancelling { package } => {
                                UpdateStatus::UpdateAvailable(package.clone())
                            }
                            _ => UpdateStatus::Idle,
                        },
                        DownloadOutcome::Failed { reason, can_retry } => {
                            UpdateStatus::Failed { reason, can_retry }
                        }
                        DownloadOutcome::CleanupFailed {
                            failed_path,
                            reason,
                        } => {
                            let package = match &self.status {
                                UpdateStatus::Downloading { package, .. }
                                | UpdateStatus::Verifying(package)
                                | UpdateStatus::Cancelling { package } => package.clone(),
                                _ => ReleasePackage {
                                    current_version: "0.0.0".into(),
                                    latest_version: "0.0.0".into(),
                                    release_url: String::new(),
                                    release_notes: None,
                                    asset: UpdateAsset {
                                        name: String::new(),
                                        download_url: String::new(),
                                        size: 0,
                                        sha256: None,
                                    },
                                    checksum_asset: None,
                                },
                            };
                            UpdateStatus::CleanupFailed {
                                package,
                                failed_path,
                                reason,
                            }
                        }
                    };
                    if self.status != new_status {
                        self.status = new_status;
                        changed = true;
                    }
                }
                UpdateMsg::InstallFailed(reason) => {
                    self.status = UpdateStatus::Failed {
                        reason,
                        can_retry: true,
                    };
                    changed = true;
                }
            }
        }
        changed
    }

    /// Triggers a manual update check.
    /// If a check is already in progress, this is a no-op (prevents duplicate requests).
    pub fn check_now(&mut self, ctx: egui::Context, include_prerelease: bool) -> bool {
        if self.status.is_busy() {
            return false;
        }

        self.status = UpdateStatus::Checking;
        let sender = self.sender.clone();
        let custom_handler = self.check_handler.clone();

        thread::Builder::new()
            .name("devtoys-update-checker".into())
            .spawn(move || {
                let current_version = SemVer::current();
                let outcome = match custom_handler {
                    Some(handler) => handler(&current_version, include_prerelease),
                    None => {
                        let client = client::UreqClient;
                        client::check_updates(
                            &client,
                            client::DEFAULT_RELEASES_URL,
                            &current_version,
                            include_prerelease,
                        )
                    }
                };
                let _ = sender.send(UpdateMsg::Check(outcome));
                ctx.request_repaint();
            })
            .ok();

        true
    }

    /// Starts downloading the selected update asset in a background thread.
    /// If already busy downloading or checking, returns false.
    pub fn start_download(&mut self, ctx: egui::Context) -> bool {
        if self.status.is_busy() {
            return false;
        }

        let package = match &self.status {
            UpdateStatus::UpdateAvailable(pkg) => pkg.clone(),
            _ => return false,
        };

        let task_id = self.next_task_id;
        self.next_task_id += 1;
        self.active_task_id = Some(task_id);

        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel_token = Some(cancel.clone());

        self.status = UpdateStatus::Downloading {
            package: package.clone(),
            downloaded_bytes: 0,
            total_bytes: Some(package.asset.size),
        };

        let sender = self.sender.clone();
        let download_handler = self.download_handler.clone();

        let update_dir = dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("devtoys-rs")
            .join("updates");

        let worker = move || {
            let s_prog = sender.clone();
            let ctx_prog = ctx.clone();
            let on_progress = move |downloaded: u64, total: Option<u64>| {
                let _ = s_prog.send(UpdateMsg::DownloadProgress {
                    task_id,
                    downloaded,
                    total,
                });
                ctx_prog.request_repaint();
            };

            let outcome = match download_handler {
                Some(handler) => handler(
                    &package.asset,
                    package.checksum_asset.as_ref(),
                    &package.latest_version,
                    &update_dir,
                    task_id,
                    &cancel,
                    &on_progress,
                ),
                None => {
                    let downloader = download::UreqFileDownloader;
                    download::download_and_verify(
                        &downloader,
                        &package.asset,
                        package.checksum_asset.as_ref(),
                        &package.latest_version,
                        &update_dir,
                        task_id,
                        &cancel,
                        on_progress,
                    )
                }
            };

            let _ = sender.send(UpdateMsg::DownloadDone { task_id, outcome });
            ctx.request_repaint();
        };

        #[cfg(test)]
        let spawn_res = match &self.thread_spawner {
            Some(spawner) => spawner(Box::new(worker)),
            None => thread::Builder::new()
                .name("devtoys-update-downloader".into())
                .spawn(worker)
                .map(|_| ()),
        };

        #[cfg(not(test))]
        let spawn_res = thread::Builder::new()
            .name("devtoys-update-downloader".into())
            .spawn(worker)
            .map(|_| ());

        if let Err(e) = spawn_res {
            self.active_task_id = None;
            self.cancel_token = None;
            self.status = UpdateStatus::Failed {
                reason: format!("启动下载后台线程失败：{e}"),
                can_retry: true,
            };
            return false;
        }

        true
    }

    /// Cancels an in-progress download, entering Cancelling cleanup state.
    /// Duplicate cancel calls return false.
    pub fn cancel_download(&mut self) -> bool {
        match &self.status {
            UpdateStatus::Downloading { package, .. } | UpdateStatus::Verifying(package) => {
                if let Some(token) = &self.cancel_token {
                    token.store(true, Ordering::Relaxed);
                }
                self.status = UpdateStatus::Cancelling {
                    package: package.clone(),
                };
                true
            }
            _ => false,
        }
    }

    /// Retries cleaning up temporary files after a cleanup failure.
    pub fn retry_cleanup(&mut self) -> bool {
        if let UpdateStatus::CleanupFailed {
            package,
            failed_path,
            ..
        } = &self.status
        {
            let pkg = package.clone();
            let path = failed_path.clone();
            if !path.exists() {
                self.status = UpdateStatus::UpdateAvailable(pkg);
                return true;
            }
            match std::fs::remove_file(&path) {
                Ok(_) => {
                    self.status = UpdateStatus::UpdateAvailable(pkg);
                    true
                }
                Err(e) => {
                    self.status = UpdateStatus::CleanupFailed {
                        package: pkg,
                        failed_path: path,
                        reason: format!("清理临时文件失败：{e}"),
                    };
                    false
                }
            }
        } else {
            false
        }
    }

    /// Spawns the installer in a background thread, monitors the handshake, and saves settings upon readiness.
    /// Runs asynchronously so the egui UI never freezes while waiting for UAC or installer startup.
    pub fn start_install<F>(
        &mut self,
        target_version: String,
        installer_path: PathBuf,
        expected_sha256: String,
        install_dir: PathBuf,
        save_fn: F,
        ctx: egui::Context,
    ) -> bool
    where
        F: Fn() -> Result<(), String> + Send + Sync + 'static,
    {
        if self.status.is_busy() {
            return false;
        }

        self.status = UpdateStatus::PreparingInstall { target_version };

        let sender = self.sender.clone();
        thread::Builder::new()
            .name("devtoys-update-installer".into())
            .spawn(move || {
                let runner = install::ProductionInstallerRunner { save_fn };
                if let Err(e) = install::execute_update_pipeline(
                    &runner,
                    &installer_path,
                    &expected_sha256,
                    &install_dir,
                ) {
                    let _ = sender.send(UpdateMsg::InstallFailed(e));
                    ctx.request_repaint();
                }
            })
            .ok();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    fn sample_package() -> ReleasePackage {
        ReleasePackage {
            current_version: "0.1.0".into(),
            latest_version: "0.2.0".into(),
            release_url: "http://release".into(),
            release_notes: None,
            asset: UpdateAsset {
                name: "setup.exe".into(),
                download_url: "http://setup.exe".into(),
                size: 1000,
                sha256: None,
            },
            checksum_asset: None,
        }
    }

    #[test]
    fn manager_prevents_duplicate_checks_and_downloads() {
        let mut manager = UpdateManager::new();
        assert_eq!(*manager.status(), UpdateStatus::Idle);
        assert!(!manager.status().is_busy());

        manager.set_status(UpdateStatus::Checking);
        assert!(manager.status().is_busy());

        let ctx = egui::Context::default();
        assert!(!manager.check_now(ctx.clone(), false));
        assert!(!manager.start_download(ctx));
    }

    #[test]
    fn manager_receives_outcome_on_tick() {
        let mut manager = UpdateManager::new();
        manager
            .sender
            .send(UpdateMsg::Check(CheckOutcome::Latest {
                current_version: "0.1.0".into(),
            }))
            .unwrap();

        let changed = manager.tick();
        assert!(changed);
        assert_eq!(
            *manager.status(),
            UpdateStatus::Latest {
                current_version: "0.1.0".into()
            }
        );

        assert!(!manager.tick());
    }

    #[test]
    fn manager_progress_and_cancel_download() {
        let mut manager = UpdateManager::new();
        let cancel = Arc::new(AtomicBool::new(false));
        manager.cancel_token = Some(cancel.clone());

        manager.set_status(UpdateStatus::Downloading {
            package: sample_package(),
            downloaded_bytes: 100,
            total_bytes: Some(1000),
        });

        assert!(manager.status().is_downloading());

        let cancelled = manager.cancel_download();
        assert!(cancelled);
        assert!(cancel.load(Ordering::Relaxed));
        assert!(matches!(*manager.status(), UpdateStatus::Cancelling { .. }));
        assert!(manager.status().is_busy());
    }

    #[test]
    fn manager_install_failed_updates_status() {
        let mut manager = UpdateManager::new();
        manager
            .sender
            .send(UpdateMsg::InstallFailed("握手超时".into()))
            .unwrap();
        assert!(manager.tick());
        assert_eq!(
            *manager.status(),
            UpdateStatus::Failed {
                reason: "握手超时".into(),
                can_retry: true,
            }
        );
    }

    #[test]
    fn manager_cancelling_state_and_busy_until_done() {
        let mut manager = UpdateManager::new();
        manager.active_task_id = Some(42);
        let cancel = Arc::new(AtomicBool::new(false));
        manager.cancel_token = Some(cancel.clone());
        manager.set_status(UpdateStatus::Downloading {
            package: sample_package(),
            downloaded_bytes: 50,
            total_bytes: Some(100),
        });

        // Cancel once -> transitions to Cancelling
        assert!(manager.cancel_download());
        assert!(cancel.load(Ordering::Relaxed));
        assert!(matches!(*manager.status(), UpdateStatus::Cancelling { .. }));
        assert!(manager.status().is_busy());

        // Duplicate cancel -> rejected
        assert!(!manager.cancel_download());

        // Cannot start download or check while busy cancelling
        let ctx = egui::Context::default();
        assert!(!manager.start_download(ctx.clone()));
        assert!(!manager.check_now(ctx, false));

        // Task completes cleanup and yields Cancelled
        manager
            .sender
            .send(UpdateMsg::DownloadDone {
                task_id: 42,
                outcome: DownloadOutcome::Cancelled,
            })
            .unwrap();

        assert!(manager.tick());
        assert!(matches!(
            *manager.status(),
            UpdateStatus::UpdateAvailable(..)
        ));
        assert!(!manager.status().is_busy());
        assert_eq!(manager.active_task_id, None);
        assert!(manager.cancel_token.is_none());
    }

    #[test]
    fn manager_ignores_stale_task_messages_and_protects_current_task() {
        let mut manager = UpdateManager::new();
        manager.set_status(UpdateStatus::UpdateAvailable(sample_package()));

        let ctx = egui::Context::default();
        // Start Task 1 via public entry point
        assert!(manager.start_download(ctx.clone()));
        assert_eq!(manager.active_task_id, Some(1));
        assert!(manager.cancel_download());
        assert!(matches!(*manager.status(), UpdateStatus::Cancelling { .. }));

        // Task 1 finishes cancellation and cleans up
        manager
            .sender
            .send(UpdateMsg::DownloadDone {
                task_id: 1,
                outcome: DownloadOutcome::Cancelled,
            })
            .unwrap();
        assert!(manager.tick());
        assert!(matches!(
            *manager.status(),
            UpdateStatus::UpdateAvailable(..)
        ));

        // Start Task 2 via public entry point
        assert!(manager.start_download(ctx));
        assert_eq!(manager.active_task_id, Some(2));
        assert!(matches!(
            *manager.status(),
            UpdateStatus::Downloading { .. }
        ));

        // Task 1 (old/cancelled) sends late progress
        manager
            .sender
            .send(UpdateMsg::DownloadProgress {
                task_id: 1,
                downloaded: 999,
                total: Some(1000),
            })
            .unwrap();
        assert!(!manager.tick(), "Stale progress must not trigger changes");
        if let UpdateStatus::Downloading {
            downloaded_bytes, ..
        } = manager.status()
        {
            assert_eq!(
                *downloaded_bytes, 0,
                "Active task downloaded_bytes must remain unchanged"
            );
        } else {
            panic!("Expected Downloading status");
        }

        // Task 1 sends late DownloadDone (e.g. Cancelled)
        manager
            .sender
            .send(UpdateMsg::DownloadDone {
                task_id: 1,
                outcome: DownloadOutcome::Cancelled,
            })
            .unwrap();
        assert!(!manager.tick(), "Stale DownloadDone must be dropped");
        assert!(matches!(
            *manager.status(),
            UpdateStatus::Downloading { .. }
        ));
        assert_eq!(manager.active_task_id, Some(2));
        assert!(
            manager.cancel_token.is_some(),
            "Task 2 cancel token must NOT be cleared"
        );

        // Task 2 can still be cancelled normally via public entry point
        assert!(manager.cancel_download());
        assert!(matches!(*manager.status(), UpdateStatus::Cancelling { .. }));
    }

    #[test]
    fn manager_background_thread_spawn_failure_exits_busy_and_shows_retryable_error() {
        let mut manager = UpdateManager::new();
        manager.set_status(UpdateStatus::UpdateAvailable(sample_package()));
        manager.set_thread_spawner(|_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "线程池资源已耗尽",
            ))
        });

        let ctx = egui::Context::default();
        let started = manager.start_download(ctx);
        assert!(!started, "start_download must fail if thread spawner fails");

        assert!(!manager.status().is_busy());
        assert_eq!(manager.active_task_id, None);
        assert!(manager.cancel_token.is_none());

        match manager.status() {
            UpdateStatus::Failed { reason, can_retry } => {
                assert!(can_retry);
                assert!(reason.contains("启动下载后台线程失败"));
                assert!(reason.contains("线程池资源已耗尽"));
            }
            other => panic!("Expected Failed status, got {other:?}"),
        }
    }

    #[test]
    fn manager_cleanup_failure_blocks_download_and_supports_retry() {
        let dir = tempfile::tempdir().unwrap();
        let failed_file = dir.path().join("residual.tmp");
        fs::write(&failed_file, b"corrupted-temp-bytes").unwrap();

        let mut manager = UpdateManager::new();
        manager.active_task_id = Some(10);
        manager.set_status(UpdateStatus::Cancelling {
            package: sample_package(),
        });

        manager
            .sender
            .send(UpdateMsg::DownloadDone {
                task_id: 10,
                outcome: DownloadOutcome::CleanupFailed {
                    failed_path: failed_file.clone(),
                    reason: "文件正由另一进程使用".into(),
                },
            })
            .unwrap();

        assert!(manager.tick());
        match manager.status() {
            UpdateStatus::CleanupFailed {
                failed_path,
                reason,
                ..
            } => {
                assert_eq!(failed_path, &failed_file);
                assert!(reason.contains("文件正由另一进程使用"));
            }
            other => panic!("Expected CleanupFailed status, got {other:?}"),
        }

        // While in CleanupFailed, starting download or checking updates is blocked
        assert!(manager.status().is_busy());
        let ctx = egui::Context::default();
        assert!(!manager.start_download(ctx.clone()));
        assert!(!manager.check_now(ctx, false));

        // User invokes retry_cleanup -> deletes file and restores UpdateAvailable
        let recovered = manager.retry_cleanup();
        assert!(recovered);
        assert!(!failed_file.exists());
        assert!(matches!(
            *manager.status(),
            UpdateStatus::UpdateAvailable(..)
        ));
        assert!(!manager.status().is_busy());
    }

    #[test]
    fn manager_deterministic_flow_download_a_cancel_reject_retry_download_b() {
        use std::sync::mpsc::channel;

        let mut manager = UpdateManager::new();
        manager.set_status(UpdateStatus::UpdateAvailable(sample_package()));
        let (task_a_unblock_tx, task_a_unblock_rx) = channel::<()>();
        let task_a_unblock_rx = Arc::new(Mutex::new(task_a_unblock_rx));
        let (task_a_cancelled_tx, task_a_cancelled_rx) = channel::<()>();

        let (task_b_unblock_tx, task_b_unblock_rx) = channel::<()>();
        let task_b_unblock_rx = Arc::new(Mutex::new(task_b_unblock_rx));
        let (task_b_cancelled_tx, task_b_cancelled_rx) = channel::<()>();

        let (task_c_finish_tx, task_c_finish_rx) = channel::<()>();
        let task_c_finish_rx = Arc::new(Mutex::new(task_c_finish_rx));

        manager.set_download_handler(move |_asset, _cs, _ver, _dir, task_id, cancel, _prog| {
            if task_id == 1 {
                // Wait until cancel is triggered
                while !cancel.load(Ordering::Relaxed) {
                    std::thread::yield_now();
                }
                task_a_cancelled_tx.send(()).unwrap();
                task_a_unblock_rx.lock().unwrap().recv().unwrap();
                DownloadOutcome::Cancelled
            } else if task_id == 2 {
                // Wait until cancel is triggered for Task B
                while !cancel.load(Ordering::Relaxed) {
                    std::thread::yield_now();
                }
                task_b_cancelled_tx.send(()).unwrap();
                task_b_unblock_rx.lock().unwrap().recv().unwrap();
                DownloadOutcome::Cancelled
            } else if task_id == 3 {
                task_c_finish_rx.lock().unwrap().recv().unwrap();
                DownloadOutcome::Success {
                    installer_path: PathBuf::from("setup.exe"),
                    expected_sha256: "hash123".into(),
                    target_version: "0.2.0".into(),
                }
            } else {
                panic!("Unexpected task_id {task_id}");
            }
        });

        let ctx = egui::Context::default();

        // 1. Start Download A (Task 1)
        assert!(manager.start_download(ctx.clone()));
        assert_eq!(manager.active_task_id, Some(1));
        assert!(matches!(
            *manager.status(),
            UpdateStatus::Downloading { .. }
        ));

        // 2. Cancel Download A
        assert!(manager.cancel_download());
        assert!(matches!(*manager.status(), UpdateStatus::Cancelling { .. }));

        // 3. During Cancelling, duplicate cancel and new download are both rejected
        assert!(!manager.cancel_download());
        assert!(!manager.start_download(ctx.clone()));

        // 4. Wait for Task A to observe cancellation
        task_a_cancelled_rx.recv().unwrap();
        // Allow Task A cleanup to complete
        task_a_unblock_tx.send(()).unwrap();

        // 5. Poll tick() until Task A completes
        while manager.active_task_id.is_some() {
            manager.tick();
            std::thread::yield_now();
        }
        assert!(matches!(
            *manager.status(),
            UpdateStatus::UpdateAvailable(..)
        ));

        // 6. Start Download B (Task 2)
        assert!(manager.start_download(ctx.clone()));
        assert_eq!(manager.active_task_id, Some(2));
        assert!(matches!(
            *manager.status(),
            UpdateStatus::Downloading { .. }
        ));

        // 7. Stale late messages from Task 1 arrive during Task 2
        manager
            .sender
            .send(UpdateMsg::DownloadProgress {
                task_id: 1,
                downloaded: 9999,
                total: Some(9999),
            })
            .unwrap();
        manager
            .sender
            .send(UpdateMsg::DownloadDone {
                task_id: 1,
                outcome: DownloadOutcome::Cancelled,
            })
            .unwrap();
        manager.tick();
        // Task 2 status and cancel capability remain completely intact
        assert!(matches!(
            *manager.status(),
            UpdateStatus::Downloading { .. }
        ));
        assert_eq!(manager.active_task_id, Some(2));

        // 8. Explicitly verify Task B cancellation capability via public API
        assert!(
            manager.cancel_download(),
            "Task B must be cancellable after A's stale messages"
        );
        assert!(matches!(*manager.status(), UpdateStatus::Cancelling { .. }));

        // Allow Task B to observe cancellation and complete cleanup
        task_b_cancelled_rx.recv().unwrap();
        task_b_unblock_tx.send(()).unwrap();
        while manager.active_task_id.is_some() {
            manager.tick();
            std::thread::yield_now();
        }
        assert!(matches!(
            *manager.status(),
            UpdateStatus::UpdateAvailable(..)
        ));

        // 9. Start Download C (Task 3) and complete successfully
        assert!(manager.start_download(ctx.clone()));
        assert_eq!(manager.active_task_id, Some(3));
        task_c_finish_tx.send(()).unwrap();
        while manager.active_task_id.is_some() {
            manager.tick();
            std::thread::yield_now();
        }
        assert!(matches!(
            *manager.status(),
            UpdateStatus::ReadyToInstall { .. }
        ));

        // 10. Stale messages from Task 1 and Task 2 arrive when ReadyToInstall
        manager
            .sender
            .send(UpdateMsg::DownloadDone {
                task_id: 1,
                outcome: DownloadOutcome::Failed {
                    reason: "Late failure from task 1".into(),
                    can_retry: true,
                },
            })
            .unwrap();
        manager
            .sender
            .send(UpdateMsg::DownloadDone {
                task_id: 2,
                outcome: DownloadOutcome::Cancelled,
            })
            .unwrap();
        manager.tick();
        // ReadyToInstall remains untouched!
        assert!(matches!(
            *manager.status(),
            UpdateStatus::ReadyToInstall { .. }
        ));
    }
}
