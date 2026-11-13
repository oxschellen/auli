import { useRef, useState } from "react";
import { perguntar } from "../../../api/chat";
import type { QuestionType } from "../lib/tipoDeConsulta";
import type { Message } from "../tipos";

/** O que a conversa diz ao usuário (pt-BR), num lugar só. */
const TEXTOS = {
  pensando: "Aguarde! Pensando...",
  semResposta: "Desculpe! Não foi possível obter resposta.",
  tempo: "Desculpe! A requisição excedeu o tempo limite.",
  indisponivel: "Desculpe!, o Servidor Auli não está disponível.",
  limite: "Muitas requisições. Aguarde alguns instantes e tente novamente.",
} as const;

/**
 * A conversa com o servidor (I8 do docs/ESTUDO-frontend-organizacao.md): as mensagens, a espera e o
 * envio. A conversa começa VAZIA — antes da primeira pergunta quem aparece é o `EstadoInicial`
 * (D-UI-10).
 *
 * `enviar` responde NA HORA se aceitou a pergunta — `false` quando outra ainda está a caminho —, e
 * a resposta chega depois, trocando a espera pelo que veio. É o que deixa o `Chat` registrar no
 * histórico e esvaziar a caixa só quando a pergunta foi mesmo enviada.
 */
export function useConversa(entityId: string) {
  const [mensagens, setMensagens] = useState<Message[]>([]);
  const [esperando, setEsperando] = useState(false);
  // Trava síncrona: dois Enter seguidos chegam antes de o `esperando` virar estado.
  const emEnvio = useRef(false);

  const responder = async (pergunta: string, tipo: QuestionType, idDaEspera: string) => {
    const inicio = Date.now();
    const r = await perguntar(pergunta, entityId, Number(tipo));
    const resposta: Message = r.ok
      ? {
          id: crypto.randomUUID(),
          from: "server",
          text: r.texto || TEXTOS.semResposta,
          showButton: true,
          logId: r.logId,
          fontes: r.fontes,
          tipo,
          duracaoMs: Date.now() - inicio,
        }
      : {
          id: crypto.randomUUID(),
          from: "server",
          // O 429 traz a mensagem amigável do próprio servidor; sem ela, a nossa.
          text: r.falha === "limite" ? r.mensagem || TEXTOS.limite : TEXTOS[r.falha],
          showButton: true,
        };
    setMensagens((atual) => atual.map((m) => (m.id === idDaEspera ? resposta : m)));
    setEsperando(false);
    emEnvio.current = false;
  };

  const enviar = (pergunta: string, tipo: QuestionType): boolean => {
    if (emEnvio.current) return false;
    emEnvio.current = true;
    setEsperando(true);
    const idDaEspera = crypto.randomUUID();
    setMensagens((atual) => [
      ...atual,
      { id: crypto.randomUUID(), from: "user", text: pergunta, showButton: true },
      // Desenhada como a animação `Pensando`; o `text` fica para quem não desenha (e os testes).
      { id: idDaEspera, from: "server", text: TEXTOS.pensando, showButton: false, pendente: true },
    ]);
    void responder(pergunta, tipo, idDaEspera);
    return true;
  };

  return { mensagens, esperando, enviar };
}
