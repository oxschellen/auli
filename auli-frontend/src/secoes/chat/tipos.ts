import type { Dispatch, SetStateAction } from "react";
import type { Fonte } from "../../api/chat";
import type { QuestionType } from "./hooks/useQuestionType";

/** Who authored a chat message. */
type MessageSender = "user" | "server";

/** A single message rendered in the chat transcript. */
export interface Message {
  id: string;
  from: MessageSender;
  text: string;
  showButton: boolean;
  /** UUID do registro de auditoria desta resposta (`log_id` do backend), quando houve gravação.
   *  Ausente na saudação, nas mensagens do usuário e quando o log falhou — e é essa ausência que
   *  esconde o ícone: não existe registro para abrir. */
  logId?: string;
  /** Lista de fontes recuperadas (D-SF-10) — só em respostas de Serviços + FAQs. */
  fontes?: Fonte[];
  /** O lugar da resposta enquanto a consulta roda: desenhado como a animação `Pensando`, não
   *  como texto. O `text` segue preenchido para quem não desenha (e para os testes). */
  pendente?: boolean;
  /** Tipo de consulta que produziu esta resposta (D-UI-9) — a linha de contexto no topo dela. */
  tipo?: QuestionType;
  /** Quanto a resposta levou, medido no navegador do envio à chegada (D-UI-9). */
  duracaoMs?: number;
}

/** Setter returned by `usePrompt`, matching React's `useState<string>` setter. */
export type SetPrompt = Dispatch<SetStateAction<string>>;
