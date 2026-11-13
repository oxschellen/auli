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
  /** Quem autorizou a hospedagem, quando e por qual meio. Exigido se `hospedado` (D-TRIB-10). */
  autorizacao?: string;
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
  /** Obrigatório: o critério editorial exige data em todo item, e a análise não tinha campo. */
  ano: number;
  resumo: string;
  link?: string;
  pdf?: string;
  hospedado?: boolean;
  /** Quem autorizou a hospedagem, quando e por qual meio. Exigido se `hospedado` (D-TRIB-10). */
  autorizacao?: string;
}

/** Como se chega ao livro (D-TRIB-24). Vira selo na lista, para o leitor saber antes de clicar. */
export type AcessoLivro = "aberto" | "comercial" | "biblioteca";

/**
 * Livro (D-TRIB-22). **Não tem `pdf` nem `hospedado`, e a ausência é o desenho**: obra comercial
 * não se hospeda, e livro aberto já está publicado por quem o publica — entra pelo `link`.
 * Aparecer `pdf` aqui um dia é decisão de schema, não descuido de quem copiou um artigo; por isso
 * a conferência do catálogo recusa esses campos em vez de ignorá-los.
 */
export interface Livro extends ItemBase {
  titulo: string;
  autores: string;
  editora: string;
  ano: number;
  /** Opcional (D-TRIB-23): edição antiga e publicação institucional muitas vezes não têm. */
  isbn?: string;
  acesso: AcessoLivro;
  /** Resenha do Curador — nunca a sinopse da editora, que é material promocional (D-TRIB-25). */
  resumo: string;
  /** Editora, repositório ou catálogo de biblioteca. Obrigatório: é a "fonte localizável". */
  link: string;
}

/**
 * Quem lê, escolhe e apresenta uma estante. O critério editorial promete o nome e o contato no
 * rodapé de cada uma — a curadoria é pessoal e nomeada de propósito: ela não é cargo nem função
 * institucional, e é isso que a distingue de uma chancela.
 */
export interface Curador {
  nome: string;
  /** Como falar com ele — e-mail, perfil, o que for. Texto livre; o rodapé linka se for URL. */
  contato?: string;
}

/** As cinco estantes. `criterios` é a régua, não é estante — por isso fica fora daqui. */
export type Estante = "artigos" | "instituicoes" | "dados" | "analises" | "livros";

/**
 * As estantes em ordem de menu, para quem precisa percorrer todas.
 *
 * Existe para que acrescentar a sexta seja uma linha e não uma caçada: antes da estante Livros, a
 * conferência do catálogo repetia o array literal em três lugares, e nada obrigava os três a
 * concordarem com o tipo `Estante`. O `satisfies` é o que amarra — sobrou ou faltou estante, não
 * compila.
 */
export const ESTANTES = [
  "artigos",
  "instituicoes",
  "dados",
  "analises",
  "livros",
] as const satisfies readonly Estante[];

export interface TributumCatalogo {
  artigos: Artigo[];
  instituicoes: Instituicao[];
  dados: Dataset[];
  analises: Analise[];
  livros: Livro[];
  /**
   * Curador por estante. **Opcional por estante**: o rodapé só aparece onde há alguém declarado.
   * Nome de curador não se inventa nem se herda — estante sem curador é estante sem rodapé, e a
   * ausência é visível para quem mantém o catálogo.
   */
  curadores?: Partial<Record<Estante, Curador>>;
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
 * D-TRIB-8 na UI: o PDF só é servido daqui quando é **nosso e a autorização está registrada** —
 * `hospedado: true`, `pdf` presente E `autorizacao` preenchida.
 *
 * Item de terceiro entra como link + resenha própria, e é por isso que os campos são conferidos
 * juntos: `pdf` preenchido sozinho não basta, e `hospedado: true` sozinho também não.
 *
 * **A `autorizacao` entrou aqui porque o critério editorial afirma que ela "fica registrada"**
 * (D-TRIB-10). Enquanto era só um booleano, `hospedado` dizia QUE houve autorização e nada sobre
 * de quem, quando ou onde — e uma regra que não se pode conferir não é regra. Com o campo exigido,
 * um item hospedado sem procedência degrada para link em vez de servir o arquivo. A conferência do
 * catálogo (`types.test.ts`) pega o mesmo caso antes, na revisão; esta é a rede de baixo.
 *
 * Predicado único para que a lista e o modal não possam divergir sobre o que é hospedado.
 */
export const temPdfHospedado = (item: {
  pdf?: string;
  hospedado?: boolean;
  autorizacao?: string;
}): item is { pdf: string; hospedado: true; autorizacao: string } =>
  item.hospedado === true && !!item.pdf && !!item.autorizacao?.trim();
