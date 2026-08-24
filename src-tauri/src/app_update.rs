use serde::Serialize;
use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const UPDATE_BASE: &str = "http://172.20.2.169:7101/appStore/Software/PC/developer/jdd-crypto";
const UPDATE_LOG_FILE: &str = "更新日志.txt";
const PROGRESS_EVENT: &str = "app://update-download-progress";
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Clone, Serialize)]
pub struct UpdateCheckResult {
    pub available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub notes: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

fn parse_version(version: &str) -> Option<(u32, u32, u32)> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn version_lt(current: &str, latest: &str) -> bool {
    match (parse_version(current), parse_version(latest)) {
        (Some(a), Some(b)) => a < b,
        _ => false,
    }
}

fn http_client(timeout: Duration) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| error.to_string())
}

fn update_log_url() -> String {
    format!("{}/{}", UPDATE_BASE, urlencoding::encode(UPDATE_LOG_FILE))
}

fn installer_url(version: &str) -> String {
    let file_name = installer_file_name(version);
    format!("{}/{}", UPDATE_BASE, urlencoding::encode(&file_name))
}

fn installer_file_name(version: &str) -> String {
    format!("多多解密_{version}_x64-setup.exe")
}

fn fetch_changelog() -> Result<String, String> {
    let client = http_client(FETCH_TIMEOUT)?;
    let response = client
        .get(update_log_url())
        .send()
        .map_err(|_| "无法连接更新服务器".to_string())?;
    if !response.status().is_success() {
        return Err("无法连接更新服务器".to_string());
    }
    response
        .text()
        .map_err(|_| "无法读取更新日志".to_string())
}

fn parse_latest_release(text: &str) -> Option<(String, String)> {
    let mut best: Option<(String, String)> = None;
    let mut index = 0;

    while index < text.len() {
        let Some(open_rel) = text[index..].find('【') else {
            break;
        };
        let open = index + open_rel;
        let Some(close_rel) = text[open..].find('】') else {
            break;
        };
        let close = open + close_rel;
        let version = text.get(open + '【'.len_utf8()..close)?.trim();
        if parse_version(version).is_none() {
            index = close + '】'.len_utf8();
            continue;
        }

        let notes_start = close + '】'.len_utf8();
        let notes_end = text[notes_start..]
            .find('【')
            .map(|offset| notes_start + offset)
            .unwrap_or(text.len());
        let notes = text[notes_start..notes_end].trim().to_string();

        let replace = best
            .as_ref()
            .map(|(current, _)| version_lt(current, version))
            .unwrap_or(true);
        if replace {
            best = Some((version.to_string(), notes));
        }

        index = notes_end;
    }

    best
}

pub fn check_update(app: &AppHandle, manual: bool) -> Result<UpdateCheckResult, String> {
    let current_version = app.package_info().version.to_string();

    let changelog = match fetch_changelog() {
        Ok(text) => text,
        Err(error) => {
            if manual {
                return Err(error);
            }
            return Ok(UpdateCheckResult {
                available: false,
                current_version,
                latest_version: None,
                notes: None,
            });
        }
    };

    let Some((latest_version, notes)) = parse_latest_release(&changelog) else {
        if manual {
            return Err("更新日志格式无效".to_string());
        }
        return Ok(UpdateCheckResult {
            available: false,
            current_version,
            latest_version: None,
            notes: None,
        });
    };

    Ok(UpdateCheckResult {
        available: version_lt(&current_version, &latest_version),
        current_version,
        latest_version: Some(latest_version),
        notes: Some(notes),
    })
}

fn emit_progress(app: &AppHandle, downloaded: u64, total: Option<u64>) {
    let _ = app.emit(
        PROGRESS_EVENT,
        DownloadProgress { downloaded, total },
    );
}

pub fn download_installer(app: &AppHandle, version: &str) -> Result<String, String> {
    if parse_version(version).is_none() {
        return Err("版本号无效".to_string());
    }

    let client = http_client(DOWNLOAD_TIMEOUT)?;
    let response = client
        .get(installer_url(version))
        .send()
        .map_err(|_| "下载失败，无法连接更新服务器".to_string())?;
    if !response.status().is_success() {
        return Err("下载失败，安装包不存在".to_string());
    }

    let total = response.content_length();
    let temp_dir = app
        .path()
        .temp_dir()
        .map_err(|error| error.to_string())?;
    let dest = temp_dir.join(installer_file_name(version));
    let mut file = File::create(&dest).map_err(|error| error.to_string())?;

    let mut source = response;
    let mut buffer = [0_u8; 8192];
    let mut downloaded = 0_u64;

    emit_progress(app, downloaded, total);

    loop {
        let read = source
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|error| error.to_string())?;
        downloaded += read as u64;
        emit_progress(app, downloaded, total);
    }

    Ok(dest.to_string_lossy().into_owned())
}

pub fn install_update(app: &AppHandle, path: &str) -> Result<(), String> {
    std::process::Command::new(path)
        .spawn()
        .map_err(|error| error.to_string())?;
    app.exit(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_latest_release;

    #[test]
    fn parses_latest_semver_and_notes() {
        let text = r#"## 多多解密升级日志 ##

【0.3.3】
1. old item

【0.4.3】
1. new ui
2. update check
"#;
        let (version, notes) = parse_latest_release(text).expect("parsed");
        assert_eq!(version, "0.4.3");
        assert!(notes.contains("new ui"));
        assert!(notes.contains("update check"));
    }
}
