import { parseQuery, buildHaystack, haystackMatches } from "../../shared/textSearch";

/** Um parecer parseado do `<id>-portal-pareceres.txt` (formato em blocos `// N`). */
export interface Parecer {
  id: number;
  /** Ex.: "PARECER Nº 26164". */
  numero: string;
  /** Ementa / assunto (uma linha). */
  assunto: string;
  /** URL da ficha (DocumentView). Pode ser vazia. */
  link: string;
  /** Texto integral do parecer. */
  corpo: string;
}

/**
 * Parseia o `.txt` autorado dos pareceres. Cada registro é um bloco delimitado por `// N`:
 *
 * ```text
 * // 1
 * ## pergunta:
 * descricao: PARECER Nº 26164
 * assunto  : ICMS – ...
 * link: http://.../DocumentView.aspx?inpKey=299748
 * ## resposta:
 * <corpo integral, multilinha>
 * ```
 *
 * O `descricao`/`assunto`/`link` (seção `## pergunta`) viram os campos; tudo após `## resposta:` é o
 * `corpo`. Blocos sem conteúdo aproveitável são descartados.
 */
export function parsePareceres(text: string): Parecer[] {
  const blocks = text.split(/^\/\/\s*\d+\s*$/m);
  const out: Parecer[] = [];
  let id = 0;

  for (const block of blocks) {
    const respIdx = block.indexOf("## resposta:");
    if (respIdx === -1) continue;

    const pergunta = block.slice(0, respIdx);
    const corpo = block.slice(respIdx + "## resposta:".length).trim();

    const numero = (pergunta.match(/^descricao:\s*(.*)$/m)?.[1] ?? "").trim();
    const assunto = (pergunta.match(/^assunto\s*:\s*(.*)$/m)?.[1] ?? "").trim();
    const link = (pergunta.match(/^link:\s*(.*)$/m)?.[1] ?? "").trim();

    if (!numero && !assunto && !corpo) continue;
    out.push({ id: id++, numero, assunto, link, corpo });
  }

  return out;
}

/**
 * Filtra pareceres por número ou assunto via `textSearch` (acentos + multi-termo, E entre termos).
 * Query vazia devolve a lista inteira (mesma referência).
 *
 * Os campos são `[numero, assunto]` — os mesmos de sempre. O `corpo` fica **de fora** de propósito:
 * é o texto integral, e buscá-lo transformaria quase toda query num acerto. Quando a tab migrar para
 * o JSON leve dos pareceres (numero/assunto/resumo/link), a **sinopse** entra aqui — é ela a chave de
 * busca boa, não o corpo.
 */
export function searchPareceres(pareceres: Parecer[], query: string): Parecer[] {
  const terms = parseQuery(query);
  if (terms.length === 0) return pareceres;
  return pareceres.filter((p) => haystackMatches(buildHaystack([p.numero, p.assunto]), terms));
}
