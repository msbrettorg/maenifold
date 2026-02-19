// T-SITE-001.2a: RTM FR-15.9 — Custom Shiki theme + build-time syntax highlighting
// Syntax highlighting happens at build time via Shiki's createHighlighter.
// No Shiki JavaScript is shipped to the client.

import { createHighlighter, type Highlighter } from "shiki";

// Warm Restraint syntax highlighting theme per PRD §7.9
const warmRestraintTheme = {
  name: "warm-restraint",
  type: "dark" as const,
  colors: {
    "editor.background": "#18151C",
    "editor.foreground": "#D4CEC8",
  },
  tokenColors: [
    {
      scope: ["keyword", "storage.type", "storage.modifier"],
      settings: { foreground: "#6AABB3" },
    },
    {
      scope: ["string", "string.quoted"],
      settings: { foreground: "#D4A76A" },
    },
    {
      scope: ["comment", "punctuation.definition.comment"],
      settings: { foreground: "#8A8480" },
    },
    {
      scope: [
        "variable.other.property",
        "entity.name.tag",
        "support.type.property-name",
      ],
      settings: { foreground: "#C4878A" },
    },
    {
      scope: ["constant.numeric", "constant.language"],
      settings: { foreground: "#8AAA7A" },
    },
    {
      scope: ["entity.name.function", "support.function"],
      settings: { foreground: "#B8A0C8" },
    },
    {
      scope: ["punctuation", "meta.brace"],
      settings: { foreground: "#7A746F" },
    },
  ],
};

// Supported languages — extend as needed
const SUPPORTED_LANGS = [
  "bash",
  "sh",
  "json",
  "typescript",
  "javascript",
  "tsx",
  "jsx",
  "css",
  "html",
  "markdown",
  "yaml",
  "toml",
  "csharp",
  "sql",
  "text",
] as const;

// Singleton highlighter — created once and reused across all renderMarkdown calls
let highlighterPromise: Promise<Highlighter> | null = null;

function getHighlighter(): Promise<Highlighter> {
  if (!highlighterPromise) {
    highlighterPromise = createHighlighter({
      themes: [warmRestraintTheme],
      langs: [...SUPPORTED_LANGS],
    });
  }
  return highlighterPromise;
}

/**
 * Highlights a code block using the Warm Restraint Shiki theme.
 *
 * Falls back to plain text rendering if the language is not supported.
 * The highlighter instance is cached — only created once per build.
 *
 * @param code - The source code to highlight
 * @param lang - The language identifier (e.g. "bash", "typescript")
 * @returns HTML string with inline Shiki styles
 */
export async function highlightCode(code: string, lang: string): Promise<string> {
  const highlighter = await getHighlighter();

  // Resolve to a supported lang or fall back to plain text
  const loadedLangs = highlighter.getLoadedLanguages();
  const resolvedLang = loadedLangs.includes(lang as never) ? lang : "text";

  return highlighter.codeToHtml(code, {
    lang: resolvedLang,
    theme: "warm-restraint",
  });
}
