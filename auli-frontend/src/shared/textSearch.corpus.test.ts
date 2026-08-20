/**
 * Guarda de corpus para o descarte de letra isolada (pendência 37).
 *
 * O `textSearch.test.ts` prova a REGRA; este prova a CONSEQUÊNCIA que autorizou aplicá-la às seis
 * abas de uma vez: **descartar letra isolada não muda o conjunto devolvido por nenhuma delas**.
 * Isso não dá para afirmar por raciocínio — depende do corpus, e uma letra que hoje está em 100%
 * dos itens pode não estar amanhã. Daí medir contra os artefatos reais de `public/rs/`, os mesmos
 * que o frontend serve.
 *
 * Se este teste quebrar, a mudança de comportamento é real e precisa de decisão humana — não de um
 * ajuste no teste.
 */
// O `@types/node` está no node_modules (vem do vite) mas não é dependência declarada, e o
// tsconfig não tem campo `types` — sem esta referência o `tsc --noEmit` não enxerga `node:fs`.
// Local, porque é o único arquivo do projeto que toca API de node.
/// <reference types="node" />
import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";

import { buildHaystack, haystackMatches, normalizeText, parseQuery } from "./textSearch";

/** O `parseQuery` de antes do conserto: sem descarte nenhum. */
function parseQuerySemDescarte(query: string): string[] {
  const q = normalizeText(query);
  return q ? q.split(" ") : [];
}

/** Achata o artefato em uma lista de itens, cada um com seus campos de texto. */
function itensDe(valor: unknown, saida: string[][] = []): string[][] {
  if (Array.isArray(valor)) {
    for (const v of valor) itensDe(v, saida);
  } else if (valor && typeof valor === "object") {
    const campos = Object.values(valor).filter((v): v is string => typeof v === "string");
    if (campos.length) saida.push(campos);
    for (const v of Object.values(valor)) if (typeof v === "object") itensDe(v, saida);
  }
  return saida;
}

/** As seis abas que compartilham o `parseQuery`, pelos artefatos que cada uma consome. */
const ABAS: [string, string][] = [
  ["legislacao", "rs-legislacao-index.json"],
  ["pareceres", "rs-pareceres-index.json"],
  ["tarf", "rs-tarf-index.json"],
  ["faqs", "rs-faqs-tree.json"],
  ["servicos", "rs-servicos-index.json"],
  ["conteudos", "rs-conteudo_site_tree.json"],
];

/** Queries com letra isolada — a primeira é a que gerou a pendência. */
const QUERIES = [
  "impostos competem à União",
  "crédito de ICMS a recolher",
  "isenção a deficiente físico",
  "prazo e forma de pagamento",
  "substituição tributária e antecipação",
  "art 5 do regulamento", // com dígito isolado: preservado, então também não muda
];

describe("descartar letra isolada não muda o conjunto filtrado (pendência 37)", () => {
  for (const [aba, arquivo] of ABAS) {
    it(aba, () => {
      const itens = itensDe(JSON.parse(readFileSync(`public/rs/${arquivo}`, "utf8"))).map((campos) =>
        buildHaystack(campos),
      );
      expect(itens.length, `${aba}: artefato vazio`).toBeGreaterThan(0);
      for (const q of QUERIES) {
        const antes = itens.filter((h) => haystackMatches(h, parseQuerySemDescarte(q))).length;
        const depois = itens.filter((h) => haystackMatches(h, parseQuery(q))).length;
        expect(depois, `${aba} / ${JSON.stringify(q)} / ${itens.length} itens`).toBe(antes);
      }
    });
  }
});
