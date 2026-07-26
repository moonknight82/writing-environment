<script lang="ts">
  import { afterUpdate, onDestroy, onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import {
    chooseLibrary,
    createLibrarySheet,
    desktopAvailable,
    duplicateLibrarySheet,
    emptyUniversalTrash as emptyUniversalTrashItems,
    ensureProjectIdentity,
    listSheetRevisions,
    listUniversalTrash,
    moveLibrarySheet,
    moveLibrarySheetToProject,
    openInbox as openInboxLibrary,
    openLibraryPath,
    preserveLocalConflict,
    readLibrarySheet,
    readSheetRevision,
    renameLibrarySheet,
    restoreUniversalTrash,
    restoreSheetRevision,
    saveLibrarySheet,
    searchLibrary,
    trashLibrarySheet,
    unwatchLibrary,
    watchLibrary,
    type LibrarySnapshot,
    type RevisionSummary,
    type SheetSummary,
    type TrashOrigin,
    type UniversalTrashItem,
  } from "./lib/library";
  import {
    getSyncAvailability,
    recoverProjectSync,
    syncProject,
    type SyncAvailability,
  } from "./lib/sync";
  import { applyTheme, themes } from "./lib/themes";

  interface FolderSummary {
    path: string;
    name: string;
    depth: number;
    count: number;
  }

  interface ProjectBookmark {
    id: string;
    name: string;
    path: string;
    pinned: boolean;
    open: boolean;
    lastOpened: number;
    lastSheetPath: string | null;
  }

  type WritingFocusMode = "off" | "paragraph" | "sentence";
  type SheetSort = "created-desc" | "created-asc" | "title-asc" | "title-desc";

  interface FocusSegment {
    text: string;
    active: boolean;
  }

  type SheetDialogMode = "create" | "rename" | "move" | "trash";

  interface LastWorkspace {
    projectPath: string;
    sheetPath: string | null;
  }

  interface LegacySyncPreference {
    remote: string;
    remotePath: string;
    automatic: boolean;
    initialized: boolean;
    recoveryTarget: SyncRecoveryTarget | null;
  }

  interface SyncTargetPreference {
    included: boolean;
    initialized: boolean;
    legacy: boolean;
    remote: string;
    remotePath: string;
    recoveryTarget: SyncRecoveryTarget | null;
  }

  interface UniversalSyncConfig {
    remote: string;
    remoteRoot: string;
    automatic: boolean;
    targets: Record<string, SyncTargetPreference>;
  }

  interface UniversalSyncTarget extends SyncTargetPreference {
    id: string;
    name: string;
    root: string;
    kind: "inbox" | "project";
    available: boolean;
  }

  interface LibraryFilesChanged {
    root: string;
  }

  type SyncPhase = "local" | "ready" | "syncing" | "synced" | "conflict" | "error";
  type SyncRecoveryTarget = "local" | "remote";

  const UPDATE_CHECK_INTERVAL = 24 * 60 * 60 * 1000;

  const prototypeSheets: SheetSummary[] = [
    {
      title: "The Arrival",
      relativePath: "Draft/the-arrival.md",
      group: "Draft",
      excerpt: "The rain arrived before anyone expected it.",
      wordCount: 1284,
      createdAt: "2026-07-18T14:20:00-03:00",
    },
    {
      title: "A Light Offshore",
      relativePath: "Draft/a-light-offshore.md",
      group: "Draft",
      excerpt: "Mara watched the beam move across the water.",
      wordCount: 942,
      createdAt: "2026-07-19T09:10:00-03:00",
    },
    {
      title: "What Elias Knew",
      relativePath: "Research/what-elias-knew.md",
      group: "Research",
      excerpt: "There were three versions of the story.",
      wordCount: 1611,
      createdAt: "2026-07-17T16:45:00-03:00",
    },
    {
      title: "The Harbor Road",
      relativePath: "Research/Locations/the-harbor-road.md",
      group: "Research",
      excerpt: "The old road reached the harbor from the north.",
      wordCount: 386,
      createdAt: "2026-07-16T11:30:00-03:00",
    },
    {
      title: "The Empty Room",
      relativePath: "Fragments/the-empty-room.md",
      group: "Fragments",
      excerpt: "By morning, every photograph had been turned over.",
      wordCount: 708,
      createdAt: "2026-07-20T08:30:00-03:00",
    },
  ];

  const prototypeSheetBodies: Record<string, string> = {
    "Draft/the-arrival.md": `# The Arrival

The rain arrived before anyone expected it.

At first it was only a change in the sound of the sea—a softening beyond the headland, as though the dark itself had drawn a breath. Mara closed the notebook on her thumb and looked toward the lighthouse.

Its beam crossed the water once, disappeared, and did not return.

She waited through ten slow heartbeats before standing.`,
    "Draft/a-light-offshore.md": `# A Light Offshore

Mara watched the beam move across the water.

It paused on the empty horizon, then swept back toward the harbor as if searching for something that had failed to arrive.`,
    "Fragments/the-empty-room.md": `# The Empty Room

By morning, every photograph had been turned over.

Nothing else in the room had moved, but the dust beneath each frame preserved the outline of a careful hand.`,
    "Research/what-elias-knew.md": `# What Elias Knew

There were three versions of the story.

Elias had written each one in a different ink, but every version ended with the same light moving offshore.`,
    "Research/Locations/the-harbor-road.md": `# The Harbor Road

The old road reached the harbor from the north.

It passed the abandoned signal house before descending between black pines to the water.`,
  };

  const prototypeProjectPath = "browser-prototype";
  const initialText = prototypeSheetBodies[prototypeSheets[0].relativePath];
  const desktopMode = desktopAvailable()
    || (import.meta.env.DEV && new URLSearchParams(globalThis.location?.search ?? "").has("desktop-preview"));

  let activeGroup = desktopMode ? "All Sheets" : "Draft";
  let activeSheet = desktopMode ? "No sheet open" : "The Arrival";
  let activeSheetPath: string | null = desktopMode ? null : prototypeSheets[0].relativePath;
  let activeThemeId = "paper";
  let libraryVisible = true;
  let sheetsVisible = true;
  let themeMenuVisible = false;
  let syncMenuVisible = false;
  let writerMenuVisible = false;
  let focusMenuVisible = false;
  let goalMenuVisible = false;
  let sortMenuVisible = false;
  let sheetSort: SheetSort = "created-desc";
  let lineHeight = 1.78;
  let editorTextSize = 19;
  let writerWidth = 72;
  let spellCheckEnabled = true;
  let automaticCorrection = false;
  let writingFocusMode: WritingFocusMode = "off";
  let cursorPosition = 0;
  let focusSegments: FocusSegment[] = [];
  let editorTextarea: HTMLTextAreaElement;
  let focusOverlay: HTMLPreElement;
  let sessionGoal = 500;
  let sessionGoalDraft = 500;
  let sessionWords = 0;
  const sessionBaselines = new Map<string, number>();
  const sessionCounts = new Map<string, number>();
  let content = desktopMode ? "" : initialText;
  let persistedContent = content;
  let saveStatus = desktopMode ? "No sheet open" : "Saved locally";
  let libraryName = desktopMode ? "No project open" : "Prototype Library";
  let libraryPath: string | null = null;
  let inboxPath: string | null = null;
  let inboxActive = false;
  let inboxSheetCount = 0;
  let sheets = desktopMode ? [] : prototypeSheets;
  let folders = folderSummaries(sheets);
  let visibleSheets = sheets.filter((sheet) => sheetIsInFolder(sheet, activeGroup));
  let projects: ProjectBookmark[] = desktopMode ? [] : [{
    id: "browser-prototype",
    name: "Prototype Library",
    path: prototypeProjectPath,
    pinned: false,
    open: true,
    lastOpened: 0,
    lastSheetPath: prototypeSheets[0].relativePath,
  }];
  let sortedProjects: ProjectBookmark[] = [];
  let sidebarProjects: ProjectBookmark[] = [];
  let activeProjectPath: string | null = null;
  let projectMenuPath: string | null = null;
  let projectMenuX = 0;
  let projectMenuY = 0;
  let trashItems: UniversalTrashItem[] = [];
  let filteredTrashItems: UniversalTrashItem[] = [];
  let trashActive = false;
  let trashOriginFilter = "all";
  let searchQuery = "";
  let searchResults: SheetSummary[] = [];
  let searching = false;
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let sheetActionsPath: string | null = null;
  let sheetDialogMode: SheetDialogMode | null = null;
  let dialogSheet: SheetSummary | null = null;
  let dialogTitle = "";
  let dialogGroup = "Draft";
  let dialogProjectPath = "";
  let dialogFolders: FolderSummary[] = [];
  let dialogError = "";
  let emptyTrashConfirmVisible = false;
  let emptyTrashError = "";
  let mutatingLibrary = false;
  let reopenLastWorkspace = true;
  let loadingLibrary = false;
  let errorMessage = "";
  let dirty = false;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let saveInFlight: Promise<boolean> | null = null;
  let syncTimer: ReturnType<typeof setTimeout> | undefined;
  let syncInterval: ReturnType<typeof setInterval> | undefined;
  let syncAvailability: SyncAvailability | null = null;
  let refreshingSync = false;
  let syncRunning = false;
  let syncPhase: SyncPhase = "local";
  let syncStatus = "Local only";
  let syncMessage = "";
  let universalSyncConfig: UniversalSyncConfig = emptyUniversalSyncConfig();
  let universalSyncTargets: UniversalSyncTarget[] = [];
  let syncDraftRemote = "";
  let syncDraftPath = "";
  let syncNeedsInitialization = true;
  let syncInitializationConfirmVisible = false;
  let syncInitializationError = "";
  let historyVisible = false;
  let historyLoading = false;
  let historyRestoring = false;
  let historyRevisions: RevisionSummary[] = [];
  let selectedRevisionId: string | null = null;
  let selectedRevision: RevisionSummary | undefined;
  let revisionPreview = "";
  let historyMessage = "";
  let restoreConfirmId: string | null = null;
  let appFullscreen = false;
  let windowStateUnlisten: (() => void) | undefined;
  let libraryChangeUnlisten: (() => void) | undefined;
  let libraryRefreshTimer: ReturnType<typeof setTimeout> | undefined;
  let libraryRefreshRunning = false;
  let libraryRefreshPending = false;
  let componentDestroyed = false;
  let externalConflictVisible = false;
  let externalConflictPath: string | null = null;
  let externalDiskContent: string | null = null;
  let resolvingExternalConflict = false;
  let appVersion = "0.4.4";
  let automaticUpdateChecks = true;
  let updateVisible = false;
  let updateChecking = false;
  let updateInstalling = false;
  let updateAvailable: Update | null = null;
  let updateMessage = "";
  let updateDownloaded = 0;
  let updateTotal = 0;
  let updateCheckTimer: ReturnType<typeof setTimeout> | undefined;

  $: visibleSheets = sortSheets(
    searchQuery.trim()
      ? searchResults
      : activeGroup === "All Sheets" || inboxActive
        ? sheets
        : sheets.filter((sheet) => sheetIsInFolder(sheet, activeGroup)),
    sheetSort,
  );
  $: sortedProjects = [...projects].sort(
    (left, right) => Number(right.pinned) - Number(left.pinned) || right.lastOpened - left.lastOpened,
  );
  $: activeProjectPath = inboxActive || trashActive
    ? null
    : libraryPath ?? (!desktopMode ? prototypeProjectPath : null);
  $: sidebarProjects = sortedProjects.filter(
    (project) => project.open || project.pinned || project.path === activeProjectPath,
  );
  $: filteredTrashItems = trashOriginFilter === "all"
    ? trashItems
    : trashItems.filter((item) => item.originId === trashOriginFilter);
  $: focusSegments = buildFocusSegments(content, cursorPosition, writingFocusMode);
  $: universalSyncTargets = buildUniversalSyncTargets();
  $: syncNeedsInitialization = universalSyncTargets.some(
    (target) => target.included && !target.initialized,
  );
  $: selectedRevision = historyRevisions.find((revision) => revision.id === selectedRevisionId);

  afterUpdate(() => {
    if (writingFocusMode !== "off") syncFocusOverlay(editorTextarea);
  });

  onMount(() => {
    const savedTheme = localStorage.getItem("writing-environment.theme");
    const storedLineHeight = localStorage.getItem("writing-environment.line-height");
    const savedLineHeight = storedLineHeight === null ? Number.NaN : Number(storedLineHeight);
    const storedEditorTextSize = localStorage.getItem("writing-environment.editor-text-size");
    const savedEditorTextSize = storedEditorTextSize === null
      ? Number.NaN
      : Number(storedEditorTextSize);
    const storedWriterWidth = localStorage.getItem("writing-environment.writer-width");
    const savedWriterWidth = storedWriterWidth === null ? Number.NaN : Number(storedWriterWidth);
    const storedSpellCheck = localStorage.getItem("writing-environment.spell-check");
    const storedAutomaticCorrection = localStorage.getItem("writing-environment.autocorrect");
    const storedFocusMode = localStorage.getItem("writing-environment.writing-focus");
    const storedReopenPreference = localStorage.getItem("writing-environment.reopen-last-workspace");
    const storedSessionGoalValue = localStorage.getItem("writing-environment.session-goal");
    const storedSessionGoal = storedSessionGoalValue === null
      ? Number.NaN
      : Number(storedSessionGoalValue);
    const storedSheetSort = localStorage.getItem("writing-environment.sheet-sort");
    const storedAutomaticUpdateChecks = localStorage.getItem("writing-environment.automatic-update-checks");
    const selected = themes.find((theme) => theme.id === savedTheme) ?? themes[0];

    activeThemeId = selected.id;
    applyTheme(selected);
    if (desktopMode) {
      projects = loadStoredProjects();
      saveProjects();
      universalSyncConfig = loadUniversalSyncConfig();
      migrateLegacySyncPreferences();
      syncDraftRemote = universalSyncConfig.remote;
      syncDraftPath = universalSyncConfig.remoteRoot;
      updateUniversalSyncSummary();
    }
    reopenLastWorkspace = storedReopenPreference !== "false";
    if (!reopenLastWorkspace) localStorage.removeItem("writing-environment.last-workspace");
    spellCheckEnabled = storedSpellCheck !== "false";
    automaticCorrection = spellCheckEnabled && storedAutomaticCorrection === "true";
    automaticUpdateChecks = storedAutomaticUpdateChecks !== "false";
    if (isSheetSort(storedSheetSort)) sheetSort = storedSheetSort;

    if (Number.isFinite(savedLineHeight)) setLineHeight(savedLineHeight);
    setEditorTextSize(Number.isFinite(savedEditorTextSize) ? savedEditorTextSize : editorTextSize);
    if (Number.isFinite(savedWriterWidth)) setWriterWidth(savedWriterWidth);

    if (isWritingFocusMode(storedFocusMode)) writingFocusMode = storedFocusMode;
    if (Number.isFinite(storedSessionGoal) && storedSessionGoal >= 0) {
      sessionGoal = Math.round(storedSessionGoal);
      sessionGoalDraft = sessionGoal;
    }

    if (!desktopMode) {
      content = readPrototypeSheet(activeSheetPath);
      persistedContent = content;
    }
    registerSessionSheet();

    if (desktopAvailable()) {
      void initializeDesktopWorkspace();
      void getVersion().then((version) => (appVersion = version));
      void initializeLibraryChangeListener();
      void refreshSyncAvailability();
      void initializeWindowState();
      scheduleStartupUpdateCheck();
    }
    syncInterval = setInterval(() => scheduleAutomaticSync(0), 5 * 60 * 1000);
  });

  onDestroy(() => {
    componentDestroyed = true;
    if (saveTimer) clearTimeout(saveTimer);
    if (searchTimer) clearTimeout(searchTimer);
    if (syncTimer) clearTimeout(syncTimer);
    if (syncInterval) clearInterval(syncInterval);
    if (libraryRefreshTimer) clearTimeout(libraryRefreshTimer);
    if (updateCheckTimer) clearTimeout(updateCheckTimer);
    windowStateUnlisten?.();
    libraryChangeUnlisten?.();
    if (desktopAvailable()) void unwatchLibrary();
    if (updateAvailable) void updateAvailable.close();
  });

  function scheduleStartupUpdateCheck(): void {
    if (!automaticUpdateChecks || !desktopAvailable()) return;
    const lastCheck = Number(localStorage.getItem("writing-environment.last-update-check") ?? "0");
    if (Number.isFinite(lastCheck) && Date.now() - lastCheck < UPDATE_CHECK_INTERVAL) return;

    updateCheckTimer = setTimeout(() => {
      updateCheckTimer = undefined;
      void checkForAppUpdate(false);
    }, 5000);
  }

  function setAutomaticUpdateChecks(enabled: boolean): void {
    automaticUpdateChecks = enabled;
    localStorage.setItem("writing-environment.automatic-update-checks", String(enabled));
    if (updateCheckTimer) clearTimeout(updateCheckTimer);
    updateCheckTimer = undefined;
    if (enabled) scheduleStartupUpdateCheck();
  }

  async function checkForAppUpdate(showWhenCurrent = true): Promise<void> {
    if (updateChecking || updateInstalling) return;
    writerMenuVisible = false;

    if (!desktopAvailable()) {
      updateVisible = true;
      updateMessage = "Update checks are available in the installed desktop app.";
      return;
    }

    if (showWhenCurrent) updateVisible = true;
    updateChecking = true;
    updateMessage = "Checking GitHub for a signed update…";
    updateDownloaded = 0;
    updateTotal = 0;
    if (updateAvailable) {
      await updateAvailable.close();
      updateAvailable = null;
    }

    try {
      const candidate = await check({ timeout: 15000 });
      localStorage.setItem("writing-environment.last-update-check", String(Date.now()));
      if (candidate) {
        updateAvailable = candidate;
        updateVisible = true;
        updateMessage = `Version ${candidate.version} is ready to install.`;
      } else {
        updateMessage = `Writing Environment ${appVersion} is up to date.`;
      }
    } catch (error) {
      if (showWhenCurrent) {
        updateVisible = true;
        updateMessage = `Could not check for updates: ${errorText(error)}`;
      }
    } finally {
      updateChecking = false;
    }
  }

  async function installAppUpdate(): Promise<void> {
    if (!updateAvailable || updateInstalling) return;
    updateInstalling = true;
    updateMessage = "Saving your current sheet before updating…";

    try {
      if (dirty && !(await persistCurrentSheet())) {
        throw new Error("The current sheet could not be saved, so the update was not installed.");
      }

      updateMessage = "Downloading and verifying the signed update…";
      await updateAvailable.downloadAndInstall((event) => {
        if (event.event === "Started") {
          updateTotal = event.data.contentLength ?? 0;
          updateDownloaded = 0;
        } else if (event.event === "Progress") {
          updateDownloaded += event.data.chunkLength;
        } else if (event.event === "Finished") {
          updateMessage = "Update installed. Restarting Writing Environment…";
        }
      });
      await relaunch();
    } catch (error) {
      updateMessage = `The update was not installed: ${errorText(error)}`;
      updateInstalling = false;
    }
  }

  async function closeUpdateDialog(): Promise<void> {
    if (updateChecking || updateInstalling) return;
    updateVisible = false;
    if (updateAvailable) await updateAvailable.close();
    updateAvailable = null;
  }

  async function initializeWindowState(): Promise<void> {
    const window = getCurrentWindow();
    await refreshWindowState();
    windowStateUnlisten = await window.onResized(() => {
      void refreshWindowState();
    });
  }

  async function initializeLibraryChangeListener(): Promise<void> {
    const unlisten = await listen<LibraryFilesChanged>("library-files-changed", (event) => {
      if (event.payload.root !== libraryPath) return;
      scheduleExternalLibraryRefresh();
    });
    if (componentDestroyed) unlisten();
    else libraryChangeUnlisten = unlisten;
  }

  async function watchActiveLibrary(path: string): Promise<void> {
    try {
      await watchLibrary(path);
    } catch (error) {
      errorMessage = `Automatic project refresh is unavailable: ${errorText(error)}`;
    }
  }

  function scheduleExternalLibraryRefresh(delay = 60): void {
    if (!libraryPath || !desktopAvailable()) return;
    if (libraryRefreshTimer) clearTimeout(libraryRefreshTimer);
    libraryRefreshTimer = setTimeout(() => {
      libraryRefreshTimer = undefined;
      void refreshLibraryAfterExternalChange();
    }, delay);
  }

  async function refreshLibraryAfterExternalChange(): Promise<void> {
    if (!libraryPath || !desktopAvailable()) return;
    if (libraryRefreshRunning) {
      libraryRefreshPending = true;
      return;
    }
    if (loadingLibrary || mutatingLibrary || syncRunning || resolvingExternalConflict) {
      scheduleExternalLibraryRefresh(400);
      return;
    }

    const projectPath = libraryPath;
    const sheetPath = activeSheetPath;
    libraryRefreshRunning = true;
    try {
      const snapshot = await openLibraryPath(projectPath);
      if (libraryPath !== projectPath) return;

      libraryName = snapshot.name;
      sheets = snapshot.sheets;
      folders = folderSummaries(snapshot.sheets);
      if (inboxActive) inboxSheetCount = snapshot.sheets.length;
      await refreshUniversalTrash();
      if (searchQuery.trim()) handleSearchInput(searchQuery);

      const activeSummary = sheetPath
        ? snapshot.sheets.find((sheet) => sheet.relativePath === sheetPath)
        : undefined;
      if (!sheetPath) {
        if (!dirty && snapshot.sheets[0]) await selectSheet(snapshot.sheets[0], true);
        return;
      }

      if (!activeSummary) {
        if (dirty) {
          showExternalConflict(sheetPath, null);
        } else {
          const fallback = snapshot.sheets[0];
          if (fallback) await selectSheet(fallback, true);
          else clearEditorForEmptyLibrary();
        }
        return;
      }

      activeSheet = activeSummary.title;
      if (!searchQuery.trim() && !trashActive) {
        activeGroup = inboxActive ? "Inbox" : sheetFolder(activeSummary);
      }
      const diskContent = await readLibrarySheet(projectPath, sheetPath);
      if (libraryPath !== projectPath || activeSheetPath !== sheetPath) return;

      if (dirty) {
        if (diskContent !== persistedContent) showExternalConflict(sheetPath, diskContent);
        return;
      }

      const changedSinceLastKnownSave = diskContent !== persistedContent;
      persistedContent = diskContent;
      if (changedSinceLastKnownSave && diskContent !== content) replaceEditorFromDisk(diskContent);
      if (!externalConflictVisible) {
        saveStatus = "Saved locally";
        errorMessage = "";
      }
    } catch (error) {
      if (libraryPath === projectPath) {
        errorMessage = `Cannot refresh external project changes: ${errorText(error)}`;
      }
    } finally {
      libraryRefreshRunning = false;
      if (libraryRefreshPending) {
        libraryRefreshPending = false;
        scheduleExternalLibraryRefresh();
      }
    }
  }

  function replaceEditorFromDisk(value: string): void {
    const selectionStart = editorTextarea?.selectionStart ?? cursorPosition;
    const selectionEnd = editorTextarea?.selectionEnd ?? selectionStart;
    const scrollTop = editorTextarea?.scrollTop ?? 0;
    const scrollLeft = editorTextarea?.scrollLeft ?? 0;
    preserveSessionProgressForExternalCount(wordCount(value));
    content = value;
    cursorPosition = Math.min(value.length, selectionStart);
    requestAnimationFrame(() => {
      if (!editorTextarea) return;
      const start = Math.min(content.length, selectionStart);
      const end = Math.min(content.length, selectionEnd);
      editorTextarea.setSelectionRange(start, Math.max(start, end));
      editorTextarea.scrollTop = scrollTop;
      editorTextarea.scrollLeft = scrollLeft;
      syncFocusOverlay(editorTextarea);
    });
  }

  function preserveSessionProgressForExternalCount(count: number): void {
    const key = sessionSheetKey();
    const previousCount = sessionCounts.get(key) ?? wordCount(content);
    const previousBaseline = sessionBaselines.get(key) ?? previousCount;
    const writtenThisSession = Math.max(0, previousCount - previousBaseline);
    sessionCounts.set(key, count);
    sessionBaselines.set(key, count - writtenThisSession);
    recomputeSessionWords();
  }

  function showExternalConflict(path: string, diskContent: string | null): void {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = undefined;
    externalConflictVisible = true;
    externalConflictPath = path;
    externalDiskContent = diskContent;
    saveStatus = diskContent === null ? "Sheet removed on disk" : "Changed on disk";
    errorMessage = "Your local draft is safe in the editor. Choose whether to keep both versions or use the disk version.";
  }

  function clearExternalConflict(): void {
    externalConflictVisible = false;
    externalConflictPath = null;
    externalDiskContent = null;
  }

  async function keepBothExternalVersions(): Promise<void> {
    if (!libraryPath || !externalConflictPath || resolvingExternalConflict) return;
    resolvingExternalConflict = true;
    try {
      const preserved = await preserveLocalConflict(libraryPath, externalConflictPath, content);
      dirty = false;
      clearExternalConflict();
      await reloadLibrary(preserved.relativePath, true);
      saveStatus = "Local draft preserved as a copy";
      scheduleAutomaticSync();
      errorMessage = "";
    } catch (error) {
      errorMessage = errorText(error);
    } finally {
      resolvingExternalConflict = false;
    }
  }

  async function useExternalDiskVersion(): Promise<void> {
    if (!libraryPath || !externalConflictPath || resolvingExternalConflict) return;
    resolvingExternalConflict = true;
    try {
      if (externalDiskContent === null) {
        dirty = false;
        clearExternalConflict();
        await reloadLibrary(null, false);
      } else {
        replaceEditorFromDisk(externalDiskContent);
        persistedContent = externalDiskContent;
        dirty = false;
        clearExternalConflict();
        await reloadLibrary(activeSheetPath, false);
      }
      saveStatus = "Using disk version";
      errorMessage = "";
    } catch (error) {
      errorMessage = errorText(error);
    } finally {
      resolvingExternalConflict = false;
    }
  }

  function clearEditorForEmptyLibrary(): void {
    if (!trashActive) activeGroup = inboxActive ? "Inbox" : "All Sheets";
    activeSheet = "No Markdown sheets";
    activeSheetPath = null;
    content = "";
    persistedContent = "";
    dirty = false;
    clearExternalConflict();
    rememberLastWorkspace();
  }

  function errorText(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  async function refreshWindowState(): Promise<void> {
    if (!desktopAvailable()) return;

    appFullscreen = await invoke<boolean>("get_presentation_mode");
  }

  async function toggleAppFullscreen(): Promise<void> {
    if (!desktopAvailable()) {
      appFullscreen = !appFullscreen;
      closeToolbarMenus();
      return;
    }

    const fullscreen = await invoke<boolean>("get_presentation_mode");
    await invoke<boolean>("set_presentation_mode", { active: !fullscreen });
    appFullscreen = !fullscreen;
    closeToolbarMenus();
  }

  function closeToolbarMenus(): void {
    themeMenuVisible = false;
    syncMenuVisible = false;
    writerMenuVisible = false;
    focusMenuVisible = false;
    goalMenuVisible = false;
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
    if (event.key !== "F11") return;
    event.preventDefault();
    void toggleAppFullscreen();
  }

  function handleWindowClick(event: MouseEvent): void {
    if (!projectMenuPath) return;
    const target = event.target;
    if (target instanceof Element && target.closest(".project-row")) return;
    projectMenuPath = null;
  }

  function selectTheme(themeId: string): void {
    const selected = themes.find((theme) => theme.id === themeId);
    if (!selected) return;

    activeThemeId = selected.id;
    applyTheme(selected);
    localStorage.setItem("writing-environment.theme", selected.id);
    themeMenuVisible = false;
  }

  function setLineHeight(value: number): void {
    lineHeight = Math.min(2.2, Math.max(1.35, Math.round(value * 100) / 100));
    document.documentElement.style.setProperty("--prose-line-height", String(lineHeight));
    localStorage.setItem("writing-environment.line-height", String(lineHeight));
  }

  function setEditorTextSize(value: number): void {
    editorTextSize = Math.min(32, Math.max(14, Math.round(value)));
    document.documentElement.style.setProperty("--editor-text-size", `${editorTextSize}px`);
    localStorage.setItem("writing-environment.editor-text-size", String(editorTextSize));
  }

  function setWriterWidth(value: number): void {
    writerWidth = Math.min(100, Math.max(50, Math.round(value)));
    document.documentElement.style.setProperty("--writer-width", `${writerWidth}%`);
    localStorage.setItem("writing-environment.writer-width", String(writerWidth));
  }

  function prototypeDraftStorageKey(relativePath: string): string {
    return `writing-environment.prototype-draft:${relativePath}`;
  }

  function readPrototypeSheet(relativePath: string | null): string {
    if (!relativePath) return "";

    const saved = localStorage.getItem(prototypeDraftStorageKey(relativePath));
    if (saved !== null) return saved;

    if (relativePath === prototypeSheets[0].relativePath) {
      const legacyDraft = localStorage.getItem("writing-environment.prototype-draft");
      if (legacyDraft !== null) return legacyDraft;
    }

    return prototypeSheetBodies[relativePath] ?? "";
  }

  function savePrototypeSheet(relativePath: string | null, value: string): void {
    if (!relativePath) return;
    localStorage.setItem(prototypeDraftStorageKey(relativePath), value);

    if (relativePath === prototypeSheets[0].relativePath) {
      localStorage.setItem("writing-environment.prototype-draft", value);
    }
  }

  function setSpellCheckEnabled(enabled: boolean): void {
    spellCheckEnabled = enabled;
    localStorage.setItem("writing-environment.spell-check", String(enabled));

    if (!enabled) setAutomaticCorrection(false);
  }

  function setAutomaticCorrection(enabled: boolean): void {
    automaticCorrection = spellCheckEnabled && enabled;
    localStorage.setItem("writing-environment.autocorrect", String(automaticCorrection));
  }

  function setWritingFocusMode(mode: WritingFocusMode): void {
    writingFocusMode = mode;
    localStorage.setItem("writing-environment.writing-focus", mode);
    focusMenuVisible = false;
    requestAnimationFrame(() => syncFocusOverlay(editorTextarea));
  }

  function setSessionGoal(value: number): void {
    sessionGoal = Math.min(100000, Math.max(0, Math.round(Number.isFinite(value) ? value : 0)));
    sessionGoalDraft = sessionGoal;
    localStorage.setItem("writing-environment.session-goal", String(sessionGoal));
  }

  function applySessionGoal(): void {
    setSessionGoal(sessionGoalDraft);
    goalMenuVisible = false;
  }

  function resetSessionProgress(): void {
    sessionBaselines.clear();
    sessionCounts.clear();
    registerSessionSheet();
  }

  function handleEditorInput(target: HTMLTextAreaElement): void {
    content = target.value;
    updateCursor(target);
    updateSessionCount();
    handleInput();
  }

  function updateCursor(target: HTMLTextAreaElement = editorTextarea): void {
    if (!target) return;
    cursorPosition = target.selectionStart;
  }

  function syncFocusOverlay(target: HTMLTextAreaElement): void {
    if (!target || !focusOverlay) return;
    const scrollbarWidth = Math.max(0, target.offsetWidth - target.clientWidth);
    focusOverlay.style.setProperty("--editor-scrollbar-width", `${scrollbarWidth}px`);
    focusOverlay.scrollTop = target.scrollTop;
    focusOverlay.scrollLeft = target.scrollLeft;
  }

  function handleInput(): void {
    dirty = true;
    if (saveTimer) clearTimeout(saveTimer);

    if (externalConflictVisible) {
      saveStatus = externalDiskContent === null ? "Sheet removed on disk" : "Changed on disk";
      return;
    }

    saveStatus = "Saving…";

    saveTimer = setTimeout(() => void persistCurrentSheet(), 650);
  }

  function persistCurrentSheet(): Promise<boolean> {
    if (saveInFlight) return saveInFlight;
    const operation = performCurrentSheetSave();
    saveInFlight = operation;
    void operation.finally(() => {
      if (saveInFlight === operation) saveInFlight = null;
    });
    return operation;
  }

  async function performCurrentSheetSave(): Promise<boolean> {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = undefined;
    const versionBeingSaved = content;
    const projectBeingSaved = libraryPath;
    const sheetBeingSaved = activeSheetPath;
    const expectedDiskContent = persistedContent;

    try {
      let savedContent = versionBeingSaved;
      if (projectBeingSaved && sheetBeingSaved) {
        savedContent = await saveLibrarySheet(
          projectBeingSaved,
          sheetBeingSaved,
          versionBeingSaved,
          expectedDiskContent,
        );
      } else {
        savePrototypeSheet(sheetBeingSaved, versionBeingSaved);
      }

      if (libraryPath !== projectBeingSaved || activeSheetPath !== sheetBeingSaved) return false;
      persistedContent = savedContent;
      errorMessage = "";
      if (content === versionBeingSaved) {
        dirty = false;
        saveStatus = "Saved locally";
        scheduleAutomaticSync();
        return true;
      } else {
        handleInput();
        return false;
      }
    } catch (error) {
      const message = errorText(error);
      if (
        message.startsWith("EXTERNAL_CHANGE:")
        && projectBeingSaved
        && sheetBeingSaved
        && libraryPath === projectBeingSaved
        && activeSheetPath === sheetBeingSaved
      ) {
        let diskContent: string | null = null;
        try {
          diskContent = await readLibrarySheet(projectBeingSaved, sheetBeingSaved);
        } catch {
          // A remove or rename is represented by a missing disk version.
        }
        showExternalConflict(sheetBeingSaved, diskContent);
      } else {
        saveStatus = "Save failed";
        errorMessage = message;
      }
      return false;
    }
  }

  async function selectSheet(sheet: SheetSummary, force = false): Promise<void> {
    if (!force && sheet.relativePath === activeSheetPath) return;
    if (dirty && !(await persistCurrentSheet())) return;

    clearExternalConflict();
    activeSheet = sheet.title;
    activeSheetPath = sheet.relativePath;
    if (!searchQuery.trim() && !trashActive) {
      activeGroup = inboxActive ? "Inbox" : sheetFolder(sheet);
    }

    if (!libraryPath) {
      content = readPrototypeSheet(sheet.relativePath);
      persistedContent = content;
      cursorPosition = 0;
      registerSessionSheet();
      dirty = false;
      saveStatus = "Saved locally";
      errorMessage = "";
      return;
    }

    saveStatus = "Opening…";
    try {
      content = await readLibrarySheet(libraryPath, sheet.relativePath);
      persistedContent = content;
      cursorPosition = 0;
      registerSessionSheet();
      dirty = false;
      saveStatus = "Saved locally";
      errorMessage = "";
      if (inboxActive) {
        localStorage.setItem("writing-environment.last-inbox-sheet", sheet.relativePath);
      } else {
        rememberActiveProjectSheet(sheet.relativePath);
        rememberLastWorkspace();
      }
    } catch (error) {
      saveStatus = "Open failed";
      errorMessage = error instanceof Error ? error.message : String(error);
    }
  }

  async function openLibraryFolder(): Promise<void> {
    if (!desktopAvailable()) {
      errorMessage = "Folder access is available in the Tauri desktop build.";
      return;
    }

    if (dirty && !(await persistCurrentSheet())) return;
    loadingLibrary = true;
    errorMessage = "";

    try {
      const selected = await chooseLibrary();
      if (!selected) return;
      await activateLibrary(selected);
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
    } finally {
      loadingLibrary = false;
    }
  }

  async function openInbox(): Promise<void> {
    if (!desktopAvailable()) return;
    if (syncRunning) {
      errorMessage = "Wait for the current project sync to finish before opening Inbox.";
      return;
    }
    if (dirty && !(await persistCurrentSheet())) return;
    loadingLibrary = true;
    errorMessage = "";
    try {
      await activateInbox(
        await openInboxLibrary(),
        localStorage.getItem("writing-environment.last-inbox-sheet"),
      );
    } catch (error) {
      errorMessage = `Cannot open Inbox: ${errorText(error)}`;
    } finally {
      loadingLibrary = false;
    }
  }

  async function openProject(project: ProjectBookmark): Promise<void> {
    projectMenuPath = null;
    if (!desktopMode && project.path === prototypeProjectPath) return;
    if (!desktopAvailable()) {
      errorMessage = "Pinned projects open in the Tauri desktop build.";
      return;
    }

    if (project.path === libraryPath) {
      trashActive = false;
      const current = activeSheetPath
        ? sheets.find((sheet) => sheet.relativePath === activeSheetPath)
        : undefined;
      activeGroup = current ? sheetFolder(current) : "All Sheets";
      return;
    }
    if (dirty && !(await persistCurrentSheet())) return;
    loadingLibrary = true;
    errorMessage = "";

    try {
      await activateLibrary(await openLibraryPath(project.path), project.lastSheetPath);
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
    } finally {
      loadingLibrary = false;
    }
  }

  function openProjectMenu(event: MouseEvent, project: ProjectBookmark): void {
    event.preventDefault();
    projectMenuPath = project.path;
    projectMenuX = Math.max(8, Math.min(event.clientX, window.innerWidth - 166));
    projectMenuY = Math.max(8, Math.min(event.clientY, window.innerHeight - 128));
  }

  function selectFolder(folder: string): void {
    trashActive = false;
    activeGroup = folder;
    searchQuery = "";
    searchResults = [];
    sortMenuVisible = false;
  }

  async function selectUniversalTrash(): Promise<void> {
    if (dirty && !(await persistCurrentSheet())) return;
    trashActive = true;
    activeGroup = "Trash";
    searchQuery = "";
    searchResults = [];
    sheetActionsPath = null;
    sortMenuVisible = false;
    try {
      await refreshUniversalTrash();
      errorMessage = "";
    } catch (error) {
      errorMessage = `Cannot open Trash: ${errorText(error)}`;
    }
  }

  async function closeProject(project: ProjectBookmark): Promise<void> {
    if (!project.open || loadingLibrary) return;
    const isActive = project.path === libraryPath;
    if (isActive && syncRunning) return;
    if (isActive && dirty && !(await persistCurrentSheet())) return;

    const nextProject = [...projects]
      .filter((candidate) => candidate.open && candidate.path !== project.path)
      .sort((left, right) => right.lastOpened - left.lastOpened)[0];

    projects = projects.map((candidate) =>
      candidate.path === project.path ? { ...candidate, open: false } : candidate,
    );
    saveProjects();
    projectMenuPath = null;
    if (!isActive) return;

    if (syncTimer) clearTimeout(syncTimer);
    syncTimer = undefined;
    if (desktopAvailable()) {
      try {
        await unwatchLibrary();
      } catch {
        // The project may already have stopped being watched.
      }
    }

    localStorage.removeItem("writing-environment.last-workspace");
    if (!nextProject) {
      try {
        await activateInbox(
          await openInboxLibrary(),
          localStorage.getItem("writing-environment.last-inbox-sheet"),
        );
      } catch (error) {
        clearWorkspace();
        errorMessage = `Closed ${project.name}, but could not open Inbox: ${errorText(error)}`;
      }
      return;
    }

    loadingLibrary = true;
    try {
      await activateLibrary(
        await openLibraryPath(nextProject.path),
        nextProject.lastSheetPath,
      );
    } catch (error) {
      clearWorkspace();
      errorMessage = `Closed ${project.name}, but could not open ${nextProject.name}: ${errorText(error)}`;
    } finally {
      loadingLibrary = false;
    }
  }

  function clearWorkspace(): void {
    inboxActive = false;
    trashActive = false;
    libraryPath = null;
    libraryName = "No project open";
    activeGroup = "All Sheets";
    activeSheet = "No sheet open";
    activeSheetPath = null;
    sheets = [];
    folders = [];
    trashItems = [];
    searchQuery = "";
    searchResults = [];
    content = "";
    persistedContent = "";
    cursorPosition = 0;
    dirty = false;
    saveStatus = "No sheet open";
    updateUniversalSyncSummary();
    clearExternalConflict();
  }

  async function activateLibrary(
    selected: LibrarySnapshot,
    preferredSheetPath: string | null = null,
  ): Promise<void> {
    const firstSheet = selected.sheets.find((sheet) => sheet.relativePath === preferredSheetPath)
      ?? selected.sheets[0];
    const firstContent = firstSheet
      ? await readLibrarySheet(selected.path, firstSheet.relativePath)
      : "";

    inboxActive = false;
    trashActive = false;
    libraryName = selected.name;
    libraryPath = selected.path;
    sheets = selected.sheets;
    folders = folderSummaries(selected.sheets);
    rememberProject(selected, firstSheet?.relativePath ?? null);

    if (firstSheet) {
      activeGroup = sheetFolder(firstSheet);
      activeSheet = firstSheet.title;
      activeSheetPath = firstSheet.relativePath;
      content = firstContent;
      persistedContent = content;
    } else {
      activeGroup = "All Sheets";
      activeSheet = "No Markdown sheets";
      activeSheetPath = null;
      content = "";
      persistedContent = "";
    }

    cursorPosition = 0;
    registerSessionSheet();
    dirty = false;
    clearExternalConflict();
    saveStatus = "Saved locally";
    await refreshUniversalTrash();
    rememberLastWorkspace();
    errorMessage = libraryWarningMessage(selected);
    await watchActiveLibrary(selected.path);
    scheduleAutomaticSync(3000);
  }

  async function activateInbox(
    selected: LibrarySnapshot,
    preferredSheetPath: string | null = null,
  ): Promise<void> {
    const firstSheet = selected.sheets.find((sheet) => sheet.relativePath === preferredSheetPath)
      ?? selected.sheets[0];
    const firstContent = firstSheet
      ? await readLibrarySheet(selected.path, firstSheet.relativePath)
      : "";

    if (syncTimer) clearTimeout(syncTimer);
    syncTimer = undefined;
    syncMenuVisible = false;
    inboxActive = true;
    trashActive = false;
    inboxPath = selected.path;
    inboxSheetCount = selected.sheets.length;
    libraryName = "Inbox";
    libraryPath = selected.path;
    sheets = selected.sheets;
    folders = [];
    trashItems = [];
    searchQuery = "";
    searchResults = [];
    updateUniversalSyncSummary();

    if (firstSheet) {
      activeGroup = "Inbox";
      activeSheet = firstSheet.title;
      activeSheetPath = firstSheet.relativePath;
      content = firstContent;
      persistedContent = content;
      localStorage.setItem("writing-environment.last-inbox-sheet", firstSheet.relativePath);
    } else {
      activeGroup = "Inbox";
      activeSheet = "No Inbox sheets";
      activeSheetPath = null;
      content = "";
      persistedContent = "";
    }

    cursorPosition = 0;
    registerSessionSheet();
    dirty = false;
    clearExternalConflict();
    saveStatus = "Saved locally";
    errorMessage = libraryWarningMessage(selected);
    await refreshUniversalTrash();
    await watchActiveLibrary(selected.path);
    scheduleAutomaticSync(3000);
  }

  async function refreshSyncAvailability(): Promise<void> {
    if (!desktopAvailable()) {
      syncAvailability = null;
      syncMessage = "Sync is available in the installed desktop app.";
      return;
    }
    refreshingSync = true;
    try {
      syncAvailability = await getSyncAvailability();
      if (!syncDraftRemote && syncAvailability.remotes.length > 0) {
        syncDraftRemote = syncAvailability.remotes[0];
      }
      if (!universalSyncConfig.remote && !syncRunning) syncMessage = syncAvailability.message;
    } catch (error) {
      syncMessage = error instanceof Error ? error.message : String(error);
    } finally {
      refreshingSync = false;
    }
  }

  function openSyncMenu(): void {
    syncMenuVisible = !syncMenuVisible;
    goalMenuVisible = false;
    focusMenuVisible = false;
    writerMenuVisible = false;
    themeMenuVisible = false;
    if (syncMenuVisible) void refreshSyncAvailability();
  }

  function saveUniversalSyncRoot(): void {
    const remote = syncDraftRemote.trim();
    const remoteRoot = normalizeRemoteRoot(syncDraftPath);
    if (!remote || !remoteRoot) {
      syncPhase = "error";
      syncStatus = "Setup incomplete";
      syncMessage = "Choose an rclone remote and a universal remote root.";
      return;
    }
    if (syncAvailability && !syncAvailability.remotes.includes(remote)) {
      syncPhase = "error";
      syncStatus = "Remote unavailable";
      syncMessage = `The rclone remote ‘${remote}’ is not configured on this computer.`;
      return;
    }

    const identityChanged = remote !== universalSyncConfig.remote
      || remoteRoot !== universalSyncConfig.remoteRoot;
    const targets = { ...universalSyncConfig.targets };
    if (identityChanged) {
      for (const [id, preference] of Object.entries(targets)) {
        if (preference.legacy) continue;
        targets[id] = {
          ...preference,
          initialized: false,
          recoveryTarget: null,
        };
      }
    }
    universalSyncConfig = {
      remote,
      remoteRoot,
      automatic: identityChanged ? false : universalSyncConfig.automatic,
      targets,
    };
    syncDraftPath = remoteRoot;
    saveUniversalSyncConfig();
    updateUniversalSyncSummary();
    syncMessage = identityChanged
      ? "Universal root saved. No files were uploaded; review included locations, then start sync explicitly."
      : "Universal sync settings saved.";
  }

  async function setUniversalTargetIncluded(
    target: UniversalSyncTarget,
    included: boolean,
  ): Promise<void> {
    let resolvedTarget = target;
    if (included && target.kind === "project" && !target.legacy) {
      try {
        resolvedTarget = await ensureUniversalProjectIdentity(target);
      } catch (error) {
        syncPhase = "error";
        syncStatus = "Project identity unavailable";
        syncMessage = errorText(error);
        return;
      }
    }

    updateSyncTargetPreference(resolvedTarget.id, {
      included,
      initialized: resolvedTarget.initialized,
      legacy: resolvedTarget.legacy,
      remote: resolvedTarget.legacy ? resolvedTarget.remote : "",
      remotePath: resolvedTarget.legacy ? resolvedTarget.remotePath : "",
      recoveryTarget: resolvedTarget.recoveryTarget,
    });
    if (!included && universalSyncConfig.automatic) {
      universalSyncConfig = { ...universalSyncConfig, automatic: false };
      saveUniversalSyncConfig();
    }
    updateUniversalSyncSummary();
  }

  async function useUniversalLocation(target: UniversalSyncTarget): Promise<void> {
    if (!universalSyncConfigured()) {
      syncPhase = "error";
      syncStatus = "Setup incomplete";
      syncMessage = "Save the universal remote root before moving a preserved legacy profile.";
      return;
    }
    let resolvedTarget: UniversalSyncTarget;
    try {
      resolvedTarget = await ensureUniversalProjectIdentity(target);
    } catch (error) {
      syncPhase = "error";
      syncStatus = "Project identity unavailable";
      syncMessage = errorText(error);
      return;
    }
    updateSyncTargetPreference(resolvedTarget.id, {
      included: resolvedTarget.included,
      initialized: false,
      legacy: false,
      remote: "",
      remotePath: "",
      recoveryTarget: null,
    });
    universalSyncConfig = { ...universalSyncConfig, automatic: false };
    saveUniversalSyncConfig();
    updateUniversalSyncSummary();
    syncMessage = `${resolvedTarget.name} will use its new universal location after explicit first-sync confirmation. Its previous remote folder was left untouched.`;
  }

  async function runUniversalSync(
    automatic = false,
    initializationConfirmed = false,
  ): Promise<void> {
    if (!desktopAvailable() || syncRunning) return;
    let included = buildUniversalSyncTargets().filter((target) => target.included);
    if (included.length === 0) {
      syncPhase = "error";
      syncStatus = "Nothing included";
      syncMessage = "Include Inbox or at least one project before syncing.";
      return;
    }
    try {
      for (const target of included) {
        if (target.kind === "project" && !target.legacy) {
          await ensureUniversalProjectIdentity(target);
        }
      }
      included = buildUniversalSyncTargets().filter((target) => target.included);
    } catch (error) {
      syncPhase = "error";
      syncStatus = "Project identity unavailable";
      syncMessage = errorText(error);
      return;
    }
    const missingConfiguration = included.filter((target) => !target.remote || !target.remotePath);
    if (missingConfiguration.length > 0) {
      syncPhase = "error";
      syncStatus = "Setup incomplete";
      syncMessage = `Save a universal root for ${missingConfiguration.map((target) => target.name).join(", ")}.`;
      return;
    }
    let targets = included;
    if (automatic) targets = targets.filter((target) => target.initialized);
    if (targets.length === 0) {
      updateUniversalSyncSummary();
      return;
    }

    const initializing = targets.filter((target) => !target.initialized);
    if (!automatic && initializing.length > 0 && !initializationConfirmed) {
      syncInitializationError = "";
      syncInitializationConfirmVisible = true;
      return;
    }

    syncRunning = true;
    syncPhase = "syncing";
    syncStatus = `Syncing 0 / ${targets.length}`;
    syncMessage = "Saving locally before sync…";
    if (syncTimer) clearTimeout(syncTimer);
    syncInitializationConfirmVisible = false;

    try {
      if (dirty) await persistCurrentSheet();
      if (dirty) throw new Error("The current sheet could not be saved, so sync did not start.");
      const failures: string[] = [];
      let completed = 0;
      let conflicts = 0;
      let activeRootChanged = false;

      for (const target of targets) {
        syncStatus = `Syncing ${completed + 1} / ${targets.length}`;
        syncMessage = target.initialized
          ? `Comparing ${target.name}…`
          : `Initializing ${target.name} in an empty remote folder…`;
        try {
          const result = await syncProject(target.root, target.remote, target.remotePath);
          conflicts += result.conflicts;
          updateSyncTargetPreference(target.id, {
            included: true,
            initialized: result.initialized,
            legacy: target.legacy,
            remote: target.legacy ? target.remote : "",
            remotePath: target.legacy ? target.remotePath : "",
            recoveryTarget: null,
          });
          if (libraryPath === target.root) activeRootChanged = true;
        } catch (error) {
          const message = errorText(error);
          const deletionGuard = syncDeletionGuard(message);
          updateSyncTargetPreference(target.id, {
            included: true,
            initialized: target.initialized,
            legacy: target.legacy,
            remote: target.legacy ? target.remote : "",
            remotePath: target.legacy ? target.remotePath : "",
            recoveryTarget: deletionGuard,
          });
          failures.push(`${target.name}: ${syncFailureMessage(message, deletionGuard)}`);
        }
        completed += 1;
        saveUniversalSyncConfig();
      }

      if (failures.length > 0) {
        universalSyncConfig = { ...universalSyncConfig, automatic: false };
        saveUniversalSyncConfig();
        syncPhase = "error";
        syncStatus = `${completed - failures.length} / ${completed} synced`;
        syncMessage = failures.slice(0, 3).join(" · ");
        if (failures.length > 3) syncMessage += ` · and ${failures.length - 3} more`;
        if (automatic) errorMessage = syncMessage;
      } else {
        syncPhase = conflicts > 0 ? "conflict" : "synced";
        syncStatus = conflicts > 0 ? "Conflicts preserved" : "Everything synced";
        syncMessage = `${completed} ${completed === 1 ? "location is" : "locations are"} up to date${conflicts > 0 ? ` with ${conflicts} preserved conflict ${conflicts === 1 ? "file" : "files"}` : ""}.`;
        errorMessage = "";
      }
      if (activeRootChanged && libraryPath) await reloadLibrary(activeSheetPath, true);
    } catch (error) {
      syncPhase = "error";
      universalSyncConfig = { ...universalSyncConfig, automatic: false };
      saveUniversalSyncConfig();
      syncStatus = "Sync needs attention";
      syncMessage = errorText(error);
      if (automatic) errorMessage = syncMessage;
    } finally {
      syncRunning = false;
    }
  }

  async function recoverPausedSync(target: UniversalSyncTarget): Promise<void> {
    if (!target.recoveryTarget || syncRunning) return;

    syncRunning = true;
    syncPhase = "syncing";
    syncStatus = "Recovering safely…";
    syncMessage = target.recoveryTarget === "local"
      ? `Restoring remote-only files to ${target.name} without overwriting local files…`
      : `Restoring local-only files from ${target.name} without overwriting remote files…`;
    try {
      if (dirty) await persistCurrentSheet();
      if (dirty) throw new Error("The current sheet could not be saved, so recovery did not start.");
      const result = await recoverProjectSync(
        target.root,
        target.remote,
        target.remotePath,
        target.recoveryTarget,
      );
      updateSyncTargetPreference(target.id, {
        included: target.included,
        initialized: result.initialized,
        legacy: target.legacy,
        remote: target.legacy ? target.remote : "",
        remotePath: target.legacy ? target.remotePath : "",
        recoveryTarget: null,
      });
      universalSyncConfig = { ...universalSyncConfig, automatic: false };
      saveUniversalSyncConfig();
      syncPhase = result.status;
      syncStatus = result.status === "conflict" ? "Conflicts preserved" : "Recovered and synced";
      syncMessage = result.message;
      if (libraryPath === target.root) await reloadLibrary(activeSheetPath, true);
      errorMessage = "";
    } catch (error) {
      syncPhase = "error";
      syncStatus = "Recovery needs attention";
      syncMessage = errorText(error);
      errorMessage = syncMessage;
    } finally {
      syncRunning = false;
    }
  }

  function setAutomaticSync(enabled: boolean): void {
    const included = buildUniversalSyncTargets().filter((target) => target.included);
    if (
      enabled
      && (
        included.length === 0
        || included.some((target) => !target.initialized || !!target.recoveryTarget)
      )
    ) {
      syncPhase = "error";
      syncStatus = "Setup incomplete";
      syncMessage = "Initialize every included location and resolve paused recovery before enabling automatic sync.";
      return;
    }
    universalSyncConfig = { ...universalSyncConfig, automatic: enabled };
    saveUniversalSyncConfig();
    syncStatus = enabled ? "Automatic sync on" : "Ready to sync";
    syncPhase = "ready";
    if (enabled) scheduleAutomaticSync(1000);
    else if (syncTimer) clearTimeout(syncTimer);
  }

  function scheduleAutomaticSync(delay = 10_000): void {
    if (syncTimer) clearTimeout(syncTimer);
    if (
      !universalSyncConfig.automatic
      || syncRunning
      || !desktopAvailable()
    ) return;
    syncTimer = setTimeout(() => {
      if (!dirty) void runUniversalSync(true);
    }, delay);
  }

  function buildUniversalSyncTargets(): UniversalSyncTarget[] {
    const targets: UniversalSyncTarget[] = [];
    if (inboxPath) targets.push(buildUniversalSyncTarget("inbox", "Inbox", inboxPath, "inbox"));
    for (const project of projects) {
      targets.push(buildUniversalSyncTarget(project.id, project.name, project.path, "project"));
    }
    return targets;
  }

  function buildUniversalSyncTarget(
    id: string,
    name: string,
    root: string,
    kind: "inbox" | "project",
  ): UniversalSyncTarget {
    const preference = universalSyncConfig.targets[id] ?? defaultSyncTargetPreference(kind);
    const remote = preference.legacy ? preference.remote : universalSyncConfig.remote;
    const remotePath = preference.legacy
      ? preference.remotePath
      : universalRemotePath(id, kind);
    return {
      ...preference,
      id,
      name,
      root,
      kind,
      available: Boolean(root),
      remote,
      remotePath,
    };
  }

  function updateSyncTargetPreference(id: string, preference: SyncTargetPreference): void {
    universalSyncConfig = {
      ...universalSyncConfig,
      targets: { ...universalSyncConfig.targets, [id]: preference },
    };
    saveUniversalSyncConfig();
  }

  async function ensureUniversalProjectIdentity(
    target: UniversalSyncTarget,
  ): Promise<UniversalSyncTarget> {
    if (target.kind !== "project") return target;
    const identity = await ensureProjectIdentity(target.root, target.id);
    if (identity === target.id) return target;

    migrateSyncTargetIdentity(target.id, identity);
    projects = projects.map((project) =>
      project.path === target.root ? { ...project, id: identity } : project,
    );
    saveProjects();
    return buildUniversalSyncTarget(identity, target.name, target.root, "project");
  }

  function migrateSyncTargetIdentity(previousId: string, identity: string): void {
    if (previousId === identity || universalSyncConfig.targets[identity]) return;
    const previous = universalSyncConfig.targets[previousId];
    if (!previous) return;
    const migrated = previous.legacy
      ? { ...previous }
      : { ...previous, initialized: false, recoveryTarget: null };
    universalSyncConfig = {
      ...universalSyncConfig,
      automatic: previous.legacy ? universalSyncConfig.automatic : false,
      targets: { ...universalSyncConfig.targets, [identity]: migrated },
    };
    saveUniversalSyncConfig();
  }

  function loadLegacySyncPreferences(): Record<string, LegacySyncPreference> {
    try {
      const parsed: unknown = JSON.parse(
        localStorage.getItem("writing-environment.sync-preferences") ?? "{}",
      );
      if (typeof parsed === "object" && parsed !== null) {
        return parsed as Record<string, LegacySyncPreference>;
      }
    } catch {
      // Ignore malformed local preferences.
    }
    return {};
  }

  function loadUniversalSyncConfig(): UniversalSyncConfig {
    try {
      const parsed: unknown = JSON.parse(
        localStorage.getItem("writing-environment.universal-sync") ?? "null",
      );
      if (typeof parsed === "object" && parsed !== null) {
        const value = parsed as Partial<UniversalSyncConfig>;
        return {
          remote: typeof value.remote === "string" ? value.remote : "",
          remoteRoot: typeof value.remoteRoot === "string" ? value.remoteRoot : "",
          automatic: value.automatic === true,
          targets: typeof value.targets === "object" && value.targets !== null
            ? value.targets as Record<string, SyncTargetPreference>
            : {},
        };
      }
    } catch {
      // Ignore malformed local preferences.
    }
    return emptyUniversalSyncConfig();
  }

  function saveUniversalSyncConfig(): void {
    localStorage.setItem(
      "writing-environment.universal-sync",
      JSON.stringify(universalSyncConfig),
    );
  }

  function migrateLegacySyncPreferences(): void {
    const legacy = loadLegacySyncPreferences();
    const targets = { ...universalSyncConfig.targets };
    let migrated = 0;
    for (const project of projects) {
      if (targets[project.id]) continue;
      const previous = legacy[project.path];
      if (!previous?.remote || !previous.remotePath) continue;
      targets[project.id] = {
        included: previous.initialized,
        initialized: previous.initialized,
        legacy: true,
        remote: previous.remote,
        remotePath: previous.remotePath,
        recoveryTarget: previous.recoveryTarget ?? null,
      };
      migrated += 1;
    }
    if (migrated === 0) return;
    universalSyncConfig = { ...universalSyncConfig, automatic: false, targets };
    saveUniversalSyncConfig();
    syncMessage = `${migrated} existing project sync ${migrated === 1 ? "setting was" : "settings were"} preserved and automatic sync was left off.`;
  }

  function emptyUniversalSyncConfig(): UniversalSyncConfig {
    return {
      remote: "",
      remoteRoot: "",
      automatic: false,
      targets: {},
    };
  }

  function defaultSyncTargetPreference(kind: "inbox" | "project"): SyncTargetPreference {
    return {
      included: kind === "inbox" && universalSyncConfigured(),
      initialized: false,
      legacy: false,
      remote: "",
      remotePath: "",
      recoveryTarget: null,
    };
  }

  function universalSyncConfigured(): boolean {
    return Boolean(universalSyncConfig.remote && universalSyncConfig.remoteRoot);
  }

  function universalRemotePath(
    id: string,
    kind: "inbox" | "project",
  ): string {
    if (!universalSyncConfig.remoteRoot) return "";
    return joinRemotePath(
      universalSyncConfig.remoteRoot,
      kind === "inbox" ? "Inbox" : `Projects/${id}`,
    );
  }

  function joinRemotePath(root: string, child: string): string {
    return `${normalizeRemoteRoot(root)}/${child.replace(/^\/+/, "")}`;
  }

  function normalizeRemoteRoot(value: string): string {
    return value.trim().replace(/^\/+|\/+$/g, "");
  }

  function updateUniversalSyncSummary(): void {
    if (syncRunning) return;
    const included = buildUniversalSyncTargets().filter((target) => target.included);
    const recovery = included.filter((target) => !!target.recoveryTarget);
    if (recovery.length > 0) {
      syncPhase = "error";
      syncStatus = `${recovery.length} paused`;
      syncMessage = "Deletion protection paused one or more locations. Use the recovery action beside each affected location.";
    } else if (included.length === 0) {
      syncPhase = "local";
      syncStatus = "Sync off";
      syncMessage = "Include Inbox or a project to begin.";
    } else if (included.some((target) => !target.remote || !target.remotePath)) {
      syncPhase = "local";
      syncStatus = "Setup required";
      syncMessage = "Save a universal remote root before syncing new locations.";
    } else if (included.some((target) => !target.initialized)) {
      syncPhase = "ready";
      syncStatus = "Initialization required";
      syncMessage = "Review the included locations, then start the first sync explicitly.";
    } else {
      syncPhase = "ready";
      syncStatus = universalSyncConfig.automatic ? "Automatic sync on" : "Ready to sync";
      syncMessage = "All included locations have isolated sync profiles.";
    }
  }

  function syncFailureMessage(
    message: string,
    deletionGuard: SyncRecoveryTarget | null,
  ): string {
    if (deletionGuard === "local") {
      return "deletion protection found too many missing local files; no changes were made";
    }
    if (deletionGuard === "remote") {
      return "deletion protection found too many missing remote files; no changes were made";
    }
    return message;
  }

  function syncDeletionGuard(message: string): SyncRecoveryTarget | null {
    if (message.startsWith("SYNC_DELETE_GUARD_LOCAL:")) return "local";
    if (message.startsWith("SYNC_DELETE_GUARD_REMOTE:")) return "remote";
    return null;
  }

  async function openHistory(): Promise<void> {
    historyVisible = true;
    historyMessage = "";
    restoreConfirmId = null;
    goalMenuVisible = false;
    focusMenuVisible = false;
    writerMenuVisible = false;
    themeMenuVisible = false;
    syncMenuVisible = false;

    if (!libraryPath || !activeSheetPath || !desktopAvailable()) {
      historyMessage = "History becomes available after opening a sheet in the desktop app.";
      return;
    }
    historyLoading = true;
    try {
      if (dirty) await persistCurrentSheet();
      if (dirty) throw new Error("Save the current sheet before opening History.");
      await refreshHistory();
    } catch (error) {
      historyMessage = error instanceof Error ? error.message : String(error);
    } finally {
      historyLoading = false;
    }
  }

  function closeHistory(): void {
    if (historyRestoring) return;
    historyVisible = false;
    restoreConfirmId = null;
  }

  async function refreshHistory(preferredRevisionId: string | null = null): Promise<void> {
    if (!libraryPath || !activeSheetPath) return;
    historyRevisions = await listSheetRevisions(libraryPath, activeSheetPath);
    const selected = historyRevisions.find((revision) => revision.id === preferredRevisionId)
      ?? historyRevisions[0];
    if (selected) await selectRevision(selected);
    else {
      selectedRevisionId = null;
      revisionPreview = "";
      historyMessage = "History will appear after this sheet’s first saved change.";
    }
  }

  async function selectRevision(revision: RevisionSummary): Promise<void> {
    if (!libraryPath || !activeSheetPath) return;
    historyLoading = true;
    restoreConfirmId = null;
    historyMessage = "";
    try {
      revisionPreview = await readSheetRevision(libraryPath, activeSheetPath, revision.id);
      selectedRevisionId = revision.id;
    } catch (error) {
      historyMessage = error instanceof Error ? error.message : String(error);
    } finally {
      historyLoading = false;
    }
  }

  async function restoreSelectedRevision(): Promise<void> {
    if (!libraryPath || !activeSheetPath || !selectedRevisionId || historyRestoring) return;
    if (restoreConfirmId !== selectedRevisionId) {
      restoreConfirmId = selectedRevisionId;
      return;
    }

    const sheetPath = activeSheetPath;
    const revisionId = selectedRevisionId;
    historyRestoring = true;
    historyMessage = "Restoring safely…";
    try {
      content = await restoreSheetRevision(libraryPath, sheetPath, revisionId);
      dirty = false;
      saveStatus = "Restored from History";
      cursorPosition = 0;
      await reloadLibrary(sheetPath, true);
      const key = sessionSheetKey();
      const restoredWords = wordCount(content);
      sessionBaselines.set(key, restoredWords);
      sessionCounts.set(key, restoredWords);
      recomputeSessionWords();
      await refreshHistory(revisionId);
      historyMessage = "Restored. The version it replaced is now in History too.";
      scheduleAutomaticSync();
      errorMessage = "";
    } catch (error) {
      historyMessage = error instanceof Error ? error.message : String(error);
    } finally {
      historyRestoring = false;
      restoreConfirmId = null;
    }
  }

  function formatRevisionDate(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return "Saved revision";
    return date.toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
      hour: "numeric",
      minute: "2-digit",
    });
  }

  function revisionDeltaLabel(delta: number): string {
    if (delta === 0) return "same length";
    return `${delta > 0 ? "+" : ""}${delta.toLocaleString()} words`;
  }

  async function initializeDesktopWorkspace(): Promise<void> {
    const workspace = reopenLastWorkspace ? loadLastWorkspace() : null;
    let inbox: LibrarySnapshot | null = null;
    loadingLibrary = true;
    try {
      inbox = await openInboxLibrary();
      inboxPath = inbox.path;
      inboxSheetCount = inbox.sheets.length;
      if (workspace) {
        try {
          await activateLibrary(await openLibraryPath(workspace.projectPath), workspace.sheetPath);
          return;
        } catch (error) {
          await activateInbox(
            inbox,
            localStorage.getItem("writing-environment.last-inbox-sheet"),
          );
          errorMessage = `Could not reopen the last project; Inbox is ready instead: ${errorText(error)}`;
          return;
        }
      }
      await activateInbox(
        inbox,
        localStorage.getItem("writing-environment.last-inbox-sheet"),
      );
    } catch (error) {
      if (workspace) {
        try {
          await activateLibrary(await openLibraryPath(workspace.projectPath), workspace.sheetPath);
          errorMessage = `Inbox is unavailable: ${errorText(error)}`;
          return;
        } catch {
          // Report the original Inbox failure below; there is no usable workspace.
        }
      }
      clearWorkspace();
      errorMessage = `Cannot prepare Inbox: ${errorText(error)}`;
    } finally {
      loadingLibrary = false;
    }
  }

  function setReopenLastWorkspace(enabled: boolean): void {
    reopenLastWorkspace = enabled;
    localStorage.setItem("writing-environment.reopen-last-workspace", String(enabled));
    if (enabled) {
      rememberLastWorkspace();
    } else {
      localStorage.removeItem("writing-environment.last-workspace");
      saveProjects();
    }
  }

  function rememberLastWorkspace(): void {
    if (!reopenLastWorkspace || !libraryPath || inboxActive) return;
    const workspace: LastWorkspace = { projectPath: libraryPath, sheetPath: activeSheetPath };
    localStorage.setItem("writing-environment.last-workspace", JSON.stringify(workspace));
  }

  function loadLastWorkspace(): LastWorkspace | null {
    try {
      const value: unknown = JSON.parse(
        localStorage.getItem("writing-environment.last-workspace") ?? "null",
      );
      if (
        typeof value === "object"
        && value !== null
        && "projectPath" in value
        && typeof value.projectPath === "string"
        && "sheetPath" in value
        && (typeof value.sheetPath === "string" || value.sheetPath === null)
      ) {
        return value as LastWorkspace;
      }
    } catch {
      // Ignore malformed local preferences.
    }
    return null;
  }

  async function refreshUniversalTrash(): Promise<void> {
    if (!desktopAvailable()) {
      trashItems = [];
      return;
    }
    const origins = universalTrashOrigins();
    trashItems = origins.length > 0 ? await listUniversalTrash(origins) : [];
    if (
      trashOriginFilter !== "all"
      && !origins.some((origin) => origin.id === trashOriginFilter)
    ) {
      trashOriginFilter = "all";
    }
  }

  function universalTrashOrigins(): TrashOrigin[] {
    const origins: TrashOrigin[] = [];
    const seen = new Set<string>();
    if (inboxPath) {
      seen.add(inboxPath);
      origins.push({ id: "inbox", name: "Inbox", path: inboxPath, kind: "inbox" });
    }
    for (const project of projects) {
      if (seen.has(project.path)) continue;
      seen.add(project.path);
      origins.push({
        id: project.id,
        name: project.name,
        path: project.path,
        kind: "project",
      });
    }
    return origins;
  }

  function originForTrashItem(item: UniversalTrashItem): TrashOrigin {
    return {
      id: item.originId,
      name: item.originName,
      path: item.originPath,
      kind: item.originKind,
    };
  }

  function handleSearchInput(value: string): void {
    searchQuery = value;
    if (searchTimer) clearTimeout(searchTimer);
    if (!value.trim()) {
      searchResults = [];
      searching = false;
      return;
    }

    if (!libraryPath || !desktopAvailable()) {
      const terms = value.toLowerCase().split(/\s+/).filter(Boolean);
      searchResults = sheets.filter((sheet) => {
        const text = `${sheet.title} ${sheet.excerpt}`.toLowerCase();
        return terms.every((term) => text.includes(term));
      });
      return;
    }

    searching = true;
    const requestedQuery = value;
    searchTimer = setTimeout(async () => {
      try {
        const results = await searchLibrary(libraryPath!, requestedQuery);
        if (searchQuery === requestedQuery) searchResults = results;
        errorMessage = "";
      } catch (error) {
        errorMessage = error instanceof Error ? error.message : String(error);
      } finally {
        if (searchQuery === requestedQuery) searching = false;
      }
    }, 180);
  }

  function openSheetDialog(mode: SheetDialogMode, sheet: SheetSummary | null = null): void {
    if (!libraryPath || !desktopAvailable()) {
      errorMessage = "Sheet management is available after opening a project in the desktop app.";
      return;
    }
    sheetActionsPath = null;
    sheetDialogMode = mode;
    dialogSheet = sheet;
    dialogTitle = mode === "rename" ? sheet?.title ?? "" : "";
    dialogGroup = inboxActive && mode === "create"
      ? "Ungrouped"
      : sheet ? sheetFolder(sheet)
      : activeGroup !== "All Sheets" && activeGroup !== "Trash"
        ? activeGroup
        : folders[0]?.path ?? "Draft";
    dialogProjectPath = inboxActive
      ? sortedProjects.find((project) => project.open)?.path ?? ""
      : libraryPath;
    dialogFolders = inboxActive ? [] : folders;
    dialogError = "";
    if (mode === "move") {
      if (!dialogProjectPath) {
        dialogError = "Open a project before moving an Inbox sheet.";
      } else {
        void loadDialogProjectFolders(dialogProjectPath);
      }
    }
  }

  async function loadDialogProjectFolders(projectPath: string): Promise<void> {
    dialogProjectPath = projectPath;
    try {
      const snapshot = await openLibraryPath(projectPath);
      if (dialogProjectPath !== projectPath || sheetDialogMode !== "move") return;
      dialogFolders = folderSummaries(snapshot.sheets);
      if (inboxActive && dialogGroup === "Ungrouped") {
        dialogGroup = dialogFolders[0]?.path ?? "Draft";
      }
      dialogError = "";
    } catch (error) {
      if (dialogProjectPath === projectPath) {
        dialogFolders = [];
        dialogError = `Cannot inspect the destination project: ${errorText(error)}`;
      }
    }
  }

  function closeSheetDialog(): void {
    if (mutatingLibrary) return;
    sheetDialogMode = null;
    dialogSheet = null;
    dialogError = "";
  }

  async function submitSheetDialog(): Promise<void> {
    if (!libraryPath || !sheetDialogMode) return;
    mutatingLibrary = true;
    dialogError = "";

    try {
      if (dirty && !(await persistCurrentSheet())) {
        throw new Error("Resolve the current sheet’s unsaved changes before modifying the library.");
      }
      if (sheetDialogMode === "create") {
        const created = await createLibrarySheet(
          libraryPath,
          inboxActive ? "Ungrouped" : dialogGroup,
          dialogTitle,
        );
        dirty = false;
        await reloadLibrary(created.relativePath, true);
      } else if (sheetDialogMode === "rename" && dialogSheet) {
        const renamed = await renameLibrarySheet(
          libraryPath,
          dialogSheet.relativePath,
          dialogTitle,
        );
        const wasActive = activeSheetPath === dialogSheet.relativePath;
        dirty = false;
        await reloadLibrary(wasActive ? renamed.relativePath : activeSheetPath, wasActive);
      } else if (sheetDialogMode === "move" && dialogSheet) {
        const sourceWasInbox = inboxActive;
        const sourceProjectPath = libraryPath;
        const destinationProjectPath = dialogProjectPath || sourceProjectPath;
        if (inboxActive && !dialogProjectPath) {
          throw new Error("Open a destination project before moving this Inbox sheet.");
        }
        const moved = destinationProjectPath === sourceProjectPath
          ? await moveLibrarySheet(sourceProjectPath, dialogSheet.relativePath, dialogGroup)
          : await moveLibrarySheetToProject(
            sourceProjectPath,
            dialogSheet.relativePath,
            destinationProjectPath,
            dialogGroup,
          );
        const wasActive = activeSheetPath === dialogSheet.relativePath;
        dirty = false;
        if (destinationProjectPath === sourceProjectPath) {
          await reloadLibrary(wasActive ? moved.relativePath : activeSheetPath, wasActive);
        } else {
          if (sourceWasInbox) inboxSheetCount = Math.max(0, inboxSheetCount - 1);
          await activateLibrary(
            await openLibraryPath(destinationProjectPath),
            moved.relativePath,
          );
        }
      } else if (sheetDialogMode === "trash" && dialogSheet) {
        const wasActive = activeSheetPath === dialogSheet.relativePath;
        await trashLibrarySheet(libraryPath, dialogSheet.relativePath);
        dirty = false;
        await reloadLibrary(wasActive ? null : activeSheetPath, false);
      }
      if (!inboxActive) scheduleAutomaticSync();
      sheetDialogMode = null;
      dialogSheet = null;
      errorMessage = "";
    } catch (error) {
      dialogError = error instanceof Error ? error.message : String(error);
    } finally {
      mutatingLibrary = false;
    }
  }

  async function duplicateSheet(sheet: SheetSummary): Promise<void> {
    if (!libraryPath) return;
    sheetActionsPath = null;
    mutatingLibrary = true;
    try {
      if (dirty && !(await persistCurrentSheet())) {
        throw new Error("Resolve the current sheet’s unsaved changes before duplicating another sheet.");
      }
      const duplicated = await duplicateLibrarySheet(
        libraryPath,
        sheet.relativePath,
        `Copy of ${sheet.title}`,
      );
      dirty = false;
      await reloadLibrary(duplicated.relativePath, true);
      if (!inboxActive) scheduleAutomaticSync();
      errorMessage = "";
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
    } finally {
      mutatingLibrary = false;
    }
  }

  async function restoreTrashItem(item: UniversalTrashItem): Promise<void> {
    mutatingLibrary = true;
    try {
      const restored = await restoreUniversalTrash(
        originForTrashItem(item),
        item.id,
        !item.originAvailable,
      );
      if (restored.restoredToInbox || item.originKind === "inbox") {
        await activateInbox(await openInboxLibrary(), restored.sheet.relativePath);
      } else {
        await activateLibrary(
          await openLibraryPath(restored.root),
          restored.sheet.relativePath,
        );
      }
      errorMessage = "";
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
    } finally {
      mutatingLibrary = false;
    }
  }

  function openEmptyTrashConfirmation(): void {
    if (filteredTrashItems.length === 0 || mutatingLibrary) return;
    emptyTrashError = "";
    emptyTrashConfirmVisible = true;
  }

  function closeEmptyTrashConfirmation(): void {
    if (mutatingLibrary) return;
    emptyTrashConfirmVisible = false;
    emptyTrashError = "";
  }

  async function emptyTrash(): Promise<void> {
    if (filteredTrashItems.length === 0 || mutatingLibrary) return;
    mutatingLibrary = true;
    emptyTrashError = "";
    try {
      await emptyUniversalTrashItems(
        universalTrashOrigins(),
        trashOriginFilter === "all" ? null : trashOriginFilter,
      );
      await refreshUniversalTrash();
      emptyTrashConfirmVisible = false;
      errorMessage = "";
    } catch (error) {
      emptyTrashError = error instanceof Error ? error.message : String(error);
    } finally {
      mutatingLibrary = false;
    }
  }

  async function reloadLibrary(
    preferredSheetPath: string | null,
    forceRead: boolean,
  ): Promise<void> {
    if (!libraryPath) return;
    const snapshot = await openLibraryPath(libraryPath);
    libraryName = snapshot.name;
    sheets = snapshot.sheets;
    folders = folderSummaries(snapshot.sheets);
    if (inboxActive) {
      libraryName = "Inbox";
      inboxSheetCount = snapshot.sheets.length;
      folders = [];
    }
    await refreshUniversalTrash();
    if (searchQuery.trim()) handleSearchInput(searchQuery);

    const preferred = preferredSheetPath
      ? snapshot.sheets.find((sheet) => sheet.relativePath === preferredSheetPath)
      : null;
    const target = preferred ?? snapshot.sheets[0];
    if (target) {
      if (forceRead || target.relativePath !== activeSheetPath) {
        await selectSheet(target, true);
      } else {
        activeSheet = target.title;
        if (!trashActive) activeGroup = inboxActive ? "Inbox" : sheetFolder(target);
      }
    } else {
      clearEditorForEmptyLibrary();
    }
  }

  function rememberProject(selected: LibrarySnapshot, sheetPath: string | null): void {
    const existing = projects.find((project) => project.path === selected.path);
    const identity = selected.projectId ?? existing?.id ?? createProjectId();
    if (existing && existing.id !== identity) {
      migrateSyncTargetIdentity(existing.id, identity);
    }
    const bookmark: ProjectBookmark = {
      id: identity,
      name: selected.name,
      path: selected.path,
      pinned: existing?.pinned ?? false,
      open: true,
      lastOpened: Date.now(),
      lastSheetPath: sheetPath,
    };
    const others = projects.filter((project) => project.path !== selected.path);

    projects = [bookmark, ...others];
    saveProjects();
    migrateLegacySyncPreferences();
  }

  function rememberActiveProjectSheet(sheetPath: string | null): void {
    if (!libraryPath || inboxActive) return;
    projects = projects.map((project) =>
      project.path === libraryPath ? { ...project, lastSheetPath: sheetPath } : project,
    );
    saveProjects();
  }

  function toggleProjectPin(path: string): void {
    projects = projects.map((project) =>
      project.path === path ? { ...project, pinned: !project.pinned } : project,
    );
    saveProjects();
  }

  function saveProjects(): void {
    const storedProjects = reopenLastWorkspace
      ? projects
      : projects.map((project) => ({ ...project, open: false }));
    localStorage.setItem("writing-environment.projects", JSON.stringify(storedProjects));
  }

  function loadStoredProjects(): ProjectBookmark[] {
    try {
      const value: unknown = JSON.parse(localStorage.getItem("writing-environment.projects") ?? "[]");
      if (!Array.isArray(value)) return [];
      const seen = new Set<string>();
      const stored: ProjectBookmark[] = [];
      for (const project of value) {
        if (
          typeof project !== "object" ||
          project === null ||
          typeof project.name !== "string" ||
          typeof project.path !== "string" ||
          typeof project.pinned !== "boolean" ||
          typeof project.lastOpened !== "number" ||
          seen.has(project.path)
        ) continue;

        seen.add(project.path);
        stored.push({
          id: "id" in project && typeof project.id === "string" && project.id
            ? project.id
            : createProjectId(),
          name: project.name,
          path: project.path,
          pinned: project.pinned,
          open: "open" in project && typeof project.open === "boolean" ? project.open : false,
          lastOpened: project.lastOpened,
          lastSheetPath: "lastSheetPath" in project
            && (typeof project.lastSheetPath === "string" || project.lastSheetPath === null)
            ? project.lastSheetPath
            : null,
        });
      }
      return stored;
    } catch {
      return [];
    }
  }

  function createProjectId(): string {
    return globalThis.crypto?.randomUUID?.()
      ?? `project-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
  }

  function libraryWarningMessage(snapshot: LibrarySnapshot): string {
    const warnings = snapshot.warnings ?? [];
    if (warnings.length === 0) return "";
    const listed = warnings.slice(0, 3).join("; ");
    const remainder = warnings.length - 3;
    return `Opened ${snapshot.name}, but skipped ${warnings.length} unreadable Markdown ${warnings.length === 1 ? "file" : "files"}: ${listed}${remainder > 0 ? `; and ${remainder} more` : ""}`;
  }

  function isSheetSort(value: string | null): value is SheetSort {
    return value === "created-desc"
      || value === "created-asc"
      || value === "title-asc"
      || value === "title-desc";
  }

  function setSheetSort(value: SheetSort): void {
    sheetSort = value;
    sortMenuVisible = false;
    localStorage.setItem("writing-environment.sheet-sort", value);
  }

  function sortSheets(source: SheetSummary[], order: SheetSort): SheetSummary[] {
    const sorted = [...source];
    sorted.sort((left, right) => {
      let result = 0;
      if (order.startsWith("created")) {
        result = (Date.parse(left.createdAt) || 0) - (Date.parse(right.createdAt) || 0);
        if (order === "created-desc") result *= -1;
      } else {
        result = left.title.localeCompare(right.title, undefined, {
          numeric: true,
          sensitivity: "base",
        });
        if (order === "title-desc") result *= -1;
      }

      return result || left.relativePath.localeCompare(right.relativePath);
    });
    return sorted;
  }

  function sheetSortLabel(value: SheetSort): string {
    return value === "created-desc"
      ? "Newest first"
      : value === "created-asc"
        ? "Oldest first"
        : value === "title-asc"
          ? "Title A–Z"
          : "Title Z–A";
  }

  function sheetFolder(sheet: SheetSummary): string {
    const separator = sheet.relativePath.lastIndexOf("/");
    return separator < 0 ? "Ungrouped" : sheet.relativePath.slice(0, separator);
  }

  function sheetIsInFolder(sheet: SheetSummary, folder: string): boolean {
    const sheetPath = sheetFolder(sheet);
    if (folder === "Ungrouped") return sheetPath === folder;
    return sheetPath === folder || sheetPath.startsWith(`${folder}/`);
  }

  function folderSummaries(source: SheetSummary[]): FolderSummary[] {
    const counts = new Map<string, number>();
    for (const sheet of source) {
      const folder = sheetFolder(sheet);
      if (folder === "Ungrouped") {
        counts.set(folder, (counts.get(folder) ?? 0) + 1);
        continue;
      }

      const segments = folder.split("/");
      for (let index = 1; index <= segments.length; index += 1) {
        const path = segments.slice(0, index).join("/");
        counts.set(path, (counts.get(path) ?? 0) + 1);
      }
    }

    return Array.from(counts.entries())
      .sort(([left], [right]) => {
        if (left === "Ungrouped") return -1;
        if (right === "Ungrouped") return 1;
        return left.localeCompare(right, undefined, { numeric: true, sensitivity: "base" });
      })
      .map(([path, count]) => {
        const segments = path === "Ungrouped" ? [] : path.split("/");
        return {
          path,
          name: path === "Ungrouped" ? "Project Root" : segments.at(-1)!,
          depth: Math.max(0, segments.length - 1),
          count,
        };
      });
  }

  function formatTrashDate(value: string): string {
    const date = new Date(value);
    return Number.isNaN(date.getTime())
      ? "Recently removed"
      : date.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
  }

  function wordCount(text: string): number {
    const body = text.replace(/^---\r?\n[\s\S]*?\r?\n---(?:\r?\n|$)/, "").trim();
    return body ? body.split(/\s+/).length : 0;
  }

  function sessionSheetKey(): string {
    return `${libraryPath ?? "prototype"}::${activeSheetPath ?? activeSheet}`;
  }

  function registerSessionSheet(): void {
    const key = sessionSheetKey();
    const count = wordCount(content);
    if (!sessionBaselines.has(key)) sessionBaselines.set(key, count);
    sessionCounts.set(key, count);
    recomputeSessionWords();
  }

  function updateSessionCount(): void {
    const key = sessionSheetKey();
    if (!sessionBaselines.has(key)) sessionBaselines.set(key, wordCount(content));
    sessionCounts.set(key, wordCount(content));
    recomputeSessionWords();
  }

  function recomputeSessionWords(): void {
    sessionWords = Array.from(sessionCounts.entries()).reduce(
      (total, [key, count]) => total + Math.max(0, count - (sessionBaselines.get(key) ?? count)),
      0,
    );
  }

  function isWritingFocusMode(value: string | null): value is WritingFocusMode {
    return value === "off" || value === "paragraph" || value === "sentence";
  }

  function buildFocusSegments(
    text: string,
    position: number,
    mode: WritingFocusMode,
  ): FocusSegment[] {
    if (!text) return [{ text: " ", active: true }];
    if (mode === "off") return [{ text, active: true }];

    const paragraph = paragraphRange(text, position);
    const activeRange = mode === "sentence" ? sentenceRange(text, position, paragraph) : paragraph;
    const segments: FocusSegment[] = [];

    if (activeRange.start > 0) segments.push({ text: text.slice(0, activeRange.start), active: false });
    segments.push({ text: text.slice(activeRange.start, activeRange.end), active: true });
    if (activeRange.end < text.length) {
      segments.push({ text: text.slice(activeRange.end), active: false });
    }

    return segments;
  }

  function paragraphRange(text: string, position: number): { start: number; end: number } {
    const cursor = Math.min(text.length, Math.max(0, position));
    const before = text.slice(0, cursor);
    const separators = Array.from(before.matchAll(/\n\s*\n/g));
    const lastSeparator = separators.at(-1);
    const start = lastSeparator?.index === undefined
      ? 0
      : lastSeparator.index + lastSeparator[0].length;
    const nextSeparator = /\n\s*\n/.exec(text.slice(cursor));
    const end = nextSeparator?.index === undefined ? text.length : cursor + nextSeparator.index;
    return { start, end: Math.max(start, end) };
  }

  function sentenceRange(
    text: string,
    position: number,
    paragraph: { start: number; end: number },
  ): { start: number; end: number } {
    const paragraphText = text.slice(paragraph.start, paragraph.end);
    const localCursor = Math.min(paragraphText.length, Math.max(0, position - paragraph.start));
    const boundary = /[.!?](?:["'”’\)\]])?(?=\s|$)/g;
    let start = 0;

    for (const match of paragraphText.matchAll(boundary)) {
      const end = (match.index ?? 0) + match[0].length;
      if (localCursor <= end) return { start: paragraph.start + start, end: paragraph.start + end };
      start = end;
      while (/\s/.test(paragraphText[start] ?? "")) start += 1;
    }

    return { start: paragraph.start + start, end: paragraph.end };
  }
</script>

<svelte:head>
  <title>{activeSheet} — Writing Environment</title>
</svelte:head>

<svelte:window onkeydown={handleWindowKeydown} onclick={handleWindowClick} />

<main
  class:library-hidden={!libraryVisible}
  class:sheets-hidden={!sheetsVisible}
  class:focus-mode={!libraryVisible && !sheetsVisible}
  class:app-fullscreen={appFullscreen}
  class:workspace-empty={desktopMode && !libraryPath}
>
  <aside class="library" aria-label="Library">
    <div class="brand">
      <span class="brand-mark" aria-hidden="true">W</span>
      <span>Writing Environment</span>
    </div>

    <section class="inbox-section" aria-label="Inbox">
      <button
        class:active={inboxActive && !trashActive}
        class="nav-row inbox-row"
        disabled={loadingLibrary}
        title={inboxPath ?? "Universal Inbox"}
        onclick={() => void openInbox()}
      >
        <span class="nav-icon" aria-hidden="true">▱</span>
        <span>Inbox</span>
        <span class="count">{inboxSheetCount}</span>
      </button>
    </section>

    <section class="projects-section" aria-label="Projects">
      <div class="section-heading">
        <p class="eyebrow">Projects</p>
        <button
          class="add-project"
          disabled={loadingLibrary}
          aria-label="Open project folder"
          title="Open project folder"
          onclick={openLibraryFolder}
        >＋</button>
      </div>

      {#if sidebarProjects.length > 0}
        <div class="project-list">
          {#each sidebarProjects as project}
            <div class="project-entry">
              <div
                class:active={activeProjectPath === project.path}
                class="project-row"
                role="group"
                oncontextmenu={(event) => {
                  if (desktopMode) openProjectMenu(event, project);
                }}
              >
                <button
                  class="project-open"
                  title={project.path}
                  onclick={() => void openProject(project)}
                >
                  <span aria-hidden="true">{activeProjectPath === project.path ? "▾" : project.open ? "▱" : "◇"}</span>
                  <span>{project.name}</span>
                </button>
                <button
                  class:pinned={project.pinned}
                  class="pin-project"
                  aria-label={`${project.pinned ? "Unpin" : "Pin"} ${project.name}`}
                  aria-pressed={project.pinned}
                  title={project.pinned ? "Unpin project" : "Pin project"}
                  onclick={() => {
                    toggleProjectPin(project.path);
                    projectMenuPath = null;
                  }}
                >{project.pinned ? "★" : "☆"}</button>
                {#if projectMenuPath === project.path}
                  <div
                    class="project-context-menu"
                    role="menu"
                    aria-label={`Actions for ${project.name}`}
                    style={`left: ${projectMenuX}px; top: ${projectMenuY}px;`}
                  >
                    {#if activeProjectPath !== project.path}
                      <button role="menuitem" onclick={() => void openProject(project)}>{project.open ? "Switch to Project" : "Open Project"}</button>
                    {/if}
                    <button
                      role="menuitem"
                      onclick={() => {
                        toggleProjectPin(project.path);
                        projectMenuPath = null;
                      }}
                    >{project.pinned ? "Remove from Favorites" : "Add to Favorites"}</button>
                    {#if project.open}
                      <div></div>
                      <button
                        class="danger-action"
                        role="menuitem"
                        disabled={syncRunning || loadingLibrary}
                        onclick={() => void closeProject(project)}
                      >Close Project</button>
                    {/if}
                  </div>
                {/if}
              </div>
              {#if activeProjectPath === project.path}
                <nav class="project-folder-tree" aria-label={`${project.name} folders`}>
                  <button
                    class:active={activeGroup === "All Sheets"}
                    class="project-tree-row"
                    onclick={() => selectFolder("All Sheets")}
                  >
                    <span aria-hidden="true">◫</span>
                    <span>All Sheets</span>
                    <span class="count">{sheets.length}</span>
                  </button>
                  {#each folders as folder}
                    <button
                      class:active={activeGroup === folder.path}
                      class="project-tree-row folder-row"
                      style={`--folder-depth: ${folder.depth}`}
                      title={folder.path === "Ungrouped" ? "Sheets in the project root" : folder.path}
                      onclick={() => selectFolder(folder.path)}
                    >
                      <span aria-hidden="true">▱</span>
                      <span>{folder.name}</span>
                      <span class="count">{folder.count}</span>
                    </button>
                  {/each}
                </nav>
              {/if}
            </div>
          {/each}
        </div>
      {:else}
        <p class="projects-empty">No project is open. Favorite projects stay here for quick access.</p>
      {/if}
    </section>

    <section class="trash-section" aria-label="Trash">
      <button
        class:active={trashActive}
        class="nav-row trash-row"
        disabled={loadingLibrary || mutatingLibrary}
        onclick={() => void selectUniversalTrash()}
      >
        <span class="nav-icon" aria-hidden="true">♲</span>
        <span>Trash</span>
        <span class="count">{trashItems.length}</span>
      </button>
    </section>

    <div class="library-footer">
      <span class:error-dot={Boolean(errorMessage)} class="status-dot"></span>
      <span title={errorMessage || libraryPath || "Browser prototype"}>
        {errorMessage || `${libraryName} · offline ready`}
      </span>
    </div>
  </aside>

  <section class="sheet-list" aria-label="Sheets">
    <header class="panel-header">
      <div>
        <p class="eyebrow">{trashActive ? "Recovery" : searchQuery.trim() ? (inboxActive ? "Inbox" : "Project") : inboxActive ? "Library" : "Folder"}</p>
        <h1>{searchQuery.trim() ? "Search" : activeGroup}</h1>
      </div>
      <div class="panel-header-actions">
        {#if trashActive}
          <select
            class="trash-origin-filter"
            aria-label="Filter Trash by origin"
            value={trashOriginFilter}
            onchange={(event) => (trashOriginFilter = event.currentTarget.value)}
          >
            <option value="all">All origins</option>
            {#each universalTrashOrigins() as origin}
              <option value={origin.id}>{origin.name}</option>
            {/each}
          </select>
          <button
            class="empty-trash-button"
            disabled={filteredTrashItems.length === 0 || mutatingLibrary}
            title={filteredTrashItems.length === 0 ? "This Trash view is already empty" : "Permanently empty this Trash view"}
            onclick={openEmptyTrashConfirmation}
          >Empty Trash</button>
        {:else}
          <div class="sheet-sort-control">
            <button
              class:active={sortMenuVisible}
              class="icon-button sheet-sort-button"
              aria-label={`Sort sheets: ${sheetSortLabel(sheetSort)}`}
              aria-haspopup="menu"
              aria-expanded={sortMenuVisible}
              title={`Sort sheets: ${sheetSortLabel(sheetSort)}`}
              onclick={() => (sortMenuVisible = !sortMenuVisible)}
            >⇅</button>
            {#if sortMenuVisible}
              <div class="sheet-sort-menu" role="menu" aria-label="Sort sheets">
                <p class="eyebrow">Sort sheets</p>
                {#each [
                  { id: "created-desc", label: "Newest first", symbol: "↓" },
                  { id: "created-asc", label: "Oldest first", symbol: "↑" },
                  { id: "title-asc", label: "Title A–Z", symbol: "A" },
                  { id: "title-desc", label: "Title Z–A", symbol: "Z" },
                ] as option}
                  <button
                    class:active={sheetSort === option.id}
                    role="menuitemradio"
                    aria-checked={sheetSort === option.id}
                    onclick={() => setSheetSort(option.id as SheetSort)}
                  >
                    <span aria-hidden="true">{option.symbol}</span>
                    <strong>{option.label}</strong>
                    {#if sheetSort === option.id}<span class="check">✓</span>{/if}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
          <button
            class="icon-button"
            aria-label="Create sheet"
            title={libraryPath ? "Create sheet" : "Open a project to create sheets"}
            disabled={!libraryPath || mutatingLibrary}
            onclick={() => openSheetDialog("create")}
          >＋</button>
        {/if}
      </div>
    </header>

    {#if !trashActive}
      <div class="library-search">
        <span aria-hidden="true">⌕</span>
        <input
          type="search"
          placeholder={inboxActive ? "Search Inbox" : "Search every sheet"}
          aria-label="Search library"
          value={searchQuery}
          oninput={(event) => handleSearchInput(event.currentTarget.value)}
        />
        {#if searching}<span class="searching" aria-label="Searching">•••</span>{/if}
      </div>
    {/if}

    <div class:trash-view={trashActive} class="sheets">
      {#if trashActive}
        {#each filteredTrashItems as item}
          <article class="trash-card">
            <strong>{item.title}</strong>
            <span>{item.originName} · {item.originalRelativePath}</span>
            {#if !item.originAvailable}
              <small class="trash-origin-warning">Original folder unavailable</small>
            {/if}
            <div>
              <small>{formatTrashDate(item.trashedAt)}</small>
              <button disabled={mutatingLibrary} onclick={() => void restoreTrashItem(item)}>{item.originAvailable ? "Restore" : "Restore to Inbox"}</button>
            </div>
          </article>
        {/each}
        {#if filteredTrashItems.length === 0}
          <p class="empty-state">{trashOriginFilter === "all" ? "Trash is empty. Removed sheets will remain recoverable here." : "No removed sheets from this location."}</p>
        {/if}
      {:else}
        {#each visibleSheets as sheet}
          <div class:active={activeSheetPath === sheet.relativePath} class="sheet-card-wrap">
            <button
              class="sheet-card"
              onclick={() => void selectSheet(sheet)}
            >
              <strong>{sheet.title}</strong>
              <span class="excerpt">{sheet.excerpt}</span>
              <span class="sheet-meta">{sheet.wordCount.toLocaleString()} words · {sheetFolder(sheet)}</span>
            </button>
            {#if libraryPath}
              <button
                class="sheet-actions-button"
                aria-label={`Actions for ${sheet.title}`}
                aria-haspopup="menu"
                aria-expanded={sheetActionsPath === sheet.relativePath}
                onclick={() => (sheetActionsPath = sheetActionsPath === sheet.relativePath ? null : sheet.relativePath)}
              >•••</button>
              {#if sheetActionsPath === sheet.relativePath}
                <div class="sheet-actions-menu" role="menu">
                  <button role="menuitem" onclick={() => openSheetDialog("rename", sheet)}>Rename</button>
                  <button role="menuitem" onclick={() => void duplicateSheet(sheet)}>Duplicate</button>
                  <button role="menuitem" onclick={() => openSheetDialog("move", sheet)}>{inboxActive ? "Move to project…" : "Move to folder or project…"}</button>
                  <div></div>
                  <button class="danger-action" role="menuitem" onclick={() => openSheetDialog("trash", sheet)}>Move to Trash</button>
                </div>
              {/if}
            {/if}
          </div>
        {/each}
        {#if visibleSheets.length === 0}
          <p class="empty-state">{searchQuery.trim() ? "No sheets match this search." : "No Markdown sheets in this group."}</p>
        {/if}
      {/if}
    </div>
  </section>

  <section class="editor-shell" aria-label="Editor">
    <header
      class="editor-toolbar"
      role="toolbar"
      aria-label="Writing controls"
      tabindex="-1"
    >
      <div class="toolbar-group">
        <button
          class:active={!libraryVisible}
          class="icon-button"
          aria-label="Toggle library"
          title="Toggle library"
          onclick={() => (libraryVisible = !libraryVisible)}
        >
          ◧
        </button>
        <button
          class:active={!sheetsVisible}
          class="icon-button"
          aria-label="Toggle sheet list"
          title="Toggle sheet list"
          onclick={() => (sheetsVisible = !sheetsVisible)}
        >
          ▤
        </button>
      </div>

      <div class="document-title">{activeSheet}</div>

      <div class="toolbar-group toolbar-end">
        <button
          class="history-button"
          disabled={!libraryPath || !activeSheetPath || historyLoading}
          aria-label="Open sheet history"
          title="Sheet history"
          onclick={() => void openHistory()}
        >
          <span class="history-symbol" aria-hidden="true">◷</span>
          <span>History</span>
        </button>

        <div class="sync-control">
          <button
            class:active={syncMenuVisible || syncPhase === "syncing" || syncPhase === "conflict"}
            class:error={syncPhase === "error"}
            class="sync-button"
            aria-label={`Universal sync: ${syncStatus}`}
            aria-haspopup="dialog"
            aria-expanded={syncMenuVisible}
            title={`Universal sync: ${syncStatus}`}
            onclick={openSyncMenu}
          >
            <span class="sync-symbol" aria-hidden="true">↕</span>
            <span>Sync</span>
            <small>{syncRunning
              ? "Working"
              : universalSyncTargets.some((target) => target.included && target.recoveryTarget)
                ? "Paused"
                : universalSyncConfig.automatic
                  ? "Auto"
                  : syncNeedsInitialization
                    ? "Setup"
                    : universalSyncConfigured() ? "Ready" : "Off"}</small>
          </button>

          {#if syncMenuVisible}
            <div class="sync-menu universal-sync-menu" role="dialog" aria-label="Universal sync">
              <div class="sync-menu-heading">
                <div>
                  <p class="eyebrow">Universal sync</p>
                  <strong>{syncStatus}</strong>
                </div>
                <span class:working={syncRunning} class:error={syncPhase === "error"} class:conflict={syncPhase === "conflict"} class="sync-indicator"></span>
              </div>

              <label for="sync-remote">rclone remote</label>
              <div class="sync-field-row">
                <select
                  id="sync-remote"
                  value={syncDraftRemote}
                  disabled={syncRunning || !syncAvailability?.compatible}
                  onchange={(event) => (syncDraftRemote = event.currentTarget.value)}
                >
                  <option value="">Choose a remote…</option>
                  {#each syncAvailability?.remotes ?? [] as remote}
                    <option value={remote}>{remote}</option>
                  {/each}
                </select>
                <button disabled={refreshingSync || syncRunning} title="Refresh rclone remotes" onclick={() => void refreshSyncAvailability()}>↻</button>
              </div>

              <label for="sync-path">Universal remote root</label>
              <input
                id="sync-path"
                type="text"
                spellcheck="false"
                placeholder="Writing Environment"
                value={syncDraftPath}
                disabled={syncRunning}
                oninput={(event) => (syncDraftPath = event.currentTarget.value)}
              />
              <button
                class="sync-secondary"
                disabled={syncRunning || !syncDraftRemote || !syncDraftPath.trim()}
                onclick={saveUniversalSyncRoot}
              >Save universal root</button>

              <div class="sync-targets">
                <p class="eyebrow">Included locations</p>
                {#each universalSyncTargets as target}
                  <div class:legacy={target.legacy} class:paused={!!target.recoveryTarget} class="sync-target">
                    <label class="sync-target-toggle">
                      <span>
                        <strong>{target.name}</strong>
                        <small>{target.kind === "inbox" ? "Universal Inbox" : target.legacy ? "Preserved existing location" : target.remotePath || "Universal root required"}</small>
                      </span>
                      <input
                        type="checkbox"
                        checked={target.included}
                        disabled={syncRunning}
                        onchange={(event) => void setUniversalTargetIncluded(target, event.currentTarget.checked)}
                      />
                    </label>
                    {#if target.included}
                      <div class="sync-target-state">
                        <span>{target.recoveryTarget ? "Paused for recovery" : target.initialized ? "Initialized" : "First sync required"}</span>
                        {#if target.legacy}
                          <button
                            disabled={syncRunning || !universalSyncConfigured()}
                            onclick={() => void useUniversalLocation(target)}
                          >Use universal location…</button>
                        {/if}
                      </div>
                      {#if target.recoveryTarget}
                        <div class="sync-recovery-note">
                          <strong>Deletion protection stopped this location.</strong>
                          <p>{target.recoveryTarget === "local"
                            ? "Restore only remote files missing locally. Existing local files will not be replaced."
                            : "Restore only local files missing remotely. Existing remote files will not be replaced."}</p>
                          <button
                            class="sync-primary"
                            disabled={syncRunning}
                            onclick={() => void recoverPausedSync(target)}
                          >{target.recoveryTarget === "local" ? "Restore missing local files" : "Restore missing remote files"}</button>
                        </div>
                      {/if}
                    {/if}
                  </div>
                {/each}
              </div>

              <p class:error-text={syncPhase === "error"} class="sync-message">
                {syncMessage || "Credentials remain in rclone’s own configuration, outside your manuscripts."}
              </p>

              {#if syncNeedsInitialization}
                <p class="sync-safety-note"><strong>First sync:</strong> every new destination must be empty. No initialization starts until you review a separate confirmation.</p>
              {/if}

              <button
                class="sync-primary"
                disabled={syncRunning || !syncAvailability?.compatible || !universalSyncTargets.some((target) => target.included)}
                onclick={() => void runUniversalSync(false)}
              >{syncRunning ? "Syncing…" : syncNeedsInitialization ? "Review and initialize…" : "Sync included now"}</button>

              <label class:disabled={syncNeedsInitialization} class="toggle-setting sync-toggle">
                <span>
                  <strong>Automatic universal sync</strong>
                  <small>Runs included initialized locations sequentially after saves and every five minutes.</small>
                </span>
                <input
                  type="checkbox"
                  checked={universalSyncConfig.automatic}
                  disabled={syncNeedsInitialization || syncRunning || universalSyncTargets.some((target) => target.included && !!target.recoveryTarget)}
                  onchange={(event) => setAutomaticSync(event.currentTarget.checked)}
                />
              </label>
              <p class="setting-note">Each location keeps an isolated rclone state, archive, conflict boundary, and deletion guard.</p>
            </div>
          {/if}
        </div>

        <div class="session-goal-control">
          <button
            class:active={goalMenuVisible || (sessionGoal > 0 && sessionWords >= sessionGoal)}
            class="session-goal-button"
            aria-label={sessionGoal > 0 ? `Session goal: ${sessionWords} of ${sessionGoal} words` : "Session goal: off"}
            aria-haspopup="dialog"
            aria-expanded={goalMenuVisible}
            title="Session word goal"
            onclick={() => {
              goalMenuVisible = !goalMenuVisible;
              sessionGoalDraft = sessionGoal;
              focusMenuVisible = false;
              writerMenuVisible = false;
              themeMenuVisible = false;
              syncMenuVisible = false;
            }}
          >
            <span class="goal-symbol" aria-hidden="true">◔</span>
            <span>Goal</span>
            <small>{sessionGoal > 0 ? `${sessionWords.toLocaleString()} / ${sessionGoal.toLocaleString()}` : "Off"}</small>
          </button>

          {#if goalMenuVisible}
            <div class="session-goal-menu" role="dialog" aria-label="Session word goal">
              <p class="eyebrow">Session goal</p>
              <form
                onsubmit={(event) => {
                  event.preventDefault();
                  applySessionGoal();
                }}
              >
                <label for="session-goal">Words to write this session</label>
                <div class="goal-entry-row">
                  <input
                    id="session-goal"
                    type="number"
                    min="0"
                    max="100000"
                    step="50"
                    value={sessionGoalDraft}
                    aria-label="Session word goal"
                    oninput={(event) => (sessionGoalDraft = Number(event.currentTarget.value))}
                  />
                  <button type="submit">Set goal</button>
                </div>
              </form>
              <div class="goal-summary">
                <progress value={sessionWords} max={sessionGoal || 1}></progress>
                <span>{sessionWords.toLocaleString()} words added</span>
              </div>
              <div class="goal-actions">
                <button onclick={resetSessionProgress}>Reset progress</button>
                {#if sessionGoal > 0}<button onclick={() => setSessionGoal(0)}>Turn goal off</button>{/if}
              </div>
              <p class="setting-note">Your target persists; progress restarts with each app session.</p>
            </div>
          {/if}
        </div>

        <div class="writing-focus-control">
          <button
            class:active={writingFocusMode !== "off" || focusMenuVisible}
            class="writing-focus-button"
            aria-label={`Writing focus: ${writingFocusMode}`}
            aria-haspopup="menu"
            aria-expanded={focusMenuVisible}
            title={`Writing focus: ${writingFocusMode}`}
            onclick={() => {
              focusMenuVisible = !focusMenuVisible;
              goalMenuVisible = false;
              writerMenuVisible = false;
              themeMenuVisible = false;
              syncMenuVisible = false;
            }}
          >
            <span class="focus-symbol" aria-hidden="true">◎</span>
            <span>Focus</span>
            <small>{writingFocusMode === "off" ? "Off" : writingFocusMode === "paragraph" ? "Paragraph" : "Sentence"}</small>
          </button>

          {#if focusMenuVisible}
            <div class="writing-focus-menu" role="menu" aria-label="Writing focus">
              <p class="eyebrow">Writing focus</p>
              {#each [
                { id: "off", name: "Off", description: "Show the full manuscript normally." },
                { id: "paragraph", name: "Paragraph", description: "Emphasize the paragraph at the caret." },
                { id: "sentence", name: "Sentence", description: "Emphasize only the current sentence." },
              ] as mode}
                <button
                  class:active={writingFocusMode === mode.id}
                  class="writing-focus-option"
                  role="menuitemradio"
                  aria-checked={writingFocusMode === mode.id}
                  onclick={() => setWritingFocusMode(mode.id as WritingFocusMode)}
                >
                  <span class="focus-option-mark" aria-hidden="true">{mode.id === "off" ? "○" : mode.id === "paragraph" ? "¶" : "—"}</span>
                  <span>
                    <strong>{mode.name}</strong>
                    <small>{mode.description}</small>
                  </span>
                  {#if writingFocusMode === mode.id}<span class="check">✓</span>{/if}
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <div class="writer-control">
          <button
            class:active={writerMenuVisible}
            class="icon-button writer-button"
            aria-label="Writer appearance"
            aria-haspopup="dialog"
            aria-expanded={writerMenuVisible}
            title="Writer appearance"
            onclick={() => {
              writerMenuVisible = !writerMenuVisible;
              goalMenuVisible = false;
              themeMenuVisible = false;
              focusMenuVisible = false;
              syncMenuVisible = false;
            }}
          >Aa</button>

          {#if writerMenuVisible}
            <div class="writer-menu" role="dialog" aria-label="Writer appearance">
              <p class="eyebrow">Writer</p>
              <div class="setting-heading writer-text-size-heading">
                <label for="editor-text-size">Text size</label>
                <output for="editor-text-size">{editorTextSize} px</output>
              </div>
              <input
                id="editor-text-size"
                type="range"
                min="14"
                max="32"
                step="1"
                value={editorTextSize}
                aria-label="Editor text size"
                oninput={(event) => setEditorTextSize(Number(event.currentTarget.value))}
              />
              <div class="range-labels" aria-hidden="true">
                <span>Small</span>
                <span>Large</span>
              </div>
              <div class="writer-divider"></div>
              <div class="setting-heading">
                <label for="line-height">Line height</label>
                <output for="line-height">{lineHeight.toFixed(2)}</output>
              </div>
              <input
                id="line-height"
                type="range"
                min="1.35"
                max="2.2"
                step="0.05"
                value={lineHeight}
                aria-label="Line height"
                oninput={(event) => setLineHeight(Number(event.currentTarget.value))}
              />
              <div class="range-labels" aria-hidden="true">
                <span>Compact</span>
                <span>Spacious</span>
              </div>
              <div class="writer-divider"></div>
              <div class="setting-heading writer-width-heading">
                <label for="writer-width">Sheet width</label>
                <output for="writer-width">{writerWidth === 100 ? "Full" : `${writerWidth}%`}</output>
              </div>
              <input
                id="writer-width"
                type="range"
                min="50"
                max="100"
                step="1"
                value={writerWidth}
                aria-label="Writer sheet width"
                oninput={(event) => setWriterWidth(Number(event.currentTarget.value))}
              />
              <div class="range-labels" aria-hidden="true">
                <span>Focused</span>
                <span>Full width</span>
              </div>
              <div class="writer-divider"></div>
              <label class="toggle-setting">
                <span>
                  <strong>Check spelling while typing</strong>
                  <small>Use your operating system’s dictionaries and suggestions.</small>
                </span>
                <input
                  type="checkbox"
                  checked={spellCheckEnabled}
                  onchange={(event) => setSpellCheckEnabled(event.currentTarget.checked)}
                />
              </label>
              <label class:disabled={!spellCheckEnabled} class="toggle-setting writer-subsetting">
                <span>
                  <strong>Automatic corrections</strong>
                  <small>Allow the operating system to replace suspected misspellings.</small>
                </span>
                <input
                  type="checkbox"
                  checked={automaticCorrection}
                  disabled={!spellCheckEnabled}
                  onchange={(event) => setAutomaticCorrection(event.currentTarget.checked)}
                />
              </label>
              <div class="writer-divider"></div>
              <label class="toggle-setting">
                <span>
                  <strong>Reopen last workspace</strong>
                  <small>Load the previous project and sheet at launch.</small>
                </span>
                <input
                  type="checkbox"
                  checked={reopenLastWorkspace}
                  onchange={(event) => setReopenLastWorkspace(event.currentTarget.checked)}
                />
              </label>
              <div class="writer-divider"></div>
              <div class="application-update-setting">
                <div class="setting-heading">
                  <span>Application updates</span>
                  <output>v{appVersion}</output>
                </div>
                <button
                  class="check-update-button"
                  disabled={updateChecking || updateInstalling}
                  onclick={() => void checkForAppUpdate(true)}
                >{updateChecking ? "Checking…" : "Check for Updates…"}</button>
                <label class="toggle-setting update-toggle">
                  <span>
                    <strong>Check automatically</strong>
                    <small>Look for signed releases about once a day. Installation always waits for you.</small>
                  </span>
                  <input
                    type="checkbox"
                    checked={automaticUpdateChecks}
                    onchange={(event) => setAutomaticUpdateChecks(event.currentTarget.checked)}
                  />
                </label>
              </div>
            </div>
          {/if}
        </div>

        <div class="theme-control">
          <button
            class="theme-button"
            aria-haspopup="menu"
            aria-expanded={themeMenuVisible}
            onclick={() => {
              themeMenuVisible = !themeMenuVisible;
              goalMenuVisible = false;
              writerMenuVisible = false;
              focusMenuVisible = false;
              syncMenuVisible = false;
            }}
          >
            <span class="theme-swatch"></span>
            {themes.find((theme) => theme.id === activeThemeId)?.name}
          </button>

          {#if themeMenuVisible}
            <div class="theme-menu" role="menu">
              <p class="eyebrow">Visual theme</p>
              {#each themes as theme}
                <button
                  class:active={theme.id === activeThemeId}
                  class="theme-option"
                  data-theme-id={theme.id}
                  role="menuitem"
                  aria-label={`Use ${theme.name} theme`}
                  onclick={() => selectTheme(theme.id)}
                >
                  <span class="theme-preview" data-preview={theme.id}></span>
                  <span>
                    <strong>{theme.name}</strong>
                    <small>{theme.description}</small>
                  </span>
                  {#if theme.id === activeThemeId}<span class="check">✓</span>{/if}
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <button
          class:active={!libraryVisible && !sheetsVisible}
          class="icon-button focus-button"
          aria-label="Toggle distraction-free layout"
          title="Distraction-free layout"
          onclick={() => {
            goalMenuVisible = false;
            focusMenuVisible = false;
            writerMenuVisible = false;
            themeMenuVisible = false;
            syncMenuVisible = false;
            const entering = libraryVisible || sheetsVisible;
            libraryVisible = !entering;
            sheetsVisible = !entering;
          }}
        >
          ⛶
        </button>

        <button
          class:active={appFullscreen}
          class="icon-button fullscreen-button"
          aria-label={appFullscreen ? "Exit full screen" : "Enter full screen"}
          title={appFullscreen ? "Exit full screen (F11)" : "Enter full screen (F11)"}
          onclick={() => void toggleAppFullscreen()}
        >
          ⤢
        </button>
      </div>
    </header>

    <div class="editor-wrap">
      {#if externalConflictVisible}
        <aside class="external-change-banner" role="alert" aria-live="assertive">
          <div>
            <strong>{externalDiskContent === null ? "This sheet was removed outside the app." : "This sheet changed outside the app."}</strong>
            <span>Your local draft is still open and has not been overwritten.</span>
          </div>
          <div class="external-change-actions">
            <button disabled={resolvingExternalConflict} onclick={() => void keepBothExternalVersions()}>
              {resolvingExternalConflict ? "Preserving…" : "Keep both"}
            </button>
            <button disabled={resolvingExternalConflict} onclick={() => void useExternalDiskVersion()}>
              {externalDiskContent === null ? "Discard local draft" : "Use disk version"}
            </button>
          </div>
        </aside>
      {/if}
      {#if desktopMode && !libraryPath}
        <div class="no-workspace">
          <span aria-hidden="true">◇</span>
          <strong>No project open</strong>
          <p>Open a project folder from the Projects sidebar to begin writing.</p>
        </div>
      {:else}
      <div class="editor-stage">
        {#if writingFocusMode !== "off"}
          <pre class="focus-overlay" aria-hidden="true" bind:this={focusOverlay}>{#each focusSegments as segment}<span class:active={segment.active}>{segment.text}</span>{/each}</pre>
        {/if}
      <textarea
        bind:this={editorTextarea}
        bind:value={content}
        class:focus-enabled={writingFocusMode !== "off"}
        aria-label="Markdown manuscript"
        autocapitalize="off"
        autocomplete="off"
        autocorrect={automaticCorrection ? "on" : "off"}
        spellcheck={spellCheckEnabled}
        oninput={(event) => handleEditorInput(event.currentTarget)}
        onfocus={(event) => updateCursor(event.currentTarget)}
        onclick={(event) => updateCursor(event.currentTarget)}
        onkeyup={(event) => updateCursor(event.currentTarget)}
        onselect={(event) => updateCursor(event.currentTarget)}
        onscroll={(event) => syncFocusOverlay(event.currentTarget)}
      ></textarea>
      </div>
      {/if}
    </div>

    <footer class="editor-status">
      <div class="editor-status-left">
        <span><span class:error-dot={saveStatus.includes("failed")} class="status-dot"></span>{saveStatus}</span>
        {#if desktopMode}
          <span class:error-text={syncPhase === "error"} class:conflict-text={syncPhase === "conflict"} class="sync-footer-status">↕ {syncStatus}</span>
        {/if}
      </div>
      <div class="status-counts">
        {#if sessionGoal > 0}
          <div class:goal-met={sessionWords >= sessionGoal} class="session-progress" title="Words added this session">
            <progress value={sessionWords} max={sessionGoal}></progress>
            <span>{sessionWords.toLocaleString()} / {sessionGoal.toLocaleString()} session</span>
          </div>
        {/if}
        <span>{wordCount(content).toLocaleString()} words</span>
      </div>
    </footer>
  </section>

  {#if updateVisible}
    <div class="modal-layer" role="presentation">
      <button
        class="modal-backdrop"
        aria-label="Close application update"
        disabled={updateChecking || updateInstalling}
        onclick={() => void closeUpdateDialog()}
      ></button>
      <div class="update-dialog" role="dialog" aria-modal="true" aria-label="Application update">
        <header>
          <div>
            <p class="eyebrow">Writing Environment</p>
            <h2>{updateAvailable ? `Update to ${updateAvailable.version}` : "Application updates"}</h2>
          </div>
          <button
            aria-label="Close application update"
            disabled={updateChecking || updateInstalling}
            onclick={() => void closeUpdateDialog()}
          >×</button>
        </header>

        <div class="update-dialog-content">
          <p class:error-text={updateMessage.includes("Could not") || updateMessage.includes("not installed")} class="update-message">{updateMessage}</p>
          {#if updateAvailable?.body}
            <div class="release-notes">
              <strong>What’s new</strong>
              <p>{updateAvailable.body}</p>
            </div>
          {/if}
          {#if updateInstalling}
            <div class="update-progress" aria-live="polite">
              <progress value={updateDownloaded} max={updateTotal || 1}></progress>
              <span>{updateTotal > 0 ? `${Math.min(100, Math.round(updateDownloaded / updateTotal * 100))}%` : "Working…"}</span>
            </div>
          {/if}
          <p class="setting-note">Updates are downloaded from the public GitHub release and verified with the application’s embedded signing key before installation.</p>
        </div>

        <footer>
          <span>Installed version {appVersion}</span>
          <div>
            <button disabled={updateChecking || updateInstalling} onclick={() => void closeUpdateDialog()}>Not now</button>
            {#if updateAvailable}
              <button class="update-primary" disabled={updateInstalling} onclick={() => void installAppUpdate()}>
                {updateInstalling ? "Installing…" : "Install and restart"}
              </button>
            {:else}
              <button class="update-primary" disabled={updateChecking} onclick={() => void checkForAppUpdate(true)}>
                {updateChecking ? "Checking…" : "Check again"}
              </button>
            {/if}
          </div>
        </footer>
      </div>
    </div>
  {/if}

  {#if historyVisible}
    <div class="modal-layer history-layer" role="presentation">
      <button class="modal-backdrop" aria-label="Close History" onclick={closeHistory}></button>
      <div class="history-dialog" role="dialog" aria-modal="true" aria-label={`History for ${activeSheet}`}>
        <header class="history-header">
          <div>
            <p class="eyebrow">Sheet history</p>
            <h2>{activeSheet}</h2>
          </div>
          <button aria-label="Close History" disabled={historyRestoring} onclick={closeHistory}>×</button>
        </header>

        <div class="history-content">
          <aside class="revision-list" aria-label="Saved revisions">
            {#each historyRevisions as revision}
              <button
                class:active={revision.id === selectedRevisionId}
                disabled={historyLoading || historyRestoring}
                onclick={() => void selectRevision(revision)}
              >
                <strong>{formatRevisionDate(revision.createdAt)}</strong>
                <span>{revision.wordCount.toLocaleString()} words · {revisionDeltaLabel(revision.wordDelta)}</span>
                <small>{revision.excerpt || "Empty sheet"}</small>
              </button>
            {/each}
            {#if historyRevisions.length === 0 && !historyLoading}
              <p>No saved revisions yet.</p>
            {/if}
          </aside>

          <div class="revision-preview">
            <div class="revision-preview-heading">
              <div>
                <strong>{selectedRevision ? formatRevisionDate(selectedRevision.createdAt) : "Revision preview"}</strong>
                {#if selectedRevision}
                  <span>{selectedRevision.wordCount.toLocaleString()} words</span>
                {/if}
              </div>
              {#if historyLoading}<span class="history-working">Loading…</span>{/if}
            </div>
            {#if revisionPreview}
              <pre>{revisionPreview}</pre>
            {:else}
              <div class="history-empty-preview">Select a saved revision to preview its Markdown.</div>
            {/if}
          </div>
        </div>

        <footer class="history-footer">
          <p class:error-text={historyMessage.toLowerCase().includes("cannot") || historyMessage.toLowerCase().includes("failed")}>{historyMessage || "Restoring also saves the current version, so the change can be undone."}</p>
          <button
            class:confirming={restoreConfirmId === selectedRevisionId && selectedRevisionId !== null}
            disabled={!selectedRevisionId || historyLoading || historyRestoring}
            onclick={() => void restoreSelectedRevision()}
          >
            {historyRestoring
              ? "Restoring…"
              : restoreConfirmId === selectedRevisionId && selectedRevisionId !== null
                ? "Confirm restore"
                : "Restore this version"}
          </button>
        </footer>
      </div>
    </div>
  {/if}

  {#if emptyTrashConfirmVisible}
    <div class="modal-layer" role="presentation">
      <button
        class="modal-backdrop"
        aria-label="Cancel empty Trash"
        onclick={closeEmptyTrashConfirmation}
      ></button>
      <form
        class="sheet-dialog"
        aria-label="Empty Trash confirmation"
        onsubmit={(event) => {
          event.preventDefault();
          void emptyTrash();
        }}
      >
        <p class="eyebrow">Trash</p>
        <h2>Empty Trash permanently?</h2>
        <p class="trash-confirmation">
          This will permanently delete {filteredTrashItems.length} {filteredTrashItems.length === 1 ? "sheet" : "sheets"}
          {trashOriginFilter === "all" ? "from every registered location" : `from ${filteredTrashItems[0]?.originName ?? "the selected location"}`}.
          This cannot be undone. Current Markdown files and History will not be affected.
        </p>
        {#if emptyTrashError}<p class="dialog-error" role="alert">{emptyTrashError}</p>{/if}
        <div class="dialog-actions">
          <button type="button" disabled={mutatingLibrary} onclick={closeEmptyTrashConfirmation}>Cancel</button>
          <button class="danger-primary" type="submit" disabled={mutatingLibrary}>
            {mutatingLibrary ? "Emptying…" : "Empty Trash"}
          </button>
        </div>
      </form>
    </div>
  {/if}

  {#if syncInitializationConfirmVisible}
    <div class="modal-layer" role="presentation">
      <button
        class="modal-backdrop"
        aria-label="Cancel first sync"
        onclick={() => {
          if (!syncRunning) syncInitializationConfirmVisible = false;
        }}
      ></button>
      <form
        class="sheet-dialog sync-initialization-dialog"
        aria-label="Confirm first universal sync"
        onsubmit={(event) => {
          event.preventDefault();
          void runUniversalSync(false, true);
        }}
      >
        <p class="eyebrow">Universal sync</p>
        <h2>Initialize new remote locations?</h2>
        <p class="trash-confirmation">
          Each destination below must be empty. Its local Markdown folder will become the starting
          copy. Existing legacy remote folders are not moved or deleted.
        </p>
        <div class="sync-initialization-list">
          {#each universalSyncTargets.filter((target) => target.included && !target.initialized) as target}
            <div>
              <strong>{target.name}</strong>
              <small>{target.remote}:{target.remotePath}</small>
            </div>
          {/each}
        </div>
        {#if syncInitializationError}<p class="dialog-error" role="alert">{syncInitializationError}</p>{/if}
        <div class="dialog-actions">
          <button
            type="button"
            disabled={syncRunning}
            onclick={() => (syncInitializationConfirmVisible = false)}
          >Cancel</button>
          <button class="sync-primary" type="submit" disabled={syncRunning}>
            {syncRunning ? "Initializing…" : "Confirm first sync"}
          </button>
        </div>
      </form>
    </div>
  {/if}

  {#if sheetDialogMode}
    <div class="modal-layer" role="presentation">
      <button class="modal-backdrop" aria-label="Close dialog" onclick={closeSheetDialog}></button>
      <form
        class="sheet-dialog"
        aria-label={`${sheetDialogMode} sheet`}
        onsubmit={(event) => {
          event.preventDefault();
          void submitSheetDialog();
        }}
      >
        <p class="eyebrow">Sheet</p>
        <h2>
          {sheetDialogMode === "create"
            ? "Create a new sheet"
            : sheetDialogMode === "rename"
              ? "Rename sheet"
              : sheetDialogMode === "move"
                ? "Move to another project or folder"
                : "Move sheet to Trash?"}
        </h2>

        {#if sheetDialogMode === "create" || sheetDialogMode === "rename"}
          <label for="sheet-title">Title</label>
          <input
            id="sheet-title"
            type="text"
            maxlength="200"
            required
            value={dialogTitle}
            oninput={(event) => (dialogTitle = event.currentTarget.value)}
          />
        {/if}

        {#if sheetDialogMode === "move"}
          <label for="sheet-project">Destination project</label>
          <select
            id="sheet-project"
            value={dialogProjectPath}
            oninput={(event) => void loadDialogProjectFolders(event.currentTarget.value)}
          >
            {#if !dialogProjectPath}<option value="">Open a project first…</option>{/if}
            {#each sortedProjects.filter((project) => project.open) as project}
              <option value={project.path}>{project.name}</option>
            {/each}
          </select>
        {/if}

        {#if (sheetDialogMode === "create" && !inboxActive) || sheetDialogMode === "move"}
          <label for="sheet-group">{sheetDialogMode === "move" ? "Destination folder" : "Project folder"}</label>
          <input
            id="sheet-group"
            type="text"
            maxlength="120"
            list="known-groups"
            required
            value={dialogGroup}
            oninput={(event) => (dialogGroup = event.currentTarget.value)}
          />
          <datalist id="known-groups">
            {#each dialogFolders as folder}
              <option value={folder.path}></option>
            {/each}
          </datalist>
          <p class="dialog-note">{sheetDialogMode === "move" ? "Choose an open project and type a folder path such as Research/Locations." : "Type a folder path such as Research/Locations."}</p>
        {/if}

        {#if sheetDialogMode === "create" && inboxActive}
          <p class="dialog-note">This sheet will be saved as ordinary Markdown in your Inbox.</p>
        {/if}

        {#if sheetDialogMode === "trash"}
          <p class="trash-confirmation">
            <strong>{dialogSheet?.title}</strong> will leave {inboxActive ? "Inbox" : "the project folder"}, but it can be restored from universal Trash.
          </p>
        {/if}

        {#if dialogError}<p class="dialog-error" role="alert">{dialogError}</p>{/if}

        <div class="dialog-actions">
          <button type="button" disabled={mutatingLibrary} onclick={closeSheetDialog}>Cancel</button>
          <button
            class:danger-primary={sheetDialogMode === "trash"}
            type="submit"
            disabled={mutatingLibrary || (sheetDialogMode === "move" && !dialogProjectPath)}
          >
            {mutatingLibrary
              ? "Working…"
              : sheetDialogMode === "create"
                ? "Create sheet"
                : sheetDialogMode === "rename"
                  ? "Rename"
                  : sheetDialogMode === "move"
                    ? "Move sheet"
                    : "Move to Trash"}
          </button>
        </div>
      </form>
    </div>
  {/if}
</main>
