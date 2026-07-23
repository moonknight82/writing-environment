use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use tauri::Manager;
use tempfile::NamedTempFile;
use uuid::Uuid;
use walkdir::WalkDir;

mod project_watch;
mod search_index;
mod sync;

const REVISION_LIMIT: usize = 30;
const EXTERNAL_CHANGE_ERROR: &str =
    "EXTERNAL_CHANGE: This sheet changed on disk while you were writing. Your unsaved text is still open.";

fn appliance_mode_enabled() -> bool {
    std::env::var("WRITING_ENVIRONMENT_APPLIANCE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn enable_native_spell_and_grammar_checking() {
    use objc2_foundation::{ns_string, NSUserDefaults};

    let defaults = NSUserDefaults::standardUserDefaults();
    defaults.setBool_forKey(true, ns_string!("WebContinuousSpellCheckingEnabled"));
    defaults.setBool_forKey(true, ns_string!("WebGrammarCheckingEnabled"));
}

#[cfg(target_os = "linux")]
fn enable_native_spell_checking(window: &tauri::WebviewWindow) -> Result<(), tauri::Error> {
    window.with_webview(|platform_webview| {
        use webkit2gtk::{WebContextExt, WebViewExt};

        if let Some(context) = platform_webview.inner().context() {
            // WebKitGTK requires both an enabled flag and at least one
            // explicitly configured language before HTML spellcheck fields
            // receive native misspelling markers.
            context.set_spell_checking_languages(&["en_US", "pt_BR"]);
            context.set_spell_checking_enabled(true);
        }
    })
}

#[cfg(target_os = "linux")]
fn apply_presentation_mode(
    window: &tauri::WebviewWindow,
    active: bool,
) -> Result<(), tauri::Error> {
    if active {
        // A native Wayland fullscreen surface prevents wf-panel-pi's hidden
        // edge from receiving pointer input. A borderless maximized window is
        // visually fullscreen while still allowing the overlay panel to rise.
        window.set_fullscreen(false)?;
        window.set_decorations(false)?;
        window.maximize()?;
    } else {
        window.set_decorations(true)?;
        window.unmaximize()?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn apply_presentation_mode(
    window: &tauri::WebviewWindow,
    active: bool,
) -> Result<(), tauri::Error> {
    window.set_fullscreen(active)
}

#[cfg(target_os = "linux")]
fn presentation_mode_active(window: &tauri::WebviewWindow) -> Result<bool, tauri::Error> {
    Ok(window.is_maximized()? && !window.is_decorated()?)
}

#[cfg(not(target_os = "linux"))]
fn presentation_mode_active(window: &tauri::WebviewWindow) -> Result<bool, tauri::Error> {
    window.is_fullscreen()
}

#[tauri::command]
fn get_presentation_mode(window: tauri::WebviewWindow) -> Result<bool, String> {
    presentation_mode_active(&window).map_err(|error| error.to_string())
}

#[tauri::command]
fn presentation_uses_borderless_window() -> bool {
    cfg!(target_os = "linux")
}

#[tauri::command]
fn set_presentation_mode(window: tauri::WebviewWindow, active: bool) -> Result<bool, String> {
    apply_presentation_mode(&window, active).map_err(|error| error.to_string())?;
    Ok(active)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LibrarySnapshot {
    name: String,
    path: String,
    sheets: Vec<SheetSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SheetSummary {
    title: String,
    relative_path: String,
    group: String,
    excerpt: String,
    word_count: usize,
    created_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrashItem {
    id: String,
    title: String,
    original_relative_path: String,
    trashed_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RevisionSummary {
    id: String,
    created_at: String,
    word_count: usize,
    word_delta: i64,
    excerpt: String,
}

#[tauri::command]
fn open_library(app: tauri::AppHandle, path: String) -> Result<LibrarySnapshot, String> {
    let root = fs::canonicalize(&path).map_err(|error| format!("Cannot open library: {error}"))?;
    if !root.is_dir() {
        return Err("The selected library is not a folder.".into());
    }

    let sheets = match search_index_path(&app, &root)
        .and_then(|index_path| search_index::synchronize(&index_path, &root))
    {
        Ok(sheets) => sheets,
        Err(error) => {
            eprintln!("Search index unavailable; using a direct library scan: {error}");
            scan_library_sheets(&root)?
        }
    };

    library_snapshot(root, sheets)
}

#[cfg(test)]
fn open_library_unindexed(path: &str) -> Result<LibrarySnapshot, String> {
    let root = fs::canonicalize(path).map_err(|error| format!("Cannot open library: {error}"))?;
    if !root.is_dir() {
        return Err("The selected library is not a folder.".into());
    }
    let sheets = scan_library_sheets(&root)?;
    library_snapshot(root, sheets)
}

fn library_snapshot(root: PathBuf, sheets: Vec<SheetSummary>) -> Result<LibrarySnapshot, String> {
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Markdown Library")
        .to_string();

    Ok(LibrarySnapshot {
        name,
        path: root.to_string_lossy().into_owned(),
        sheets,
    })
}

fn scan_library_sheets(root: &Path) -> Result<Vec<SheetSummary>, String> {
    let mut sheets = Vec::new();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|error| format!("Cannot scan library: {error}"))?;
        let file_path = entry.path();

        if !entry.file_type().is_file() || !is_markdown(file_path) {
            continue;
        }

        let relative = file_path
            .strip_prefix(root)
            .map_err(|_| "A sheet resolved outside the library.".to_string())?;
        let content = fs::read_to_string(file_path)
            .map_err(|error| format!("Cannot read {}: {error}", relative.display()))?;

        sheets.push(summarize_sheet(file_path, relative, &content));
    }

    sheets.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(sheets)
}

fn summarize_sheet(file_path: &Path, relative: &Path, content: &str) -> SheetSummary {
    let body = markdown_body(content);
    SheetSummary {
        title: sheet_title(content, file_path),
        relative_path: path_for_frontend(relative),
        group: sheet_group(relative),
        excerpt: sheet_excerpt(body),
        word_count: body.split_whitespace().count(),
        created_at: sheet_created_at(file_path, content),
    }
}

#[tauri::command]
fn read_sheet(root: String, relative_path: String) -> Result<String, String> {
    let target = resolve_existing_sheet(&root, &relative_path)?;
    fs::read_to_string(&target).map_err(|error| format!("Cannot read sheet: {error}"))
}

#[tauri::command]
fn save_sheet(
    app: tauri::AppHandle,
    root: String,
    relative_path: String,
    content: String,
    expected_content: String,
) -> Result<String, String> {
    let target = resolve_existing_sheet(&root, &relative_path)?;
    let existing = fs::read_to_string(&target)
        .map_err(|error| format!("Cannot read sheet before saving: {error}"))?;
    ensure_expected_sheet_version(&existing, &expected_content)?;
    let created_at = sheet_created_at(&target, &existing);
    let content = if frontmatter_field(&content, "created").is_some() {
        content
    } else {
        set_frontmatter_field(&content, "created", &created_at).unwrap_or(content)
    };
    snapshot_existing_sheet(&app, &target, &root, &relative_path)?;
    atomic_replace(&target, content.as_bytes())?;

    let canonical_root = canonical_library_root(&root)?;
    if let Ok(index_path) = search_index_path(&app, &canonical_root) {
        if let Err(error) = search_index::update_sheet(&index_path, &canonical_root, &target) {
            eprintln!("Saved the sheet, but could not refresh its search entry: {error}");
        }
    }
    Ok(content)
}

fn ensure_expected_sheet_version(existing: &str, expected: &str) -> Result<(), String> {
    if existing == expected {
        Ok(())
    } else {
        Err(EXTERNAL_CHANGE_ERROR.into())
    }
}

#[tauri::command]
fn preserve_local_conflict(
    root: String,
    relative_path: String,
    content: String,
) -> Result<SheetSummary, String> {
    let root = canonical_library_root(&root)?;
    let relative = validated_relative_markdown_path(&relative_path)?;
    let original = root.join(&relative);
    let parent = original
        .parent()
        .ok_or_else(|| "The local draft has no project folder.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Cannot recreate the local draft folder: {error}"))?;

    let stem = original
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("untitled");
    let target = unique_markdown_path(parent, &format!("{stem}-writing-environment-conflict"));
    let title = format!("{} — Local conflict copy", sheet_title(&content, &original));
    let content = set_document_title(&content, &title)?;
    let content = set_document_id(&content, &Uuid::new_v4().to_string())?;
    let content =
        set_frontmatter_field(&content, "created", &Utc::now().to_rfc3339()).unwrap_or(content);
    create_new_file(&target, content.as_bytes())?;
    summarize_existing(&root, &target)
}

#[tauri::command]
fn list_sheet_revisions(
    app: tauri::AppHandle,
    root: String,
    relative_path: String,
) -> Result<Vec<RevisionSummary>, String> {
    let canonical_root = canonical_library_root(&root)?;
    let target = resolve_existing_sheet(&root, &relative_path)?;
    let current = fs::read_to_string(&target)
        .map_err(|error| format!("Cannot read the current sheet: {error}"))?;
    let revision_dir = revision_dir_for(&app, &canonical_root, &relative_path)?;
    list_revisions_from(&revision_dir, &current)
}

#[tauri::command]
fn read_sheet_revision(
    app: tauri::AppHandle,
    root: String,
    relative_path: String,
    revision_id: String,
) -> Result<String, String> {
    let canonical_root = canonical_library_root(&root)?;
    resolve_existing_sheet(&root, &relative_path)?;
    let revision_dir = revision_dir_for(&app, &canonical_root, &relative_path)?;
    read_revision_from(&revision_dir, &revision_id)
}

#[tauri::command]
fn restore_sheet_revision(
    app: tauri::AppHandle,
    root: String,
    relative_path: String,
    revision_id: String,
) -> Result<String, String> {
    let canonical_root = canonical_library_root(&root)?;
    let target = resolve_existing_sheet(&root, &relative_path)?;
    let revision_dir = revision_dir_for(&app, &canonical_root, &relative_path)?;
    restore_revision_from(&target, &revision_dir, &revision_id)
}

#[tauri::command]
fn create_sheet(root: String, group: String, title: String) -> Result<SheetSummary, String> {
    let root = canonical_library_root(&root)?;
    let title = validated_title(&title)?;
    let parent = group_directory(&root, &group)?;
    fs::create_dir_all(&parent).map_err(|error| format!("Cannot create group folder: {error}"))?;

    let target = unique_markdown_path(&parent, &slugify(&title));
    let id = Uuid::new_v4();
    let created_at = Utc::now().to_rfc3339();
    let quoted_title = serde_json::to_string(&title)
        .map_err(|error| format!("Cannot encode sheet title: {error}"))?;
    let content = format!(
        "---\nid: {id}\ntitle: {quoted_title}\ncreated: {created_at}\n---\n\n# {title}\n\n"
    );
    create_new_file(&target, content.as_bytes())?;
    summarize_existing(&root, &target)
}

#[tauri::command]
fn rename_sheet(
    app: tauri::AppHandle,
    root: String,
    relative_path: String,
    title: String,
) -> Result<SheetSummary, String> {
    let canonical_root = canonical_library_root(&root)?;
    let target = resolve_existing_sheet(&root, &relative_path)?;
    let title = validated_title(&title)?;
    let content = fs::read_to_string(&target)
        .map_err(|error| format!("Cannot read sheet before renaming: {error}"))?;
    let updated = set_document_title(&content, &title)?;
    snapshot_existing_sheet(&app, &target, &root, &relative_path)?;

    let parent = target
        .parent()
        .ok_or_else(|| "The sheet has no parent folder.".to_string())?;
    let requested = parent.join(format!("{}.md", slugify(&title)));
    let renamed = if requested == target {
        target.clone()
    } else if requested.exists() {
        unique_markdown_path(parent, &slugify(&title))
    } else {
        requested
    };

    if renamed != target {
        fs::rename(&target, &renamed).map_err(|error| format!("Cannot rename sheet: {error}"))?;
    }
    atomic_replace(&renamed, updated.as_bytes())?;
    let summary = summarize_existing(&canonical_root, &renamed)?;
    migrate_revision_history(
        &app,
        &canonical_root,
        &relative_path,
        &summary.relative_path,
    )?;
    Ok(summary)
}

#[tauri::command]
fn duplicate_sheet(
    root: String,
    relative_path: String,
    title: String,
) -> Result<SheetSummary, String> {
    let canonical_root = canonical_library_root(&root)?;
    let source = resolve_existing_sheet(&root, &relative_path)?;
    let title = validated_title(&title)?;
    let content = fs::read_to_string(&source)
        .map_err(|error| format!("Cannot read sheet before duplicating: {error}"))?;
    let with_title = set_document_title(&content, &title)?;
    let duplicated = set_document_id(&with_title, &Uuid::new_v4().to_string())?;
    let duplicated = set_frontmatter_field(&duplicated, "created", &Utc::now().to_rfc3339())
        .unwrap_or(duplicated);
    let parent = source
        .parent()
        .ok_or_else(|| "The sheet has no parent folder.".to_string())?;
    let target = unique_markdown_path(parent, &slugify(&title));
    create_new_file(&target, duplicated.as_bytes())?;
    summarize_existing(&canonical_root, &target)
}

#[tauri::command]
fn move_sheet(
    app: tauri::AppHandle,
    root: String,
    relative_path: String,
    group: String,
) -> Result<SheetSummary, String> {
    let canonical_root = canonical_library_root(&root)?;
    let summary = move_sheet_on_disk(&root, &relative_path, &group)?;
    migrate_revision_history(
        &app,
        &canonical_root,
        &relative_path,
        &summary.relative_path,
    )?;
    Ok(summary)
}

fn move_sheet_on_disk(
    root: &str,
    relative_path: &str,
    group: &str,
) -> Result<SheetSummary, String> {
    let canonical_root = canonical_library_root(root)?;
    let source = resolve_existing_sheet(root, relative_path)?;
    let parent = group_directory(&canonical_root, group)?;
    fs::create_dir_all(&parent).map_err(|error| format!("Cannot create group folder: {error}"))?;
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("untitled");
    let requested = parent.join(format!("{stem}.md"));
    let target = if requested == source {
        source.clone()
    } else if requested.exists() {
        unique_markdown_path(&parent, stem)
    } else {
        requested
    };

    if target != source {
        let source_parent = source
            .parent()
            .ok_or_else(|| "The sheet has no source folder.".to_string())?
            .to_path_buf();
        fs::rename(&source, &target).map_err(|error| format!("Cannot move sheet: {error}"))?;
        sync_parent(&target)?;
        #[cfg(unix)]
        File::open(&source_parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("Cannot flush source group folder: {error}"))?;
    }
    summarize_existing(&canonical_root, &target)
}

#[tauri::command]
fn trash_sheet(app: tauri::AppHandle, root: String, relative_path: String) -> Result<(), String> {
    let canonical_root = canonical_library_root(&root)?;
    let source = resolve_existing_sheet(&root, &relative_path)?;
    let trash = trash_library_dir(&app, &canonical_root)?;
    trash_sheet_into(&canonical_root, &source, &trash)
}

fn trash_sheet_into(root: &Path, source: &Path, trash: &Path) -> Result<(), String> {
    let content = fs::read_to_string(source)
        .map_err(|error| format!("Cannot read sheet before moving it to Trash: {error}"))?;
    let id = Uuid::new_v4().to_string();
    let item = TrashItem {
        id: id.clone(),
        title: sheet_title(&content, source),
        original_relative_path: path_for_frontend(
            source
                .strip_prefix(root)
                .map_err(|_| "The sheet is outside the selected library.".to_string())?,
        ),
        trashed_at: Utc::now().to_rfc3339(),
    };
    let item_dir = trash.join(&id);
    fs::create_dir_all(&item_dir)
        .map_err(|error| format!("Cannot create Trash folder: {error}"))?;
    fs::copy(source, item_dir.join("sheet.md"))
        .map_err(|error| format!("Cannot copy sheet to Trash: {error}"))?;
    let metadata = serde_json::to_vec_pretty(&item)
        .map_err(|error| format!("Cannot encode Trash metadata: {error}"))?;
    create_new_file(&item_dir.join("metadata.json"), &metadata)?;
    fs::remove_file(source).map_err(|error| format!("Cannot remove original sheet: {error}"))?;
    Ok(())
}

#[tauri::command]
fn list_trash(app: tauri::AppHandle, root: String) -> Result<Vec<TrashItem>, String> {
    let canonical_root = canonical_library_root(&root)?;
    let trash = trash_library_dir(&app, &canonical_root)?;
    list_trash_from(&trash)
}

fn list_trash_from(trash: &Path) -> Result<Vec<TrashItem>, String> {
    if !trash.exists() {
        return Ok(Vec::new());
    }

    let mut items = fs::read_dir(trash)
        .map_err(|error| format!("Cannot inspect Trash: {error}"))?
        .filter_map(Result::ok)
        .filter_map(|entry| fs::read(entry.path().join("metadata.json")).ok())
        .filter_map(|metadata| serde_json::from_slice::<TrashItem>(&metadata).ok())
        .collect::<Vec<_>>();
    items.sort_by(|left, right| right.trashed_at.cmp(&left.trashed_at));
    Ok(items)
}

#[tauri::command]
fn empty_trash(app: tauri::AppHandle, root: String) -> Result<usize, String> {
    let canonical_root = canonical_library_root(&root)?;
    let trash = trash_library_dir(&app, &canonical_root)?;
    empty_trash_from(&trash)
}

fn empty_trash_from(trash: &Path) -> Result<usize, String> {
    if !trash.exists() {
        return Ok(0);
    }

    // Validate the complete set first. This keeps an unexpected file or link from
    // turning a project-scoped cleanup into a partial or overly broad deletion.
    let mut item_dirs = Vec::new();
    for entry in fs::read_dir(trash).map_err(|error| format!("Cannot inspect Trash: {error}"))? {
        let entry = entry.map_err(|error| format!("Cannot inspect a Trash item: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Cannot inspect a Trash item type: {error}"))?;
        let id = entry
            .file_name()
            .into_string()
            .map_err(|_| "Trash contains an unrecognized item; nothing was deleted.".to_string())?;

        if !file_type.is_dir() || file_type.is_symlink() || Uuid::parse_str(&id).is_err() {
            return Err("Trash contains an unrecognized item; nothing was deleted.".to_string());
        }

        let item_dir = entry.path();
        let sheet = item_dir.join("sheet.md");
        let metadata_path = item_dir.join("metadata.json");
        if !sheet.is_file() || !metadata_path.is_file() {
            return Err("Trash contains an incomplete item; nothing was deleted.".to_string());
        }

        let metadata = fs::read(&metadata_path)
            .map_err(|error| format!("Cannot read Trash metadata: {error}"))?;
        let item: TrashItem = serde_json::from_slice(&metadata)
            .map_err(|error| format!("Cannot decode Trash metadata: {error}"))?;
        if item.id != id {
            return Err("Trash contains mismatched metadata; nothing was deleted.".to_string());
        }
        item_dirs.push(item_dir);
    }

    for item_dir in &item_dirs {
        fs::remove_dir_all(item_dir)
            .map_err(|error| format!("Cannot permanently remove a Trash item: {error}"))?;
    }

    #[cfg(unix)]
    File::open(trash)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("Cannot flush Trash folder: {error}"))?;

    Ok(item_dirs.len())
}

#[tauri::command]
fn restore_trash_item(
    app: tauri::AppHandle,
    root: String,
    trash_id: String,
) -> Result<SheetSummary, String> {
    let canonical_root = canonical_library_root(&root)?;
    let trash = trash_library_dir(&app, &canonical_root)?;
    restore_trash_from(&canonical_root, &trash, &trash_id)
}

fn restore_trash_from(root: &Path, trash: &Path, trash_id: &str) -> Result<SheetSummary, String> {
    Uuid::parse_str(trash_id).map_err(|_| "The Trash item identifier is invalid.".to_string())?;
    let item_dir = trash.join(trash_id);
    let metadata = fs::read(item_dir.join("metadata.json"))
        .map_err(|error| format!("Cannot read Trash metadata: {error}"))?;
    let item: TrashItem = serde_json::from_slice(&metadata)
        .map_err(|error| format!("Cannot decode Trash metadata: {error}"))?;
    let relative = validated_relative_markdown_path(&item.original_relative_path)?;
    let requested = root.join(&relative);
    let parent = requested
        .parent()
        .ok_or_else(|| "The restored sheet has no parent folder.".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("Cannot recreate group folder: {error}"))?;
    let target = if requested.exists() {
        let stem = requested
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("restored");
        unique_markdown_path(parent, &format!("{stem}-restored"))
    } else {
        requested
    };

    fs::copy(item_dir.join("sheet.md"), &target)
        .map_err(|error| format!("Cannot restore sheet: {error}"))?;
    sync_file_and_parent(&target)?;
    fs::remove_dir_all(&item_dir)
        .map_err(|error| format!("Cannot remove restored Trash item: {error}"))?;
    summarize_existing(root, &target)
}

#[tauri::command]
fn search_library(
    app: tauri::AppHandle,
    root: String,
    query: String,
) -> Result<Vec<SheetSummary>, String> {
    let root = canonical_library_root(&root)?;
    if query.split_whitespace().next().is_none() {
        return Ok(Vec::new());
    }

    match search_index_path(&app, &root) {
        Ok(index_path) => {
            if !index_path.exists() {
                if let Err(error) = search_index::synchronize(&index_path, &root) {
                    eprintln!("Could not prepare the search index: {error}");
                }
            }
            match search_index::search(&index_path, &query) {
                Ok(results) => return Ok(results),
                Err(error) => {
                    eprintln!("Indexed search unavailable; using a direct scan: {error}");
                }
            }
        }
        Err(error) => {
            eprintln!("Cannot locate the search index; using a direct scan: {error}");
        }
    }

    search_library_by_scan(&root, &query)
}

fn search_library_by_scan(root: &Path, query: &str) -> Result<Vec<SheetSummary>, String> {
    let terms = query
        .split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|error| format!("Cannot search library: {error}"))?;
        if !entry.file_type().is_file() || !is_markdown(entry.path()) {
            continue;
        }
        let content = fs::read_to_string(entry.path())
            .map_err(|error| format!("Cannot search {}: {error}", entry.path().display()))?;
        let searchable =
            format!("{}\n{}", sheet_title(&content, entry.path()), content).to_lowercase();
        if !terms.iter().all(|term| searchable.contains(term)) {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| "A search result resolved outside the library.".to_string())?;
        let mut summary = summarize_sheet(entry.path(), relative, &content);
        summary.excerpt = search_excerpt(markdown_body(&content), &terms[0]);
        results.push(summary);
        if results.len() >= 200 {
            break;
        }
    }
    results.sort_by(|left, right| left.title.cmp(&right.title));
    Ok(results)
}

fn resolve_existing_sheet(root: &str, relative_path: &str) -> Result<PathBuf, String> {
    let root = canonical_library_root(root)?;
    let relative = validated_relative_markdown_path(relative_path)?;

    let target = fs::canonicalize(root.join(&relative))
        .map_err(|error| format!("Cannot resolve sheet: {error}"))?;

    if !target.starts_with(&root) || !target.is_file() {
        return Err("The sheet is outside the selected library.".into());
    }

    Ok(target)
}

fn canonical_library_root(root: &str) -> Result<PathBuf, String> {
    let root =
        fs::canonicalize(root).map_err(|error| format!("Cannot resolve library: {error}"))?;
    if !root.is_dir() {
        return Err("The selected library is not a folder.".into());
    }
    Ok(root)
}

fn validated_relative_markdown_path(relative_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(relative_path);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        || !is_markdown(relative)
    {
        return Err("The sheet path is invalid.".into());
    }
    Ok(relative.to_path_buf())
}

fn validated_title(title: &str) -> Result<String, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("A sheet title is required.".into());
    }
    if title.chars().count() > 200 || title.contains(['\n', '\r']) {
        return Err("The sheet title is too long or contains a line break.".into());
    }
    Ok(title.to_string())
}

fn group_directory(root: &Path, group: &str) -> Result<PathBuf, String> {
    let group = group.trim();
    if group.is_empty() || group == "Ungrouped" {
        return Ok(root.to_path_buf());
    }
    let relative = Path::new(group);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(name) if !name.is_empty()))
    {
        return Err("The project folder path must stay inside the project.".into());
    }
    Ok(root.join(relative))
}

fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut separator_pending = false;
    for character in title.chars() {
        if character.is_alphanumeric() {
            if separator_pending && !slug.is_empty() {
                slug.push('-');
            }
            slug.extend(character.to_lowercase());
            separator_pending = false;
        } else {
            separator_pending = true;
        }
    }
    if slug.is_empty() {
        "untitled".to_string()
    } else {
        slug
    }
}

fn unique_markdown_path(parent: &Path, stem: &str) -> PathBuf {
    let first = parent.join(format!("{stem}.md"));
    if !first.exists() {
        return first;
    }
    for suffix in 2..10000 {
        let candidate = parent.join(format!("{stem}-{suffix}.md"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem}-{}.md", Uuid::new_v4()))
}

fn create_new_file(target: &Path, content: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(|error| format!("Cannot create sheet: {error}"))?;
    file.write_all(content)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("Cannot flush new sheet: {error}"))?;
    sync_parent(target)
}

fn sync_file_and_parent(target: &Path) -> Result<(), String> {
    File::open(target)
        .and_then(|file| file.sync_all())
        .map_err(|error| format!("Cannot flush restored sheet: {error}"))?;
    sync_parent(target)
}

fn sync_parent(target: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        let parent = target
            .parent()
            .ok_or_else(|| "The sheet has no parent folder.".to_string())?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("Cannot flush manuscript folder: {error}"))?;
    }
    Ok(())
}

fn summarize_existing(root: &Path, target: &Path) -> Result<SheetSummary, String> {
    if !target.starts_with(root) {
        return Err("The sheet is outside the selected library.".into());
    }
    let relative = target
        .strip_prefix(root)
        .map_err(|_| "The sheet is outside the selected library.".to_string())?;
    let content = fs::read_to_string(target)
        .map_err(|error| format!("Cannot read sheet metadata: {error}"))?;
    Ok(summarize_sheet(target, relative, &content))
}

fn set_document_title(content: &str, title: &str) -> Result<String, String> {
    let quoted = serde_json::to_string(title)
        .map_err(|error| format!("Cannot encode sheet title: {error}"))?;
    let has_frontmatter = content.starts_with("---\n") || content.starts_with("---\r\n");
    let titled =
        set_frontmatter_field(content, "title", &quoted).unwrap_or_else(|| content.to_string());
    let mut updated = replace_first_heading(&titled, title).unwrap_or_else(|| {
        if has_frontmatter {
            titled
        } else {
            format!("# {title}\n\n{content}")
        }
    });
    if content.ends_with('\n') && !updated.ends_with('\n') {
        updated.push('\n');
    }
    Ok(updated)
}

fn replace_first_heading(content: &str, title: &str) -> Option<String> {
    let mut replaced = false;
    let mut updated = content
        .lines()
        .map(|line| {
            if !replaced && line.trim_start().starts_with("# ") {
                replaced = true;
                format!("# {title}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if content.ends_with('\n') {
        updated.push('\n');
    }
    replaced.then_some(updated)
}

fn set_document_id(content: &str, id: &str) -> Result<String, String> {
    Ok(set_frontmatter_field(content, "id", id)
        .unwrap_or_else(|| format!("---\nid: {id}\n---\n\n{content}")))
}

fn set_frontmatter_field(content: &str, field: &str, value: &str) -> Option<String> {
    if !content.starts_with("---\n") && !content.starts_with("---\r\n") {
        return None;
    }
    let newline = if content.starts_with("---\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let lines = content.split_inclusive('\n').collect::<Vec<_>>();
    let closing = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.trim() == "---")
        .map(|(index, _)| index)?;
    let prefix = format!("{field}:");
    let existing = lines
        .iter()
        .enumerate()
        .take(closing)
        .skip(1)
        .find(|(_, line)| line.trim_start().starts_with(&prefix))
        .map(|(index, _)| index);
    let replacement = format!("{field}: {value}{newline}");
    let mut output = String::new();
    for (index, line) in lines.iter().enumerate() {
        if existing == Some(index) {
            output.push_str(&replacement);
        } else if existing.is_none() && index == closing {
            output.push_str(&replacement);
            output.push_str(line);
        } else {
            output.push_str(line);
        }
    }
    Some(output)
}

fn trash_library_dir(app: &tauri::AppHandle, root: &Path) -> Result<PathBuf, String> {
    let key = blake3::hash(root.to_string_lossy().as_bytes()).to_hex();
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Cannot locate application data: {error}"))?
        .join("trash")
        .join(key.as_str()))
}

fn search_index_path(app: &tauri::AppHandle, root: &Path) -> Result<PathBuf, String> {
    let key = blake3::hash(root.to_string_lossy().as_bytes()).to_hex();
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Cannot locate application data: {error}"))?
        .join("search-indexes")
        .join(key.as_str())
        .join("library.sqlite3"))
}

fn snapshot_existing_sheet(
    app: &tauri::AppHandle,
    target: &Path,
    root: &str,
    relative_path: &str,
) -> Result<(), String> {
    let canonical_root = canonical_library_root(root)?;
    let revision_dir = revision_dir_for(app, &canonical_root, relative_path)?;
    snapshot_to_dir(target, &revision_dir)
}

fn revision_dir_for(
    app: &tauri::AppHandle,
    root: &Path,
    relative_path: &str,
) -> Result<PathBuf, String> {
    validated_relative_markdown_path(relative_path)?;
    let revision_key =
        blake3::hash(format!("{}\0{relative_path}", root.to_string_lossy()).as_bytes()).to_hex();
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Cannot locate application data: {error}"))?
        .join("revisions")
        .join(revision_key.as_str()))
}

fn migrate_revision_history(
    app: &tauri::AppHandle,
    root: &Path,
    old_relative_path: &str,
    new_relative_path: &str,
) -> Result<(), String> {
    if old_relative_path == new_relative_path {
        return Ok(());
    }
    let old_dir = revision_dir_for(app, root, old_relative_path)?;
    if !old_dir.exists() {
        return Ok(());
    }
    let new_dir = revision_dir_for(app, root, new_relative_path)?;
    if !new_dir.exists() {
        fs::rename(&old_dir, &new_dir)
            .map_err(|error| format!("Cannot carry History to the renamed sheet: {error}"))?;
        return Ok(());
    }

    fs::create_dir_all(&new_dir)
        .map_err(|error| format!("Cannot prepare renamed sheet History: {error}"))?;
    for entry in fs::read_dir(&old_dir)
        .map_err(|error| format!("Cannot inspect the previous History folder: {error}"))?
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        let requested = new_dir.join(entry.file_name());
        let destination = if requested.exists() {
            new_dir.join(format!("migrated-{}.md", Uuid::new_v4()))
        } else {
            requested
        };
        fs::rename(entry.path(), destination)
            .map_err(|error| format!("Cannot carry a revision to the renamed sheet: {error}"))?;
    }
    fs::remove_dir(&old_dir)
        .map_err(|error| format!("Cannot finish moving sheet History: {error}"))?;
    trim_revisions(&new_dir)
}

fn snapshot_to_dir(target: &Path, revision_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(revision_dir)
        .map_err(|error| format!("Cannot create revision folder: {error}"))?;

    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let revision_path = revision_dir.join(format!("{timestamp}-{}.md", Uuid::new_v4()));
    fs::copy(target, &revision_path)
        .map_err(|error| format!("Cannot create recovery snapshot: {error}"))?;

    trim_revisions(revision_dir)
}

fn list_revisions_from(
    revision_dir: &Path,
    current_content: &str,
) -> Result<Vec<RevisionSummary>, String> {
    if !revision_dir.exists() {
        return Ok(Vec::new());
    }
    let current_words = markdown_body(current_content).split_whitespace().count() as i64;
    let mut revisions = fs::read_dir(revision_dir)
        .map_err(|error| format!("Cannot inspect revisions: {error}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let id = entry.file_name().to_str()?.to_string();
            validated_revision_path(revision_dir, &id).ok()?;
            let content = fs::read_to_string(entry.path()).ok()?;
            let body = markdown_body(&content);
            let word_count = body.split_whitespace().count();
            Some(RevisionSummary {
                created_at: revision_created_at(&id, &entry).to_rfc3339(),
                word_delta: word_count as i64 - current_words,
                word_count,
                excerpt: sheet_excerpt(body),
                id,
            })
        })
        .collect::<Vec<_>>();
    revisions.sort_by(|left, right| right.id.cmp(&left.id));
    Ok(revisions)
}

fn revision_created_at(id: &str, entry: &fs::DirEntry) -> DateTime<Utc> {
    let timestamp = id
        .strip_suffix(".md")
        .unwrap_or(id)
        .split('-')
        .next()
        .unwrap_or_default();
    if let Ok(parsed) = NaiveDateTime::parse_from_str(timestamp, "%Y%m%dT%H%M%S%.3fZ") {
        return DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc);
    }
    entry
        .metadata()
        .and_then(|metadata| metadata.modified())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now())
}

fn validated_revision_path(revision_dir: &Path, revision_id: &str) -> Result<PathBuf, String> {
    let candidate = Path::new(revision_id);
    if candidate.components().count() != 1
        || !matches!(candidate.components().next(), Some(Component::Normal(_)))
        || !is_markdown(candidate)
    {
        return Err("The revision identifier is invalid.".into());
    }
    Ok(revision_dir.join(candidate))
}

fn read_revision_from(revision_dir: &Path, revision_id: &str) -> Result<String, String> {
    let revision = validated_revision_path(revision_dir, revision_id)?;
    if !revision.is_file() {
        return Err("That revision is no longer available.".into());
    }
    fs::read_to_string(revision).map_err(|error| format!("Cannot read revision: {error}"))
}

fn restore_revision_from(
    target: &Path,
    revision_dir: &Path,
    revision_id: &str,
) -> Result<String, String> {
    let revision_content = read_revision_from(revision_dir, revision_id)?;
    snapshot_to_dir(target, revision_dir)?;
    atomic_replace(target, revision_content.as_bytes())?;
    Ok(revision_content)
}

fn trim_revisions(revision_dir: &Path) -> Result<(), String> {
    let mut revisions = fs::read_dir(revision_dir)
        .map_err(|error| format!("Cannot inspect revisions: {error}"))?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "md")
        })
        .collect::<Vec<_>>();

    revisions.sort_by_key(|entry| entry.file_name());
    let remove_count = revisions.len().saturating_sub(REVISION_LIMIT);

    for revision in revisions.into_iter().take(remove_count) {
        fs::remove_file(revision.path())
            .map_err(|error| format!("Cannot prune old revision: {error}"))?;
    }

    Ok(())
}

fn atomic_replace(target: &Path, content: &[u8]) -> Result<(), String> {
    atomic_replace_with_hook(target, content, |_| Ok(()))
}

fn atomic_replace_with_hook<F>(
    target: &Path,
    content: &[u8],
    before_persist: F,
) -> Result<(), String>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    let parent = target
        .parent()
        .ok_or_else(|| "The sheet has no parent folder.".to_string())?;
    let permissions = fs::metadata(target)
        .map_err(|error| format!("Cannot inspect existing sheet: {error}"))?
        .permissions();
    let mut temporary = NamedTempFile::new_in(parent)
        .map_err(|error| format!("Cannot create temporary sheet: {error}"))?;

    temporary
        .write_all(content)
        .and_then(|_| temporary.as_file_mut().set_permissions(permissions))
        .and_then(|_| temporary.as_file_mut().sync_all())
        .map_err(|error| format!("Cannot flush temporary sheet: {error}"))?;

    // Keeping this boundary injectable makes the most important crash rule
    // deterministic to test: until persist succeeds, the original remains the
    // only authoritative file and the complete temporary copy is disposable.
    before_persist(temporary.path())?;

    temporary
        .persist(target)
        .map_err(|error| format!("Cannot replace sheet: {}", error.error))?;

    #[cfg(unix)]
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("Cannot flush manuscript folder: {error}"))?;

    Ok(())
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

fn markdown_body(content: &str) -> &str {
    if !content.starts_with("---\n") && !content.starts_with("---\r\n") {
        return content;
    }

    let mut offset = 0;
    for (index, line) in content.split_inclusive('\n').enumerate() {
        offset += line.len();
        if index > 0 && line.trim() == "---" {
            return &content[offset..];
        }
    }

    content
}

fn sheet_title(content: &str, path: &Path) -> String {
    if content.starts_with("---\n") || content.starts_with("---\r\n") {
        for line in content.lines().skip(1) {
            if line.trim() == "---" {
                break;
            }
            if let Some(title) = line.trim().strip_prefix("title:") {
                let title = title.trim().trim_matches(['\'', '"']);
                if !title.is_empty() {
                    return title.to_string();
                }
            }
        }
    }

    if let Some(heading) = markdown_body(content)
        .lines()
        .find_map(|line| line.trim().strip_prefix("# "))
    {
        if !heading.trim().is_empty() {
            return heading.trim().to_string();
        }
    }

    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Untitled")
        .replace(['-', '_'], " ")
}

fn sheet_created_at(path: &Path, content: &str) -> String {
    if let Some(created) = frontmatter_field(content, "created") {
        if let Ok(timestamp) = DateTime::parse_from_rfc3339(&created) {
            return timestamp.with_timezone(&Utc).to_rfc3339();
        }
    }

    path.metadata()
        .ok()
        .and_then(|metadata| metadata.created().or_else(|_| metadata.modified()).ok())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(Utc::now)
        .to_rfc3339()
}

fn frontmatter_field(content: &str, field: &str) -> Option<String> {
    if !content.starts_with("---\n") && !content.starts_with("---\r\n") {
        return None;
    }

    let prefix = format!("{field}:");
    content
        .lines()
        .skip(1)
        .take_while(|line| line.trim() != "---")
        .find_map(|line| {
            line.trim()
                .strip_prefix(&prefix)
                .map(|value| value.trim().trim_matches(['\'', '"']).to_string())
                .filter(|value| !value.is_empty())
        })
}

fn sheet_group(relative: &Path) -> String {
    relative
        .parent()
        .and_then(|parent| parent.components().next())
        .and_then(|component| match component {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .unwrap_or("Ungrouped")
        .to_string()
}

fn sheet_excerpt(body: &str) -> String {
    let text = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join(" ");

    let mut excerpt = text.chars().take(160).collect::<String>();
    if text.chars().count() > 160 {
        excerpt.push('…');
    }
    excerpt
}

fn search_excerpt(body: &str, term: &str) -> String {
    let matching_line = body
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && line.to_lowercase().contains(term));
    let source = matching_line.unwrap_or(body);
    let mut excerpt = source.chars().take(160).collect::<String>();
    if source.chars().count() > 160 {
        excerpt.push('…');
    }
    excerpt
}

fn path_for_frontend(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "macos")]
    enable_native_spell_and_grammar_checking();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(project_watch::ProjectWatchState::default())
        .setup(|app| {
            #[cfg(target_os = "linux")]
            if let Some(window) = app.get_webview_window("main") {
                enable_native_spell_checking(&window)?;
            }

            if appliance_mode_enabled() {
                if let Some(window) = app.get_webview_window("main") {
                    apply_presentation_mode(&window, true)?;
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_presentation_mode,
            presentation_uses_borderless_window,
            set_presentation_mode,
            open_library,
            read_sheet,
            save_sheet,
            preserve_local_conflict,
            list_sheet_revisions,
            read_sheet_revision,
            restore_sheet_revision,
            create_sheet,
            rename_sheet,
            duplicate_sheet,
            move_sheet,
            trash_sheet,
            list_trash,
            empty_trash,
            restore_trash_item,
            search_library,
            project_watch::watch_library,
            project_watch::unwatch_library,
            sync::sync_availability,
            sync::sync_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running Writing Environment");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_title_wins() {
        let content = "---\ntitle: The Real Title\n---\n# Other Title\nBody";
        assert_eq!(
            sheet_title(content, Path::new("fallback.md")),
            "The Real Title"
        );
    }

    #[test]
    fn excerpt_ignores_headings_and_limits_length() {
        let body = format!("# Heading\n\n{}", "word ".repeat(50));
        let excerpt = sheet_excerpt(&body);
        assert!(!excerpt.contains("Heading"));
        assert!(excerpt.chars().count() <= 161);
    }

    #[test]
    fn invalid_relative_paths_are_rejected() {
        let root = tempfile::tempdir().expect("temporary library");
        assert!(
            resolve_existing_sheet(root.path().to_str().expect("root path"), "../outside.md")
                .is_err()
        );
    }

    #[test]
    fn library_scan_extracts_portable_sheet_metadata() {
        let root = tempfile::tempdir().expect("temporary library");
        let draft = root.path().join("Draft");
        fs::create_dir(&draft).expect("draft folder");
        fs::write(
            draft.join("arrival.md"),
            "---\ntitle: The Arrival\ncreated: 2026-07-18T14:20:00-03:00\n---\n\n# Ignored Heading\n\nFirst paragraph.",
        )
        .expect("test sheet");

        let library = open_library_unindexed(&root.path().to_string_lossy()).expect("scan library");
        assert_eq!(library.sheets.len(), 1);
        assert_eq!(library.sheets[0].title, "The Arrival");
        assert_eq!(library.sheets[0].group, "Draft");
        assert_eq!(library.sheets[0].relative_path, "Draft/arrival.md");
        assert_eq!(library.sheets[0].excerpt, "First paragraph.");
        assert_eq!(library.sheets[0].created_at, "2026-07-18T17:20:00+00:00");
    }

    #[test]
    fn reading_a_selected_sheet_returns_its_own_contents() {
        let root = tempfile::tempdir().expect("temporary library");
        let draft = root.path().join("Draft");
        fs::create_dir(&draft).expect("draft folder");
        fs::write(draft.join("first.md"), "# First\n\nFirst manuscript.").expect("first sheet");
        fs::write(draft.join("second.md"), "# Second\n\nSecond manuscript.").expect("second sheet");
        let root_string = root.path().to_string_lossy().into_owned();

        assert_eq!(
            read_sheet(root_string.clone(), "Draft/first.md".into()).expect("read first sheet"),
            "# First\n\nFirst manuscript."
        );
        assert_eq!(
            read_sheet(root_string, "Draft/second.md".into()).expect("read second sheet"),
            "# Second\n\nSecond manuscript."
        );
    }

    #[test]
    fn atomic_replace_keeps_a_complete_file() {
        let root = tempfile::tempdir().expect("temporary library");
        let sheet = root.path().join("sheet.md");
        fs::write(&sheet, "old manuscript").expect("original sheet");

        atomic_replace(&sheet, b"new manuscript").expect("atomic save");

        assert_eq!(
            fs::read_to_string(sheet).expect("saved sheet"),
            "new manuscript"
        );
    }

    #[test]
    fn interrupted_atomic_replace_keeps_original_and_discards_temporary_file() {
        let root = tempfile::tempdir().expect("temporary library");
        let sheet = root.path().join("sheet.md");
        fs::write(&sheet, "known good manuscript").expect("original sheet");

        let error = atomic_replace_with_hook(&sheet, b"complete new manuscript", |temporary| {
            assert_eq!(
                fs::read_to_string(temporary).expect("flushed temporary copy"),
                "complete new manuscript"
            );
            assert_eq!(
                fs::read_to_string(&sheet).expect("original before commit"),
                "known good manuscript"
            );
            Err("simulated interruption before atomic rename".into())
        })
        .expect_err("injected interruption");

        assert!(error.contains("simulated interruption"));
        assert_eq!(
            fs::read_to_string(&sheet).expect("original after interruption"),
            "known good manuscript"
        );
        let remaining = fs::read_dir(root.path())
            .expect("library directory")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect::<Vec<_>>();
        assert_eq!(remaining, vec![sheet.file_name().unwrap().to_os_string()]);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_preserves_existing_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let root = tempfile::tempdir().expect("temporary library");
        let sheet = root.path().join("sheet.md");
        fs::write(&sheet, "old").expect("original sheet");
        fs::set_permissions(&sheet, fs::Permissions::from_mode(0o640))
            .expect("set original permissions");

        atomic_replace(&sheet, b"new").expect("atomic save");

        assert_eq!(
            fs::metadata(&sheet)
                .expect("saved metadata")
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
    }

    #[test]
    fn save_version_check_refuses_external_changes() {
        assert!(ensure_expected_sheet_version("same", "same").is_ok());
        let error = ensure_expected_sheet_version("changed on disk", "previous version")
            .expect_err("external change must stop the save");
        assert_eq!(error, EXTERNAL_CHANGE_ERROR);
    }

    #[test]
    fn local_conflict_copy_preserves_both_versions() {
        let root = tempfile::tempdir().expect("temporary library");
        let draft = root.path().join("Draft");
        fs::create_dir(&draft).expect("draft folder");
        let original = draft.join("chapter.md");
        fs::write(&original, "# Chapter\n\nExternal version").expect("external version");

        let preserved = preserve_local_conflict(
            root.path().to_string_lossy().into_owned(),
            "Draft/chapter.md".into(),
            "# Chapter\n\nUnsaved local version".into(),
        )
        .expect("preserve local conflict");

        assert!(preserved
            .relative_path
            .contains("writing-environment-conflict"));
        assert_eq!(
            fs::read_to_string(&original).expect("original remains"),
            "# Chapter\n\nExternal version"
        );
        let local_copy = fs::read_to_string(root.path().join(&preserved.relative_path))
            .expect("local conflict copy");
        assert!(local_copy.contains("Unsaved local version"));
        assert!(local_copy.contains("Local conflict copy"));
        assert!(local_copy.contains("id: "));
    }

    #[test]
    fn create_and_duplicate_sheet_assign_distinct_ids() {
        let root = tempfile::tempdir().expect("temporary library");
        let root_string = root.path().to_string_lossy().into_owned();
        let created = create_sheet(root_string.clone(), "Draft".into(), "New Arrival".into())
            .expect("create sheet");
        let duplicated = duplicate_sheet(
            root_string.clone(),
            created.relative_path.clone(),
            "Copy of New Arrival".into(),
        )
        .expect("duplicate sheet");

        let original =
            fs::read_to_string(root.path().join(created.relative_path)).expect("original content");
        let copy = fs::read_to_string(root.path().join(&duplicated.relative_path))
            .expect("duplicated content");
        let original_id = original.lines().find(|line| line.starts_with("id: "));
        let copy_id = copy.lines().find(|line| line.starts_with("id: "));
        assert!(original_id.is_some());
        assert!(copy_id.is_some());
        assert_ne!(original_id, copy_id);
        assert!(copy.contains("title: \"Copy of New Arrival\""));

        let moved = move_sheet_on_disk(&root_string, &duplicated.relative_path, "Archive")
            .expect("move duplicated sheet");
        assert_eq!(moved.group, "Archive");
        assert!(root.path().join(moved.relative_path).is_file());
    }

    #[test]
    fn title_update_preserves_unknown_frontmatter() {
        let original = "---\nid: stable-id\ntags: [sea, rain]\ntitle: Old\n---\n\n# Old\n\nBody\n";
        let updated = set_document_title(original, "New Title").expect("update title");
        assert!(updated.contains("id: stable-id"));
        assert!(updated.contains("tags: [sea, rain]"));
        assert!(updated.contains("title: \"New Title\""));
        assert!(updated.contains("# New Title"));
        assert!(updated.ends_with("Body\n"));
    }

    #[test]
    fn search_finds_terms_outside_the_excerpt() {
        let root = tempfile::tempdir().expect("temporary library");
        fs::write(
            root.path().join("notes.md"),
            "# Notes\n\nOpening paragraph.\n\nThe lighthouse code is hidden here.",
        )
        .expect("test sheet");

        let results =
            search_library_by_scan(root.path(), "lighthouse code").expect("search library");
        assert_eq!(results.len(), 1);
        assert!(results[0].excerpt.contains("lighthouse"));
    }

    #[test]
    fn nested_project_folders_are_allowed_without_path_escape() {
        let root = tempfile::tempdir().expect("temporary library");
        assert_eq!(
            group_directory(root.path(), "Research/Locations").expect("nested folder"),
            root.path().join("Research").join("Locations")
        );
        assert!(group_directory(root.path(), "../Outside").is_err());
        assert!(group_directory(root.path(), "/Outside").is_err());

        let root_string = root.path().to_string_lossy().into_owned();
        let created = create_sheet(
            root_string.clone(),
            "Research/Locations".into(),
            "Harbor Road".into(),
        )
        .expect("create nested sheet");
        assert_eq!(created.relative_path, "Research/Locations/harbor-road.md");

        let moved = move_sheet_on_disk(&root_string, &created.relative_path, "Draft/Chapter One")
            .expect("move between nested folders");
        assert_eq!(moved.relative_path, "Draft/Chapter One/harbor-road.md");
        assert!(root.path().join(&moved.relative_path).is_file());
    }

    #[test]
    fn trash_round_trip_restores_the_original_sheet() {
        let root = tempfile::tempdir().expect("temporary library");
        let trash = tempfile::tempdir().expect("temporary trash");
        let root_string = root.path().to_string_lossy().into_owned();
        let created =
            create_sheet(root_string, "Draft".into(), "Recover Me".into()).expect("create sheet");
        let source = root.path().join(&created.relative_path);

        trash_sheet_into(root.path(), &source, trash.path()).expect("trash sheet");
        assert!(!source.exists());
        let items = list_trash_from(trash.path()).expect("list trash");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].original_relative_path, created.relative_path);

        let restored =
            restore_trash_from(root.path(), trash.path(), &items[0].id).expect("restore sheet");
        assert_eq!(restored.title, "Recover Me");
        assert!(root.path().join(restored.relative_path).is_file());
        assert!(list_trash_from(trash.path())
            .expect("empty trash")
            .is_empty());
    }

    #[test]
    fn empty_trash_removes_all_valid_items() {
        let root = tempfile::tempdir().expect("temporary library");
        let trash = tempfile::tempdir().expect("temporary trash");
        let root_string = root.path().to_string_lossy().into_owned();
        let first = create_sheet(root_string.clone(), "Draft".into(), "First".into())
            .expect("create first sheet");
        let second = create_sheet(root_string, "Draft".into(), "Second".into())
            .expect("create second sheet");

        trash_sheet_into(
            root.path(),
            &root.path().join(first.relative_path),
            trash.path(),
        )
        .expect("trash first sheet");
        trash_sheet_into(
            root.path(),
            &root.path().join(second.relative_path),
            trash.path(),
        )
        .expect("trash second sheet");

        assert_eq!(empty_trash_from(trash.path()).expect("empty trash"), 2);
        assert!(list_trash_from(trash.path())
            .expect("list empty trash")
            .is_empty());
    }

    #[test]
    fn empty_trash_refuses_unrecognized_entries_before_deleting() {
        let root = tempfile::tempdir().expect("temporary library");
        let trash = tempfile::tempdir().expect("temporary trash");
        let root_string = root.path().to_string_lossy().into_owned();
        let created =
            create_sheet(root_string, "Draft".into(), "Keep Safe".into()).expect("create sheet");
        trash_sheet_into(
            root.path(),
            &root.path().join(created.relative_path),
            trash.path(),
        )
        .expect("trash sheet");
        fs::write(trash.path().join("unexpected-file"), "do not delete").expect("unexpected file");

        assert!(empty_trash_from(trash.path()).is_err());
        assert_eq!(list_trash_from(trash.path()).expect("list trash").len(), 1);
        assert!(trash.path().join("unexpected-file").is_file());
    }

    #[test]
    fn revision_listing_is_newest_first_with_word_deltas() {
        let revisions = tempfile::tempdir().expect("temporary revisions");
        fs::write(
            revisions.path().join("20260720T120000.000Z-old.md"),
            "# Draft\n\none two",
        )
        .expect("old revision");
        fs::write(
            revisions.path().join("20260720T130000.000Z-new.md"),
            "# Draft\n\none two three four",
        )
        .expect("new revision");

        let listed = list_revisions_from(revisions.path(), "# Draft\n\none two three")
            .expect("list revisions");
        assert_eq!(listed.len(), 2);
        assert!(listed[0].id.contains("130000"));
        assert_eq!(listed[0].word_count, 6);
        assert_eq!(listed[0].word_delta, 1);
        assert_eq!(listed[1].word_delta, -1);
    }

    #[test]
    fn revision_restore_snapshots_the_version_it_replaces() {
        let root = tempfile::tempdir().expect("temporary sheet");
        let revisions = tempfile::tempdir().expect("temporary revisions");
        let sheet = root.path().join("draft.md");
        let revision_id = "20260720T120000.000Z-history.md";
        fs::write(&sheet, "# Current\n\nCurrent text").expect("current sheet");
        fs::write(
            revisions.path().join(revision_id),
            "# Earlier\n\nEarlier text",
        )
        .expect("revision");

        let restored =
            restore_revision_from(&sheet, revisions.path(), revision_id).expect("restore revision");
        assert_eq!(restored, "# Earlier\n\nEarlier text");
        assert_eq!(
            fs::read_to_string(&sheet).expect("restored sheet"),
            restored
        );
        let snapshots = fs::read_dir(revisions.path())
            .expect("revision directory")
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        assert_eq!(snapshots.len(), 2);
        assert!(snapshots.iter().any(|entry| {
            fs::read_to_string(entry.path()).ok().as_deref() == Some("# Current\n\nCurrent text")
        }));
    }

    #[test]
    fn revision_ids_cannot_escape_the_history_folder() {
        let revisions = tempfile::tempdir().expect("temporary revisions");
        assert!(validated_revision_path(revisions.path(), "../outside.md").is_err());
        assert!(validated_revision_path(revisions.path(), "nested/revision.md").is_err());
        assert!(validated_revision_path(revisions.path(), "revision.txt").is_err());
    }
}
