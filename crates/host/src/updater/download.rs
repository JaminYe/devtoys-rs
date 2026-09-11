//! Streaming download, progress tracking, and SHA-256 verification.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};

use super::client::UpdateAsset;

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DownloadOutcome {
    Success {
        installer_path: PathBuf,
        expected_sha256: String,
        target_version: String,
    },
    Cancelled,
    Failed {
        reason: String,
        can_retry: bool,
    },
    CleanupFailed {
        failed_path: PathBuf,
        reason: String,
    },
}

pub fn parse_checksums_txt(content: &str, target_filename: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 2 {
            let hash = parts[0].trim().to_lowercase();
            let fname = parts[1].trim().trim_start_matches('*');
            if fname.eq_ignore_ascii_case(target_filename)
                && hash.len() == 64
                && hash.chars().all(|c| c.is_ascii_hexdigit())
            {
                return Some(hash);
            }
        }
    }
    None
}

pub fn compute_sha256(path: &Path) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub trait FileDownloader: Send + Sync {
    fn download(
        &self,
        url: &str,
        dest: &Path,
        cancel: &AtomicBool,
        on_progress: &dyn Fn(u64, Option<u64>),
    ) -> Result<(), String>;

    fn fetch_text(&self, url: &str) -> Result<String, String>;
}

pub struct UreqFileDownloader;

fn create_agent(connect_secs: u64, read_secs: u64) -> ureq::Agent {
    let mut builder = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(connect_secs))
        .timeout_read(std::time::Duration::from_secs(read_secs))
        .user_agent(&format!("devtoys-rs/{}", crate::version::CURRENT_VERSION));

    if let Some(proxy_url) = super::client::detect_proxy_url() {
        if let Ok(proxy) = ureq::Proxy::new(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }

    builder.build()
}

impl FileDownloader for UreqFileDownloader {
    fn download(
        &self,
        url: &str,
        dest: &Path,
        cancel: &AtomicBool,
        on_progress: &dyn Fn(u64, Option<u64>),
    ) -> Result<(), String> {
        let agent = create_agent(15, 20);

        let resp = agent.get(url).call().map_err(|e| match e {
            ureq::Error::Status(code, _) => format!("下载请求失败（HTTP {code}）"),
            ureq::Error::Transport(_) => "网络连接中断或失败，请检查网络设置后重试".to_string(),
        })?;
        let total_bytes = resp
            .header("Content-Length")
            .and_then(|val| val.parse::<u64>().ok());

        let part_path = dest.with_extension("part");
        if let Some(parent) = part_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut file =
            File::create(&part_path).map_err(|e| format!("创建临时下载文件失败：{e}"))?;

        let mut reader = resp.into_reader();
        let mut buffer = [0u8; 64 * 1024];
        let mut downloaded: u64 = 0;

        let download_res = (|| -> Result<(), String> {
            loop {
                if cancel.load(Ordering::Relaxed) {
                    return Err("cancelled".into());
                }

                let n = reader
                    .read(&mut buffer)
                    .map_err(|e| format!("数据流读取中断：{e}"))?;

                if n == 0 {
                    break;
                }

                file.write_all(&buffer[..n])
                    .map_err(|e| format!("写入临时文件失败：{e}"))?;

                downloaded += n as u64;
                on_progress(downloaded, total_bytes);
            }

            file.flush()
                .map_err(|e| format!("刷新文件缓冲区失败：{e}"))?;
            Ok(())
        })();

        drop(file);

        let remove_part = |path: &Path| -> Result<(), String> {
            if path.exists() {
                if let Err(cleanup_err) = fs::remove_file(path) {
                    return Err(format!("cleanup_failed|{}|{}", path.display(), cleanup_err));
                }
            }
            Ok(())
        };

        if let Err(e) = download_res {
            remove_part(&part_path)?;
            return Err(e);
        }
        if let Some(expected_size) = total_bytes {
            if downloaded != expected_size {
                remove_part(&part_path)?;
                return Err(format!(
                    "下载文件大小与 Content-Length 不符（已下载 {downloaded} 字节，预期 {expected_size} 字节）"
                ));
            }
        }

        if let Err(e) = fs::rename(&part_path, dest) {
            remove_part(&part_path)?;
            return Err(format!("重命名下载文件失败：{e}"));
        }

        Ok(())
    }

    fn fetch_text(&self, url: &str) -> Result<String, String> {
        let agent = create_agent(10, 10);

        let resp = agent.get(url).call().map_err(|e| match e {
            ureq::Error::Status(code, _) => format!("请求校验文件失败（HTTP {code}）"),
            ureq::Error::Transport(_) => "网络连接超时或中断，获取校验文件失败".to_string(),
        })?;
        resp.into_string()
            .map_err(|e| format!("读取校验文件内容失败：{e}"))
    }
}

fn cleanup_staging_file(path: &Path, action_desc: &str) -> Result<(), DownloadOutcome> {
    if path.exists() {
        if let Err(err) = fs::remove_file(path) {
            return Err(DownloadOutcome::CleanupFailed {
                failed_path: path.to_path_buf(),
                reason: format!("{action_desc}：{err}"),
            });
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn download_and_verify<D: FileDownloader>(
    downloader: &D,
    installer_asset: &UpdateAsset,
    checksum_asset: Option<&UpdateAsset>,
    target_version: &str,
    target_dir: &Path,
    task_id: u64,
    cancel: &AtomicBool,
    on_progress: impl Fn(u64, Option<u64>),
) -> DownloadOutcome {
    // 1. Resolve expected SHA-256 hash
    // Priority 1: Direct Release asset SHA-256 digest
    // Priority 2: Fallback to checksums.txt in the same release
    let expected_hash = match &installer_asset.sha256 {
        Some(hash) => hash.clone(),
        None => match checksum_asset {
            Some(cs_asset) => match downloader.fetch_text(&cs_asset.download_url) {
                Ok(content) => match parse_checksums_txt(&content, &installer_asset.name) {
                    Some(hash) => hash,
                    None => {
                        return DownloadOutcome::Failed {
                            reason: format!(
                                "校验文件 checksums.txt 中未包含安装包 {} 的 SHA-256 摘要",
                                installer_asset.name
                            ),
                            can_retry: true,
                        };
                    }
                },
                Err(e) => {
                    return DownloadOutcome::Failed {
                        reason: format!("获取校验文件失败：{e}"),
                        can_retry: true,
                    };
                }
            },
            None => {
                return DownloadOutcome::Failed {
                    reason: "此版本未提供 SHA-256 校验摘要或校验文件，禁止下载未经校验的安装包"
                        .into(),
                    can_retry: true,
                };
            }
        },
    };
    if cancel.load(Ordering::Relaxed) {
        return DownloadOutcome::Cancelled;
    }

    // 2. Download to task-specific staging file
    let _ = fs::create_dir_all(target_dir);
    let final_dest = target_dir.join(&installer_asset.name);
    let staging_dest = target_dir.join(format!("{}.task_{}.tmp", installer_asset.name, task_id));

    if let Err(e) = downloader.download(
        &installer_asset.download_url,
        &staging_dest,
        cancel,
        &on_progress,
    ) {
        if let Some(rest) = e.strip_prefix("cleanup_failed|") {
            if let Some((path_str, err_msg)) = rest.split_once('|') {
                return DownloadOutcome::CleanupFailed {
                    failed_path: PathBuf::from(path_str),
                    reason: format!("清理临时文件失败：{err_msg}"),
                };
            }
        }
        if let Err(outcome) = cleanup_staging_file(&staging_dest, "清理临时文件失败") {
            return outcome;
        }
        if e == "cancelled" || cancel.load(Ordering::Relaxed) {
            return DownloadOutcome::Cancelled;
        }
        return DownloadOutcome::Failed {
            reason: format!("下载安装包失败：{e}"),
            can_retry: true,
        };
    }

    if cancel.load(Ordering::Relaxed) {
        if let Err(outcome) = cleanup_staging_file(&staging_dest, "清理临时文件失败") {
            return outcome;
        }
        return DownloadOutcome::Cancelled;
    }

    // 3. 计算并校验 SHA-256 摘要
    let computed_hash = match compute_sha256(&staging_dest) {
        Ok(h) => h,
        Err(e) => {
            if let Err(outcome) = cleanup_staging_file(&staging_dest, "校验失败后清理临时文件失败")
            {
                return outcome;
            }
            return DownloadOutcome::Failed {
                reason: format!("计算安装包 SHA-256 摘要失败：{e}"),
                can_retry: true,
            };
        }
    };

    if !computed_hash.eq_ignore_ascii_case(&expected_hash) {
        if let Err(outcome) = cleanup_staging_file(&staging_dest, "校验失败后清理损坏文件失败")
        {
            return outcome;
        }
        return DownloadOutcome::Failed {
            reason: format!(
                "SHA-256 校验不匹配：预期 {expected_hash}，实际计算得到 {computed_hash}，文件可能已损坏"
            ),
            can_retry: true,
        };
    }

    // 校验完成后检查是否已取消；若取消则收尾清理并退出，严禁进入安装就绪
    if cancel.load(Ordering::Relaxed) {
        if let Err(outcome) = cleanup_staging_file(&staging_dest, "清理临时文件失败") {
            return outcome;
        }
        return DownloadOutcome::Cancelled;
    }

    // 4. 校验通过 - 安全重命名至目标安装文件
    if let Err(e) = fs::rename(&staging_dest, &final_dest) {
        if let Err(outcome) = cleanup_staging_file(&staging_dest, "重命名失败后清理临时文件失败")
        {
            return outcome;
        }
        return DownloadOutcome::Failed {
            reason: format!("重命名已校验安装包失败：{e}"),
            can_retry: true,
        };
    }
    DownloadOutcome::Success {
        installer_path: final_dest,
        expected_sha256: expected_hash,
        target_version: target_version.to_string(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_checksums_various_formats() {
        let text = r#"
# devtoys checksums
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  devtoys-x86_64-pc-windows-msvc-setup.exe
2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae *devtoys-mac.tar.gz
"#;
        let found = parse_checksums_txt(text, "devtoys-x86_64-pc-windows-msvc-setup.exe");
        assert_eq!(
            found,
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into())
        );

        // Case insensitive match on filename
        let found2 = parse_checksums_txt(text, "DEVTOYS-X86_64-PC-WINDOWS-MSVC-SETUP.EXE");
        assert_eq!(
            found2,
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into())
        );

        // Missing file
        assert_eq!(parse_checksums_txt(text, "other.exe"), None);
    }

    struct MockDownloader {
        download_data: Result<Vec<u8>, String>,
        checksum_text: Result<String, String>,
    }

    impl FileDownloader for MockDownloader {
        fn download(
            &self,
            _url: &str,
            dest: &Path,
            cancel: &AtomicBool,
            on_progress: &dyn Fn(u64, Option<u64>),
        ) -> Result<(), String> {
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".into());
            }
            match &self.download_data {
                Ok(bytes) => {
                    on_progress(bytes.len() as u64, Some(bytes.len() as u64));
                    fs::write(dest, bytes).map_err(|e| e.to_string())?;
                    Ok(())
                }
                Err(e) => Err(e.clone()),
            }
        }

        fn fetch_text(&self, _url: &str) -> Result<String, String> {
            self.checksum_text.clone()
        }
    }

    #[test]
    fn download_success_and_verify_sha256() {
        let dir = tempfile::tempdir().unwrap();
        let payload = b"devtoys-installer-binary-data";
        let expected_hash = hex::encode(Sha256::digest(payload));

        let checksum_text = format!("{expected_hash}  devtoys-x86_64-pc-windows-msvc-setup.exe\n");
        let downloader = MockDownloader {
            download_data: Ok(payload.to_vec()),
            checksum_text: Ok(checksum_text),
        };

        let installer_asset = UpdateAsset {
            name: "devtoys-x86_64-pc-windows-msvc-setup.exe".into(),
            download_url: "http://mock/setup.exe".into(),
            size: payload.len() as u64,
            sha256: None,
        };
        let checksum_asset = UpdateAsset {
            name: "checksums.txt".into(),
            download_url: "http://mock/checksums.txt".into(),
            size: 100,
            sha256: None,
        };

        let cancel = AtomicBool::new(false);
        let outcome = download_and_verify(
            &downloader,
            &installer_asset,
            Some(&checksum_asset),
            "0.2.0",
            dir.path(),
            1,
            &cancel,
            |_, _| {},
        );

        match outcome {
            DownloadOutcome::Success {
                installer_path,
                expected_sha256,
                target_version,
            } => {
                assert!(installer_path.exists());
                assert_eq!(expected_sha256, expected_hash);
                assert_eq!(target_version, "0.2.0");
            }
            other => panic!("Expected Success, got {other:?}"),
        }
    }

    #[test]
    fn download_corrupted_payload_fails_and_deletes_file() {
        let dir = tempfile::tempdir().unwrap();
        let payload = b"devtoys-installer-binary-data";
        let bad_hash = "0000000000000000000000000000000000000000000000000000000000000000";

        let checksum_text = format!("{bad_hash}  devtoys-x86_64-pc-windows-msvc-setup.exe\n");
        let downloader = MockDownloader {
            download_data: Ok(payload.to_vec()),
            checksum_text: Ok(checksum_text),
        };

        let installer_asset = UpdateAsset {
            name: "devtoys-x86_64-pc-windows-msvc-setup.exe".into(),
            download_url: "http://mock/setup.exe".into(),
            size: payload.len() as u64,
            sha256: None,
        };
        let checksum_asset = UpdateAsset {
            name: "checksums.txt".into(),
            download_url: "http://mock/checksums.txt".into(),
            size: 100,
            sha256: None,
        };

        let cancel = AtomicBool::new(false);
        let outcome = download_and_verify(
            &downloader,
            &installer_asset,
            Some(&checksum_asset),
            "0.2.0",
            dir.path(),
            1,
            &cancel,
            |_, _| {},
        );

        match outcome {
            DownloadOutcome::Failed { reason, can_retry } => {
                assert!(can_retry);
                assert!(reason.contains("SHA-256 校验不匹配"));
                let dest = dir.path().join(&installer_asset.name);
                assert!(!dest.exists(), "Corrupted file must be deleted");
            }
            other => panic!("Expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn download_cancelled_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let payload = b"devtoys-installer-binary-data";
        let expected_hash = hex::encode(Sha256::digest(payload));

        let checksum_text = format!("{expected_hash}  devtoys-x86_64-pc-windows-msvc-setup.exe\n");
        let downloader = MockDownloader {
            download_data: Ok(payload.to_vec()),
            checksum_text: Ok(checksum_text),
        };

        let installer_asset = UpdateAsset {
            name: "devtoys-x86_64-pc-windows-msvc-setup.exe".into(),
            download_url: "http://mock/setup.exe".into(),
            size: payload.len() as u64,
            sha256: None,
        };
        let checksum_asset = UpdateAsset {
            name: "checksums.txt".into(),
            download_url: "http://mock/checksums.txt".into(),
            size: 100,
            sha256: None,
        };

        let cancel = AtomicBool::new(true); // Cancelled from the start
        let outcome = download_and_verify(
            &downloader,
            &installer_asset,
            Some(&checksum_asset),
            "0.2.0",
            dir.path(),
            1,
            &cancel,
            |_, _| {},
        );

        assert_eq!(outcome, DownloadOutcome::Cancelled);
        let dest = dir.path().join(&installer_asset.name);
        assert!(!dest.exists());
    }

    #[test]
    fn download_missing_checksums_fails() {
        let dir = tempfile::tempdir().unwrap();
        let downloader = MockDownloader {
            download_data: Ok(vec![]),
            checksum_text: Ok("".into()),
        };

        let installer_asset = UpdateAsset {
            name: "devtoys-x86_64-pc-windows-msvc-setup.exe".into(),
            download_url: "http://mock/setup.exe".into(),
            size: 100,
            sha256: None,
        };

        let cancel = AtomicBool::new(false);
        // None checksum asset
        let outcome = download_and_verify(
            &downloader,
            &installer_asset,
            None,
            "0.2.0",
            dir.path(),
            1,
            &cancel,
            |_, _| {},
        );

        match outcome {
            DownloadOutcome::Failed { reason, can_retry } => {
                assert!(can_retry);
                assert!(reason.contains("未提供 SHA-256"));
            }
            other => panic!("Expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn download_uses_direct_sha256_digest_without_checksums_txt() {
        let dir = tempfile::tempdir().unwrap();
        let payload = b"devtoys-installer-binary-data";
        let expected_hash = hex::encode(Sha256::digest(payload));

        let downloader = MockDownloader {
            download_data: Ok(payload.to_vec()),
            checksum_text: Err("Should not be fetched".into()),
        };

        let installer_asset = UpdateAsset {
            name: "devtoys-x86_64-pc-windows-msvc-setup.exe".into(),
            download_url: "http://mock/setup.exe".into(),
            size: payload.len() as u64,
            sha256: Some(expected_hash.clone()),
        };

        let cancel = AtomicBool::new(false);
        let outcome = download_and_verify(
            &downloader,
            &installer_asset,
            None,
            "0.2.0",
            dir.path(),
            1,
            &cancel,
            |_, _| {},
        );

        match outcome {
            DownloadOutcome::Success {
                installer_path,
                expected_sha256,
                target_version,
            } => {
                assert!(installer_path.exists());
                assert_eq!(expected_sha256, expected_hash);
                assert_eq!(target_version, "0.2.0");
            }
            other => panic!("Expected Success, got {other:?}"),
        }
    }

    #[test]
    fn download_cancelled_cleans_up_staging_and_preserves_other_task_final_dest() {
        let dir = tempfile::tempdir().unwrap();
        let installer_name = "devtoys-setup.exe";
        let final_dest = dir.path().join(installer_name);
        fs::write(&final_dest, b"already-verified-installer-from-task-2").unwrap();

        let downloader = MockDownloader {
            download_data: Ok(b"task-1-data".to_vec()),
            checksum_text: Ok("".into()),
        };
        let installer_asset = UpdateAsset {
            name: installer_name.into(),
            download_url: "http://mock/setup.exe".into(),
            size: 100,
            sha256: Some("abcd".into()),
        };

        let cancel = AtomicBool::new(true);
        let outcome = download_and_verify(
            &downloader,
            &installer_asset,
            None,
            "0.2.0",
            dir.path(),
            1, // task 1
            &cancel,
            |_, _| {},
        );

        assert_eq!(outcome, DownloadOutcome::Cancelled);
        let task_1_staging = dir.path().join(format!("{installer_name}.task_1.tmp"));
        assert!(
            !task_1_staging.exists(),
            "Task 1 staging file must be cleaned up"
        );
        assert!(
            final_dest.exists(),
            "Other task's final dest must remain untouched"
        );
        assert_eq!(
            fs::read(&final_dest).unwrap(),
            b"already-verified-installer-from-task-2"
        );
    }

    #[test]
    fn download_corrupted_payload_cleans_up_staging_and_preserves_other_task_final_dest() {
        let dir = tempfile::tempdir().unwrap();
        let installer_name = "devtoys-setup.exe";
        let final_dest = dir.path().join(installer_name);
        fs::write(&final_dest, b"already-verified-installer-from-task-2").unwrap();

        let downloader = MockDownloader {
            download_data: Ok(b"task-1-corrupted-data".to_vec()),
            checksum_text: Ok("".into()),
        };
        let installer_asset = UpdateAsset {
            name: installer_name.into(),
            download_url: "http://mock/setup.exe".into(),
            size: 100,
            sha256: Some("0000000000000000000000000000000000000000000000000000000000000000".into()),
        };

        let cancel = AtomicBool::new(false);
        let outcome = download_and_verify(
            &downloader,
            &installer_asset,
            None,
            "0.2.0",
            dir.path(),
            1, // task 1
            &cancel,
            |_, _| {},
        );

        assert!(matches!(outcome, DownloadOutcome::Failed { .. }));
        let task_1_staging = dir.path().join(format!("{installer_name}.task_1.tmp"));
        assert!(
            !task_1_staging.exists(),
            "Task 1 staging file must be deleted on sha mismatch"
        );
        assert!(
            final_dest.exists(),
            "Other task's final dest must NOT be deleted by task 1 failure"
        );
        assert_eq!(
            fs::read(&final_dest).unwrap(),
            b"already-verified-installer-from-task-2"
        );
    }
}
