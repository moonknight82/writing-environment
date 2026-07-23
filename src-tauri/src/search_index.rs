use super::{
    invalid_utf8_warning, is_markdown, markdown_body, path_for_frontend, summarize_sheet,
    LibraryScan, SheetSummary,
};
use rusqlite::{params, Connection, Statement, Transaction};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use walkdir::WalkDir;

const SCHEMA_VERSION: i64 = 1;
const RESULT_LIMIT: i64 = 200;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileState {
    modified_ns: i64,
    size: i64,
}

pub(super) fn synchronize(index_path: &Path, root: &Path) -> Result<LibraryScan, String> {
    match synchronize_once(index_path, root) {
        Ok(sheets) => Ok(sheets),
        Err(first_error) => {
            remove_database_files(index_path)?;
            synchronize_once(index_path, root).map_err(|second_error| {
                format!(
                    "Cannot rebuild the library search index: {second_error} (initial error: {first_error})"
                )
            })
        }
    }
}

pub(super) fn update_sheet(index_path: &Path, root: &Path, target: &Path) -> Result<(), String> {
    if !index_path.exists() {
        return Ok(());
    }

    let relative = target
        .strip_prefix(root)
        .map_err(|_| "The indexed sheet is outside the selected library.".to_string())?;
    let content = fs::read_to_string(target)
        .map_err(|error| format!("Cannot read {} for indexing: {error}", relative.display()))?;
    let metadata = fs::metadata(target).map_err(|error| {
        format!(
            "Cannot inspect {} for indexing: {error}",
            relative.display()
        )
    })?;
    let state = file_state(&metadata);
    let summary = summarize_sheet(target, relative, &content);

    let mut connection = open_connection(index_path)?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Cannot begin the index update: {error}"))?;
    let mut writer = IndexWriter::new(&transaction)?;
    writer.index(&summary, markdown_body(&content), state)?;
    drop(writer);
    transaction
        .commit()
        .map_err(|error| format!("Cannot finish the index update: {error}"))
}

pub(super) fn search(index_path: &Path, query: &str) -> Result<Vec<SheetSummary>, String> {
    let expression = match fts_expression(query) {
        Some(expression) => expression,
        None => return Ok(Vec::new()),
    };
    let connection = open_connection(index_path)?;
    let mut statement = connection
        .prepare(
            "SELECT d.title,
                    d.relative_path,
                    d.group_name,
                    snippet(documents_fts, 2, '', '', ' … ', 24),
                    d.excerpt,
                    d.word_count,
                    d.created_at
             FROM documents_fts
             JOIN documents AS d ON d.relative_path = documents_fts.relative_path
             WHERE documents_fts MATCH ?1
             ORDER BY bm25(documents_fts), d.title COLLATE NOCASE
             LIMIT ?2",
        )
        .map_err(|error| format!("Cannot prepare the indexed search: {error}"))?;

    let rows = statement
        .query_map(params![expression, RESULT_LIMIT], |row| {
            let snippet: String = row.get(3)?;
            let fallback: String = row.get(4)?;
            let word_count: i64 = row.get(5)?;
            Ok(SheetSummary {
                title: row.get(0)?,
                relative_path: row.get(1)?,
                group: row.get(2)?,
                excerpt: clean_snippet(&snippet, &fallback),
                word_count: word_count.max(0) as usize,
                created_at: row.get(6)?,
            })
        })
        .map_err(|error| format!("Cannot run the indexed search: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Cannot read an indexed search result: {error}"))
}

fn synchronize_once(index_path: &Path, root: &Path) -> Result<LibraryScan, String> {
    let mut connection = open_connection(index_path)?;
    let existing = existing_file_states(&connection)?;
    let mut seen = HashSet::with_capacity(existing.len());
    let mut warnings = Vec::new();
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Cannot begin the library index refresh: {error}"))?;
    let mut writer = IndexWriter::new(&transaction)?;

    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|error| format!("Cannot scan library for indexing: {error}"))?;
        if !entry.file_type().is_file() || !is_markdown(entry.path()) {
            continue;
        }

        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| "An indexed sheet resolved outside the library.".to_string())?;
        let relative_path = path_for_frontend(relative);
        seen.insert(relative_path.clone());

        let metadata = entry
            .metadata()
            .map_err(|error| format!("Cannot inspect {}: {error}", relative.display()))?;
        let state = file_state(&metadata);
        if existing.get(&relative_path) == Some(&state) {
            continue;
        }

        let content = match fs::read_to_string(entry.path()) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
                warnings.push(invalid_utf8_warning(relative));
                writer.remove(&relative_path)?;
                continue;
            }
            Err(error) => return Err(format!("Cannot read {}: {error}", relative.display())),
        };
        let summary = summarize_sheet(entry.path(), relative, &content);
        writer.index(&summary, markdown_body(&content), state)?;
    }

    for relative_path in existing.keys().filter(|path| !seen.contains(*path)) {
        writer.remove(relative_path)?;
    }

    drop(writer);
    transaction
        .commit()
        .map_err(|error| format!("Cannot finish the library index refresh: {error}"))?;
    Ok(LibraryScan {
        sheets: list_summaries(&connection)?,
        warnings,
    })
}

fn open_connection(index_path: &Path) -> Result<Connection, String> {
    let parent = index_path
        .parent()
        .ok_or_else(|| "The search index has no parent folder.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Cannot create the search-index folder: {error}"))?;

    let connection = Connection::open(index_path)
        .map_err(|error| format!("Cannot open the library search index: {error}"))?;
    connection
        .busy_timeout(Duration::from_secs(3))
        .map_err(|error| format!("Cannot configure the library search index: {error}"))?;
    initialize_schema(&connection)?;
    Ok(connection)
}

fn initialize_schema(connection: &Connection) -> Result<(), String> {
    let version = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .map_err(|error| format!("Cannot inspect the search-index schema: {error}"))?;
    if version != 0 && version != SCHEMA_VERSION {
        connection
            .execute_batch(
                "DROP TABLE IF EXISTS documents_fts;
                 DROP TABLE IF EXISTS documents;",
            )
            .map_err(|error| format!("Cannot reset the old search-index schema: {error}"))?;
    }

    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS documents (
                 relative_path TEXT PRIMARY KEY NOT NULL,
                 title TEXT NOT NULL,
                 group_name TEXT NOT NULL,
                 excerpt TEXT NOT NULL,
                 word_count INTEGER NOT NULL,
                 created_at TEXT NOT NULL,
                 modified_ns INTEGER NOT NULL,
                 file_size INTEGER NOT NULL
             );
             CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
                 relative_path UNINDEXED,
                 title,
                 body,
                 tokenize = 'unicode61 remove_diacritics 2'
             );",
        )
        .map_err(|error| format!("Cannot initialize the search-index schema: {error}"))?;
    connection
        .pragma_update(None, "user_version", SCHEMA_VERSION)
        .map_err(|error| format!("Cannot record the search-index schema: {error}"))?;
    Ok(())
}

fn existing_file_states(connection: &Connection) -> Result<HashMap<String, FileState>, String> {
    let mut statement = connection
        .prepare("SELECT relative_path, modified_ns, file_size FROM documents")
        .map_err(|error| format!("Cannot inspect indexed sheets: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                FileState {
                    modified_ns: row.get(1)?,
                    size: row.get(2)?,
                },
            ))
        })
        .map_err(|error| format!("Cannot read indexed sheet state: {error}"))?;

    rows.collect::<Result<HashMap<_, _>, _>>()
        .map_err(|error| format!("Cannot decode indexed sheet state: {error}"))
}

struct IndexWriter<'transaction> {
    upsert_metadata: Statement<'transaction>,
    delete_text: Statement<'transaction>,
    insert_text: Statement<'transaction>,
    delete_metadata: Statement<'transaction>,
}

impl<'transaction> IndexWriter<'transaction> {
    fn new(transaction: &'transaction Transaction<'_>) -> Result<Self, String> {
        Ok(Self {
            upsert_metadata: transaction
                .prepare(
                    "INSERT INTO documents (
                 relative_path, title, group_name, excerpt, word_count,
                 created_at, modified_ns, file_size
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(relative_path) DO UPDATE SET
                 title = excluded.title,
                 group_name = excluded.group_name,
                 excerpt = excluded.excerpt,
                 word_count = excluded.word_count,
                 created_at = excluded.created_at,
                 modified_ns = excluded.modified_ns,
                 file_size = excluded.file_size",
                )
                .map_err(|error| format!("Cannot prepare indexed sheet metadata: {error}"))?,
            delete_text: transaction
                .prepare("DELETE FROM documents_fts WHERE relative_path = ?1")
                .map_err(|error| format!("Cannot prepare indexed text replacement: {error}"))?,
            insert_text: transaction
                .prepare(
                    "INSERT INTO documents_fts (relative_path, title, body) VALUES (?1, ?2, ?3)",
                )
                .map_err(|error| format!("Cannot prepare sheet text indexing: {error}"))?,
            delete_metadata: transaction
                .prepare("DELETE FROM documents WHERE relative_path = ?1")
                .map_err(|error| format!("Cannot prepare stale metadata removal: {error}"))?,
        })
    }

    fn index(
        &mut self,
        summary: &SheetSummary,
        body: &str,
        state: FileState,
    ) -> Result<(), String> {
        self.upsert_metadata
            .execute(params![
                &summary.relative_path,
                &summary.title,
                &summary.group,
                &summary.excerpt,
                summary.word_count as i64,
                &summary.created_at,
                state.modified_ns,
                state.size,
            ])
            .map_err(|error| format!("Cannot update indexed sheet metadata: {error}"))?;
        self.delete_text
            .execute(params![&summary.relative_path])
            .map_err(|error| format!("Cannot replace indexed sheet text: {error}"))?;
        self.insert_text
            .execute(params![&summary.relative_path, &summary.title, body])
            .map_err(|error| format!("Cannot index sheet text: {error}"))?;
        Ok(())
    }

    fn remove(&mut self, relative_path: &str) -> Result<(), String> {
        self.delete_text
            .execute(params![relative_path])
            .map_err(|error| format!("Cannot remove stale indexed text: {error}"))?;
        self.delete_metadata
            .execute(params![relative_path])
            .map_err(|error| format!("Cannot remove stale indexed metadata: {error}"))?;
        Ok(())
    }
}

fn list_summaries(connection: &Connection) -> Result<Vec<SheetSummary>, String> {
    let mut statement = connection
        .prepare(
            "SELECT title, relative_path, group_name, excerpt, word_count, created_at
             FROM documents
             ORDER BY relative_path",
        )
        .map_err(|error| format!("Cannot prepare the indexed library list: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            let word_count: i64 = row.get(4)?;
            Ok(SheetSummary {
                title: row.get(0)?,
                relative_path: row.get(1)?,
                group: row.get(2)?,
                excerpt: row.get(3)?,
                word_count: word_count.max(0) as usize,
                created_at: row.get(5)?,
            })
        })
        .map_err(|error| format!("Cannot read the indexed library list: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Cannot decode an indexed sheet: {error}"))
}

fn file_state(metadata: &fs::Metadata) -> FileState {
    let modified_ns = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or(0);
    FileState {
        modified_ns,
        size: metadata.len().min(i64::MAX as u64) as i64,
    }
}

fn fts_expression(query: &str) -> Option<String> {
    let terms = query
        .split_whitespace()
        .filter(|term| term.chars().any(char::is_alphanumeric))
        .map(|term| format!("\"{}\"*", term.replace('"', "\"\"")))
        .collect::<Vec<_>>();
    (!terms.is_empty()).then(|| terms.join(" AND "))
}

fn clean_snippet(snippet: &str, fallback: &str) -> String {
    let cleaned = snippet.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

fn remove_database_files(index_path: &Path) -> Result<(), String> {
    let mut paths = vec![index_path.to_path_buf()];
    paths.push(PathBuf::from(format!(
        "{}-wal",
        index_path.to_string_lossy()
    )));
    paths.push(PathBuf::from(format!(
        "{}-shm",
        index_path.to_string_lossy()
    )));
    for path in paths {
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|error| format!("Cannot reset {}: {error}", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_refreshes_changed_and_removed_files() {
        let root = tempfile::tempdir().expect("temporary library");
        let state = tempfile::tempdir().expect("temporary index state");
        let index = state.path().join("library.sqlite3");
        let first = root.path().join("first.md");
        let second = root.path().join("second.md");
        fs::write(&first, "# First\n\nA copper lighthouse.").expect("first sheet");
        fs::write(&second, "# Second\n\nA blue window.").expect("second sheet");

        assert_eq!(
            synchronize(&index, root.path())
                .expect("first index")
                .sheets
                .len(),
            2
        );
        assert_eq!(search(&index, "lighthouse").expect("first search").len(), 1);

        fs::write(&first, "# First\n\nA copper compass.").expect("changed sheet");
        fs::remove_file(&second).expect("remove second sheet");
        assert_eq!(
            synchronize(&index, root.path())
                .expect("refresh index")
                .sheets
                .len(),
            1
        );
        assert!(search(&index, "lighthouse")
            .expect("stale search")
            .is_empty());
        assert_eq!(search(&index, "compass").expect("updated search").len(), 1);
    }

    #[test]
    fn search_uses_all_terms_and_prefixes() {
        let root = tempfile::tempdir().expect("temporary library");
        let state = tempfile::tempdir().expect("temporary index state");
        let index = state.path().join("library.sqlite3");
        fs::write(
            root.path().join("notes.md"),
            "# Notes\n\nThe lighthouse code is hidden here.",
        )
        .expect("test sheet");
        synchronize(&index, root.path()).expect("build index");

        assert_eq!(
            search(&index, "light code").expect("prefix search").len(),
            1
        );
        assert!(search(&index, "light window")
            .expect("and search")
            .is_empty());
    }

    #[test]
    fn direct_sheet_update_refreshes_saved_text() {
        let root = tempfile::tempdir().expect("temporary library");
        let state = tempfile::tempdir().expect("temporary index state");
        let index = state.path().join("library.sqlite3");
        let sheet = root.path().join("draft.md");
        fs::write(&sheet, "# Draft\n\nThe first signal.").expect("initial sheet");
        synchronize(&index, root.path()).expect("build index");

        fs::write(&sheet, "# Draft\n\nThe second lantern.").expect("saved sheet");
        update_sheet(&index, root.path(), &sheet).expect("update one index entry");

        assert!(search(&index, "signal").expect("old search").is_empty());
        assert_eq!(search(&index, "lantern").expect("new search").len(), 1);
    }

    #[test]
    fn index_skips_invalid_utf8_without_blocking_valid_sheets() {
        let root = tempfile::tempdir().expect("temporary library");
        let state = tempfile::tempdir().expect("temporary index state");
        let index = state.path().join("library.sqlite3");
        fs::write(root.path().join("safe.md"), "# Safe\n\nSearchable lantern.")
            .expect("valid sheet");
        fs::write(root.path().join("legacy.md"), [0xff, 0xfe, 0x61]).expect("invalid sheet");

        let scan = synchronize(&index, root.path()).expect("build partial index");

        assert_eq!(scan.sheets.len(), 1);
        assert_eq!(scan.warnings.len(), 1);
        assert!(scan.warnings[0].contains("legacy.md is not UTF-8"));
        assert_eq!(search(&index, "lantern").expect("valid search").len(), 1);
    }

    #[test]
    fn corrupt_database_is_rebuilt_from_markdown() {
        let root = tempfile::tempdir().expect("temporary library");
        let state = tempfile::tempdir().expect("temporary index state");
        let index = state.path().join("library.sqlite3");
        fs::write(root.path().join("safe.md"), "# Safe\n\nKeep this text.").expect("test sheet");
        fs::write(&index, "not a sqlite database").expect("corrupt index");

        let sheets = synchronize(&index, root.path()).expect("rebuild corrupt index");

        assert_eq!(sheets.sheets.len(), 1);
        assert_eq!(search(&index, "keep").expect("rebuilt search").len(), 1);
    }

    #[test]
    #[ignore = "repeatable 10,000-sheet scale benchmark"]
    fn indexes_ten_thousand_sheets() {
        let root = tempfile::tempdir().expect("temporary library");
        let state = tempfile::tempdir().expect("temporary index state");
        let index = state.path().join("library.sqlite3");
        for group_number in 0..10 {
            let group = root.path().join(format!("Group-{group_number:02}"));
            fs::create_dir(&group).expect("benchmark group");
            for sheet_number in 0..1_000 {
                let global_number = group_number * 1_000 + sheet_number;
                let marker = if global_number == 9_999 {
                    " singular-lantern-marker"
                } else {
                    ""
                };
                fs::write(
                    group.join(format!("sheet-{sheet_number:04}.md")),
                    format!(
                        "# Sheet {global_number}\n\nA short benchmark manuscript number {global_number}.{marker}\n"
                    ),
                )
                .expect("benchmark sheet");
            }
        }

        let cold_start = std::time::Instant::now();
        let sheets = synchronize(&index, root.path()).expect("cold index build");
        let cold_elapsed = cold_start.elapsed();
        let search_start = std::time::Instant::now();
        let results = search(&index, "singular lantern").expect("indexed benchmark search");
        let search_elapsed = search_start.elapsed();
        let warm_start = std::time::Instant::now();
        let warm_sheets = synchronize(&index, root.path()).expect("warm index refresh");
        let warm_elapsed = warm_start.elapsed();

        eprintln!(
            "10k index: cold={cold_elapsed:?}, search={search_elapsed:?}, warm={warm_elapsed:?}"
        );
        assert_eq!(sheets.sheets.len(), 10_000);
        assert_eq!(warm_sheets.sheets.len(), 10_000);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].relative_path, "Group-09/sheet-0999.md");
    }
}
