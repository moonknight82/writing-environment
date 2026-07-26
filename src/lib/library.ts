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
  projectId: string | null;
  sheets: SheetSummary[];
  warnings: string[];
}

export interface TrashItem {
  id: string;
  title: string;
  originalRelativePath: string;
  trashedAt: string;
}

export interface TrashOrigin {
  id: string;
  name: string;
  path: string;
  kind: "inbox" | "project";
}

export interface UniversalTrashItem extends TrashItem {
  originId: string;
  originName: string;
  originPath: string;
  originKind: "inbox" | "project";
  originAvailable: boolean;
}

export interface TrashRestoreResult {
  root: string;
  restoredToInbox: boolean;
  sheet: SheetSummary;
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

export function openInbox(): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("open_inbox");
}

export function ensureProjectIdentity(root: string, preferredId: string): Promise<string> {
  return invoke<string>("ensure_project_identity", { root, preferredId });
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

export function listUniversalTrash(origins: TrashOrigin[]): Promise<UniversalTrashItem[]> {
  return invoke<UniversalTrashItem[]>("list_universal_trash", { origins });
}

export function emptyLibraryTrash(root: string): Promise<number> {
  return invoke<number>("empty_trash", { root });
}

export function emptyUniversalTrash(
  origins: TrashOrigin[],
  originId: string | null,
): Promise<number> {
  return invoke<number>("empty_universal_trash", { origins, originId });
}

export function restoreLibraryTrash(root: string, trashId: string): Promise<SheetSummary> {
  return invoke<SheetSummary>("restore_trash_item", { root, trashId });
}

export function restoreUniversalTrash(
  origin: TrashOrigin,
  trashId: string,
  restoreToInbox: boolean,
): Promise<TrashRestoreResult> {
  return invoke<TrashRestoreResult>("restore_universal_trash_item", {
    origin,
    trashId,
    restoreToInbox,
  });
}

export function searchLibrary(root: string, query: string): Promise<SheetSummary[]> {
  return invoke<SheetSummary[]>("search_library", { root, query });
}
