//! GitHub release checking and version comparison.

use serde::{Deserialize, Serialize};

use crate::version::SemVer;

pub const DEFAULT_RELEASES_URL: &str = "https://api.github.com/repos/JaminYe/devtoys-rs/releases";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub digest: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    pub html_url: String,
    pub body: Option<String>,
    #[serde(default)]
    pub assets: Vec<GitHubAsset>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateAsset {
    pub name: String,
    pub download_url: String,
    pub size: u64,
    pub sha256: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReleasePackage {
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
    pub release_notes: Option<String>,
    pub asset: UpdateAsset,
    pub checksum_asset: Option<UpdateAsset>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CheckOutcome {
    Latest { current_version: String },
    UpdateAvailable(Box<ReleasePackage>),
    NoRelease,
    Failed { reason: String, can_retry: bool },
}

pub trait HttpClient: Send + Sync {
    fn get(&self, url: &str) -> Result<(u16, String), String>;
}

pub struct UreqClient;

impl HttpClient for UreqClient {
    fn get(&self, url: &str) -> Result<(u16, String), String> {
        let mut builder = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(10))
            .timeout_read(std::time::Duration::from_secs(15))
            .user_agent(&format!("devtoys-rs/{}", crate::version::CURRENT_VERSION));

        let proxy_env = std::env::var("HTTPS_PROXY")
            .or_else(|_| std::env::var("https_proxy"))
            .or_else(|_| std::env::var("ALL_PROXY"))
            .or_else(|_| std::env::var("all_proxy"))
            .or_else(|_| std::env::var("HTTP_PROXY"))
            .or_else(|_| std::env::var("http_proxy"));

        if let Ok(proxy_url) = proxy_env {
            let proxy_url = proxy_url.trim();
            if !proxy_url.is_empty() {
                if let Ok(proxy) = ureq::Proxy::new(proxy_url) {
                    builder = builder.proxy(proxy);
                }
            }
        }

        let agent = builder.build();

        let response = agent
            .get(url)
            .set("Accept", "application/vnd.github+json")
            .call();

        match response {
            Ok(resp) => {
                let status = resp.status();
                let body = resp
                    .into_string()
                    .map_err(|e| format!("读取响应内容失败: {e}"))?;
                Ok((status, body))
            }
            Err(ureq::Error::Status(code, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                if code == 403 || code == 429 {
                    Err("GitHub API 请求过于频繁（触发速率限制），请稍后重试".into())
                } else if code == 404 {
                    Ok((404, body))
                } else {
                    Err(format!("GitHub API 返回错误状态码 {code}"))
                }
            }
            Err(ureq::Error::Transport(e)) => {
                let err_str = e.to_string();
                let friendly_msg = if err_str.contains("10054")
                    || err_str.contains("Connection reset")
                    || err_str.contains("forcibly closed")
                    || err_str.contains("tls connection init failed")
                {
                    "连接 GitHub 服务器被重置，请检查网络连接或代理设置后重试".to_string()
                } else if err_str.contains("timed out") || err_str.contains("Timeout") {
                    "连接 GitHub 服务器超时，请检查网络连接后重试".to_string()
                } else if err_str.contains("dns") || err_str.contains("resolve") {
                    "GitHub 域名解析失败，请检查 DNS 或网络连接后重试".to_string()
                } else {
                    "无法连接到 GitHub 更新服务器，请检查网络设置后重试".to_string()
                };
                Err(friendly_msg)
            }
        }
    }
}

/// Evaluates a list of GitHub releases against the given current version.
pub fn evaluate_releases(releases: &[GitHubRelease], current_version: &SemVer) -> CheckOutcome {
    // 1. Filter out draft and prerelease releases
    let mut official_releases: Vec<(&GitHubRelease, SemVer)> = Vec::new();
    for rel in releases {
        if rel.draft || rel.prerelease {
            continue;
        }
        if let Ok(ver) = SemVer::parse(&rel.tag_name) {
            // Ignore pre-releases even if tag had prerelease identifier
            if !ver.is_prerelease() {
                official_releases.push((rel, ver));
            }
        }
    }

    if official_releases.is_empty() {
        return CheckOutcome::NoRelease;
    }

    // 2. Sort by SemVer descending to find the highest official release
    official_releases.sort_by(|a, b| b.1.cmp(&a.1));

    let (latest_rel, latest_ver) = &official_releases[0];

    // 3. Prohibit downgrade: if latest <= current, we are up-to-date
    if !latest_ver.is_newer_than(current_version) {
        return CheckOutcome::Latest {
            current_version: current_version.to_string(),
        };
    }

    // 4. Look for matching Windows x64 setup executable asset
    let mut setup_asset = None;
    let mut checksum_asset = None;

    for asset in &latest_rel.assets {
        let name_lower = asset.name.to_lowercase();
        if is_windows_installer_asset(&name_lower) {
            let sha256 = asset
                .digest
                .as_deref()
                .and_then(|d| {
                    d.strip_prefix("sha256:")
                        .or(Some(d))
                        .map(|s| s.to_lowercase())
                })
                .filter(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()));

            setup_asset = Some(UpdateAsset {
                name: asset.name.clone(),
                download_url: asset.browser_download_url.clone(),
                size: asset.size,
                sha256,
            });
        } else if is_checksum_asset(&name_lower) {
            checksum_asset = Some(UpdateAsset {
                name: asset.name.clone(),
                download_url: asset.browser_download_url.clone(),
                size: asset.size,
                sha256: None,
            });
        }
    }

    match setup_asset {
        Some(asset) => CheckOutcome::UpdateAvailable(Box::new(ReleasePackage {
            current_version: current_version.to_string(),
            latest_version: latest_ver.to_string(),
            release_url: latest_rel.html_url.clone(),
            release_notes: latest_rel.body.clone(),
            asset,
            checksum_asset,
        })),
        None => CheckOutcome::Failed {
            reason: format!(
                "最新版本 v{} 未包含适用于 Windows x64 的安装包资产",
                latest_ver
            ),
            can_retry: true,
        },
    }
}

pub fn check_updates<C: HttpClient>(
    client: &C,
    url: &str,
    current_version: &SemVer,
) -> CheckOutcome {
    match client.get(url) {
        Ok((200, body)) => match serde_json::from_str::<Vec<GitHubRelease>>(&body) {
            Ok(releases) => evaluate_releases(&releases, current_version),
            Err(e) => CheckOutcome::Failed {
                reason: format!("解析 GitHub 发布数据失败：{e}"),
                can_retry: true,
            },
        },
        Ok((404, _)) => CheckOutcome::NoRelease,
        Ok((code, _)) => CheckOutcome::Failed {
            reason: format!("GitHub API 响应异常（状态码 {code}）"),
            can_retry: true,
        },
        Err(e) => CheckOutcome::Failed {
            reason: e,
            can_retry: true,
        },
    }
}

fn is_windows_installer_asset(name_lower: &str) -> bool {
    name_lower.ends_with(".exe")
        && (name_lower.contains("setup") || name_lower.contains("installer"))
        && (name_lower.contains("x86_64") || name_lower.contains("x64"))
}

fn is_checksum_asset(name_lower: &str) -> bool {
    name_lower == "checksums.txt" || name_lower.ends_with("checksums.txt")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockClient {
        result: Result<(u16, String), String>,
    }

    impl HttpClient for MockClient {
        fn get(&self, _url: &str) -> Result<(u16, String), String> {
            self.result.clone()
        }
    }

    fn sample_release(
        tag: &str,
        draft: bool,
        prerelease: bool,
        asset_names: &[&str],
    ) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag.to_string(),
            name: Some(format!("Release {tag}")),
            draft,
            prerelease,
            html_url: format!("https://github.com/JaminYe/devtoys-rs/releases/tag/{tag}"),
            body: Some("Release notes".to_string()),
            assets: asset_names
                .iter()
                .map(|name| GitHubAsset {
                    name: (*name).to_string(),
                    browser_download_url: format!("https://github.com/download/{name}"),
                    size: 1024,
                    content_type: Some("application/octet-stream".into()),
                    digest: None,
                })
                .collect(),
        }
    }

    #[test]
    fn check_newer_version_available() {
        let releases = vec![sample_release(
            "v0.2.0",
            false,
            false,
            &["devtoys-x86_64-pc-windows-msvc-setup.exe", "checksums.txt"],
        )];
        let json = serde_json::to_string(&releases).unwrap();
        let client = MockClient {
            result: Ok((200, json)),
        };

        let current = SemVer::parse("0.1.0").unwrap();
        let outcome = check_updates(&client, "http://mock", &current);
        match outcome {
            CheckOutcome::UpdateAvailable(pkg) => {
                assert_eq!(pkg.latest_version, "0.2.0");
                assert_eq!(pkg.asset.name, "devtoys-x86_64-pc-windows-msvc-setup.exe");
                assert_eq!(pkg.checksum_asset.unwrap().name, "checksums.txt");
            }
            other => panic!("Expected UpdateAvailable, got {other:?}"),
        }
    }

    #[test]
    fn check_equal_version_is_latest() {
        let releases = vec![sample_release(
            "v0.1.0",
            false,
            false,
            &["devtoys-x86_64-pc-windows-msvc-setup.exe"],
        )];
        let json = serde_json::to_string(&releases).unwrap();
        let client = MockClient {
            result: Ok((200, json)),
        };

        let current = SemVer::parse("0.1.0").unwrap();
        let outcome = check_updates(&client, "http://mock", &current);
        assert_eq!(
            outcome,
            CheckOutcome::Latest {
                current_version: "0.1.0".into()
            }
        );
    }

    #[test]
    fn check_lower_version_prohibits_downgrade() {
        let releases = vec![sample_release(
            "v0.0.9",
            false,
            false,
            &["devtoys-x86_64-pc-windows-msvc-setup.exe"],
        )];
        let json = serde_json::to_string(&releases).unwrap();
        let client = MockClient {
            result: Ok((200, json)),
        };

        let current = SemVer::parse("0.1.0").unwrap();
        let outcome = check_updates(&client, "http://mock", &current);
        assert_eq!(
            outcome,
            CheckOutcome::Latest {
                current_version: "0.1.0".into()
            }
        );
    }

    #[test]
    fn check_filters_draft_and_prerelease() {
        let releases = vec![
            sample_release(
                "v0.3.0",
                true,
                false,
                &["devtoys-x86_64-pc-windows-msvc-setup.exe"],
            ),
            sample_release(
                "v0.2.5",
                false,
                true,
                &["devtoys-x86_64-pc-windows-msvc-setup.exe"],
            ),
            sample_release(
                "v0.1.0",
                false,
                false,
                &["devtoys-x86_64-pc-windows-msvc-setup.exe"],
            ),
        ];
        let json = serde_json::to_string(&releases).unwrap();
        let client = MockClient {
            result: Ok((200, json)),
        };

        let current = SemVer::parse("0.1.0").unwrap();
        let outcome = check_updates(&client, "http://mock", &current);
        assert_eq!(
            outcome,
            CheckOutcome::Latest {
                current_version: "0.1.0".into()
            }
        );
    }

    #[test]
    fn check_missing_windows_installer_asset() {
        let releases = vec![sample_release(
            "v0.2.0",
            false,
            false,
            &["devtoys-mac.tar.gz"],
        )];
        let json = serde_json::to_string(&releases).unwrap();
        let client = MockClient {
            result: Ok((200, json)),
        };

        let current = SemVer::parse("0.1.0").unwrap();
        let outcome = check_updates(&client, "http://mock", &current);
        match outcome {
            CheckOutcome::Failed { reason, can_retry } => {
                assert!(can_retry);
                assert!(reason.contains("未包含适用于 Windows x64 的安装包资产"));
            }
            other => panic!("Expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn check_empty_releases_returns_no_release() {
        let client = MockClient {
            result: Ok((200, "[]".into())),
        };
        let current = SemVer::parse("0.1.0").unwrap();
        let outcome = check_updates(&client, "http://mock", &current);
        assert_eq!(outcome, CheckOutcome::NoRelease);
    }

    #[test]
    fn check_404_returns_no_release() {
        let client = MockClient {
            result: Ok((404, "Not Found".into())),
        };
        let current = SemVer::parse("0.1.0").unwrap();
        let outcome = check_updates(&client, "http://mock", &current);
        assert_eq!(outcome, CheckOutcome::NoRelease);
    }

    #[test]
    fn check_rate_limit_error() {
        let client = MockClient {
            result: Err("GitHub API 请求过于频繁（触发速率限制），请稍后重试".into()),
        };
        let current = SemVer::parse("0.1.0").unwrap();
        let outcome = check_updates(&client, "http://mock", &current);
        match outcome {
            CheckOutcome::Failed { reason, can_retry } => {
                assert!(can_retry);
                assert!(reason.contains("速率限制"));
            }
            other => panic!("Expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn check_network_error() {
        let client = MockClient {
            result: Err("网络连接超时或失败，请检查网络设置后重试：timeout".into()),
        };
        let current = SemVer::parse("0.1.0").unwrap();
        let outcome = check_updates(&client, "http://mock", &current);
        match outcome {
            CheckOutcome::Failed { reason, can_retry } => {
                assert!(can_retry);
                assert!(reason.contains("网络连接超时"));
            }
            other => panic!("Expected Failed, got {other:?}"),
        }
    }
}
