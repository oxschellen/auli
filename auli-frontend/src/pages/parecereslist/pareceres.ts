import { parseQuery, buildHaystack, haystackMatches } from "../../shared/textSearch";

/**
 * Uma entrada do `<id>-pareceres-index.json` — o índice leve derivado da árvore `docs/pareceres/*.md`
 * por `auli-collections <id> indice`.
 *
 * **Sem `corpo` de propósito.** O texto integral fica no portal (`link`); a tab mostra a sinopse. Foi
 * o que tirou o SP de 147 MB para 33 MB (7,4 MB gzipado) — a mesma escolha da G3 no servidor.
 */
export interface Parecer {
  /** Ex.: "PARECER Nº 26164". Identidade do documento — única na coleção. */
  numero: string;
  /** Ementa / assunto (uma linha). */
  assunto: string;
  /** Sinopse em markdown (descrição resumida + palavras-chave). Vazia se o `.md` está pendente. */
  resumo: string;
  /** URL da ficha no portal. Pode ser vazia. */
  link: string;
}

/**
 * Pré-normaliza os campos de busca de cada parecer — **uma vez por acervo, não por tecla**.
 *
 * É a disciplina que o `textSearch` documenta e que esta lista precisava mais que as outras: o
 * `resumo` do SP são 17,4 milhões de caracteres, e normalizá-los a cada letra digitada custava
 * ~400 ms de bloqueio da thread principal. Pré-computado, a tecla só paga `includes`.
 */
export function buildPareceresIndex(pareceres: Parecer[]): string[][] {
  return pareceres.map((p) => buildHaystack([p.numero, p.assunto, p.resumo]));
}

/**
 * Filtra pareceres por número, assunto ou sinopse via `textSearch` (acentos + multi-termo, E entre
 * termos). Query vazia devolve a lista inteira (mesma referência).
 *
 * O `resumo` entra na busca — é ele a chave boa, porque a sinopse carrega as palavras-chave do tema
 * que a ementa costuma omitir. O corpo integral nunca esteve aqui (buscá-lo transformaria quase toda
 * query num acerto) e agora nem chega ao navegador.
 *
 * `index` é a saída de `buildPareceresIndex` para ESTA lista, na mesma ordem. Omitido, o haystack é
 * construído na hora — cômodo para listas pequenas e testes, caro no acervo do SP.
 */
export function searchPareceres(
  pareceres: Parecer[],
  query: string,
  index?: string[][],
): Parecer[] {
  const terms = parseQuery(query);
  if (terms.length === 0) return pareceres;
  return pareceres.filter((p, i) =>
    haystackMatches(index?.[i] ?? buildHaystack([p.numero, p.assunto, p.resumo]), terms),
  );
}
