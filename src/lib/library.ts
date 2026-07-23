import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface SheetSummary {
  title: string;
  relativePath: string;
  group: string;
  excerpt: string;
  wordCount: number;
  createdAt: string;
}

export interface LibrarySnapshot {
  name: string;
  path: string;
  sheets: SheetSummary[];
  warnings: string[];
}

export interface TrashItem {
  id: string;
  title: string;
  originalRelativePath: string;
  trashedAt: string;
}

export interface RevisionSummary {
  id: string;
  createdAt: string;
  wordCount: number;
  wordDelta: number;
  excerpt: string;
}

export function desktopAvailable(): boolean {
  return isTauri();
}

export async function chooseLibrary(): Promise<LibrarySnapshot | null> {
  if (!desktopAvailable()) return null;

  const selection = await open({
    directory: true,
    multiple: false,
    title: "Open Markdown library",
  });

  if (typeof selection !== "string") return null;
  return openLibraryPath(selection);
}

export function openLibraryPath(path: string): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("open_library", { path });
}

export function readLibrarySheet(root: string, relativePath: string): Promise<string> {
  return invoke<string>("read_sheet", { root, relativePath });
}

export function saveLibrarySheet(
  root: string,
  relativePath: string,
  content: string,
  expectedContent: string,
): Promise<string> {
  return invoke<string>("save_sheet", { root, relativePath, content, expectedContent });
}

export function preserveLocalConflict(
  root: string,
  relativePath: string,
  content: string,
): Promise<SheetSummary> {
  return invoke<SheetSummary>("preserve_local_conflict", { root, relativePath, content });
}

export function watchLibrary(root: string): Promise<string> {
  return invoke<string>("watch_library", { path: root });
}

export function unwatchLibrary(): Promise<void> {
  return invoke<void>("unwatch_library");
}

export function listSheetRevisions(
  root: string,
  relativePath: string,
): Promise<RevisionSummary[]> {
  return invoke<RevisionSummary[]>("list_sheet_revisions", { root, relativePath });
}

export function readSheetRevision(
  root: string,
  relativePath: string,
  revisionId: string,
): Promise<string> {
  return invoke<string>("read_sheet_revision", { root, relativePath, revisionId });
}

export function restoreSheetRevision(
  root: string,
  relativePath: string,
  revisionId: string,
): Promise<string> {
  return invoke<string>("restore_sheet_revision", { root, relativePath, revisionId });
}

export function createLibrarySheet(root: string, group: string, title: string): Promise<SheetSummary> {
  return invoke<SheetSummary>("create_sheet", { root, group, title });
}

export function renameLibrarySheet(
  root: string,
  relativePath: string,
  title: string,
): Promise<SheetSummary> {
  return invoke<SheetSummary>("rename_sheet", { root, relativePath, title });
}

export function duplicateLibrarySheet(
  root: string,
  relativePath: string,
  title: string,
): Promise<SheetSummary> {
  return invoke<SheetSummary>("duplicate_sheet", { root, relativePath, title });
}

export function moveLibrarySheet(
  root: string,
  relativePath: string,
  group: string,
): Promise<SheetSummary> {
  return invoke<SheetSummary>("move_sheet", { root, relativePath, group });
}

export function moveLibrarySheetToProject(
  sourceRoot: string,
  relativePath: string,
  destinationRoot: string,
  group: string,
): Promise<SheetSummary> {
  return invoke<SheetSummary>("move_sheet_to_project", {
    sourceRoot,
    relativePath,
    destinationRoot,
    group,
  });
}

export function trashLibrarySheet(root: string, relativePath: string): Promise<void> {
  return invoke<void>("trash_sheet", { root, relativePath });
}

export function listLibraryTrash(root: string): Promise<TrashItem[]> {
  return invoke<TrashItem[]>("list_trash", { root });
}

export function emptyLibraryTrash(root: string): Promise<number> {
  return invoke<number>("empty_trash", { root });
}

export function restoreLibraryTrash(root: string, trashId: string): Promise<SheetSummary> {
  return invoke<SheetSummary>("restore_trash_item", { root, trashId });
}

export function searchLibrary(root: string, query: string): Promise<SheetSummary[]> {
  return invoke<SheetSummary[]>("search_library", { root, query });
}
