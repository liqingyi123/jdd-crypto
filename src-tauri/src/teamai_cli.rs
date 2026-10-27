//! Detect and resolve the teamai CLI (Windows PATH / Volta shims).

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use tauri::AppHandle;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Outer `None` = cache empty; inner = last resolve result (may be not-found).
static RESOLVED_BIN: Mutex<Option<Option<PathBuf>>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamaiCliStatus {
    pub installed: bool,
    pub bin: Option<String>,
    pub log: String,
}

/// Absolute or PATH-relative path to the teamai executable, if found.
pub fn resolve_teamai_bin() -> Option<PathBuf> {
    let mut guard = RESOLVED_BIN.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(cached) = guard.as_ref() {
        return cached.clone();
    }
    let bin = detect_uncached().1;
    *guard = Some(bin.clone());
    bin
}

fn clear_resolved_bin() {
    let mut guard = RESOLVED_BIN.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
}

fn store_resolved_bin(bin: Option<PathBuf>) {
    let mut guard = RESOLVED_BIN.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(bin);
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamaiRunResult {
    pub ok: bool,
    pub log: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamaiUpgradeResult {
    pub ok: bool,
    pub up_to_date: bool,
    pub version: Option<String>,
    pub log: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamaiDeployResult {
    pub ok: bool,
    pub version: Option<String>,
    pub log: String,
}

/// 读取本地 `~/.teamai/config.yaml` 的 `updatePolicy`（`auto` / `prompt` / `skip`）。
/// 文件不存在或未设置时返回 `Ok(None)`。
pub fn get_update_policy() -> Result<Option<String>, String> {
    let path = teamai_config_path()?;
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("读取 config.yaml 失败: {e}"))?;
    Ok(parse_update_policy(&text))
}

/// 写入本地 `updatePolicy`：`auto` 启用自动升级，`skip` 禁用。
pub fn set_update_policy(policy: &str) -> TeamaiRunResult {
    let mut log = String::from("# teamai 自动升级策略\n");
    let policy = policy.trim();
    if policy != "auto" && policy != "skip" {
        log.push_str(&format!("error: 非法策略 `{policy}`，仅支持 auto / skip\n"));
        return TeamaiRunResult { ok: false, log };
    }

    let path = match teamai_config_path() {
        Ok(p) => p,
        Err(e) => {
            log.push_str(&format!("error: {e}\n"));
            return TeamaiRunResult { ok: false, log };
        }
    };
    if !path.is_file() {
        log.push_str(&format!(
            "error: 未找到 {}\n请先完成 Teamai 部署\n",
            path.display()
        ));
        return TeamaiRunResult { ok: false, log };
    }

    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            log.push_str(&format!("error: 读取 config.yaml 失败: {e}\n"));
            return TeamaiRunResult { ok: false, log };
        }
    };

    let next = upsert_update_policy(&text, policy);
    log.push_str(&format!("$ 设置 updatePolicy: {policy}\n"));
    log.push_str(&format!("config: {}\n", path.display()));

    match fs::write(&path, next) {
        Ok(()) => {
            let label = if policy == "skip" {
                "已禁用自动升级"
            } else {
                "已启用自动升级"
            };
            log.push_str(&format!("ok: {label}\n"));
            TeamaiRunResult { ok: true, log }
        }
        Err(e) => {
            log.push_str(&format!("error: 写入 config.yaml 失败: {e}\n"));
            TeamaiRunResult { ok: false, log }
        }
    }
}

fn teamai_config_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or_else(|| "无法解析用户主目录".to_string())?;
    Ok(PathBuf::from(home).join(".teamai").join("config.yaml"))
}

fn parse_update_policy(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("updatePolicy:") else {
            continue;
        };
        let value = rest
            .trim()
            .trim_matches(|c| c == '"' || c == '\'')
            .trim();
        if matches!(value, "auto" | "prompt" | "skip") {
            return Some(value.to_string());
        }
    }
    None
}

fn upsert_update_policy(text: &str, policy: &str) -> String {
    let mut out = String::with_capacity(text.len() + 32);
    let mut replaced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("updatePolicy:") {
            out.push_str("updatePolicy: ");
            out.push_str(policy);
            out.push('\n');
            replaced = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !replaced {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("updatePolicy: ");
        out.push_str(policy);
        out.push('\n');
    }
    out
}

/// Deploy / redeploy: sync env, install CLI, init --scope user, pull.
pub fn cli_deploy(app: &AppHandle, repo_url: String, access_token: String) -> TeamaiDeployResult {
    let mut log = String::from("# teamai 部署\n");
    let repo_url = repo_url.trim().to_string();
    let access_token = access_token.trim().to_string();

    if repo_url.is_empty() {
        log.push_str("error: 仓库地址不能为空\n");
        return TeamaiDeployResult {
            ok: false,
            version: None,
            log,
        };
    }

    // 1) Persist pref + GITLAB_URL / GITLAB_TOKEN
    log.push_str("$ 同步 GITLAB_URL / GITLAB_TOKEN 并保存偏好\n");
    let pref = crate::teamai_pref::TeamaiPref {
        repo_url: repo_url.clone(),
        access_token: access_token.clone(),
    };
    match crate::teamai_pref::save(app, pref) {
        Ok(_) => log.push_str("ok: 环境变量与偏好已保存\n"),
        Err(e) => {
            log.push_str(&format!("error: {e}\n"));
            return TeamaiDeployResult {
                ok: false,
                version: None,
                log,
            };
        }
    }

    // 2) Install CLI（有 Volta 则 volta install，否则 npm -g）
    match install_teamai_cli() {
        Ok((cmd, out)) => {
            log.push_str(&format!("{cmd}\n"));
            log.push_str(&out.combined);
            if !out.combined.ends_with('\n') {
                log.push('\n');
            }
            if !out.ok {
                return TeamaiDeployResult {
                    ok: false,
                    version: None,
                    log,
                };
            }
        }
        Err(e) => {
            log.push_str(&format!("error: {e}\n"));
            return TeamaiDeployResult {
                ok: false,
                version: None,
                log,
            };
        }
    }
    clear_resolved_bin();

    // 3) init
    log.push_str(&format!("$ teamai init {repo_url} --scope user\n"));
    match run_teamai(&["init", &repo_url, "--scope", "user"]) {
        Ok(out) => {
            log.push_str(&out.combined);
            if !out.combined.ends_with('\n') {
                log.push('\n');
            }
            if !out.ok {
                return TeamaiDeployResult {
                    ok: false,
                    version: cli_version().ok(),
                    log,
                };
            }
        }
        Err(e) => {
            log.push_str(&format!("error: {e}\n"));
            return TeamaiDeployResult {
                ok: false,
                version: cli_version().ok(),
                log,
            };
        }
    }

    // 4) pull
    log.push_str("$ teamai pull\n");
    match run_teamai(&["pull"]) {
        Ok(out) => {
            log.push_str(&out.combined);
            if !out.combined.ends_with('\n') {
                log.push('\n');
            }
            if !out.ok {
                return TeamaiDeployResult {
                    ok: false,
                    version: cli_version().ok(),
                    log,
                };
            }
        }
        Err(e) => {
            log.push_str(&format!("error: {e}\n"));
            return TeamaiDeployResult {
                ok: false,
                version: cli_version().ok(),
                log,
            };
        }
    }

    append_status(&mut log);
    let version = cli_version().ok();
    log.push_str("ok: 部署完成\n");
    TeamaiDeployResult {
        ok: true,
        version,
        log,
    }
}

/// 有 Volta 时用 `volta install`；否则 `npm install -g`。不因 Volta 失败再回退 npm。
fn install_teamai_cli() -> Result<(&'static str, ProcessOutput), String> {
    if let Some(volta) = find_volta_exe() {
        let out = run_bin_with_args(&volta, &["install", "teamai-cli@latest"])?;
        return Ok(("$ volta install teamai-cli@latest", out));
    }
    let out = run_npm_install_latest()?;
    Ok(("$ npm install -g teamai-cli@latest", out))
}

fn run_npm_install_latest() -> Result<ProcessOutput, String> {
    #[cfg(windows)]
    {
        let npm = resolve_npm_bin().unwrap_or_else(|| PathBuf::from("npm.cmd"));
        return run_bin_with_args(&npm, &["install", "-g", "teamai-cli@latest"]);
    }
    #[cfg(not(windows))]
    {
        run_bin_with_args(Path::new("npm"), &["install", "-g", "teamai-cli@latest"])
    }
}

/// Read installed CLI version via `teamai -V`.
pub fn cli_version() -> Result<String, String> {
    let out = run_teamai(&["-V"])?;
    let ver = out
        .combined
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .trim_start_matches('v')
        .trim()
        .to_string();
    if ver.is_empty() {
        return Err("无法解析 teamai 版本输出".to_string());
    }
    if !out.ok {
        return Err(format!("teamai -V 失败\n{}", out.combined));
    }
    Ok(ver)
}

/// `teamai pull` — pull latest team config.
pub fn cli_pull() -> TeamaiRunResult {
    match run_teamai(&["pull"]) {
        Ok(out) => {
            let mut log = format!("$ teamai pull\n{}", out.combined);
            if !log.ends_with('\n') {
                log.push('\n');
            }
            if out.ok {
                append_status(&mut log);
            }
            TeamaiRunResult { ok: out.ok, log }
        }
        Err(e) => TeamaiRunResult {
            ok: false,
            log: format!("$ teamai pull\nerror: {e}\n"),
        },
    }
}

const PUSH_PATHS: &[&str] = &["rules", "skills", "mcp"];

/// Whether `rules` / `skills` / `mcp` have uncommitted changes in the team repo.
pub fn has_push_changes() -> Result<bool, String> {
    let repo = resolve_team_repo_dir()?;
    let out = run_git(
        &repo,
        &["status", "--porcelain", "--", "rules", "skills", "mcp"],
    )?;
    if !out.ok {
        return Err(format!("git status 失败\n{}", out.combined.trim_end()));
    }
    Ok(!out.combined.trim().is_empty())
}

/// Push local team-repo changes via git: new branch, add rules/skills/mcp, commit, push.
pub fn cli_push(branch: String, message: String) -> TeamaiRunResult {
    let branch = branch.trim().to_string();
    let message = message.trim().to_string();
    let mut log = String::from("# teamai 上传配置（本地仓 Git）\n");

    if branch.is_empty() {
        log.push_str("error: 分支名不能为空\n");
        return TeamaiRunResult { ok: false, log };
    }
    if branch.chars().any(|c| c.is_whitespace()) {
        log.push_str("error: 分支名不能包含空白字符\n");
        return TeamaiRunResult { ok: false, log };
    }
    if message.is_empty() {
        log.push_str("error: 推送描述不能为空\n");
        return TeamaiRunResult { ok: false, log };
    }

    let repo = match resolve_team_repo_dir() {
        Ok(p) => {
            log.push_str(&format!("repo: {}\n", p.display()));
            p
        }
        Err(e) => {
            log.push_str(&format!("error: {e}\n"));
            return TeamaiRunResult { ok: false, log };
        }
    };

    let base = match detect_default_branch(&repo) {
        Ok(b) => {
            log.push_str(&format!("base: {b}\n"));
            b
        }
        Err(e) => {
            log.push_str(&format!("error: {e}\n"));
            return TeamaiRunResult { ok: false, log };
        }
    };

    if !git_step(&repo, &mut log, &["checkout", &base]) {
        return TeamaiRunResult { ok: false, log };
    }
    if !git_step(&repo, &mut log, &["pull", "--ff-only"]) {
        return TeamaiRunResult { ok: false, log };
    }
    if !git_step(&repo, &mut log, &["checkout", "-b", &branch]) {
        return TeamaiRunResult { ok: false, log };
    }

    let mut add_args = vec!["add", "--"];
    add_args.extend_from_slice(PUSH_PATHS);
    if !git_step(&repo, &mut log, &add_args) {
        return TeamaiRunResult { ok: false, log };
    }

    match run_git(&repo, &["diff", "--cached", "--quiet"]) {
        Ok(out) => {
            // exit 0 = no staged changes; exit 1 = has staged changes
            if out.ok {
                log.push_str("error: 暂存区无 rules/skills/mcp 变更，已取消提交\n");
                return TeamaiRunResult { ok: false, log };
            }
        }
        Err(e) => {
            log.push_str(&format!("error: git diff --cached 失败: {e}\n"));
            return TeamaiRunResult { ok: false, log };
        }
    }

    if !git_step(&repo, &mut log, &["commit", "-m", &message]) {
        return TeamaiRunResult { ok: false, log };
    }
    if !git_step(&repo, &mut log, &["push", "-u", "origin", &branch]) {
        return TeamaiRunResult { ok: false, log };
    }

    log.push_str("ok: 分支已推送，请到 GitLab 创建并合并 MR\n");
    TeamaiRunResult { ok: true, log }
}

fn git_step(repo: &Path, log: &mut String, args: &[&str]) -> bool {
    log.push_str(&format!("$ git {}\n", args.join(" ")));
    match run_git(repo, args) {
        Ok(out) => {
            if !out.combined.is_empty() {
                log.push_str(&out.combined);
                if !out.combined.ends_with('\n') {
                    log.push('\n');
                }
            }
            if !out.ok {
                log.push_str("error: 命令失败\n");
            }
            out.ok
        }
        Err(e) => {
            log.push_str(&format!("error: {e}\n"));
            false
        }
    }
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<ProcessOutput, String> {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(p) = effective_path() {
        cmd.env("PATH", p);
    }
    apply_no_window(&mut cmd);
    capture_output(cmd)
}

fn detect_default_branch(repo: &Path) -> Result<String, String> {
    if let Ok(out) = run_git(
        repo,
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    ) {
        if out.ok {
            let name = out.combined.lines().next().unwrap_or("").trim();
            let branch = name
                .strip_prefix("origin/")
                .unwrap_or(name)
                .trim()
                .to_string();
            if !branch.is_empty() {
                return Ok(branch);
            }
        }
    }
    for candidate in ["main", "master"] {
        let local_ref = format!("refs/heads/{candidate}");
        if let Ok(out) = run_git(repo, &["rev-parse", "--verify", &local_ref]) {
            if out.ok {
                return Ok(candidate.to_string());
            }
        }
        let remote_ref = format!("refs/remotes/origin/{candidate}");
        if let Ok(out) = run_git(repo, &["rev-parse", "--verify", &remote_ref]) {
            if out.ok {
                return Ok(candidate.to_string());
            }
        }
    }
    Err("无法确定主干分支（origin/HEAD / main / master）".to_string())
}

/// Upgrade CLI: `teamai update` first, volta/npm install as fallback; pull after real upgrade.
pub fn cli_upgrade() -> TeamaiUpgradeResult {
    let mut log = String::from("# teamai 升级\n");

    let update = match run_teamai(&["update"]) {
        Ok(out) => {
            log.push_str("$ teamai update\n");
            log.push_str(&out.combined);
            if !out.combined.ends_with('\n') {
                log.push('\n');
            }
            Some(out)
        }
        Err(e) => {
            log.push_str(&format!("$ teamai update\nerror: {e}\n"));
            None
        }
    };

    if let Some(out) = &update {
        if out.ok && is_already_up_to_date(&out.combined) {
            let version = cli_version().ok();
            return TeamaiUpgradeResult {
                ok: true,
                up_to_date: true,
                version,
                log,
            };
        }
        if out.ok {
            return finish_upgrade_with_pull(log);
        }
        // Non-zero from teamai update: still try volta/npm fallback below.
        log.push_str("teamai update 未成功，尝试 Volta/npm 保底升级\n");
    } else {
        log.push_str("teamai update 无法启动，尝试 Volta/npm 保底升级\n");
    }

    match install_teamai_cli() {
        Ok((cmd, out)) => {
            log.push_str(&format!("{cmd}\n"));
            log.push_str(&out.combined);
            if !out.combined.ends_with('\n') {
                log.push('\n');
            }
            if !out.ok {
                return TeamaiUpgradeResult {
                    ok: false,
                    up_to_date: false,
                    version: cli_version().ok(),
                    log,
                };
            }
            clear_resolved_bin();
            finish_upgrade_with_pull(log)
        }
        Err(e) => {
            log.push_str(&format!("error: {e}\n"));
            TeamaiUpgradeResult {
                ok: false,
                up_to_date: false,
                version: cli_version().ok(),
                log,
            }
        }
    }
}

fn finish_upgrade_with_pull(mut log: String) -> TeamaiUpgradeResult {
    log.push_str("$ teamai pull\n");
    match run_teamai(&["pull"]) {
        Ok(out) => {
            log.push_str(&out.combined);
            if !out.combined.ends_with('\n') {
                log.push('\n');
            }
            let version = cli_version().ok();
            if out.ok {
                append_status(&mut log);
            }
            TeamaiUpgradeResult {
                ok: out.ok,
                up_to_date: false,
                version,
                log,
            }
        }
        Err(e) => {
            log.push_str(&format!("error: {e}\n"));
            TeamaiUpgradeResult {
                ok: false,
                up_to_date: false,
                version: cli_version().ok(),
                log,
            }
        }
    }
}

/// Append `teamai status` output; failures only go into the log.
fn append_status(log: &mut String) {
    log.push_str("$ teamai status\n");
    match run_teamai(&["status"]) {
        Ok(out) => {
            log.push_str(&out.combined);
            if !out.combined.ends_with('\n') {
                log.push('\n');
            }
        }
        Err(e) => {
            log.push_str(&format!("error: {e}\n"));
        }
    }
}

/// Open Teamai local repo dir in the system file manager.
pub fn open_local_dir(app: &AppHandle) -> Result<(), String> {
    let path = resolve_team_repo_dir()?;
    open_path_dir(app, &path)
}

/// Resolve Teamai team-repo path (default `~/.teamai/team-repo`, else `teamai status` local:).
pub fn resolve_team_repo_dir() -> Result<PathBuf, String> {
    if let Some(default_dir) = default_team_repo_dir() {
        if is_git_repo_dir(&default_dir) {
            return Ok(default_dir);
        }
    }

    let out = run_teamai(&["status"]).map_err(|e| format!("无法执行 teamai status: {e}"))?;
    let path = parse_local_path_from_status(&out.combined)
        .ok_or_else(|| "未能从 teamai status 解析 local 目录".to_string())?;
    let path = PathBuf::from(path);
    if !is_git_repo_dir(&path) {
        return Err(format!("本地目录不是有效的 git 仓库: {}", path.display()));
    }
    Ok(path)
}

fn is_git_repo_dir(path: &Path) -> bool {
    path.is_dir() && path.join(".git").exists()
}

fn open_path_dir(app: &AppHandle, path: &Path) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path.to_string_lossy().as_ref(), None::<&str>)
        .map_err(|e| format!("打开本地目录失败: {e}"))
}

fn default_team_repo_dir() -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".teamai").join("team-repo"))
}

fn parse_local_path_from_status(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        let Some((key, rest)) = trimmed.split_once(':') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case("local") {
            let path = rest.trim();
            if !path.is_empty() {
                return Some(path.to_string());
            }
        }
    }
    None
}

fn is_already_up_to_date(output: &str) -> bool {
    let lower = output.to_ascii_lowercase();
    lower.contains("already up to date") || lower.contains("already latest")
}

struct ProcessOutput {
    ok: bool,
    combined: String,
}

fn run_teamai(args: &[&str]) -> Result<ProcessOutput, String> {
    // Prefer Volta exe so we never hit .cmd quoting issues on Windows GUI PATH.
    #[cfg(windows)]
    {
        if let Some(volta) = find_volta_exe() {
            let mut full = Vec::with_capacity(args.len() + 2);
            full.push("run");
            full.push("teamai");
            full.extend_from_slice(args);
            return run_bin_with_args(&volta, &full);
        }
    }

    let bin = resolve_teamai_bin().ok_or_else(|| "teamai 未安装".to_string())?;
    run_bin_with_args(&bin, args)
}

#[cfg(windows)]
fn resolve_npm_bin() -> Option<PathBuf> {
    let mut candidates = vec![PathBuf::from(r"C:\Program Files\Volta\npm.cmd")];
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        candidates.push(PathBuf::from(local).join("Volta\\bin\\npm.cmd"));
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        candidates.push(PathBuf::from(appdata).join("npm\\npm.cmd"));
    }
    candidates.into_iter().find(|c| c.is_file())
}

/// Build the `/C` payload: `"C:\path\file.cmd" arg1 arg2` then wrap as
/// `""C:\path\file.cmd" arg1 arg2"` for cmd.exe (must be passed via raw_arg).
#[cfg(windows)]
fn cmd_script_c_arg(bin: &Path, args: &[&str]) -> String {
    let mut inner = format!("\"{}\"", bin.display());
    for a in args {
        inner.push(' ');
        if a.is_empty() || a.contains([' ', '\t', '"', '&', '|', '<', '>']) {
            inner.push('"');
            inner.push_str(&a.replace('"', "\\\""));
            inner.push('"');
        } else {
            inner.push_str(a);
        }
    }
    format!("\"{inner}\"")
}

fn run_bin_with_args(bin: &Path, args: &[&str]) -> Result<ProcessOutput, String> {
    let path_env = effective_path();

    #[cfg(windows)]
    {
        if is_cmd_script(bin) {
            let c_arg = cmd_script_c_arg(bin, args);
            let mut cmd = Command::new("cmd.exe");
            cmd.arg("/D");
            // Avoid Rust's CreateProcess quoting which breaks cmd /C batch paths.
            cmd.raw_arg(format!("/C {c_arg}"));
            cmd.stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            if let Some(p) = &path_env {
                cmd.env("PATH", p);
            }
            apply_no_window(&mut cmd);
            return capture_output(cmd);
        }
    }

    let mut cmd = Command::new(bin);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(p) = &path_env {
        cmd.env("PATH", p);
    }
    apply_no_window(&mut cmd);
    capture_output(cmd)
}

fn decode_console_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    #[cfg(windows)]
    {
        let (cow, _, _) = encoding_rs::GBK.decode(bytes);
        return cow.into_owned();
    }
    #[cfg(not(windows))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

fn capture_output(mut cmd: Command) -> Result<ProcessOutput, String> {
    let output = cmd.output().map_err(|e| e.to_string())?;
    let mut combined = String::new();
    let stdout = decode_console_bytes(&output.stdout);
    let stderr = decode_console_bytes(&output.stderr);
    if !stdout.is_empty() {
        combined.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !combined.is_empty() && !combined.ends_with('\n') {
            combined.push('\n');
        }
        combined.push_str(&stderr);
    }
    Ok(ProcessOutput {
        ok: output.status.success(),
        combined,
    })
}

/// Probe installation and return diagnostics for the UI log pane.
pub fn detect() -> TeamaiCliStatus {
    let (status, bin) = detect_uncached();
    store_resolved_bin(bin);
    status
}

fn detect_uncached() -> (TeamaiCliStatus, Option<PathBuf>) {
    let mut log = LogBuf::new();
    log.line("# teamai CLI 检测");

    // Prefer a short, decisive probe order.
    let mut candidates: Vec<PathBuf> = Vec::new();

    #[cfg(windows)]
    {
        // Volta user shim (npm -g via Volta lands here).
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(PathBuf::from(&local).join("Volta\\bin\\teamai.cmd"));
        }
        // Classic npm global.
        if let Some(appdata) = std::env::var_os("APPDATA") {
            candidates.push(PathBuf::from(&appdata).join("npm\\teamai.cmd"));
        }
    }

    // PATH hits (native shims only).
    for p in find_native_shims_on_path("teamai", &mut log) {
        if !candidates.iter().any(|c| paths_equal(c, &p)) {
            candidates.push(p);
        }
    }

    #[cfg(not(windows))]
    {
        for p in find_native_shims_on_path("teamai", &mut log) {
            if !candidates.iter().any(|c| paths_equal(c, &p)) {
                candidates.push(p);
            }
        }
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            candidates.push(home.join(".volta/bin/teamai"));
            candidates.push(home.join(".npm-global/bin/teamai"));
            candidates.push(home.join(".local/bin/teamai"));
        }
    }

    // Drop missing / non-native (e.g. extensionless bash shims).
    candidates.retain(|p| p.is_file() && is_runnable_shim(p));

    if candidates.is_empty() {
        log.line("未在 PATH / Volta / npm 全局目录找到 teamai.cmd");
        // Last resort: `volta run teamai --version`
        if try_volta_run(&mut log) {
            let bin = PathBuf::from("teamai");
            log.line("ok: 通过 volta run teamai");
            return (ok_status(bin.clone(), &log), Some(bin));
        }
        log.line("fail: 未安装或无法执行 teamai");
        return (fail_status(&log), None);
    }

    log.line(format!("候选 {} 个（已过滤无效 shim）", candidates.len()));
    for cand in &candidates {
        log.line(format!("> {}", cand.display()));
        match run_version(cand) {
            Ok(true) => {
                log.line("ok: --version 成功");
                return (ok_status(cand.clone(), &log), Some(cand.clone()));
            }
            Ok(false) => {
                log.line("  --version 退出码非 0");
            }
            Err(e) => {
                log.line(format!("  启动失败: {e}"));
            }
        }
    }

    if try_volta_run(&mut log) {
        let bin = candidates
            .into_iter()
            .next()
            .unwrap_or_else(|| PathBuf::from("teamai"));
        log.line("ok: 通过 volta run teamai");
        return (ok_status(bin.clone(), &log), Some(bin));
    }

    log.line("fail: 找到入口但无法执行（请确认 Volta/Node PATH）");
    (fail_status(&log), None)
}

fn ok_status(bin: PathBuf, log: &LogBuf) -> TeamaiCliStatus {
    TeamaiCliStatus {
        installed: true,
        bin: Some(bin.display().to_string()),
        log: log.finish(),
    }
}

fn fail_status(log: &LogBuf) -> TeamaiCliStatus {
    TeamaiCliStatus {
        installed: false,
        bin: None,
        log: log.finish(),
    }
}

struct LogBuf {
    lines: Vec<String>,
}

impl LogBuf {
    fn new() -> Self {
        Self { lines: Vec::new() }
    }
    fn line(&mut self, s: impl Into<String>) {
        self.lines.push(s.into());
    }
    fn finish(&self) -> String {
        let mut out = self.lines.join("\n");
        out.push('\n');
        out
    }
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    #[cfg(windows)]
    {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        a == b
    }
}

fn is_runnable_shim(path: &Path) -> bool {
    #[cfg(windows)]
    {
        is_windows_native_shim(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        true
    }
}

fn is_windows_native_shim(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            e.eq_ignore_ascii_case("cmd")
                || e.eq_ignore_ascii_case("bat")
                || e.eq_ignore_ascii_case("exe")
                || e.eq_ignore_ascii_case("com")
        })
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_cmd_script(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat"))
        .unwrap_or(false)
}

fn run_version(bin: &Path) -> Result<bool, String> {
    run_bin_with_args(bin, &["--version"]).map(|out| out.ok)
}

fn try_volta_run(log: &mut LogBuf) -> bool {
    #[cfg(windows)]
    {
        let Some(volta) = find_volta_exe() else {
            log.line("未找到 volta.exe，跳过 volta run");
            return false;
        };
        log.line(format!("> {} run teamai --version", volta.display()));
        let mut cmd = Command::new(&volta);
        cmd.args(["run", "teamai", "--version"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(p) = effective_path() {
            cmd.env("PATH", p);
        }
        apply_no_window(&mut cmd);
        match cmd.output() {
            Ok(out) if out.status.success() => true,
            Ok(out) => {
                let err = decode_console_bytes(&out.stderr);
                let brief: String = err.trim().chars().take(120).collect();
                if !brief.is_empty() {
                    log.line(format!("  volta run 失败: {brief}"));
                } else {
                    log.line("  volta run 退出码非 0");
                }
                false
            }
            Err(e) => {
                log.line(format!("  volta run 启动失败: {e}"));
                false
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = log;
        false
    }
}

fn find_volta_exe() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let mut candidates = vec![
            PathBuf::from(r"C:\Program Files\Volta\volta.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Volta\volta.exe"),
        ];
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(PathBuf::from(local).join(r"Volta\bin\volta.exe"));
        }
        if let Some(user) = std::env::var_os("USERPROFILE") {
            candidates.push(PathBuf::from(user).join(r".volta\bin\volta.exe"));
        }
        for c in candidates {
            if c.is_file() {
                return Some(c);
            }
        }
        let mut cmd = Command::new("where.exe");
        cmd.arg("volta.exe")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Some(p) = effective_path() {
            cmd.env("PATH", p);
        }
        apply_no_window(&mut cmd);
        let output = cmd.output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let p = PathBuf::from(line.trim());
            if p.is_file() {
                return Some(p);
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        let mut candidates = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            candidates.push(PathBuf::from(&home).join(".volta/bin/volta"));
        }
        candidates.push(PathBuf::from("/usr/local/bin/volta"));
        for c in candidates {
            if c.is_file() {
                return Some(c);
            }
        }
        let mut cmd = Command::new("which");
        cmd.arg("volta")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Some(p) = effective_path() {
            cmd.env("PATH", p);
        }
        let output = cmd.output().ok()?;
        if !output.status.success() {
            return None;
        }
        let line = String::from_utf8_lossy(&output.stdout);
        let p = PathBuf::from(line.trim());
        if p.is_file() {
            return Some(p);
        }
        None
    }
}

fn apply_no_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = cmd;
}

fn find_native_shims_on_path(name: &str, log: &mut LogBuf) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("where.exe");
        cmd.arg(name)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Some(p) = effective_path() {
            cmd.env("PATH", p);
        }
        apply_no_window(&mut cmd);
        let output = match cmd.output() {
            Ok(o) => o,
            Err(e) => {
                log.line(format!("where.exe 失败: {e}"));
                return Vec::new();
            }
        };
        if !output.status.success() {
            return Vec::new();
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut paths = Vec::new();
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let path = PathBuf::from(trimmed);
            // Skip Volta node image copies; prefer bin shim.
            let lossy = path.to_string_lossy();
            if lossy.contains(r"\tools\image\") {
                continue;
            }
            if path.is_file() && is_windows_native_shim(&path) {
                paths.push(path);
            }
        }
        paths
    }
    #[cfg(not(windows))]
    {
        let mut cmd = Command::new("which");
        cmd.arg(name)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Some(p) = effective_path() {
            cmd.env("PATH", p);
        }
        let output = match cmd.output() {
            Ok(o) => o,
            Err(e) => {
                log.line(format!("which 失败: {e}"));
                return Vec::new();
            }
        };
        if !output.status.success() {
            return Vec::new();
        }
        let line = String::from_utf8_lossy(&output.stdout);
        let trimmed = line.lines().next().unwrap_or("").trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        let p = PathBuf::from(trimmed);
        if p.is_file() {
            vec![p]
        } else {
            Vec::new()
        }
    }
}

fn effective_path() -> Option<String> {
    #[cfg(windows)]
    {
        Some(merged_windows_path())
    }
    #[cfg(not(windows))]
    {
        let current = std::env::var_os("PATH")?;
        let mut parts: Vec<PathBuf> = Vec::new();
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            for extra in [
                home.join(".volta/bin"),
                home.join(".npm-global/bin"),
                home.join(".local/bin"),
            ] {
                if extra.is_dir() {
                    parts.push(extra);
                }
            }
        }
        parts.extend(std::env::split_paths(&current));
        std::env::join_paths(parts)
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    }
}

#[cfg(windows)]
fn merged_windows_path() -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut push = |raw: Option<String>| {
        let Some(raw) = raw else {
            return;
        };
        for part in raw.split(';') {
            let p = part.trim();
            if p.is_empty() {
                continue;
            }
            if !parts.iter().any(|e| e.eq_ignore_ascii_case(p)) {
                parts.push(p.to_string());
            }
        }
    };

    // Keep System32 so cmd.exe / where.exe remain resolvable when we override PATH.
    if let Ok(root) = std::env::var("SystemRoot") {
        push(Some(format!(r"{root}\System32")));
        push(Some(format!(r"{root}\SysWOW64")));
        push(Some(root));
    }

    push(std::env::var("PATH").ok());
    push(read_user_path());
    push(read_machine_path());

    // Volta requires both user shims and Program Files volta.exe.
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        push(Some(
            PathBuf::from(local)
                .join("Volta\\bin")
                .to_string_lossy()
                .into_owned(),
        ));
    }
    push(Some(r"C:\Program Files\Volta".to_string()));
    push(Some(r"C:\Program Files (x86)\Volta".to_string()));
    if let Some(appdata) = std::env::var_os("APPDATA") {
        push(Some(
            PathBuf::from(appdata)
                .join("npm")
                .to_string_lossy()
                .into_owned(),
        ));
    }

    parts.join(";")
}

#[cfg(windows)]
fn read_user_path() -> Option<String> {
    read_registry_path(
        windows_sys::Win32::System::Registry::HKEY_CURRENT_USER,
        windows_sys::core::w!("Environment"),
    )
}

#[cfg(windows)]
fn read_machine_path() -> Option<String> {
    read_registry_path(
        windows_sys::Win32::System::Registry::HKEY_LOCAL_MACHINE,
        windows_sys::core::w!("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment"),
    )
}

#[cfg(windows)]
fn read_registry_path(
    hive: windows_sys::Win32::System::Registry::HKEY,
    subkey: windows_sys::core::PCWSTR,
) -> Option<String> {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, KEY_READ, REG_EXPAND_SZ, REG_SZ,
    };

    unsafe {
        let mut hkey = std::mem::zeroed();
        let open = RegOpenKeyExW(hive, subkey, 0, KEY_READ, &mut hkey);
        if open != ERROR_SUCCESS {
            return None;
        }

        let name = windows_sys::core::w!("Path");
        let mut ty = 0u32;
        let mut size = 0u32;
        let probe = RegQueryValueExW(
            hkey,
            name,
            std::ptr::null_mut(),
            &mut ty,
            std::ptr::null_mut(),
            &mut size,
        );
        if probe != ERROR_SUCCESS || size == 0 {
            RegCloseKey(hkey);
            return None;
        }

        let mut buf = vec![0u8; size as usize];
        let query = RegQueryValueExW(
            hkey,
            name,
            std::ptr::null_mut(),
            &mut ty,
            buf.as_mut_ptr(),
            &mut size,
        );
        RegCloseKey(hkey);
        if query != ERROR_SUCCESS {
            return None;
        }
        if ty != REG_SZ && ty != REG_EXPAND_SZ {
            return None;
        }

        let wide_len = (size as usize) / 2;
        let wide = std::slice::from_raw_parts(buf.as_ptr() as *const u16, wide_len);
        let end = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
        let raw = String::from_utf16_lossy(&wide[..end]);
        if ty == REG_EXPAND_SZ {
            Some(expand_env_strings(&raw))
        } else {
            Some(raw)
        }
    }
}

#[cfg(windows)]
fn expand_env_strings(raw: &str) -> String {
    use windows_sys::Win32::System::Environment::ExpandEnvironmentStringsW;

    let wide: Vec<u16> = raw.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let needed = ExpandEnvironmentStringsW(wide.as_ptr(), std::ptr::null_mut(), 0);
        if needed == 0 {
            return raw.to_string();
        }
        let mut out = vec![0u16; needed as usize];
        let written = ExpandEnvironmentStringsW(wide.as_ptr(), out.as_mut_ptr(), needed);
        if written == 0 {
            return raw.to_string();
        }
        let end = out.iter().position(|&c| c == 0).unwrap_or(out.len());
        String::from_utf16_lossy(&out[..end])
    }
}
