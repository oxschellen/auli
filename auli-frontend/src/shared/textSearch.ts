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
 * - **Query vazia** (ou só espaços) casa tudo — devolve a lista inteira, como hoje. **Letras
 *   isoladas** são descartadas pelo `parseQuery` (ver lá o porquê); query inteira de letras
 *   isoladas cai no mesmo caso da query vazia. Dígito isolado é preservado.
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

/**
 * Uma **letra** sozinha, depois da normalização. Em português as palavras de uma letra são todas
 * átonas (`a`/`à`, `e`/`é`, `o`), e o NFD as colapsa nestas três — que aparecem em 100% dos itens
 * de todos os corpora medidos (legislação, pareceres, FAQs). Dígito isolado fica de fora do corte:
 * ele discrimina de verdade (`5` está em 86% dos pareceres, não em 100%), e descartá-lo mudaria o
 * conjunto devolvido — medido: "art 5 do regulamento" iria de 70 para 92 pareceres.
 */
const LETRA_SOZINHA = /^[a-z]$/;

/**
 * Quebra a query em termos normalizados, **descartando letras isoladas**. Query vazia, só espaços
 * ou só letras isoladas → `[]` (casa tudo).
 *
 * O descarte não é higiene: num filtro por substring, uma letra sozinha casa TODO item do corpus e
 * portanto **não filtra nada** — mas MARCA tudo. Buscar `impostos competem à União` deixava o `a`
 * (o `à` normaliza para `a`) marcado dentro de "Quais", "da", "estrangeiros". O ruído sempre
 * existiu na linha do título; ficou intolerável quando a legislação passou a exibir o corpo da
 * resposta (D-LEG-11a) e o realce saiu de uma linha para um parágrafo inteiro.
 *
 * **O conjunto devolvido não muda** — é o que justifica mexer nas seis abas de uma vez. As três
 * letras em jogo estão em 100% dos itens, então elas nunca foram o que filtrava; quem filtra são
 * os outros termos do E lógico. Há teste medindo isso contra os corpora reais.
 *
 * O corte vive aqui, e não no `highlight`, para preservar o invariante que sustenta o módulo:
 * filtro e marcação consomem **a mesma** lista de termos, então nunca discordam sobre o que casou.
 * Filtrar só na marcação consertaria a tela ao preço de fazer os dois divergirem.
 *
 * **Query inteira de letras isoladas devolve a lista inteira**, como a query vazia. É escolha, não
 * consequência: `a` não carrega sinal nenhum contra este corpus, e devolver tudo é o mesmo que já
 * fazemos quando não há termo utilizável.
 */
export function parseQuery(query: string): string[] {
  const q = normalizeText(query);
  return q ? q.split(" ").filter((t) => !LETRA_SOZINHA.test(t)) : [];
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
