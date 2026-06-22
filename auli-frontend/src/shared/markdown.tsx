/**
 * Shared ReactMarkdown component maps. Centralizes the renderers that were
 * duplicated across the chat bubble, the FAQ answers, and the About page —
 * most importantly the link renderer, which carries the security-relevant
 * `target="_blank" rel="noopener noreferrer"` and must stay consistent.
 *
 * - `compactMarkdownComponents`: tight spacing for chat bubbles and FAQ answers.
 * - `proseMarkdownComponents`: long-form spacing + headings for the About page.
 */

import type { Components } from "react-markdown";

const accent = "var(--chakra-colors-accent)";

const MarkdownLink: Components["a"] = ({ href, children }) => (
  <a
    href={href}
    target="_blank"
    rel="noopener noreferrer"
    style={{ color: accent, textDecoration: "underline", textUnderlineOffset: "2px" }}
  >
    {children}
  </a>
);

export const compactMarkdownComponents: Components = {
  a: MarkdownLink,
  p: ({ children }) => <p style={{ marginBottom: "0.5em" }}>{children}</p>,
  ul: ({ children }) => <ul style={{ paddingLeft: "1.2em", marginBottom: "0.5em" }}>{children}</ul>,
  ol: ({ children }) => <ol style={{ paddingLeft: "1.2em", marginBottom: "0.5em" }}>{children}</ol>,
  li: ({ children }) => <li style={{ marginBottom: "0.25em" }}>{children}</li>,
};

export const proseMarkdownComponents: Components = {
  h1: ({ children }) => (
    <h1 style={{ fontSize: "1.8rem", fontWeight: 700, marginBottom: "0.75em", color: "var(--chakra-colors-fg)" }}>
      {children}
    </h1>
  ),
  h2: ({ children }) => (
    <h2 style={{ fontSize: "1.3rem", fontWeight: 600, marginTop: "1.5em", marginBottom: "0.5em", color: "var(--chakra-colors-fg)" }}>
      {children}
    </h2>
  ),
  h3: ({ children }) => (
    <h3 style={{ fontSize: "1.1rem", fontWeight: 600, marginTop: "1.25em", marginBottom: "0.4em", color: "var(--chakra-colors-fg)" }}>
      {children}
    </h3>
  ),
  p: ({ children }) => <p style={{ marginBottom: "0.85em" }}>{children}</p>,
  ul: ({ children }) => <ul style={{ paddingLeft: "1.4em", marginBottom: "0.75em" }}>{children}</ul>,
  ol: ({ children }) => <ol style={{ paddingLeft: "1.4em", marginBottom: "0.75em" }}>{children}</ol>,
  li: ({ children }) => <li style={{ marginBottom: "0.3em" }}>{children}</li>,
  a: MarkdownLink,
  hr: () => <hr style={{ border: "none", borderTop: `1px solid var(--chakra-colors-border)`, margin: "1.5em 0" }} />,
};
