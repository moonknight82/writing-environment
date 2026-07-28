import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";

export type ExportFormat = "docx" | "pdf" | "epub";

interface ExportDefinition {
  command: string;
  dialogTitle: string;
  filterName: string;
}

const exportDefinitions: Record<ExportFormat, ExportDefinition> = {
  docx: {
    command: "export_sheet_docx",
    dialogTitle: "Export current sheet as a Word document",
    filterName: "Word document",
  },
  pdf: {
    command: "export_sheet_pdf",
    dialogTitle: "Export current sheet as a PDF",
    filterName: "PDF document",
  },
  epub: {
    command: "export_sheet_epub",
    dialogTitle: "Export current sheet as an ebook",
    filterName: "EPUB ebook",
  },
};

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

export async function exportSheet(
  format: ExportFormat,
  title: string,
  content: string,
): Promise<string | null> {
  const definition = exportDefinitions[format];
  const selection = await save({
    title: definition.dialogTitle,
    defaultPath: exportFileName(title, format),
    filters: [{ name: definition.filterName, extensions: [format] }],
  });
  if (!selection) return null;

  const path = selection.toLowerCase().endsWith(`.${format}`)
    ? selection
    : `${selection}.${format}`;
  return invoke<string>(definition.command, { path, title, content });
}
