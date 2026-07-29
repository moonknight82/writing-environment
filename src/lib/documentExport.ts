import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";

export type ExportFormat = "docx" | "pdf" | "epub";

interface ExportDefinition {
  label: string;
  filterName: string;
}

const exportDefinitions: Record<ExportFormat, ExportDefinition> = {
  docx: {
    label: "Word document",
    filterName: "Word document",
  },
  pdf: {
    label: "PDF",
    filterName: "PDF document",
  },
  epub: {
    label: "EPUB",
    filterName: "EPUB ebook",
  },
};

export interface ExportSection {
  relativePath: string;
  title: string;
}

export interface DocumentExportRequest {
  format: ExportFormat;
  root: string;
  title: string;
  sections: ExportSection[];
  activeRelativePath: string | null;
  activeContent: string | null;
  titlePage: boolean;
  pageBreaks: boolean;
  author: string;
  language: string;
}

function exportFileName(title: string, extension: ExportFormat): string {
  const safeTitle = Array.from(title.trim())
    .filter((character) => character >= " " && !'\\/:*?"<>|'.includes(character))
    .join("")
    .replace(/\s+/g, " ")
    .replace(/[. ]+$/g, "")
    .slice(0, 96)
    .trim();
  return `${safeTitle || "Untitled"}.${extension}`;
}

export async function exportDocument(
  request: DocumentExportRequest,
): Promise<string | null> {
  const definition = exportDefinitions[request.format];
  const selection = await save({
    title: `Export ${request.title} as ${definition.label}`,
    defaultPath: exportFileName(request.title, request.format),
    filters: [{ name: definition.filterName, extensions: [request.format] }],
  });
  if (!selection) return null;

  const path = selection.toLowerCase().endsWith(`.${request.format}`)
    ? selection
    : `${selection}.${request.format}`;
  return invoke<string>("export_document", { request: { path, ...request } });
}
