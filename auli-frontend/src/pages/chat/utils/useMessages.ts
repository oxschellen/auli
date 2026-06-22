import { useState } from "react";
import type { Message } from "../../../types/chat";

export const useMessages = () => {
  const [messages, setMessages] = useState<Message[]>([
    {
      id: crypto.randomUUID(),
      from: "server",
      text: "Olá! Como posso ajudar?",
      showButton: false,
    },
  ]);

  return { messages, setMessages };
};
