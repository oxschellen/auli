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
 * Filtra pareceres por número, assunto ou sinopse via `textSearch` (acentos + multi-termo, E entre
 * termos). Query vazia devolve a lista inteira (mesma referência).
 *
 * O `resumo` entra na busca — é ele a chave boa, porque a sinopse carrega as palavras-chave do tema
 * que a ementa costuma omitir. O corpo integral nunca esteve aqui (buscá-lo transformaria quase toda
 * query num acerto) e agora nem chega ao navegador.
 */
export function searchPareceres(pareceres: Parecer[], query: string): Parecer[] {
  const terms = parseQuery(query);
  if (terms.length === 0) return pareceres;
  return pareceres.filter((p) =>
    haystackMatches(buildHaystack([p.numero, p.assunto, p.resumo]), terms),
  );
}
