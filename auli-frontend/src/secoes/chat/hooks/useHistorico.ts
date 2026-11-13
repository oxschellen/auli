import { useMemo } from "react";
import { CHAVES, gravarTexto, lerTexto, useTextoGravado } from "../../../lib/storage";
import * as historico from "../lib/historico";
import type { ItemHistorico } from "../lib/historico";
import type { QuestionType } from "../lib/tipoDeConsulta";

/**
 * As perguntas enviadas nesta entidade, guardadas só neste navegador (Etapa 1: sem login e sem
 * servidor).
 *
 * Toda gravação relê a lista antes de alterá-la, em vez de partir da que está na tela: com duas
 * abas abertas, a que grava depois não apaga o que a outra acabou de registrar.
 *
 * É a ÚNICA porta para a lista. Na Etapa 2 (login), o que muda é este arquivo — quem usa o hook
 * não sabe onde a lista mora.
 */
export function useHistorico(entityId: string) {
  const chave = CHAVES.historico(entityId);
  const bruto = useTextoGravado(chave);
  // O parse só se refaz quando o texto gravado muda.
  const itens = useMemo(() => historico.desserializar(bruto), [bruto]);

  const alterar = (mudar: (atual: ItemHistorico[]) => ItemHistorico[]) =>
    gravarTexto(chave, historico.serializar(mudar(historico.desserializar(lerTexto(chave)))));

  return {
    itens,
    registrar: (pergunta: string, tipo: QuestionType) =>
      alterar((atual) =>
        historico.registrar(atual, pergunta, tipo, Date.now(), crypto.randomUUID()),
      ),
    alternarFavorita: (id: string) => alterar((atual) => historico.alternarFavorita(atual, id)),
    remover: (id: string) => alterar((atual) => historico.remover(atual, id)),
    limparRecentes: () => alterar(historico.limparRecentes),
  };
}
