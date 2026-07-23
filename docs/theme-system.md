# Visual theme system

Themes are a product capability, not unrestricted custom CSS. Each theme is declarative data mapped to a stable set of design tokens.

## Theme scope

A theme may define:

- canvas, panel, elevated-surface, text, muted-text, border, and accent colors;
- selection, focus-ring, and status colors;
- interface, prose, and monospace font stacks;
- editor measure, base prose size, and theme defaults for letter spacing and text shadow;
- a static editor background treatment, such as subtle terminal scanlines;
- compact or comfortable panel density.

A theme may not move controls, run scripts, load remote resources, or disable accessibility states.

## Initial schema

```ts
interface VisualTheme {
  id: string;
  name: string;
  mode: "light" | "dark";
  tokens: {
    canvas: string;
    surface: string;
    surfaceRaised: string;
    text: string;
    textMuted: string;
    border: string;
    accent: string;
    accentSoft: string;
    selection: string;
    proseFont: string;
    interfaceFont: string;
    monoFont: string;
  };
}
```

## Built-in themes

- **Paper:** neutral daylight theme with a restrained blue accent.
- **Sepia:** warmer paper and ink colors for long sessions.
- **Midnight:** low-glare dark theme without pure black surfaces.
- **Old Terminal:** phosphor-green monospace type on a near-black screen, with subtle static scanlines and glow.

Theme choice is stored globally at first. Per-project and automatic time-based themes can be added later without changing the token model.

Line height and sheet width are separate writer preferences rather than part of a visual theme. Line height can be adjusted from 1.35 to 2.2, while sheet width ranges from a focused 50% column to the full available editor area. Both persist across launches, so changing atmosphere never unexpectedly changes manuscript spacing or layout.

## Performance and accessibility

- Theme switching only updates CSS custom properties.
- Built-in themes meet WCAG AA contrast for ordinary interface text.
- Focus rings remain visible in every theme.
- Reduced-motion preferences disable nonessential transitions.
- Old Terminal's scanlines are a static CSS gradient; the theme has no animation loop.
- No theme requires network access.
