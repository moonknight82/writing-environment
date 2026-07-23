import { invoke } from "@tauri-apps/api/core";

export interface SyncAvailability {
  installed: boolean;
  compatible: boolean;
  version: string | null;
  remotes: string[];
  message: string;
}

export interface SyncResult {
  status: "synced" | "conflict";
  message: string;
  conflicts: number;
  initialized: boolean;
  completedAt: string;
}

export function getSyncAvailability(): Promise<SyncAvailability> {
  return invoke<SyncAvailability>("sync_availability");
}

export function syncProject(
  root: string,
  remote: string,
  remotePath: string,
): Promise<SyncResult> {
  return invoke<SyncResult>("sync_project", { root, remote, remotePath });
}
