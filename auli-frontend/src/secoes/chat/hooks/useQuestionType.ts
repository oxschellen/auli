import { useState } from "react";
import type { Entity } from "../../../contexto/entities";
import { CHAVES, gravarTexto, lerTexto } from "../../../lib/storage";
import {
  DEFAULT_TYPE,
  isQuestionType,
  tiposDisponiveis,
  type QuestionType,
} from "../lib/tipoDeConsulta";

/** Lê o tipo persistido (cai no padrão se ausente/desconhecido). */
function readStored(): QuestionType {
  const stored = lerTexto(CHAVES.tipoDeConsulta);
  return isQuestionType(stored) ? stored : DEFAULT_TYPE;
}

/**
 * Tipo de consulta selecionado, persistido globalmente (uma chave de localStorage, compartilhada
 * entre entidades).
 *
 * A chave é global de propósito — a preferência é do usuário, não do estado. O preço disso é que ela
 * pode apontar para uma coleção que a entidade atual não tem (escolheu TARF no RS, trocou para SP):
 * o efeito abaixo devolve ao padrão nesse caso, em vez de deixar o chat mandar um tipo impossível.
 */
export const useQuestionType = (entity: Entity) => {
  const [questionType, setQuestionType] = useState<QuestionType>(readStored);

  const disponiveis = tiposDisponiveis(entity);

  // O tipo EFETIVO é derivado a cada render, não sincronizado por efeito: a escolha do usuário só
  // vale onde a entidade tem a coleção, e fora dela o chat usa o padrão. O estado cru pode ficar
  // apontando para um tipo indisponível, e isso é inofensivo — ninguém o lê (é o `efetivo` que sai
  // daqui), e `updateQuestionType` valida o valor NOVO, nunca o antigo. É também o que faz a
  // preferência voltar a valer sozinha quando o usuário retorna a uma entidade que a tenha.
  const efetivo = disponiveis.includes(questionType) ? questionType : DEFAULT_TYPE;

  // RadioGroup devolve `string | null`; valida antes de persistir/aplicar.
  const updateQuestionType = (value: string | null) => {
    if (!isQuestionType(value) || !disponiveis.includes(value)) return;
    gravarTexto(CHAVES.tipoDeConsulta, value);
    setQuestionType(value);
  };

  return { questionType: efetivo, updateQuestionType, disponiveis };
};
