/**
 * `textSearch` — utilitário compartilhado de busca client-side das listas (Serviços, FAQs,
 * Conteúdos, Pareceres): normalização de acentos + multi-termo com **E lógico**.
 *
 * Agnóstico ao corpus por contrato: recebe `fields` (strings) + query e não sabe se são títulos,
 * ementas, sinopses ou corpos — cada lista decide o que entregar. Semântica:
 *
 * - **Normalização**: NFD + remoção de diacríticos (inclui ç→c) + lowercase + colapso de
 *   whitespace. "credito icms" acha "Crédito de ICMS"; "cabecalho" acha "cabeçalho".
 * - **Multi-termo**: a query é quebrada por whitespace; **todos** os termos precisam aparecer
 *   (substring) em **algum** campo — E entre termos, OU entre campos. Termos podem casar em
 *   campos diferentes do mesmo item; a ordem é irrelevante.
 * - **Query vazia** (ou só espaços) casa tudo — devolve a lista inteira, como hoje.
 * - Query de um termo aceita um **superset** do filtro antigo (`toLowerCase().includes`):
 *   nenhum resultado se perde (ver o teste de invariante em `textSearch.test.ts`).
 *
 * Performance: `normalizeText` por tecla sobre o corpus inteiro seria O(n·tamanho) desnecessário.
 * As listas devem pré-computar o haystack por item (`buildHaystack` dentro de um `useMemo` sobre
 * os dados carregados) e filtrar por tecla com `parseQuery` (do valor deferred) +
 * `haystackMatches`. `matchesQuery` é o açúcar de conveniência para listas pequenas e testes.
 * Sem índice, sem worker, sem ranking: filtro linear é suficiente na escala alvo (≤ ~20k itens).
 */

/**
 * Faixa dos diacríticos combinantes que o NFD separa (acentos, til, cedilha…). Exportado porque o
 * `highlight` precisa da **mesma** definição para normalizar caractere a caractere — duas faixas
 * divergentes fariam a marcação cair fora do trecho que a busca casou.
 */
export const DIACRITICOS = /[\u0300-\u036f]/g;

/**
 * Colapsa em forma canônica de busca: NFD + remove diacríticos (ç→c de graça, porque o NFD
 * decompõe ç em c + combinante) + lowercase + colapsa qualquer corrida de whitespace em um
 * espaço, com trim nas pontas.
 */
export function normalizeText(s: string): string {
  return s
    .normalize("NFD")
    .replace(DIACRITICOS, "")
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .join(" ");
}

/**
 * Pré-computa os campos de um item, já normalizados — **uma vez por item, não por tecla**.
 * Campos `null`/`undefined` são tolerados (viram `""`, que não casa termo não-vazio).
 */
export function buildHaystack(fields: readonly (string | null | undefined)[]): string[] {
  return fields.map((f) => normalizeText(f ?? ""));
}

/** Quebra a query em termos normalizados. Query vazia ou só espaços → `[]` (casa tudo). */
export function parseQuery(query: string): string[] {
  const q = normalizeText(query);
  return q ? q.split(" ") : [];
}

/**
 * `true` se **todos** os termos aparecem (substring) em **algum** campo do haystack.
 * `terms` vazio → `true` (lista inteira). Pré-condição: haystack e termos já normalizados
 * (saídas de `buildHaystack`/`parseQuery`).
 */
export function haystackMatches(
  haystack: readonly string[],
  terms: readonly string[],
): boolean {
  return terms.every((t) => haystack.some((h) => h.includes(t)));
}

/**
 * Açúcar: `buildHaystack` + `parseQuery` + `haystackMatches` numa chamada. Para listas pequenas
 * e testes; nas listas grandes, memoize o haystack e chame `haystackMatches` direto.
 */
export function matchesQuery(
  fields: readonly (string | null | undefined)[],
  query: string,
): boolean {
  return haystackMatches(buildHaystack(fields), parseQuery(query));
}
