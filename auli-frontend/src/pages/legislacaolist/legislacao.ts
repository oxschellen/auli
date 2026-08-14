import { parseQuery, buildHaystack, haystackMatches } from "../../shared/textSearch";

/**
 * Uma entrada do `<id>-legislacao-index.json` — o índice leve derivado da árvore
 * `docs/legislacao/<lei>/*.md` por `auli-collections <id> indice legislacao`.
 *
 * **Shape próprio, com nomes limpos** (D-LEG-11): o índice de jurisprudência carrega
 * `numero`/`assunto` por compatibilidade com uma tab que já existia; aqui não há contrato legado, e
 * os campos usam o vocabulário unificado das árvores.
 *
 * Sem `resumo` — legislação não tem `## resumo`; a resposta É o `corpo`, e ele **vem no índice**
 * (D-LEG-11a), ao contrário das coleções de jurisprudência. Lá o corpo é a íntegra de um acórdão e
 * fica de fora por tamanho; aqui são 187 respostas somando 73 KB. Com ele, a aba deixa de ser só um
 * índice navegável e passa a responder — sem uma segunda requisição por linha aberta.
 */
export interface Dispositivo {
  /** A pergunta. Ex.: "Qual o prazo e a forma da impugnação ao Auto de Lançamento?" */
  titulo: string;
  /** Hierarquia da lei, `Lei | Título | Capítulo` — o 1º segmento é a lei (ver `leiDe`). */
  trilha: string;
  /** O dispositivo. Ex.: "Art. 28", "Art. 11, § 3º". */
  ementa: string;
  /** A resposta, em markdown. É o `## corpo` do `.md` da árvore. */
  corpo: string;
  /** URL da lei no portal oficial. */
  link: string;
}

/** Separador dos segmentos da trilha — o mesmo das outras coleções. */
const SEP = " | ";

/**
 * A lei de uma trilha: o 1º segmento. Espelha o `lei_da_trilha` do contrato (Rust), e a razão de
 * existir é a mesma — é por ele que se agrupa, e ele é também o nome da subpasta na árvore.
 */
export function leiDe(trilha: string): string {
  const i = trilha.indexOf(SEP);
  return i === -1 ? trilha : trilha.slice(0, i);
}

/** O caminho DENTRO da lei (título > capítulo > seção), sem repetir o nome dela no cabeçalho. */
export function caminhoNaLei(trilha: string): string {
  const i = trilha.indexOf(SEP);
  return i === -1 ? "" : trilha.slice(i + SEP.length);
}

/**
 * Pré-normaliza os campos de busca — uma vez por acervo, não por tecla (mesma disciplina do
 * `buildPareceresIndex`). Aqui o acervo é pequeno, mas a regra vale igual quando entrarem CF e CTN.
 */
export function buildLegislacaoIndex(itens: Dispositivo[]): string[][] {
  return itens.map((d) => buildHaystack([d.titulo, d.trilha, d.ementa, d.corpo]));
}

/**
 * Filtra por pergunta, trilha, dispositivo ou resposta (acentos + multi-termo, E entre termos).
 *
 * A `trilha` entra na busca porque é ela que carrega o vocabulário estrutural da lei — quem digita
 * "infrações formais" está procurando o capítulo, não uma palavra da pergunta. E o `ementa` é o que
 * faz "art. 11" achar o dispositivo direto, que é como um analista busca.
 *
 * O `corpo` entra pela mesma razão que a sinopse entra no `buildPareceresIndex`: ele é exibido na
 * tela, e texto visível que a busca não encontra é uma armadilha — o termo está ali, marcado em
 * amarelo quando a linha abre, mas a linha não aparece no filtro.
 */
export function searchLegislacao(
  itens: Dispositivo[],
  query: string,
  index?: string[][],
): Dispositivo[] {
  const terms = parseQuery(query);
  if (terms.length === 0) return itens;
  return itens.filter((d, i) =>
    haystackMatches(index?.[i] ?? buildHaystack([d.titulo, d.trilha, d.ementa, d.corpo]), terms),
  );
}

/**
 * Agrupa por lei **preservando a ordem de chegada** — que é a do índice derivado: leis em ordem
 * alfabética e, dentro de cada uma, os dispositivos na ordem do texto legal (D-LEG-12).
 *
 * Reordenar aqui desfaria o trabalho do derivador, que é onde a ordem é decidida (e testada).
 */
export function agruparPorLei(itens: Dispositivo[]): [string, Dispositivo[]][] {
  const grupos = new Map<string, Dispositivo[]>();
  for (const d of itens) {
    const lei = leiDe(d.trilha);
    const atual = grupos.get(lei);
    if (atual) atual.push(d);
    else grupos.set(lei, [d]);
  }
  return [...grupos.entries()];
}
