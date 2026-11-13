import { useSyncExternalStore } from "react";
import * as historico from "../lib/historico";
import type { ItemHistorico } from "../lib/historico";
import type { QuestionType } from "./useQuestionType";

/** Uma chave por entidade (D-HIST-1): a pergunta feita no RS não aparece em SP. */
const PREFIXO = "auli.historico.";

const VAZIO: ItemHistorico[] = [];
const ouvintes = new Set<() => void>();
/** Última leitura de cada chave. O `useSyncExternalStore` exige a MESMA referência enquanto nada
 *  muda, então só se refaz o parse quando o texto gravado muda. */
const cache = new Map<string, { bruto: string | null; itens: ItemHistorico[] }>();

function ler(chave: string): ItemHistorico[] {
  let bruto: string | null;
  try {
    bruto = localStorage.getItem(chave);
  } catch {
    // Sem localStorage (bloqueado pelo navegador): a lista vale só nesta sessão, na memória.
    return cache.get(chave)?.itens ?? VAZIO;
  }
  const visto = cache.get(chave);
  if (visto && visto.bruto === bruto) return visto.itens;
  const itens = historico.desserializar(bruto);
  cache.set(chave, { bruto, itens });
  return itens;
}

function gravar(chave: string, itens: ItemHistorico[]) {
  const bruto = historico.serializar(itens);
  try {
    localStorage.setItem(chave, bruto);
  } catch {
    // Não fatal, como nas outras preferências: segue valendo nesta sessão.
  }
  cache.set(chave, { bruto, itens });
  ouvintes.forEach((avisar) => avisar());
}

function assinar(avisar: () => void) {
  ouvintes.add(avisar);
  // `storage` dispara nas OUTRAS abas quando uma grava: duas abas da Auli mostram a mesma lista.
  window.addEventListener("storage", avisar);
  return () => {
    ouvintes.delete(avisar);
    window.removeEventListener("storage", avisar);
  };
}

/**
 * As perguntas enviadas nesta entidade, guardadas só neste navegador (Etapa 1: sem login e sem
 * servidor).
 *
 * Toda gravação relê a lista antes de alterá-la, em vez de partir da que está na tela: com duas
 * abas abertas, a que grava depois não apaga o que a outra acabou de registrar.
 *
 * É a ÚNICA porta para o armazenamento. Na Etapa 2 (login), o que muda é este arquivo — quem usa
 * o hook não sabe onde a lista mora.
 */
export function useHistorico(entityId: string) {
  const chave = PREFIXO + entityId;
  const itens = useSyncExternalStore(assinar, () => ler(chave));

  return {
    itens,
    registrar: (pergunta: string, tipo: QuestionType) =>
      gravar(
        chave,
        historico.registrar(ler(chave), pergunta, tipo, Date.now(), crypto.randomUUID()),
      ),
    alternarFavorita: (id: string) => gravar(chave, historico.alternarFavorita(ler(chave), id)),
    remover: (id: string) => gravar(chave, historico.remover(ler(chave), id)),
    limparRecentes: () => gravar(chave, historico.limparRecentes(ler(chave))),
  };
}
