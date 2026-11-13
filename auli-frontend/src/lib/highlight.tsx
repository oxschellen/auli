/**
 * `highlight` — marca no texto exibido os termos que a busca casou.
 *
 * O par natural do `textSearch`: aquele decide **se** um item entra, este mostra **por quê**. Recebe
 * os mesmos `terms` já normalizados (saída de `parseQuery`), então marcação e filtro nunca discordam.
 *
 * ## O problema que este módulo resolve
 *
 * `normalizeText` **não preserva comprimento**: tira diacríticos ("é" → "e") e colapsa corridas de
 * whitespace. Um índice encontrado na string normalizada não aponta para o mesmo caractere na
 * original — marcar por ele desloca o realce, cada vez mais a cada acento à esquerda.
 *
 * Então normalizamos **caractere a caractere**, guardando de onde cada caractere normalizado veio
 * (`map`). O casamento acontece no texto normalizado; o recorte, no original.
 *
 * Whitespace é a única divergência deliberada em relação ao `normalizeText`: aqui as corridas **não**
 * são colapsadas (colapsar quebraria o mapa). Isso é seguro porque `parseQuery` quebra a query por
 * whitespace — **nenhum termo contém espaço** —, e um termo sem espaço jamais casaria através de uma
 * corrida de espaços de qualquer forma. Colapsar ou não é indiferente para o resultado.
 */

import type { ReactNode } from "react";
import { Box } from "@chakra-ui/react";
import { DIACRITICOS } from "./textSearch";

/** Um trecho a marcar, em índices da string **original**: `[início, fim)`. */
export type Range = [number, number];

/**
 * Normaliza preservando a origem: `norm` é comparável aos termos, e `map[i]` é o índice na `text`
 * de onde saiu `norm[i]`. `map` tem um sentinela no fim (`text.length`), para que o fim de um
 * casamento em `norm[a..b)` seja lido como `map[b]` sem caso especial.
 *
 * Itera por **code point** (`for…of`), não por code unit: um par surrogate (emoji) conta como um
 * caractere só, e `ch.length` devolve quantas unidades ele consumiu na original.
 */
function normalizeWithMap(text: string): { norm: string; map: number[] } {
  let norm = "";
  const map: number[] = [];
  let i = 0;
  for (const ch of text) {
    // Uma decomposição pode render 0 caracteres (o próprio diacrítico combinante, que some) ou mais
    // de um (algumas maiúsculas em lowercase) — daí o laço interno, e não um push por caractere.
    for (const c of ch.normalize("NFD").replace(DIACRITICOS, "").toLowerCase()) {
      norm += c;
      // Uma entrada por **code unit**, não por code point: `indexOf` indexa em code units, então um
      // emoji (2 unidades) precisa de 2 entradas ou tudo à direita dele desliza uma casa.
      for (let k = 0; k < c.length; k++) map.push(i);
    }
    i += ch.length;
  }
  map.push(text.length);
  return { norm, map };
}

/** Une faixas que se tocam ou se sobrepõem, em ordem. Dois termos podem casar o mesmo trecho. */
function mergeRanges(ranges: Range[]): Range[] {
  if (ranges.length < 2) return ranges;
  const ordered = [...ranges].sort((a, b) => a[0] - b[0]);
  const out: Range[] = [ordered[0]];
  for (const [start, end] of ordered.slice(1)) {
    const last = out[out.length - 1];
    if (start <= last[1]) last[1] = Math.max(last[1], end);
    else out.push([start, end]);
  }
  return out;
}

/**
 * Faixas de `text` (índices originais) casadas por qualquer um dos `terms`. Termos vazios e query
 * vazia devolvem `[]` — nada a marcar.
 *
 * Pré-condição: `terms` já normalizados (`parseQuery`). Passar termo cru com acento não casa nada,
 * que é o bug exato que este módulo existe para não repetir.
 */
export function findHighlightRanges(text: string, terms: readonly string[]): Range[] {
  if (!text || terms.length === 0) return [];
  const { norm, map } = normalizeWithMap(text);
  const ranges: Range[] = [];
  for (const term of terms) {
    if (!term) continue;
    let from = 0;
    for (;;) {
      const at = norm.indexOf(term, from);
      if (at === -1) break;
      ranges.push([map[at], map[at + term.length]]);
      from = at + term.length; // ocorrências do mesmo termo não se sobrepõem
    }
  }
  return mergeRanges(ranges);
}

/**
 * O texto com os termos casados envoltos em `<mark>`. Sem casamento (ou sem termos), devolve a
 * string crua — nenhum elemento a mais no DOM no caso comum, que é a lista inteira sem busca.
 */
export function Highlight({ text, terms }: { text: string; terms: readonly string[] }): ReactNode {
  const ranges = findHighlightRanges(text, terms);
  if (ranges.length === 0) return text;

  const out: ReactNode[] = [];
  let cursor = 0;
  for (const [start, end] of ranges) {
    if (start > cursor) out.push(text.slice(cursor, start));
    out.push(
      <Box
        as="mark"
        key={start}
        bg="bg.highlight"
        color="fg.highlight"
        px="0.1em"
        borderRadius="2px"
      >
        {text.slice(start, end)}
      </Box>,
    );
    cursor = end;
  }
  if (cursor < text.length) out.push(text.slice(cursor));
  return <>{out}</>;
}
