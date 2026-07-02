import axios from "axios";
import type { Dispatch, SetStateAction } from "react";
import type { Message } from "../../../types/chat";

interface CallServerAPIArgs {
  prompt: string;
  messages: Message[];
  setMessages: Dispatch<SetStateAction<Message[]>>;
  setPrompt: Dispatch<SetStateAction<string>>;
  setLoading: Dispatch<SetStateAction<boolean>>;
  API_URL: string;
  /** Active entity id (state), so the backend queries the right tenant's collections. */
  entityId?: string;
}

interface QuestionResponse {
  answer?: string;
}

/** Abort the request if the server hasn't responded within this window. */
const REQUEST_TIMEOUT_MS = 25_000;

/** User-facing copy (pt-BR). Kept here so the wording lives in one place. */
const MESSAGES = {
  thinking: "Aguarde! Pensando...",
  emptyAnswer: "Desculpe! Não foi possível obter resposta.",
  timeout: "Desculpe! A requisição excedeu o tempo limite.",
  unavailable: "Desculpe!, o Servidor Auli não está disponível.",
  rateLimited: "Muitas requisições. Aguarde alguns instantes e tente novamente.",
} as const;

export const callServerAPI = async ({
  prompt,
  messages,
  setMessages,
  setPrompt,
  setLoading,
  API_URL,
  entityId,
}: CallServerAPIArgs): Promise<void> => {
  setLoading(true);

  const messagesArray: Message[] = [...messages];

  messagesArray.push({
    id: crypto.randomUUID(),
    from: "user",
    text: prompt,
    showButton: true,
  });

  messagesArray.push({
    id: crypto.randomUUID(),
    from: "server",
    text: MESSAGES.thinking,
    showButton: false,
  });

  setMessages([...messagesArray]);

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);

  try {
    const res = await axios.post<QuestionResponse>(
      API_URL,
      entityId ? { question: prompt, entity: entityId } : { question: prompt },
      { signal: controller.signal },
    );

    const serverMessage = res.data?.answer || MESSAGES.emptyAnswer;

    messagesArray.pop();
    messagesArray.push({
      id: crypto.randomUUID(),
      from: "server",
      text: serverMessage,
      showButton: true,
    });

    setPrompt("");
  } catch (e) {
    // Distinguish: timeout (aborted) → rate limited (HTTP 429, server sends a friendly pt-BR
    // `{ error }` body) → anything else (server down / network). Without this, a throttled user is
    // wrongly told the server is unavailable.
    let serverMessage: string;
    if (axios.isCancel(e)) {
      serverMessage = MESSAGES.timeout;
    } else {
      const err = e as { response?: { status?: number; data?: { error?: string } } };
      serverMessage =
        err.response?.status === 429
          ? err.response.data?.error || MESSAGES.rateLimited
          : MESSAGES.unavailable;
    }

    messagesArray.pop();
    messagesArray.push({
      id: crypto.randomUUID(),
      from: "server",
      text: serverMessage,
      showButton: true,
    });

    setPrompt("");
  } finally {
    clearTimeout(timeoutId);
    setMessages([...messagesArray]);
    setLoading(false);
  }
};
