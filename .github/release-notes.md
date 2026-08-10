Version 0.8.0 adds optional, on-demand grammar and style review through a
writer-controlled, self-hosted LanguageTool server. Review supports English and
Portuguese language variants and stays outside the typing and autosave paths.
The connection test reads only LanguageTool's language list; manuscript text is
sent only when the writer explicitly chooses **Check sheet**.

Review findings remain non-destructive. Each suggestion includes its rule or
category, explanation, context, and available replacements. Markdown syntax,
front matter, code, and link destinations are masked without shifting source
offsets. Suggestions become stale when the sheet changes, and a replacement is
applied only while its reviewed source range still matches the current draft.

Self-hosted deployments may run on the writer's computer, NAS, or another
private server. Public unencrypted endpoints are rejected, while private-network
HTTP requires a visible acknowledgement. The repository includes a reproducible
Docker and Portainer package built from the checksummed official LanguageTool
6.6 standalone archive for ARM64 and x86-64 hosts. Dependency audits now run in
continuous integration as an additional release check.

This is a personal project under active development. Back up important writing
and review the release notes before updating.
