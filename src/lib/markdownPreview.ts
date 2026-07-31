import DOMPurify from "dompurify";
import { marked } from "marked";

const FRONT_MATTER = /^---\r?\n[\s\S]*?\r?\n---(?:\r?\n|$)/;

export function renderMarkdownPreview(markdown: string): string {
  const body = markdown.replace(FRONT_MATTER, "");
  const rendered = marked.parse(body, {
    async: false,
    breaks: false,
    gfm: true,
  });

  return DOMPurify.sanitize(rendered, {
    USE_PROFILES: { html: true },
  });
}
