import type { Dispatch, SetStateAction } from "react";
import { perguntar } from "../../../api/chat";
import type { Message } from "../tipos";
import type { QuestionType } from "./useQuestionType";

interface CallServerAPIArgs {
  prompt: string;
  messages: Message[];
  setMessages: Dispatch<SetStateAction<Message[]>>;
  setPrompt: Dispatch<SetStateAction<string>>;
  setLoading: Dispatch<SetStateAction<boolean>>;
  /** Active entity id (state), so the backend queries the right tenant's collections. */
  entityId?: string;
  /** Tipo de consulta selecionado ("1" = Serviços+FAQs, "2" = Pareceres, "3" = Acórdãos TARF);
   *  enviado como o campo numérico `type`. */
  questionType: QuestionType;
}

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
  entityId,
  questionType,
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
    pendente: true,
  });

  setMessages([...messagesArray]);
  // A caixa esvazia no ENVIO, não na chegada (D-UI-2): a pergunta já está no balão do usuário,
  // e deixá-la no campo com "Pronto para enviar" durante a espera convidava a reenviar. Quem
  // quiser reaproveitá-la usa o botão do próprio balão.
  setPrompt("");
  const inicio = Date.now();

  const r = await perguntar(prompt, entityId, Number(questionType));

  messagesArray.pop();
  if (r.ok) {
    messagesArray.push({
      id: crypto.randomUUID(),
      from: "server",
      text: r.texto || MESSAGES.emptyAnswer,
      showButton: true,
      logId: r.logId,
      fontes: r.fontes,
      tipo: questionType,
      duracaoMs: Date.now() - inicio,
    });
  } else {
    // O 429 traz a mensagem amigável do próprio servidor; sem ela, a nossa.
    const texto =
      r.falha === "tempo"
        ? MESSAGES.timeout
        : r.falha === "limite"
          ? r.mensagem || MESSAGES.rateLimited
          : MESSAGES.unavailable;
    messagesArray.push({ id: crypto.randomUUID(), from: "server", text: texto, showButton: true });
  }
  setMessages([...messagesArray]);
  setLoading(false);
};
