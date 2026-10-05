import type { ReactNode } from "react";
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
import remarkGfm from "remark-gfm";

/** Shared remark plugins for every ReactMarkdown instance — GFM adds tables, strikethrough, and
 *  autolinks (without it, an LLM table renders as one flowing line of pipes). */
export const markdownPlugins = [remarkGfm];

const accent = "var(--chakra-colors-accent)";

/** GFM table renderers: bordered cells, scrollable wrapper (chat bubbles are narrow). Shared by the
 *  compact and prose maps. */
const tableComponents: Components = {
  table: ({ children }) => (
    <div style={{ overflowX: "auto", marginBottom: "0.5em" }}>
      <table style={{ borderCollapse: "collapse", width: "100%", fontSize: "0.9em" }}>{children}</table>
    </div>
  ),
  th: ({ children }) => (
    <th
      style={{
        border: "1px solid var(--chakra-colors-border)",
        padding: "4px 8px",
        textAlign: "left",
        fontWeight: 600,
      }}
    >
      {children}
    </th>
  ),
  td: ({ children }) => (
    <td
      style={{
        border: "1px solid var(--chakra-colors-border)",
        padding: "4px 8px",
        verticalAlign: "top",
      }}
    >
      {children}
    </td>
  ),
};

/** LLM/portal links sometimes arrive without a scheme (e.g. `www.legislacao…` as a markdown-link
 *  target). A scheme-less href is resolved as relative and hangs when clicked, so force any
 *  external-looking URL to an absolute `http://` one. In-app (`/…`) and anchor (`#…`) links pass through. */
export function toAbsoluteHref(href?: string): string | undefined {
  if (!href) return href;
  if (/^[a-z][a-z0-9+.-]*:/i.test(href)) return href; // already has a scheme (http:, https:, mailto:, tel:)
  if (href.startsWith("/") || href.startsWith("#")) return href;
  return `http://${href.replace(/^\/+/, "")}`;
}

/** Aspas curvas que o autolink do GFM engole no fim de um endereço citado: `acesse “https://x/y”`
 *  virava link para `https://x/y%E2%80%9D`, e o portal respondia 404. As retas o GFM já deixa de
 *  fora; as curvas, não — e são as que o modelo escreve. Chegam aqui já codificadas no `href`. */
const ASPAS_CURVAS_NO_FIM = /(?:%E2%80%9[89CD]|%C2%[AB]B|[“”‘’«»])+$/i;

/** Exportado para o `SystemMessage`, que troca o `a` do mapa compacto pelo selo de citação e cai
 *  neste para os demais links. */
export const MarkdownLink = ({ href, children }: { href?: string; children?: ReactNode }) => {
  const sobra = href?.match(ASPAS_CURVAS_NO_FIM)?.[0];
  const aspas = sobra ? decodeURIComponent(sobra) : "";
  const destino = sobra ? href!.slice(0, -sobra.length) : href;
  // No autolink o texto É o endereço, aspas incluídas: elas saem do link e voltam logo depois dele.
  const unico = Array.isArray(children) && children.length === 1 ? children[0] : children;
  const texto =
    aspas && typeof unico === "string" && unico.endsWith(aspas) ? unico.slice(0, -aspas.length) : null;
  return (
    <>
      <a
        href={toAbsoluteHref(destino)}
        target="_blank"
        rel="noopener noreferrer"
        style={{ color: accent, textDecoration: "underline", textUnderlineOffset: "2px" }}
      >
        {texto ?? children}
      </a>
      {texto !== null && aspas}
    </>
  );
};

export const compactMarkdownComponents: Components = {
  ...tableComponents,
  a: MarkdownLink,
  p: ({ children }) => <p style={{ marginBottom: "0.5em" }}>{children}</p>,
  // `listStyleType` explícito: o reset do Chakra zera o `list-style`, e sem ele os itens viravam
  // parágrafos recuados soltos (D-UI-1).
  ul: ({ children }) => <ul style={{ paddingLeft: "1.3em", marginBottom: "0.5em", listStyleType: "disc" }}>{children}</ul>,
  ol: ({ children }) => <ol style={{ paddingLeft: "1.3em", marginBottom: "0.5em", listStyleType: "decimal" }}>{children}</ol>,
  li: ({ children }) => <li style={{ marginBottom: "0.25em" }}>{children}</li>,
};

export const proseMarkdownComponents: Components = {
  ...tableComponents,
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
  ul: ({ children }) => <ul style={{ paddingLeft: "1.4em", marginBottom: "0.75em", listStyleType: "disc" }}>{children}</ul>,
  ol: ({ children }) => <ol style={{ paddingLeft: "1.4em", marginBottom: "0.75em", listStyleType: "decimal" }}>{children}</ol>,
  li: ({ children }) => <li style={{ marginBottom: "0.3em" }}>{children}</li>,
  a: MarkdownLink,
  hr: () => <hr style={{ border: "none", borderTop: `1px solid var(--chakra-colors-border)`, margin: "1.5em 0" }} />,
};
