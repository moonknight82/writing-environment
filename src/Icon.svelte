<script lang="ts">
  export let name:
    | "inbox" | "folder" | "folderOpen" | "files" | "trash" | "plus" | "sort"
    | "sidebar" | "list" | "download" | "history" | "sync"
    | "focus" | "fullscreen" | "more" | "search";
  export let size = 16;
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor"
  stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">
  {#if name === "inbox"}
    <path d="M4 4h16l2 11v5H2v-5L4 4Z"/><path d="M2.5 15H8l2 3h4l2-3h5.5"/>
  {:else if name === "folder"}
    <path d="M3 6h7l2 2h9v11H3V6Z"/>
  {:else if name === "folderOpen"}
    <path d="M3 6h7l2 2h8v3M3 6v13h16l3-8H6l-3 8"/>
  {:else if name === "files"}
    <rect x="4" y="4" width="12" height="15" rx="2"/><path d="M9 9h10a2 2 0 0 1 2 2v9H9"/>
  {:else if name === "trash"}
    <path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v5M14 11v5"/>
  {:else if name === "plus"}
    <path d="M12 5v14M5 12h14"/>
  {:else if name === "sort"}
    <path d="M7 4v16m0 0-3-3m3 3 3-3M17 20V4m0 0-3 3m3-3 3 3"/>
  {:else if name === "sidebar"}
    <rect x="2.5" y="3" width="19" height="18" rx="2"/><path d="M8.5 3v18"/>
  {:else if name === "list"}
    <path d="M4 6h16M4 12h16M4 18h16"/>
  {:else if name === "download"}
    <path d="M12 3v12m0 0-4-4m4 4 4-4M4 18v3h16v-3"/>
  {:else if name === "history"}
    <path d="M3 12a9 9 0 1 0 3-6.7M3 4v5h5M12 7v5l3 2"/>
  {:else if name === "sync"}
    <path d="M20 7h-5l2.5-2.5M20 7l-2.5 2.5M4 17h5l-2.5-2.5M4 17l2.5 2.5"/><path d="M5.5 10a7 7 0 0 1 12-3M18.5 14a7 7 0 0 1-12 3"/>
  {:else if name === "focus"}
    <path d="M8 3H4v4M16 3h4v4M4 17v4h4M20 17v4h-4"/>
  {:else if name === "fullscreen"}
    <path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5"/>
  {:else if name === "more"}
    <circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>
  {:else if name === "search"}
    <circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/>
  {/if}
</svg>
