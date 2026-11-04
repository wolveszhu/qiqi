//! 远程更新检测（轻量自建）：拉取 GitHub 上的 `latest.json` 与当前版本比较。
//!
//! 设计边界（计划选型）：不引入 `tauri-plugin-updater`——只"提醒 + 跳转下载"，
//! 由用户手动运行安装包，规避代码签名证书、updater 密钥对与发布服务器配置。
//! 版本清单托管于仓库根，多源回退读取；`downloadUrl` 由发布者手工维护、语义明确。
//!
//! 可见性前提：清单源均为**匿名**读取——仓库必须为 Public（私有仓库的 raw/CDN/API
//! 一律 404，GitHub 故意不区分"私有"与"不存在"）；私密分发需另行换托管（Gitee 等）。

use std::time::Duration;

use serde::Deserialize;
use tauri::{AppHandle, Emitter};

use crate::error::AppError;

/// `latest.json` 托管地址（按序回退；发布新版本时同步维护仓库根文件与 Releases）。
///
/// ① raw：即时性最好（jsDelivr 对分支引用有最长 12h 缓存，发版日可能滞后）；
/// ②③ jsDelivr CDN：国内网络对 raw.githubusercontent 阻断/污染时的兜底，可达性好。
const LATEST_JSON_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/wolveszhu/qiqi/main/latest.json",
    "https://cdn.jsdelivr.net/gh/wolveszhu/qiqi@main/latest.json",
    "https://fastly.jsdelivr.net/gh/wolveszhu/qiqi@main/latest.json",
];

/// `latest.json` 契约（`downloadUrl` 由发布者手工维护；未知字段忽略）。
#[derive(Debug, Deserialize)]
struct LatestManifest {
    /// 纯语义化版本号，如 `0.2.0`（**不含 `v` 前缀**，否则无法解析为 semver）。
    version: String,
    /// 更新说明（多行文本，可选）。
    #[serde(default)]
    notes: Option<String>,
    /// 安装包下载地址（GitHub Releases 的稳定 latest 资产或具体版本资产均可）。
    #[serde(default, rename = "downloadUrl")]
    download_url: Option<String>,
}

/// 前端可读的更新信息（`camelCase` 序列化，与 `src/api/update.ts` 契约对应）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// 是否存在可更新版本。
    pub available: bool,
    /// 当前安装版本。
    pub current: String,
    /// 远端最新版本。
    pub latest: String,
    /// 更新说明（仅 available 时有意义）。
    pub notes: Option<String>,
    /// 安装包下载地址（仅 available 时有意义）。
    pub download_url: Option<String>,
}

/// 当前应用版本（编译期注入；发版须与 `tauri.conf.json` 的 `version` 同步，二者当前均为 0.1.0）。
fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 拉取单个源并解析（内部错误用 `String` 承载，便于多源失败信息合并展示）。
async fn fetch_one(client: &reqwest::Client, url: &str) -> Result<LatestManifest, String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        // 404 单独提示：最常见原因是仓库为私有（匿名访问一律 404）或清单未推送
        return Err(
            "HTTP 404——清单不存在：若仓库为私有则匿名无法读取（需公开仓库或更换托管），或 latest.json 未推送到 main".to_string(),
        );
    }
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = resp.text().await.map_err(|e| format!("读取失败: {e}"))?;
    serde_json::from_str::<LatestManifest>(&text).map_err(|e| format!("解析失败: {e}"))
}

/// 拉取并解析 `latest.json`：按 `LATEST_JSON_URLS` 顺序尝试，首个成功即返回；
/// 全部失败时合并各源错误向上传播（手动检查时前端可完整展示，便于定位网络/可见性问题）。
async fn fetch_latest() -> Result<LatestManifest, AppError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent(concat!("qiqi/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| AppError::Llm(format!("构建更新检查客户端失败: {e}")))?;
    let mut failures: Vec<String> = Vec::new();
    for url in LATEST_JSON_URLS {
        match fetch_one(&client, url).await {
            Ok(m) => return Ok(m),
            Err(e) => failures.push(format!("{url} → {e}")),
        }
    }
    Err(AppError::Llm(format!(
        "所有更新源均不可达（{}）",
        failures.join("；")
    )))
}

/// 比较当前版本与远端清单，产出 `UpdateInfo`。
///
/// 版本不可解析（如 latest 带 `v` 前缀、当前版本异常）时**回落为"无更新"**——
/// 宁可漏报也不误报；错误由调用方（手动检查）或后台循环（静默吞掉）处理。
fn detect(current: &str, latest: &LatestManifest) -> UpdateInfo {
    let available = semver::Version::parse(current).ok().is_some_and(|c| {
        semver::Version::parse(&latest.version)
            .map(|l| l > c)
            .unwrap_or(false)
    });
    UpdateInfo {
        available,
        current: current.to_string(),
        latest: latest.version.clone(),
        notes: latest.notes.clone(),
        download_url: latest.download_url.clone(),
    }
}

/// 命令：检测更新（手动「检查更新」按钮与后台检查共用）。
///
/// 拉取 `latest.json` 并比较版本；网络/解析失败向上传播为 `AppError`
/// （手动检查时前端给出友好提示；后台检查在循环内吞掉，不打扰用户）。
#[tauri::command]
pub async fn check_update() -> Result<UpdateInfo, AppError> {
    let latest = fetch_latest().await?;
    Ok(detect(&current_version(), &latest))
}

/// 后台周期检查：发现新版本时 emit `app:update-available`（仅新版本出现时，避免刷屏）。
///
/// 单任务单点循环（参考 C2 探针线程语义）；`release` 为 `panic="abort"`，
/// 循环内一切错误必须吞掉，绝不让检查任务 panic 拖垮进程。
pub fn spawn_update_checker(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // 启动后延迟首查，避免阻塞启动与首屏渲染
        tokio::time::sleep(Duration::from_secs(10)).await;
        loop {
            if let Ok(latest) = fetch_latest().await {
                let info = detect(&current_version(), &latest);
                if info.available {
                    // 事件发送失败（窗口全关等）不应终止检查循环
                    let _ = app.emit("app:update-available", &info);
                }
            }
            // 失败（离线/404/解析）静默跳过，不写日志噪音
            tokio::time::sleep(Duration::from_secs(6 * 60 * 60)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_marks_newer_as_available() {
        let m = LatestManifest {
            version: "0.2.0".into(),
            notes: Some("新增远程更新".into()),
            download_url: Some("https://github.com/wolveszhu/qiqi/releases/download/v0.2.0/qiqi.msi".into()),
        };
        let info = detect("0.1.0", &m);
        assert!(info.available, "更高版本应标记可更新");
        assert_eq!(info.latest, "0.2.0");
        assert_eq!(
            info.download_url.as_deref(),
            Some("https://github.com/wolveszhu/qiqi/releases/download/v0.2.0/qiqi.msi")
        );
    }

    #[test]
    fn detect_same_or_older_is_not_available() {
        assert!(
            !detect(
                "0.1.0",
                &LatestManifest { version: "0.1.0".into(), notes: None, download_url: None }
            )
            .available,
            "同版本不更新"
        );
        assert!(
            !detect(
                "0.1.0",
                &LatestManifest { version: "0.0.9".into(), notes: None, download_url: None }
            )
            .available,
            "更旧版本不更新"
        );
    }

    #[test]
    fn detect_tolerates_unparseable_latest_version() {
        // latest 带 `v` 前缀等非法 semver → 回落"无更新"，不误报也不 panic
        assert!(
            !detect(
                "0.1.0",
                &LatestManifest { version: "v0.2.0".into(), notes: None, download_url: None }
            )
            .available
        );
    }

    #[test]
    fn parse_manifest_handles_optional_fields() {
        let m: LatestManifest = serde_json::from_str(
            r#"{"version":"0.3.0","notes":"hi","downloadUrl":"https://x/a.msi"}"#,
        )
        .expect("应解析带可选字段");
        assert_eq!(m.version, "0.3.0");
        assert_eq!(m.download_url.as_deref(), Some("https://x/a.msi"));

        // 缺省可选字段（notes/downloadUrl）也能解析，不报错
        let m2: LatestManifest = serde_json::from_str(r#"{"version":"0.3.1"}"#).expect("应解析最小字段");
        assert_eq!(m2.version, "0.3.1");
        assert!(m2.notes.is_none());
        assert!(m2.download_url.is_none());
    }
}
