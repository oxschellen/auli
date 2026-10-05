import { useState } from "react";
import type { Message } from "../../../types/chat";

/** A conversa começa VAZIA: antes da primeira pergunta quem aparece é o `EstadoInicial`
 *  (D-UI-10). A saudação "Olá! Como posso ajudar?" que morava aqui ressurgia acima da pergunta
 *  assim que a lista passava a ser desenhada. */
export const useMessages = () => {
  const [messages, setMessages] = useState<Message[]>([]);

  return { messages, setMessages };
};
