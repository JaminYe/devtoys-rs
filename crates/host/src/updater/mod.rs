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
                | Self::Verifying { .. }
                | Self::PreparingInstall { .. }
        )
    }

    #[allow(dead_code)]
    pub fn is_downloading(&self) -> bool {
        matches!(self, Self::Downloading { .. } | Self::Verifying { .. })
    }
}

pub enum UpdateMsg {
    Check(CheckOutcome),
    DownloadProgress {
        downloaded: u64,
        total: Option<u64>,
    },
    #[allow(dead_code)]
    Verifying,
    DownloadDone(DownloadOutcome),
    InstallFailed(String),
}

pub type CheckHandler = Arc<dyn Fn(&SemVer) -> CheckOutcome + Send + Sync>;
pub type DownloadHandler = Arc<
    dyn Fn(
            &UpdateAsset,
            Option<&UpdateAsset>,
            &str,
            &Path,
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
        F: Fn(&SemVer) -> CheckOutcome + Send + Sync + 'static,
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
                &AtomicBool,
                &dyn Fn(u64, Option<u64>),
            ) -> DownloadOutcome
            + Send
            + Sync
            + 'static,
    {
        self.download_handler = Some(Arc::new(handler));
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
                UpdateMsg::DownloadProgress { downloaded, total } => {
                    if let UpdateStatus::Downloading { ref package, .. } = self.status {
                        self.status = UpdateStatus::Downloading {
                            package: package.clone(),
                            downloaded_bytes: downloaded,
                            total_bytes: total,
                        };
                        changed = true;
                    }
                }
                UpdateMsg::Verifying => {
                    if let UpdateStatus::Downloading { ref package, .. } = self.status {
                        self.status = UpdateStatus::Verifying(package.clone());
                        changed = true;
                    }
                }
                UpdateMsg::DownloadDone(outcome) => {
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
                            | UpdateStatus::Verifying(package) => {
                                UpdateStatus::UpdateAvailable(package.clone())
                            }
                            _ => UpdateStatus::Idle,
                        },
                        DownloadOutcome::Failed { reason, can_retry } => {
                            UpdateStatus::Failed { reason, can_retry }
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
    pub fn check_now(&mut self, ctx: egui::Context) -> bool {
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
                    Some(handler) => handler(&current_version),
                    None => {
                        let client = client::UreqClient;
                        client::check_updates(
                            &client,
                            client::DEFAULT_RELEASES_URL,
                            &current_version,
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

        thread::Builder::new()
            .name("devtoys-update-downloader".into())
            .spawn(move || {
                let s_prog = sender.clone();
                let ctx_prog = ctx.clone();
                let on_progress = move |downloaded: u64, total: Option<u64>| {
                    let _ = s_prog.send(UpdateMsg::DownloadProgress { downloaded, total });
                    ctx_prog.request_repaint();
                };

                let outcome = match download_handler {
                    Some(handler) => handler(
                        &package.asset,
                        package.checksum_asset.as_ref(),
                        &package.latest_version,
                        &update_dir,
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
                            &cancel,
                            on_progress,
                        )
                    }
                };

                let _ = sender.send(UpdateMsg::DownloadDone(outcome));
                ctx.request_repaint();
            })
            .ok();

        true
    }

    /// Cancels an in-progress download.
    pub fn cancel_download(&mut self) -> bool {
        if let Some(token) = &self.cancel_token {
            token.store(true, Ordering::Relaxed);
            match &self.status {
                UpdateStatus::Downloading { package, .. } | UpdateStatus::Verifying(package) => {
                    self.status = UpdateStatus::UpdateAvailable(package.clone());
                }
                _ => {}
            }
            true
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
        assert!(!manager.check_now(ctx.clone()));
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
        assert!(matches!(
            *manager.status(),
            UpdateStatus::UpdateAvailable(..)
        ));
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
}
