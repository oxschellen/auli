import type { Dispatch, SetStateAction } from "react";

/** Who authored a chat message. */
export type MessageSender = "user" | "server";

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
}

/** Setter returned by `usePrompt`, matching React's `useState<string>` setter. */
export type SetPrompt = Dispatch<SetStateAction<string>>;
