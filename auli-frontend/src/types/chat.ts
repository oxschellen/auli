import type { Dispatch, SetStateAction } from "react";

/** Who authored a chat message. */
export type MessageSender = "user" | "server";

/** Um documento recuperado para a resposta, como a lista de fontes o mostra (D-SF-10). O `rotulo`
 *  é o mesmo da citação no texto (`"Serviço 2"`, `"FAQ 4"`), para o analista bater um com o outro. */
export interface Fonte {
  rotulo: string;
  titulo: string;
  link: string;
  /** Veredito da triagem (D-SF-9). Ausente = o documento foi à resposta sem veredito (sem triagem,
   *  triagem que falhou, ou documento que o modelo omitiu). */
  triagem?: "aplica" | "condicional" | "descarta";
}

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
}

/** Setter returned by `usePrompt`, matching React's `useState<string>` setter. */
export type SetPrompt = Dispatch<SetStateAction<string>>;
