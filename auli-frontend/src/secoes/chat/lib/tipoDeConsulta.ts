import type { Collection, Entity } from "../../../contexto/entities";
import { hasCollection } from "../../../contexto/entities";

/**
 * O tipo de consulta enviado com a pergunta do chat: "1" = Serviços+FAQs (padrão), "2" = Pareceres,
 * "3" = Acórdãos TARF, "4" = Legislação.
 *
 * Os valores são **contrato com o backend** (`QueryType::from_code` no `rag.rs`) e não podem ser
 * reordenados: "2" é pareceres desde a primeira versão da tab do auditor.
 */
export type QuestionType = "1" | "2" | "3" | "4";

export const DEFAULT_TYPE: QuestionType = "1";

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

export function isQuestionType(value: string | null): value is QuestionType {
  return value === "1" || value === "2" || value === "3" || value === "4";
}
