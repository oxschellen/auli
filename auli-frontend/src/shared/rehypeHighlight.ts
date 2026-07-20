/**
 * `rehypeHighlight` — o `Highlight` para conteúdo renderizado como **markdown**.
 *
 * A sinopse dos pareceres é markdown, e é campo de busca. Marcar por split de string não serve: o
 * que chega na tela é uma árvore (`### título`, `- **item**`), e o termo pode cair dentro de um
 * `<strong>` no meio de um `<li>`. Então marcamos na árvore, não no texto: um plugin rehype que
 * visita os nós de texto e troca cada casamento por um elemento `<mark>`.
 *
 * Reusa `findHighlightRanges` — mesmo casamento, mesmos offsets, mesma coerência com o filtro que a
 * versão de texto simples. Tipagem local e mínima em vez de `@types/hast`: as duas formas de nó que
 * importam são `text` e `element`, e depender de um pacote transitivo do react-markdown seria
 * frágil (ele não é dependência direta nossa).
 */

import { findHighlightRanges } from "./highlight";

/** Recorte mínimo de um nó hast — só o que este plugin lê ou escreve. */
export interface HastNode {
  type: string;
  tagName?: string;
  value?: string;
  properties?: Record<string, unknown>;
  children?: HastNode[];
}

/** Elementos cujo texto NÃO é marcado: código é literal, marcar dentro dele altera o que se lê. */
const OPACOS = new Set(["code", "pre"]);

function marcarNo(node: HastNode, terms: readonly string[]): void {
  if (!node.children?.length) return;

  const out: HastNode[] = [];
  for (const child of node.children) {
    if (child.type === "text" && child.value) {
      const texto = child.value;
      const ranges = findHighlightRanges(texto, terms);
      if (ranges.length === 0) {
        out.push(child);
        continue;
      }
      let cursor = 0;
      for (const [start, end] of ranges) {
        if (start > cursor) out.push({ type: "text", value: texto.slice(cursor, start) });
        out.push({
          type: "element",
          tagName: "mark",
          properties: {},
          children: [{ type: "text", value: texto.slice(start, end) }],
        });
        cursor = end;
      }
      if (cursor < texto.length) out.push({ type: "text", value: texto.slice(cursor) });
      continue;
    }

    if (!(child.type === "element" && OPACOS.has(child.tagName ?? ""))) marcarNo(child, terms);
    out.push(child);
  }
  node.children = out;
}

/**
 * Plugin rehype que marca `terms` (já normalizados, saída de `parseQuery`) no texto da árvore.
 * Sem termos, devolve um transformador que não faz nada — o caso comum, sem busca ativa.
 */
export function rehypeHighlight(terms: readonly string[]) {
  return () => (tree: HastNode) => {
    if (terms.length === 0) return;
    marcarNo(tree, terms);
  };
}
