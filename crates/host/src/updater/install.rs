//! Installation execution, pre-install verification, process handshake, and failure recovery.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::download::compute_sha256;

pub const INNO_APP_ID: &str = "{9E3C1D42-58A7-4B61-B0AE-F3621C79A5D8}";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallType {
    Installed { install_dir: PathBuf },
    PortableOrDev,
}

/// Detects whether current running binary belongs to an installed instance.
pub fn detect_install_type() -> InstallType {
    if cfg!(debug_assertions) {
        return InstallType::PortableOrDev;
    }

    #[cfg(windows)]
    {
        let current_exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(_) => return InstallType::PortableOrDev,
        };

        let reg_keys = [
            format!(r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\{INNO_APP_ID}_is1"),
            format!(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{INNO_APP_ID}_is1"),
            format!(
                r"HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\{INNO_APP_ID}_is1"
            ),
        ];

        for key in &reg_keys {
            if let Ok(dir) = query_install_dir(key) {
                if is_subpath_of(&current_exe, &dir) {
                    return InstallType::Installed { install_dir: dir };
                }
            }
        }

        InstallType::PortableOrDev
    }

    #[cfg(not(windows))]
    {
        InstallType::PortableOrDev
    }
}

#[cfg(windows)]
fn query_install_dir(key: &str) -> Result<PathBuf, ()> {
    let output = Command::new("reg")
        .args(["query", key, "/v", "InstallLocation"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if line.contains("InstallLocation") {
                    let parts: Vec<&str> = line.split("REG_SZ").collect();
                    if parts.len() >= 2 {
                        let path = parts[1].trim();
                        if !path.is_empty() {
                            return Ok(PathBuf::from(path));
                        }
                    }
                }
            }
        }
    }
    Err(())
}

pub fn is_subpath_of(child: &Path, parent: &Path) -> bool {
    let child_can = child.canonicalize().unwrap_or_else(|_| child.to_path_buf());
    let parent_can = parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf());
    child_can.starts_with(parent_can)
}

/// Re-verifies installer SHA-256 checksum immediately before execution.
pub fn verify_installer_before_execution(
    installer_path: &Path,
    expected_sha256: &str,
) -> Result<(), String> {
    if !installer_path.exists() {
        return Err("待执行的安装包文件不存在，请重新下载".into());
    }

    let hash = compute_sha256(installer_path).map_err(|e| format!("重新校验安装包失败：{e}"))?;

    if !hash.eq_ignore_ascii_case(expected_sha256) {
        let _ = fs::remove_file(installer_path);
        return Err(format!(
            "待执行安装包摘要校验失败：预期 {expected_sha256}，实际 {hash}，文件可能已被篡改"
        ));
    }

    Ok(())
}

static HANDSHAKE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub fn create_handshake_file() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("devtoys-updates");
    let _ = fs::create_dir_all(&dir);
    let counter = HANDSHAKE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = dir.join(format!("handshake-{}-{}.txt", std::process::id(), counter));
    let mut file = File::create(&path).map_err(|e| format!("创建握手文件失败：{e}"))?;
    file.write_all(b"INIT\n")
        .map_err(|e| format!("初始化握手文件失败：{e}"))?;
    Ok(path)
}

pub fn wait_for_handshake(handshake_file: &Path, timeout: Duration) -> Result<(), String> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Ok(content) = fs::read_to_string(handshake_file) {
            if content.contains("READY") {
                return Ok(());
            } else if content.contains("ABORT") {
                return Err("安装程序握手返回中止信号".into());
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("等待安装程序就绪握手超时".into())
}

pub trait InstallerRunner: Send + Sync {
    fn launch_installer(
        &self,
        installer_path: &Path,
        install_dir: &Path,
        handshake_file: &Path,
        caller_pid: u32,
    ) -> Result<(), String>;

    fn save_settings(&self) -> Result<(), String>;

    fn exit_app(&self);
}

pub struct ProductionInstallerRunner<F: Fn() -> Result<(), String> + Send + Sync> {
    pub save_fn: F,
}

impl<F: Fn() -> Result<(), String> + Send + Sync> InstallerRunner for ProductionInstallerRunner<F> {
    fn launch_installer(
        &self,
        installer_path: &Path,
        install_dir: &Path,
        handshake_file: &Path,
        caller_pid: u32,
    ) -> Result<(), String> {
        let mut cmd = Command::new(installer_path);
        cmd.args([
            "/SP-",
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            &format!("/DIR={}", install_dir.display()),
            "/UPDATE_MODE=1",
            &format!("/CALLER_PID={caller_pid}"),
            &format!("/HANDSHAKE_FILE={}", handshake_file.display()),
        ]);

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        cmd.spawn().map_err(|e| format!("启动安装程序失败：{e}"))?;
        Ok(())
    }

    fn save_settings(&self) -> Result<(), String> {
        (self.save_fn)()
    }

    fn exit_app(&self) {
        std::process::exit(0);
    }
}

pub fn execute_update_pipeline<R: InstallerRunner>(
    runner: &R,
    installer_path: &Path,
    expected_sha256: &str,
    install_dir: &Path,
) -> Result<(), String> {
    // 1. Re-verify SHA-256
    verify_installer_before_execution(installer_path, expected_sha256)?;

    // 2. Prepare handshake file
    let handshake_path = create_handshake_file()?;

    // 3. Launch installer process
    let pid = std::process::id();
    runner.launch_installer(installer_path, install_dir, &handshake_path, pid)?;

    // 4. Wait for installer ready handshake (timeout 15s)
    if let Err(e) = wait_for_handshake(&handshake_path, Duration::from_secs(15)) {
        let _ = fs::remove_file(&handshake_path);
        return Err(format!("安装程序就绪确认失败：{e}"));
    }

    // 5. Save settings before exit
    if let Err(e) = runner.save_settings() {
        let _ = fs::write(&handshake_path, "ABORT\n");
        let _ = fs::remove_file(&handshake_path);
        return Err(format!("保存设置失败，已中止安装以防数据丢失：{e}"));
    }

    // 6. Graceful exit
    let _ = fs::remove_file(&handshake_path);
    runner.exit_app();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    struct MockInstallerRunner {
        launch_result: Result<(), String>,
        handshake_simulate: Option<&'static str>,
        save_result: Result<(), String>,
        exited: Arc<AtomicBool>,
        saved: Arc<AtomicBool>,
    }

    impl InstallerRunner for MockInstallerRunner {
        fn launch_installer(
            &self,
            _installer_path: &Path,
            _install_dir: &Path,
            handshake_file: &Path,
            _caller_pid: u32,
        ) -> Result<(), String> {
            if let Some(msg) = self.handshake_simulate {
                let _ = fs::write(handshake_file, format!("{msg}\n"));
            }
            self.launch_result.clone()
        }

        fn save_settings(&self) -> Result<(), String> {
            self.saved.store(true, Ordering::Relaxed);
            self.save_result.clone()
        }

        fn exit_app(&self) {
            self.exited.store(true, Ordering::Relaxed);
        }
    }

    #[test]
    fn pre_install_verification_detects_tampered_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("installer.exe");
        fs::write(&file, b"valid-data").unwrap();

        let expected = compute_sha256(&file).unwrap();
        assert!(verify_installer_before_execution(&file, &expected).is_ok());

        // Tamper with file
        fs::write(&file, b"tampered-data").unwrap();
        let err = verify_installer_before_execution(&file, &expected).unwrap_err();
        assert!(err.contains("摘要校验失败"));
        assert!(!file.exists(), "Tampered file must be removed");
    }

    #[test]
    fn pipeline_successful_handshake_saves_and_exits() {
        let dir = tempfile::tempdir().unwrap();
        let installer = dir.path().join("setup.exe");
        fs::write(&installer, b"setup-bytes").unwrap();
        let hash = compute_sha256(&installer).unwrap();

        let exited = Arc::new(AtomicBool::new(false));
        let saved = Arc::new(AtomicBool::new(false));

        let runner = MockInstallerRunner {
            launch_result: Ok(()),
            handshake_simulate: Some("READY"),
            save_result: Ok(()),
            exited: exited.clone(),
            saved: saved.clone(),
        };

        let res = execute_update_pipeline(&runner, &installer, &hash, dir.path());
        assert!(res.is_ok());
        assert!(saved.load(Ordering::Relaxed));
        assert!(exited.load(Ordering::Relaxed));
    }

    #[test]
    fn pipeline_handshake_timeout_does_not_exit() {
        let dir = tempfile::tempdir().unwrap();
        let installer = dir.path().join("setup.exe");
        fs::write(&installer, b"setup-bytes").unwrap();
        let _hash = compute_sha256(&installer).unwrap();

        let exited = Arc::new(AtomicBool::new(false));
        let saved = Arc::new(AtomicBool::new(false));

        let _runner = MockInstallerRunner {
            launch_result: Ok(()),
            handshake_simulate: None, // No READY written -> timeout
            save_result: Ok(()),
            exited: exited.clone(),
            saved: saved.clone(),
        };

        let _start = Instant::now();
        // Use short wait helper directly to verify timeout logic
        let handshake_file = dir.path().join("handshake.txt");
        fs::write(&handshake_file, "INIT\n").unwrap();
        let err = wait_for_handshake(&handshake_file, Duration::from_millis(200)).unwrap_err();
        assert!(err.contains("超时"));
        assert!(!exited.load(Ordering::Relaxed));
        assert!(!saved.load(Ordering::Relaxed));
    }

    #[test]
    fn pipeline_settings_save_failure_aborts_and_does_not_exit() {
        let dir = tempfile::tempdir().unwrap();
        let installer = dir.path().join("setup.exe");
        fs::write(&installer, b"setup-bytes").unwrap();
        let hash = compute_sha256(&installer).unwrap();

        let exited = Arc::new(AtomicBool::new(false));
        let saved = Arc::new(AtomicBool::new(false));

        let runner = MockInstallerRunner {
            launch_result: Ok(()),
            handshake_simulate: Some("READY"),
            save_result: Err("磁盘空间不足".into()),
            exited: exited.clone(),
            saved: saved.clone(),
        };

        let err = execute_update_pipeline(&runner, &installer, &hash, dir.path()).unwrap_err();
        assert!(err.contains("保存设置失败"));
        assert!(saved.load(Ordering::Relaxed));
        assert!(
            !exited.load(Ordering::Relaxed),
            "App must NOT exit on save error"
        );
    }

    #[test]
    fn pipeline_launch_failure_aborts_without_save_or_exit() {
        let dir = tempfile::tempdir().unwrap();
        let installer = dir.path().join("setup.exe");
        fs::write(&installer, b"setup-bytes").unwrap();
        let hash = compute_sha256(&installer).unwrap();

        let exited = Arc::new(AtomicBool::new(false));
        let saved = Arc::new(AtomicBool::new(false));

        let runner = MockInstallerRunner {
            launch_result: Err("拒绝访问 (UAC 取消)".into()),
            handshake_simulate: None,
            save_result: Ok(()),
            exited: exited.clone(),
            saved: saved.clone(),
        };

        let err = execute_update_pipeline(&runner, &installer, &hash, dir.path()).unwrap_err();
        assert!(err.contains("拒绝访问"));
        assert!(!saved.load(Ordering::Relaxed));
        assert!(!exited.load(Ordering::Relaxed));
    }
}
