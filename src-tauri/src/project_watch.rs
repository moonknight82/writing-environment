use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tauri::Emitter;

const CHANGE_EVENT: &str = "library-files-changed";
const QUIET_PERIOD: Duration = Duration::from_millis(500);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LibraryFilesChanged {
    root: String,
}

#[derive(Default)]
pub(super) struct ProjectWatchState {
    active: Mutex<Option<ProjectWatch>>,
}

struct ProjectWatch {
    root: PathBuf,
    watcher: Option<RecommendedWatcher>,
    worker: Option<JoinHandle<()>>,
}

impl ProjectWatch {
    fn start(app: tauri::AppHandle, root: PathBuf) -> Result<Self, String> {
        let payload = LibraryFilesChanged {
            root: root.to_string_lossy().into_owned(),
        };
        Self::start_with_callback(root, move || {
            if let Err(error) = app.emit(CHANGE_EVENT, payload.clone()) {
                eprintln!("Cannot report a project folder change: {error}");
            }
        })
    }

    fn start_with_callback<F>(root: PathBuf, callback: F) -> Result<Self, String>
    where
        F: Fn() + Send + 'static,
    {
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("writing-environment-project-watch".into())
            .spawn(move || emit_coalesced_changes(receiver, callback))
            .map_err(|error| format!("Cannot start the project-change worker: {error}"))?;

        let callback_sender = sender.clone();
        let mut watcher = match notify::recommended_watcher(move |result: notify::Result<Event>| {
            if result.as_ref().is_ok_and(event_affects_markdown) {
                queue_change(&callback_sender);
            }
        }) {
            Ok(watcher) => watcher,
            Err(error) => {
                drop(sender);
                let _ = worker.join();
                return Err(format!(
                    "Cannot prepare project change notifications: {error}"
                ));
            }
        };

        if let Err(error) = watcher.watch(&root, RecursiveMode::Recursive) {
            drop(watcher);
            drop(sender);
            let _ = worker.join();
            return Err(format!("Cannot watch the project folder: {error}"));
        }
        drop(sender);

        Ok(Self {
            root,
            watcher: Some(watcher),
            worker: Some(worker),
        })
    }
}

impl Drop for ProjectWatch {
    fn drop(&mut self) {
        // The event callback owns the last sender. Dropping the watcher closes
        // the channel, allowing the coalescing worker to finish promptly.
        self.watcher.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[tauri::command]
pub(super) fn watch_library(
    app: tauri::AppHandle,
    state: tauri::State<'_, ProjectWatchState>,
    path: String,
) -> Result<String, String> {
    let root = std::fs::canonicalize(&path)
        .map_err(|error| format!("Cannot resolve the project folder for notifications: {error}"))?;
    if !root.is_dir() {
        return Err("The project notification path is not a folder.".into());
    }

    let mut active = state
        .active
        .lock()
        .map_err(|_| "Project change notifications are unavailable.".to_string())?;
    if active.as_ref().is_some_and(|watch| watch.root == root) {
        return Ok(root.to_string_lossy().into_owned());
    }

    let replacement = ProjectWatch::start(app, root.clone())?;
    *active = Some(replacement);
    Ok(root.to_string_lossy().into_owned())
}

#[tauri::command]
pub(super) fn unwatch_library(state: tauri::State<'_, ProjectWatchState>) -> Result<(), String> {
    let previous = state
        .active
        .lock()
        .map_err(|_| "Project change notifications are unavailable.".to_string())?
        .take();
    drop(previous);
    Ok(())
}

fn queue_change(sender: &SyncSender<()>) {
    // A full one-item channel already represents pending work, so additional
    // filesystem events can be discarded until the worker drains it.
    let _ = sender.try_send(());
}

fn emit_coalesced_changes<F>(receiver: Receiver<()>, callback: F)
where
    F: Fn(),
{
    while receiver.recv().is_ok() {
        let mut deadline = Instant::now() + QUIET_PERIOD;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match receiver.recv_timeout(remaining) {
                Ok(()) => deadline = Instant::now() + QUIET_PERIOD,
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }

        callback();
    }
}

fn event_affects_markdown(event: &Event) -> bool {
    if matches!(event.kind, EventKind::Access(_)) {
        return false;
    }
    event
        .paths
        .iter()
        .any(|path| path_may_affect_markdown(path))
}

fn path_may_affect_markdown(path: &Path) -> bool {
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        return true;
    }

    let name = path.file_name().and_then(|value| value.to_str());
    if name.is_some_and(|value| value.starts_with('.')) {
        return false;
    }

    path.extension().is_none() // A created, removed, or renamed group folder.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn watches_markdown_and_group_paths_only() {
        assert!(path_may_affect_markdown(Path::new("Draft/chapter.md")));
        assert!(path_may_affect_markdown(Path::new("Draft/chapter.MD")));
        assert!(path_may_affect_markdown(Path::new("Draft/.chapter.md")));
        assert!(path_may_affect_markdown(Path::new("Draft")));
        assert!(!path_may_affect_markdown(Path::new("cover.jpg")));
        assert!(!path_may_affect_markdown(Path::new(
            ".writing-environment-sync"
        )));
        assert!(!path_may_affect_markdown(Path::new("Draft/.temporary")));
    }

    #[test]
    fn native_watcher_coalesces_a_markdown_write_burst() {
        let root = tempfile::tempdir().expect("temporary watched project");
        let draft = root.path().join("Draft");
        fs::create_dir(&draft).expect("draft folder");
        let (sender, receiver) = mpsc::channel();
        let _watch = ProjectWatch::start_with_callback(root.path().to_path_buf(), move || {
            sender.send(()).expect("report coalesced change");
        })
        .expect("start native watcher");

        let sheet = draft.join("chapter.md");
        fs::write(&sheet, "one").expect("first write");
        fs::write(&sheet, "one two").expect("second write");
        fs::write(&sheet, "one two three").expect("third write");

        receiver
            .recv_timeout(Duration::from_secs(4))
            .expect("one coalesced notification");
        assert!(matches!(
            receiver.recv_timeout(Duration::from_millis(750)),
            Err(RecvTimeoutError::Timeout)
        ));
    }
}
