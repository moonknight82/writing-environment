export interface VisualTheme {
  id: string;
  name: string;
  description: string;
  mode: "light" | "dark";
  tokens: Record<string, string>;
}

const systemSans = 'Inter, ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
const systemSerif = '"Iowan Old Style", "Palatino Linotype", Palatino, Georgia, serif';
const systemMono = '"SFMono-Regular", Consolas, "Liberation Mono", monospace';

export const themes: VisualTheme[] = [
  {
    id: "paper",
    name: "Paper",
    description: "Clear and neutral for daylight writing.",
    mode: "light",
    tokens: {
      canvas: "#f4f1ea",
      surface: "#ebe7dd",
      surfaceRaised: "#faf8f3",
      text: "#252724",
      textMuted: "#73756e",
      border: "#d8d3c7",
      accent: "#426a78",
      accentSoft: "#dbe6e8",
      selection: "#c7dade",
      success: "#55745c",
      editorBackground: "none",
      proseLetterSpacing: "normal",
      proseTextShadow: "none",
      proseFont: systemSerif,
      interfaceFont: systemSans,
      monoFont: systemMono,
    },
  },
  {
    id: "sepia",
    name: "Sepia",
    description: "Warm paper and soft ink for long sessions.",
    mode: "light",
    tokens: {
      canvas: "#eee4d1",
      surface: "#e4d6be",
      surfaceRaised: "#f7eedc",
      text: "#332d25",
      textMuted: "#7b6d5c",
      border: "#d2c1a5",
      accent: "#8a5d38",
      accentSoft: "#ead7bc",
      selection: "#dcc4a4",
      success: "#66734e",
      editorBackground: "none",
      proseLetterSpacing: "normal",
      proseTextShadow: "none",
      proseFont: systemSerif,
      interfaceFont: systemSans,
      monoFont: systemMono,
    },
  },
  {
    id: "midnight",
    name: "Midnight",
    description: "Low-glare charcoal without pure black.",
    mode: "dark",
    tokens: {
      canvas: "#181b1d",
      surface: "#202427",
      surfaceRaised: "#24292c",
      text: "#e5e3dd",
      textMuted: "#9a9d9c",
      border: "#343a3d",
      accent: "#8ab2bd",
      accentSoft: "#293c42",
      selection: "#38545c",
      success: "#91b397",
      editorBackground: "none",
      proseLetterSpacing: "normal",
      proseTextShadow: "none",
      proseFont: systemSerif,
      interfaceFont: systemSans,
      monoFont: systemMono,
    },
  },
  {
    id: "old-terminal",
    name: "Old Terminal",
    description: "Soft green phosphor on a deep black screen.",
    mode: "dark",
    tokens: {
      canvas: "#071009",
      surface: "#0a150d",
      surfaceRaised: "#0d1a10",
      text: "#b6f5bd",
      textMuted: "#6cae77",
      border: "#18351e",
      accent: "#74f28a",
      accentSoft: "#12321a",
      selection: "#285f35",
      success: "#74f28a",
      editorBackground:
        "repeating-linear-gradient(to bottom, rgba(116, 242, 138, 0.022) 0, rgba(116, 242, 138, 0.022) 1px, transparent 1px, transparent 4px)",
      proseLetterSpacing: "0.015em",
      proseTextShadow: "0 0 8px rgba(116, 242, 138, 0.16)",
      proseFont: systemMono,
      interfaceFont: systemMono,
      monoFont: systemMono,
    },
  },
];

export function applyTheme(theme: VisualTheme): void {
  const root = document.documentElement;
  root.dataset.theme = theme.id;
  root.style.colorScheme = theme.mode;

  for (const [token, value] of Object.entries(theme.tokens)) {
    const cssName = token.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
    root.style.setProperty(`--${cssName}`, value);
  }

  document.querySelector('meta[name="theme-color"]')?.setAttribute("content", theme.tokens.canvas);
}
