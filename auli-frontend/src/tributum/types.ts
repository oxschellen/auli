/**
 * Tipos do catálogo da seção editorial **Tributum** (D-TRIB-1..9).
 *
 * Curadoria, não acervo: nada aqui entra no vocabulário `.md`, no enum `Kind`, no registry de
 * coleções ou nos prompts. O catálogo é um JSON único e compartilhado por todas as entidades
 * (D-TRIB-2/5) — o mesmo conteúdo em rs, sc, pr, sp.
 */

export interface ItemBase {
  id: string;
  /** D-TRIB-7: ausente ⇒ tratar como placeholder (fail-safe). */
  placeholder?: boolean;
}

export interface Artigo extends ItemBase {
  titulo: string;
  autores: string;
  instituicao: string;
  ano: number;
  resumo: string;
  link: string;
  /** Caminho local do PDF; só é usado se `hospedado === true` (D-TRIB-8). */
  pdf?: string;
  hospedado?: boolean;
}

export interface Instituicao extends ItemBase {
  nome: string;
  descricao: string;
  homepage: string;
}

export interface Dataset extends ItemBase {
  titulo: string;
  fonte: string;
  periodo: string;
  formato: string;
  atualizado_em: string;
  resumo: string;
  /** Download hospedado OU link externo — exatamente um dos dois. */
  arquivo?: string;
  link?: string;
}

export interface Analise extends ItemBase {
  titulo: string;
  autores: string;
  resumo: string;
  link?: string;
  pdf?: string;
  hospedado?: boolean;
}

export interface TributumCatalogo {
  artigos: Artigo[];
  instituicoes: Instituicao[];
  dados: Dataset[];
  analises: Analise[];
}

/**
 * D-TRIB-7, o default seguro: **só `placeholder: false` explícito publica**.
 *
 * A comparação é com `false`, e não `!!item.placeholder`, de propósito — é o que faz o campo
 * ausente cair no lado do exemplo. Um item novo, colado sem o campo, ganha o selo; para tirá-lo é
 * preciso afirmar por escrito que o conteúdo foi revisado. O modo de falha do desenho oposto é
 * silencioso e público: conteúdo de rascunho servido como curadoria por esquecimento.
 */
export const ehPlaceholder = (item: ItemBase): boolean => item.placeholder !== false;

/**
 * D-TRIB-8 na UI: o PDF só é servido daqui quando é **nosso** — `hospedado: true` E `pdf` presente.
 *
 * Item de terceiro entra como link + resenha própria, e é por isso que os dois campos são
 * conferidos juntos: `pdf` preenchido sozinho não basta, porque a autorização mora no `hospedado`.
 * Predicado único para que a lista e o modal não possam divergir sobre o que é hospedado.
 */
export const temPdfHospedado = (item: {
  pdf?: string;
  hospedado?: boolean;
}): item is { pdf: string; hospedado: true } => item.hospedado === true && !!item.pdf;
