import type { Dispatch, SetStateAction } from "react";
import type { Fonte } from "../../api/chat";
import type { QuestionType } from "./lib/tipoDeConsulta";

/**
 * O que uma mensagem da conversa É — e daí quem a desenha e como (C4 do
 * docs/ESTUDO-frontend-organizacao.md, que tirou o `showButton`, o `from` e o `pendente`):
 *
 * - `pergunta`: o balão do usuário.
 * - `espera`: o lugar da resposta enquanto a consulta roda, desenhado como a animação `Pensando`.
 * - `resposta`: o cartão largo, com o tipo, as fontes, o log e o copiar; a rolagem vai ao COMEÇO
 *   dela (D-UI-2).
 * - `erro`: tempo esgotado, servidor fora ou limite de requisições — um balão simples, sem ações.
 */
export type Natureza = "pergunta" | "espera" | "resposta" | "erro";

/** A single message rendered in the chat transcript. */
export interface Message {
  id: string;
  natureza: Natureza;
  /** O texto. Na `espera`, segue preenchido para quem não desenha a animação (e para os testes). */
  text: string;
  /** UUID do registro de auditoria desta resposta (`log_id` do backend), quando houve gravação.
   *  Ausente fora das respostas e quando o log falhou — e é essa ausência que
   *  esconde o ícone: não existe registro para abrir. */
  logId?: string;
  /** Lista de fontes recuperadas (D-SF-10) — só em respostas de Serviços + FAQs. */
  fontes?: Fonte[];
  /** Tipo de consulta que produziu esta resposta (D-UI-9) — a linha de contexto no topo dela. */
  tipo?: QuestionType;
  /** Quanto a resposta levou, medido no navegador do envio à chegada (D-UI-9). */
  duracaoMs?: number;
}

/** O setter da caixa de mensagem (o `useState<string>` do `Chat`). */
export type SetPrompt = Dispatch<SetStateAction<string>>;
