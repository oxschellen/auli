import type { QuestionType } from "./useQuestionType";

/**
 * O texto de cada tipo de consulta, num lugar só (D-UI-9): rótulo do seletor, descrição do menu,
 * apresentação do estado inicial, texto de exemplo do campo e as perguntas de exemplo.
 *
 * **Texto editorial**: as perguntas de exemplo são sugestões genéricas, não frases do portal —
 * revisar antes de cada mudança, porque é a primeira coisa que quem chega lê.
 */
export interface TextoDoTipo {
  rotulo: string;
  descricao: string;
  apresentacao: string;
  placeholder: string;
  exemplos: string[];
}

export const TIPOS: Record<QuestionType, TextoDoTipo> = {
  "1": {
    rotulo: "Serviços + FAQs",
    descricao: "Qual serviço do portal se aplica ao caso",
    apresentacao:
      "procura nos serviços e nas perguntas frequentes do portal e indica qual se aplica ao caso — cada afirmação vem com a fonte.",
    placeholder: "Pergunte sobre serviços e FAQs do portal…",
    exemplos: [
      "Empresa nova: como apurar e recolher o ICMS?",
      "Como solicitar isenção de IPVA para pessoa com deficiência?",
      "Como emitir a certidão de situação fiscal?",
    ],
  },
  "2": {
    rotulo: "Pareceres",
    descricao: "Respostas oficiais a consultas tributárias",
    apresentacao: "procura nos pareceres — as respostas oficiais a consultas sobre a legislação tributária.",
    placeholder: "Pergunte sobre o entendimento dos pareceres…",
    exemplos: [
      "Incide ICMS na transferência de mercadoria entre estabelecimentos do mesmo titular?",
      "Como fica o crédito de ICMS na aquisição de energia elétrica?",
      "Qual o tratamento do diferencial de alíquota na compra para uso e consumo?",
    ],
  },
  "3": {
    rotulo: "Acórdãos TARF",
    descricao: "Decisões do Tribunal Administrativo de Recursos Fiscais",
    apresentacao: "procura nos acórdãos do Tribunal Administrativo de Recursos Fiscais.",
    placeholder: "Pergunte como o TARF tem decidido…",
    exemplos: [
      "Como o TARF decide sobre multa por falta de emissão de nota fiscal?",
      "Há acórdãos sobre glosa de crédito presumido?",
      "O que o TARF entende sobre decadência no lançamento por homologação?",
    ],
  },
  "4": {
    rotulo: "Legislação",
    descricao: "Leis e regulamentos, em perguntas e respostas",
    apresentacao: "procura nas leis e regulamentos, organizados em perguntas e respostas.",
    placeholder: "Pergunte sobre leis e regulamentos…",
    exemplos: [
      "Quais as hipóteses de isenção do ITCD?",
      "O que a Lei Complementar 123 define como microempresa?",
      "Quais penalidades se aplicam à falta de recolhimento do ICMS?",
    ],
  },
};
