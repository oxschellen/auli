import { useState } from "react";
import type { Collection, Entity } from "../../../shared/entities";
import { hasCollection } from "../../../shared/entities";

/**
 * O tipo de consulta enviado com a pergunta do chat: "1" = Serviços+FAQs (padrão), "2" = Pareceres,
 * "3" = Acórdãos TARF, "4" = Legislação.
 *
 * Os valores são **contrato com o backend** (`QueryType::from_code` no `rag.rs`) e não podem ser
 * reordenados: "2" é pareceres desde a primeira versão da tab do auditor.
 */
export type QuestionType = "1" | "2" | "3" | "4";

const STORAGE_KEY = "auli.questionType";
const DEFAULT_TYPE: QuestionType = "1";

/**
 * A coleção que cada tipo consulta — `null` para o tipo padrão, que é serviços+FAQs e existe em toda
 * entidade (a parte ausente simplesmente sai vazia do contexto, tolerância antiga do chat).
 *
 * É esta tabela que liga o seletor ao `registry.toml`: entidade sem a coleção não vê a opção. Sem
 * isso, escolher "Acórdãos TARF" em SP mandaria uma pergunta que só pode voltar como "não está
 * disponível para esta entidade" — um caminho que a interface não deveria oferecer.
 */
const COLECAO_DO_TIPO: Record<QuestionType, Collection | null> = {
  "1": null,
  "2": "pareceres",
  "3": "tarf",
  "4": "legislacao",
};

/** Os tipos que fazem sentido para esta entidade, na ordem de exibição. */
export function tiposDisponiveis(entity: Entity): QuestionType[] {
  return (Object.keys(COLECAO_DO_TIPO) as QuestionType[]).filter((tipo) => {
    const colecao = COLECAO_DO_TIPO[tipo];
    return colecao === null || hasCollection(entity, colecao);
  });
}

function isQuestionType(value: string | null): value is QuestionType {
  return value === "1" || value === "2" || value === "3" || value === "4";
}

/** Lê o tipo persistido (cai no padrão se ausente/desconhecido). */
function readStored(): QuestionType {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return isQuestionType(stored) ? stored : DEFAULT_TYPE;
  } catch {
    return DEFAULT_TYPE;
  }
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
    try {
      localStorage.setItem(STORAGE_KEY, value);
    } catch {
      // Não fatal: a seleção vale nesta sessão, só não sobrevive ao reload.
    }
    setQuestionType(value);
  };

  return { questionType: efetivo, updateQuestionType, disponiveis };
};
