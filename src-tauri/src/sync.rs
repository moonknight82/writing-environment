use super::{atomic_replace, is_markdown, read_unicode_markdown_file, sync_file_and_parent};
use chrono::Utc;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use tauri::Manager;
use tempfile::NamedTempFile;
use wait_timeout::ChildExt;
use walkdir::WalkDir;

const MIN_RCLONE_VERSION: (u32, u32) = (1, 66);
const ACCESS_MARKER: &str = ".writing-environment-sync";
const CONFLICT_SUFFIX: &str = "writing-environment-conflict";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(15 * 60);
static SYNC_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncAvailability {
    installed: bool,
    compatible: bool,
    version: Option<String>,
    remotes: Vec<String>,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    status: String,
    message: String,
    conflicts: usize,
    initialized: bool,
    completed_at: String,
}

#[derive(Debug)]
struct CommandResult {
    stdout: String,
}

#[tauri::command]
pub fn sync_availability() -> SyncAvailability {
    availability_for("rclone")
}

#[tauri::command]
pub async fn sync_project(
    app: tauri::AppHandle,
    root: String,
    remote: String,
    remote_path: String,
) -> Result<SyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = SYNC_LOCK
            .try_lock()
            .map_err(|_| "A sync is already running.".to_string())?;
        sync_project_blocking(&app, &root, &remote, &remote_path)
    })
    .await
    .map_err(|error| format!("The sync worker stopped unexpectedly: {error}"))?
}

#[tauri::command]
pub async fn recover_sync_project(
    app: tauri::AppHandle,
    root: String,
    remote: String,
    remote_path: String,
    restore_to: String,
) -> Result<SyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = SYNC_LOCK
            .try_lock()
            .map_err(|_| "A sync is already running.".to_string())?;
        recover_sync_project_blocking(&app, &root, &remote, &remote_path, &restore_to)
    })
    .await
    .map_err(|error| format!("The sync recovery worker stopped unexpectedly: {error}"))?
}

fn sync_project_blocking(
    app: &tauri::AppHandle,
    root: &str,
    remote: &str,
    remote_path: &str,
) -> Result<SyncResult, String> {
    let availability = availability_for("rclone");
    if !availability.compatible {
        return Err(availability.message);
    }

    let remote = normalize_remote(remote)?;
    if !availability.remotes.iter().any(|item| item == &remote) {
        return Err(format!(
            "The rclone remote ‘{remote}’ is not configured. Run ‘rclone config’ and refresh remotes."
        ));
    }
    let remote_path = normalize_remote_path(remote_path)?;
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("Cannot open the project folder: {error}"))?;
    if !canonical_root.is_dir() {
        return Err("The project path is not a folder.".into());
    }

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Cannot locate the app data folder: {error}"))?;
    let profile = sync_profile_dir(&app_data, &canonical_root, &remote, &remote_path);
    let workdir = profile.join("work");
    let local_backup = profile.join("local-backups");
    let state_file = profile.join("state");
    fs::create_dir_all(&workdir)
        .and_then(|_| fs::create_dir_all(&local_backup))
        .map_err(|error| format!("Cannot prepare sync state: {error}"))?;

    let remote_target = format!("{remote}:{remote_path}");
    let archive_target = format!("{remote}:{}", remote_archive_path(&remote_path));
    let phase = fs::read_to_string(&state_file).unwrap_or_default();
    let initialized = phase.trim() == "initialized";
    let initializing = phase.trim() == "initializing";

    if !initialized {
        initialize_sync(
            &canonical_root,
            &remote_target,
            &archive_target,
            &workdir,
            &local_backup,
            &state_file,
            initializing,
        )?;
    } else {
        ensure_access_marker(&canonical_root)?;
        let repaired_unreadable = repair_unreadable_local_markdown(
            &canonical_root,
            &remote_target,
            &profile.join("unreadable-backups"),
        )?;
        run_bisync(
            &canonical_root,
            &remote_target,
            &archive_target,
            &workdir,
            &local_backup,
            false,
        )?;

        let conflicts = count_conflicts(&canonical_root);
        return Ok(sync_result(conflicts, repaired_unreadable));
    }

    let conflicts = count_conflicts(&canonical_root);
    Ok(sync_result(conflicts, 0))
}

fn sync_result(conflicts: usize, repaired_unreadable: usize) -> SyncResult {
    let repaired_message = match repaired_unreadable {
        0 => String::new(),
        1 => " One unreadable local Markdown file was safely restored from the remote copy; its original bytes remain in recovery data.".to_string(),
        count => format!(
            " {count} unreadable local Markdown files were safely restored from their remote copies; the original bytes remain in recovery data."
        ),
    };
    let (status, message) = if conflicts > 0 {
        (
            "conflict".to_string(),
            format!(
                "Synced with {conflicts} preserved conflict file{}. Review the files marked ‘{CONFLICT_SUFFIX}’.{repaired_message}",
                if conflicts == 1 { "" } else { "s" },
            ),
        )
    } else {
        (
            "synced".to_string(),
            format!("All changes are synced.{repaired_message}"),
        )
    };
    SyncResult {
        status,
        message,
        conflicts,
        initialized: true,
        completed_at: Utc::now().to_rfc3339(),
    }
}

fn recover_sync_project_blocking(
    app: &tauri::AppHandle,
    root: &str,
    remote: &str,
    remote_path: &str,
    restore_to: &str,
) -> Result<SyncResult, String> {
    let availability = availability_for("rclone");
    if !availability.compatible {
        return Err(availability.message);
    }
    let remote = normalize_remote(remote)?;
    if !availability.remotes.iter().any(|item| item == &remote) {
        return Err(format!(
            "The rclone remote ‘{remote}’ is not configured. Run ‘rclone config’ and refresh remotes."
        ));
    }
    let remote_path = normalize_remote_path(remote_path)?;
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("Cannot open the project folder: {error}"))?;
    if !canonical_root.is_dir() {
        return Err("The project path is not a folder.".into());
    }

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Cannot locate the app data folder: {error}"))?;
    let profile = sync_profile_dir(&app_data, &canonical_root, &remote, &remote_path);
    let workdir = profile.join("work");
    let local_backup = profile.join("local-backups");
    let state_file = profile.join("state");
    let phase = fs::read_to_string(&state_file).unwrap_or_default();
    if phase.trim() != "initialized" {
        return Err(
            "This sync has not been initialized, so deletion recovery is unavailable.".into(),
        );
    }
    fs::create_dir_all(&workdir)
        .and_then(|_| fs::create_dir_all(&local_backup))
        .map_err(|error| format!("Cannot prepare sync recovery state: {error}"))?;

    let remote_target = format!("{remote}:{remote_path}");
    let archive_target = format!("{remote}:{}", remote_archive_path(&remote_path));
    ensure_access_marker(&canonical_root)?;
    run_rclone(
        &recovery_copy_args(&canonical_root, &remote_target, restore_to)?,
        COMMAND_TIMEOUT,
    )?;
    let repaired_unreadable = repair_unreadable_local_markdown(
        &canonical_root,
        &remote_target,
        &profile.join("unreadable-backups"),
    )?;
    run_bisync(
        &canonical_root,
        &remote_target,
        &archive_target,
        &workdir,
        &local_backup,
        false,
    )?;

    let conflicts = count_conflicts(&canonical_root);
    let repaired_message = match repaired_unreadable {
        0 => String::new(),
        1 => " One unreadable local Markdown file was also restored from the remote copy; its original bytes remain in recovery data.".to_string(),
        count => format!(
            " {count} unreadable local Markdown files were also restored from their remote copies; the original bytes remain in recovery data."
        ),
    };
    let (status, message) = if conflicts > 0 {
        (
            "conflict".to_string(),
            format!(
                "Missing files were restored without overwriting existing files. Sync then preserved {conflicts} conflict file{}. Review files marked ‘{CONFLICT_SUFFIX}’.{repaired_message}",
                if conflicts == 1 { "" } else { "s" },
            ),
        )
    } else {
        (
            "synced".to_string(),
            format!("Missing files were restored without overwriting existing files. The project is synced; automatic sync remains off until you re-enable it.{repaired_message}"),
        )
    };
    Ok(SyncResult {
        status,
        message,
        conflicts,
        initialized: true,
        completed_at: Utc::now().to_rfc3339(),
    })
}

fn repair_unreadable_local_markdown(
    root: &Path,
    remote_target: &str,
    backup_root: &Path,
) -> Result<usize, String> {
    let unreadable = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && is_markdown(entry.path()))
        .filter(|entry| read_unicode_markdown_file(entry.path()).is_err())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    let mut repaired = 0;

    for local_path in unreadable {
        let relative = local_path
            .strip_prefix(root)
            .map_err(|_| "An unreadable sheet resolved outside the project.".to_string())?;
        let remote_file = remote_markdown_target(remote_target, relative)?;
        let temporary = NamedTempFile::new()
            .map_err(|error| format!("Cannot prepare an unreadable-sheet recovery: {error}"))?;
        let copy_result = run_rclone(
            &unreadable_recovery_copy_args(remote_file, temporary.path()),
            COMMAND_TIMEOUT,
        );
        if copy_result.is_err() || read_unicode_markdown_file(temporary.path()).is_err() {
            continue;
        }

        let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
        let relative_parent = relative.parent().unwrap_or_else(|| Path::new(""));
        let backup_parent = backup_root
            .join(timestamp.to_string())
            .join(relative_parent);
        let backup_name = format!(
            "{}.bin",
            relative
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unreadable.md")
        );
        let backup = backup_parent.join(backup_name);
        preserve_and_replace_unreadable(&local_path, temporary.path(), &backup)?;
        repaired += 1;
    }

    Ok(repaired)
}

fn unreadable_recovery_copy_args(remote_file: String, temporary: &Path) -> Vec<String> {
    vec![
        "copyto".into(),
        remote_file,
        temporary.to_string_lossy().into_owned(),
        "--check-first".into(),
        "--ignore-times".into(),
    ]
}

fn preserve_and_replace_unreadable(
    local_path: &Path,
    recovered_path: &Path,
    backup_path: &Path,
) -> Result<(), String> {
    read_unicode_markdown_file(recovered_path)
        .map_err(|_| "The remote recovery copy is not readable Markdown text.".to_string())?;
    let backup_parent = backup_path
        .parent()
        .ok_or_else(|| "The unreadable-sheet backup has no parent folder.".to_string())?;
    fs::create_dir_all(backup_parent)
        .map_err(|error| format!("Cannot preserve an unreadable local sheet: {error}"))?;
    fs::copy(local_path, backup_path)
        .map_err(|error| format!("Cannot preserve an unreadable local sheet: {error}"))?;
    sync_file_and_parent(backup_path)?;

    let remote_bytes = fs::read(recovered_path)
        .map_err(|error| format!("Cannot read the recovered remote sheet: {error}"))?;
    atomic_replace(local_path, &remote_bytes)
}

fn remote_markdown_target(remote_target: &str, relative: &Path) -> Result<String, String> {
    let relative = relative
        .components()
        .map(|component| match component {
            std::path::Component::Normal(value) => value
                .to_str()
                .map(str::to_string)
                .ok_or_else(|| "A Markdown filename is not valid Unicode.".to_string()),
            _ => Err("A Markdown path escaped its project folder.".to_string()),
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("/");
    Ok(format!(
        "{}/{relative}",
        remote_target.trim_end_matches('/')
    ))
}

fn recovery_copy_args(
    root: &Path,
    remote_target: &str,
    restore_to: &str,
) -> Result<Vec<String>, String> {
    let local = root.to_string_lossy().into_owned();
    let (source, destination) = match restore_to {
        "local" => (remote_target.to_string(), local),
        "remote" => (local, remote_target.to_string()),
        _ => return Err("Choose whether to restore missing files locally or remotely.".into()),
    };
    Ok(vec![
        "copy".into(),
        source,
        destination,
        "--ignore-existing".into(),
        "--create-empty-src-dirs".into(),
        "--check-first".into(),
        "--checkers".into(),
        "4".into(),
        "--transfers".into(),
        "2".into(),
    ])
}

fn initialize_sync(
    root: &Path,
    remote_target: &str,
    archive_target: &str,
    workdir: &Path,
    local_backup: &Path,
    state_file: &Path,
    resuming: bool,
) -> Result<(), String> {
    if !resuming {
        run_rclone(&["mkdir".into(), remote_target.into()], COMMAND_TIMEOUT)?;
        let listing = run_rclone(
            &[
                "lsf".into(),
                remote_target.into(),
                "--max-depth".into(),
                "1".into(),
            ],
            COMMAND_TIMEOUT,
        )?;
        if !listing.stdout.trim().is_empty() {
            return Err(
                "For safety, first-time sync requires an empty remote folder. Choose a new folder or empty it yourself after confirming its contents are backed up."
                    .into(),
            );
        }
        fs::write(state_file, "initializing\n")
            .map_err(|error| format!("Cannot record sync initialization: {error}"))?;
    }

    ensure_access_marker(root)?;
    run_rclone(
        &[
            "copy".into(),
            root.to_string_lossy().into_owned(),
            remote_target.into(),
            "--create-empty-src-dirs".into(),
            "--check-first".into(),
            "--checkers".into(),
            "4".into(),
            "--transfers".into(),
            "2".into(),
        ],
        COMMAND_TIMEOUT,
    )?;
    run_bisync(
        root,
        remote_target,
        archive_target,
        workdir,
        local_backup,
        true,
    )?;
    fs::write(state_file, "initialized\n")
        .map_err(|error| format!("Cannot finish sync initialization: {error}"))?;
    Ok(())
}

fn run_bisync(
    root: &Path,
    remote_target: &str,
    archive_target: &str,
    workdir: &Path,
    local_backup: &Path,
    resync: bool,
) -> Result<(), String> {
    run_rclone(
        &bisync_args(
            root,
            remote_target,
            archive_target,
            workdir,
            local_backup,
            resync,
        ),
        COMMAND_TIMEOUT,
    )
    .map(|_| ())
    .map_err(classify_bisync_error)
}

fn classify_bisync_error(error: String) -> String {
    if !error.contains("Safety abort: too many deletes") {
        return error;
    }
    if error.contains("Path1") {
        return format!(
            "SYNC_DELETE_GUARD_LOCAL: More than 25% of tracked files are missing locally. {error}"
        );
    }
    if error.contains("Path2") {
        return format!(
            "SYNC_DELETE_GUARD_REMOTE: More than 25% of tracked files are missing remotely. {error}"
        );
    }
    error
}

fn bisync_args(
    root: &Path,
    remote_target: &str,
    archive_target: &str,
    workdir: &Path,
    local_backup: &Path,
    resync: bool,
) -> Vec<String> {
    let mut args = vec![
        "bisync".into(),
        root.to_string_lossy().into_owned(),
        remote_target.into(),
        "--workdir".into(),
        workdir.to_string_lossy().into_owned(),
        "--check-access".into(),
        "--check-filename".into(),
        ACCESS_MARKER.into(),
        "--conflict-resolve".into(),
        "none".into(),
        "--conflict-loser".into(),
        "num".into(),
        "--conflict-suffix".into(),
        CONFLICT_SUFFIX.into(),
        "--suffix-keep-extension".into(),
        "--resilient".into(),
        "--recover".into(),
        "--max-lock".into(),
        "2m".into(),
        "--create-empty-src-dirs".into(),
        "--track-renames".into(),
        "--max-delete".into(),
        "25".into(),
        "--backup-dir1".into(),
        local_backup.to_string_lossy().into_owned(),
        "--backup-dir2".into(),
        archive_target.into(),
        "--suffix".into(),
        "{DateOnly}-backup".into(),
        "--checkers".into(),
        "4".into(),
        "--transfers".into(),
        "2".into(),
        "--retries".into(),
        "3".into(),
        "--retries-sleep".into(),
        "5s".into(),
    ];
    if resync {
        args.extend(["--resync".into(), "--resync-mode".into(), "path1".into()]);
    }
    args
}

fn availability_for(binary: &str) -> SyncAvailability {
    let version_output = match Command::new(binary).arg("version").output() {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return SyncAvailability {
                installed: true,
                compatible: false,
                version: None,
                remotes: vec![],
                message: if message.is_empty() {
                    "rclone could not report its version.".into()
                } else {
                    message
                },
            };
        }
        Err(_) => {
            return SyncAvailability {
                installed: false,
                compatible: false,
                version: None,
                remotes: vec![],
                message: "rclone is not installed. Install it, then run ‘rclone config’ to add Dropbox or another provider.".into(),
            };
        }
    };

    let version_text = String::from_utf8_lossy(&version_output.stdout);
    let version = version_text
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    let compatible = parse_rclone_version(&version)
        .map(|found| found >= MIN_RCLONE_VERSION)
        .unwrap_or(false);
    if !compatible {
        return SyncAvailability {
            installed: true,
            compatible: false,
            version: Some(version.clone()),
            remotes: vec![],
            message: format!(
                "Writing Environment requires rclone 1.66 or newer; {version} is installed."
            ),
        };
    }

    let remote_output = Command::new(binary)
        .args(["listremotes", "--ask-password=false"])
        .output();
    let (remotes, message) = match remote_output {
        Ok(output) if output.status.success() => {
            let mut remotes = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|line| line.trim().trim_end_matches(':').to_string())
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>();
            remotes.sort();
            let message = if remotes.is_empty() {
                "rclone is ready, but no remotes are configured. Run ‘rclone config’ first.".into()
            } else {
                "rclone is ready.".into()
            };
            (remotes, message)
        }
        Ok(output) => {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            (
                vec![],
                if detail.is_empty() {
                    "Could not read rclone remotes. If the config is encrypted, unlock it in a terminal first."
                        .into()
                } else {
                    detail
                },
            )
        }
        Err(error) => (vec![], format!("Could not read rclone remotes: {error}")),
    };

    SyncAvailability {
        installed: true,
        compatible: true,
        version: Some(version),
        remotes,
        message,
    }
}

fn run_rclone(args: &[String], timeout: Duration) -> Result<CommandResult, String> {
    let stdout_file = NamedTempFile::new()
        .map_err(|error| format!("Cannot create the sync output log: {error}"))?;
    let stderr_file = NamedTempFile::new()
        .map_err(|error| format!("Cannot create the sync error log: {error}"))?;
    let stdout_writer = stdout_file
        .reopen()
        .map_err(|error| format!("Cannot open the sync output log: {error}"))?;
    let stderr_writer = stderr_file
        .reopen()
        .map_err(|error| format!("Cannot open the sync error log: {error}"))?;
    let mut command_args = args.to_vec();
    command_args.push("--ask-password=false".into());
    let mut child = Command::new("rclone")
        .args(&command_args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_writer))
        .stderr(Stdio::from(stderr_writer))
        .spawn()
        .map_err(|error| format!("Cannot start rclone: {error}"))?;

    let status = match child
        .wait_timeout(timeout)
        .map_err(|error| format!("Cannot wait for rclone: {error}"))?
    {
        Some(status) => status,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(
                "Sync timed out after 15 minutes. Your local files were not removed.".into(),
            );
        }
    };
    let stdout = fs::read_to_string(stdout_file.path()).unwrap_or_default();
    let stderr = fs::read_to_string(stderr_file.path()).unwrap_or_default();
    if !status.success() {
        let detail = concise_error(&stderr, &stdout);
        return Err(if detail.is_empty() {
            format!("rclone stopped with {status}.")
        } else {
            format!("Sync failed: {detail}")
        });
    }
    Ok(CommandResult { stdout })
}

fn concise_error(stderr: &str, stdout: &str) -> String {
    let source = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    let mut lines = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.len() > 8 {
        lines = lines.split_off(lines.len() - 8);
    }
    lines.join(" ")
}

fn normalize_remote(value: &str) -> Result<String, String> {
    let remote = value.trim().trim_end_matches(':');
    if remote.is_empty() || remote.chars().any(char::is_control) || remote.contains(':') {
        return Err("Choose a valid rclone remote.".into());
    }
    Ok(remote.to_string())
}

fn normalize_remote_path(value: &str) -> Result<String, String> {
    let path = value.trim().trim_matches('/');
    if path.is_empty()
        || path.starts_with('-')
        || path.contains(':')
        || path.chars().any(char::is_control)
        || path
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(
            "Choose a simple remote folder path, such as ‘Writing Environment/My Novel’.".into(),
        );
    }
    Ok(path.to_string())
}

fn remote_archive_path(remote_path: &str) -> String {
    format!("{remote_path}.writing-environment-archive")
}

fn sync_profile_dir(app_data: &Path, root: &Path, remote: &str, remote_path: &str) -> PathBuf {
    let key = format!("{}\0{remote}\0{remote_path}", root.to_string_lossy());
    let hash = blake3::hash(key.as_bytes()).to_hex().to_string();
    app_data.join("sync").join(hash)
}

fn ensure_access_marker(root: &Path) -> Result<(), String> {
    let marker = root.join(ACCESS_MARKER);
    if marker.exists() {
        return Ok(());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&marker)
        .map_err(|error| format!("Cannot create the sync safety marker: {error}"))?;
    file.write_all(b"Writing Environment sync access check\n")
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("Cannot save the sync safety marker: {error}"))
}

fn count_conflicts(root: &Path) -> usize {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .file_name()
                    .to_str()
                    .map(|name| name.contains(&format!(".{CONFLICT_SUFFIX}")))
                    .unwrap_or(false)
        })
        .count()
}

fn parse_rclone_version(value: &str) -> Option<(u32, u32)> {
    let value = value.trim().strip_prefix("rclone v")?;
    let mut parts = value.split('.');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argument_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
        args.iter()
            .position(|argument| argument == flag)
            .and_then(|index| args.get(index + 1))
            .map(String::as_str)
    }

    #[test]
    fn parses_rclone_versions() {
        assert_eq!(parse_rclone_version("rclone v1.71.2"), Some((1, 71)));
        assert_eq!(parse_rclone_version("rclone v2.0.0-beta"), Some((2, 0)));
        assert_eq!(parse_rclone_version("not rclone"), None);
    }

    #[test]
    fn validates_remote_paths() {
        assert_eq!(
            normalize_remote_path("/Writing Environment/My Novel/"),
            Ok("Writing Environment/My Novel".into())
        );
        assert!(normalize_remote_path("../Novel").is_err());
        assert!(normalize_remote_path("dropbox:Novel").is_err());
        assert!(normalize_remote_path("").is_err());
    }

    #[test]
    fn classifies_delete_guard_by_protected_side() {
        let local = classify_bisync_error(
            "Sync failed: Safety abort: too many deletes (>25%, 2 of 3) on Path1".into(),
        );
        let remote = classify_bisync_error(
            "Sync failed: Safety abort: too many deletes (>25%, 2 of 3) on Path2".into(),
        );

        assert!(local.starts_with("SYNC_DELETE_GUARD_LOCAL:"));
        assert!(remote.starts_with("SYNC_DELETE_GUARD_REMOTE:"));
        assert_eq!(
            classify_bisync_error("Sync failed: network unavailable".into()),
            "Sync failed: network unavailable"
        );
    }

    #[test]
    fn recovery_copy_restores_only_missing_files_in_the_safe_direction() {
        let root = Path::new("/project");
        let local = recovery_copy_args(root, "dropbox:Novel", "local").expect("local recovery");
        let remote = recovery_copy_args(root, "dropbox:Novel", "remote").expect("remote recovery");

        assert_eq!(&local[1..3], &["dropbox:Novel", "/project"]);
        assert_eq!(&remote[1..3], &["/project", "dropbox:Novel"]);
        for args in [&local, &remote] {
            assert!(args.iter().any(|argument| argument == "--ignore-existing"));
            assert!(!args.iter().any(|argument| argument == "--force"));
            assert!(!args.iter().any(|argument| argument == "--resync"));
        }
    }

    #[test]
    fn unreadable_recovery_preserves_original_bytes_before_atomic_replacement() {
        let root = tempfile::tempdir().expect("temporary recovery");
        let local = root.path().join("one.md");
        let recovered = root.path().join("remote-one.md");
        let backup = root.path().join("recovery").join("one.md.bin");
        let corrupt = [0x00, 0x01, 0x02, 0xff];
        fs::write(&local, corrupt).expect("unreadable local sheet");
        fs::write(&recovered, "# One\n\nThe remote text — intact.").expect("valid remote sheet");

        preserve_and_replace_unreadable(&local, &recovered, &backup).expect("safe recovery");

        assert_eq!(fs::read(&backup).expect("preserved bytes"), corrupt);
        assert_eq!(
            fs::read_to_string(&local).expect("restored local sheet"),
            "# One\n\nThe remote text — intact."
        );
    }

    #[test]
    fn unreadable_recovery_maps_nested_paths_to_the_same_remote_file() {
        assert_eq!(
            remote_markdown_target(
                "dropbox:Writing Environment/The arrival",
                Path::new("Draft/one.md"),
            ),
            Ok("dropbox:Writing Environment/The arrival/Draft/one.md".into())
        );
        assert!(remote_markdown_target("dropbox:Novel", Path::new("../outside.md")).is_err());

        let args = unreadable_recovery_copy_args(
            "dropbox:Writing Environment/The arrival/Draft/one.md".into(),
            Path::new("/tmp/recovered"),
        );
        assert_eq!(args[0], "copyto");
        assert!(args.iter().any(|argument| argument == "--ignore-times"));
    }

    #[test]
    fn archive_is_a_non_overlapping_sibling() {
        assert_eq!(
            remote_archive_path("Writing Environment/Novel"),
            "Writing Environment/Novel.writing-environment-archive"
        );
    }

    #[test]
    fn bisync_arguments_preserve_conflicts_and_both_backup_sides() {
        let root = Path::new("/projects/novel");
        let workdir = Path::new("/state/work");
        let local_backup = Path::new("/state/local-backups");
        let remote_target = "dropbox:Writing Environment/Novel";
        let archive_target = "dropbox:Writing Environment/Novel.writing-environment-archive";
        let args = bisync_args(
            root,
            remote_target,
            archive_target,
            workdir,
            local_backup,
            false,
        );

        assert_eq!(argument_value(&args, "--conflict-resolve"), Some("none"));
        assert_eq!(argument_value(&args, "--conflict-loser"), Some("num"));
        assert_eq!(
            argument_value(&args, "--conflict-suffix"),
            Some(CONFLICT_SUFFIX)
        );
        assert!(args
            .iter()
            .any(|argument| argument == "--suffix-keep-extension"));
        assert!(args.iter().any(|argument| argument == "--resilient"));
        assert!(args.iter().any(|argument| argument == "--recover"));
        assert_eq!(argument_value(&args, "--max-delete"), Some("25"));
        assert_eq!(
            argument_value(&args, "--backup-dir1"),
            Some("/state/local-backups")
        );
        assert_eq!(argument_value(&args, "--backup-dir2"), Some(archive_target));
        assert_ne!(remote_target, archive_target);
        assert!(!args.iter().any(|argument| argument == "--resync"));
    }

    #[test]
    fn first_bisync_resync_treats_local_project_as_authoritative() {
        let args = bisync_args(
            Path::new("/projects/novel"),
            "dropbox:Novel",
            "dropbox:Novel.writing-environment-archive",
            Path::new("/state/work"),
            Path::new("/state/local-backups"),
            true,
        );

        assert!(args.iter().any(|argument| argument == "--resync"));
        assert_eq!(argument_value(&args, "--resync-mode"), Some("path1"));
        assert_eq!(args.get(1).map(String::as_str), Some("/projects/novel"));
    }

    #[test]
    fn sync_profiles_are_isolated_by_project_and_remote_identity() {
        let app_data = Path::new("/app-data");
        let first = sync_profile_dir(app_data, Path::new("/projects/first"), "dropbox", "Novel");
        let other_project =
            sync_profile_dir(app_data, Path::new("/projects/second"), "dropbox", "Novel");
        let other_remote_path = sync_profile_dir(
            app_data,
            Path::new("/projects/first"),
            "dropbox",
            "Other Novel",
        );

        assert_ne!(first, other_project);
        assert_ne!(first, other_remote_path);
        assert!(first.starts_with(app_data.join("sync")));
    }

    #[test]
    fn access_marker_creation_is_durable_and_never_overwrites_existing_content() {
        let project = tempfile::tempdir().expect("temporary project");
        ensure_access_marker(project.path()).expect("create access marker");
        let marker = project.path().join(ACCESS_MARKER);
        assert_eq!(
            fs::read_to_string(&marker).expect("marker content"),
            "Writing Environment sync access check\n"
        );

        fs::write(&marker, "provider-owned marker content\n").expect("replace marker content");
        ensure_access_marker(project.path()).expect("keep existing marker");
        assert_eq!(
            fs::read_to_string(marker).expect("preserved marker"),
            "provider-owned marker content\n"
        );
    }

    #[test]
    fn conflict_files_are_counted() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("chapter.md"), "clean").unwrap();
        fs::write(
            directory
                .path()
                .join("chapter.writing-environment-conflict1.md"),
            "conflict",
        )
        .unwrap();
        assert_eq!(count_conflicts(directory.path()), 1);
    }
}
