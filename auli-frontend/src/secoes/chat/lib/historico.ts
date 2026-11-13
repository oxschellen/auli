import { isQuestionType, type QuestionType } from "./tipoDeConsulta";

/**
 * Uma pergunta enviada, como o painel "Minhas perguntas" a guarda (D-HIST-2).
 *
 * Só a pergunta: a resposta NÃO é guardada. Ela é derivada do acervo, que muda a cada coleta, e
 * reenviar a pergunta a refaz com o acervo do dia.
 */
export interface ItemHistorico {
  id: string;
  pergunta: string;
  tipo: QuestionType;
  /** Quando foi enviada pela última vez (epoch em ms). */
  em: number;
  favorita: boolean;
}

/** Teto das NÃO favoritas (D-HIST-5). Favorita nunca é descartada sozinha. */
export const MAX_RECENTES = 50;

/** Versão do formato gravado. Mudou o formato, muda o número — e o que está gravado é ignorado. */
const VERSAO = 1;

/** A identidade de uma pergunta para a deduplicação: caixa e espaços não a tornam outra. */
const normalizar = (texto: string) => texto.trim().replace(/\s+/g, " ").toLowerCase();

/**
 * Registra um envio (D-HIST-3/4). A lista fica sempre da mais recente para a mais antiga.
 *
 * Pergunta repetida NO MESMO TIPO não duplica: sobe para o topo com a data nova e mantém o `id` e
 * a estrela. A mesma frase em outro tipo é outra consulta (outra coleção) e entra à parte.
 * `em` e `id` vêm de fora para a função seguir pura.
 */
export function registrar(
  itens: ItemHistorico[],
  pergunta: string,
  tipo: QuestionType,
  em: number,
  id: string,
): ItemHistorico[] {
  const chave = normalizar(pergunta);
  const antiga = itens.find((i) => i.tipo === tipo && normalizar(i.pergunta) === chave);
  const nova: ItemHistorico = {
    id: antiga?.id ?? id,
    pergunta: pergunta.trim(),
    tipo,
    em,
    favorita: antiga?.favorita ?? false,
  };
  let recentes = 0;
  return [nova, ...itens.filter((i) => i !== antiga)].filter(
    (i) => i.favorita || ++recentes <= MAX_RECENTES,
  );
}

export function alternarFavorita(itens: ItemHistorico[], id: string): ItemHistorico[] {
  return itens.map((i) => (i.id === id ? { ...i, favorita: !i.favorita } : i));
}

export function remover(itens: ItemHistorico[], id: string): ItemHistorico[] {
  return itens.filter((i) => i.id !== id);
}

/** Apaga o histórico e preserva as favoritas — que são o que a pessoa escolheu guardar. */
export function limparRecentes(itens: ItemHistorico[]): ItemHistorico[] {
  return itens.filter((i) => i.favorita);
}

export function serializar(itens: ItemHistorico[]): string {
  return JSON.stringify({ v: VERSAO, itens });
}

function eItem(x: unknown): x is ItemHistorico {
  if (typeof x !== "object" || x === null) return false;
  const i = x as Record<string, unknown>;
  return (
    typeof i.id === "string" &&
    typeof i.pergunta === "string" &&
    typeof i.tipo === "string" &&
    isQuestionType(i.tipo) &&
    typeof i.em === "number" &&
    typeof i.favorita === "boolean"
  );
}

/**
 * Lê o que está gravado. O localStorage é texto que qualquer um edita (e que uma versão futura
 * pode ter escrito): o que não tiver a forma esperada é descartado, nunca derruba o chat.
 */
export function desserializar(bruto: string | null): ItemHistorico[] {
  if (!bruto) return [];
  try {
    const dados: unknown = JSON.parse(bruto);
    if (typeof dados !== "object" || dados === null) return [];
    const { v, itens } = dados as { v?: unknown; itens?: unknown };
    if (v !== VERSAO || !Array.isArray(itens)) return [];
    return itens.filter(eItem);
  } catch {
    return [];
  }
}
