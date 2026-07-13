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

/** Filtra pareceres por número ou assunto (case-insensitive). Query vazia devolve a lista inteira. */
export function searchPareceres(pareceres: Parecer[], query: string): Parecer[] {
  const q = query.trim().toLowerCase();
  if (!q) return pareceres;
  return pareceres.filter(
    (p) => p.numero.toLowerCase().includes(q) || p.assunto.toLowerCase().includes(q),
  );
}
