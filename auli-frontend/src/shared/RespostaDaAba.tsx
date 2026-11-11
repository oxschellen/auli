import { createContext, use, useCallback, useState } from "react";
import type { ReactNode } from "react";

/**
 * O indicador de resposta nas abas de estado. Uma pergunta segue rodando com a aba escondida
 * (D-ABAS-5), e sem isso nada diz quando ela terminou: a conversa de cada aba avisa quando começa
 * e quando para de esperar a resposta, e a faixa põe um spinner nas abas que estão esperando.
 */

const AvisarResposta = createContext<(respondendo: boolean) => void>(() => {});

/** As abas esperando resposta, pela chave. Mora no `App`, que tem a faixa e os painéis. */
export function useAbasRespondendo() {
  const [respondendo, setRespondendo] = useState<ReadonlySet<string>>(() => new Set());
  // Estável: o aviso de cada aba é memoizado por ele, e a conversa o chama num efeito.
  const marcarRespondendo = useCallback(
    (chave: string, sim: boolean) =>
      setRespondendo((s) => {
        if (s.has(chave) === sim) return s;
        const novo = new Set(s);
        if (sim) novo.add(chave);
        else novo.delete(chave);
        return novo;
      }),
    [],
  );
  return { respondendo, marcarRespondendo };
}

/** Liga a conversa da aba `chave` ao `marcarRespondendo` do `App`. */
export function RespostaDaAbaProvider({
  chave,
  marcar,
  children,
}: {
  chave: string;
  marcar: (chave: string, sim: boolean) => void;
  children: ReactNode;
}) {
  const avisar = useCallback((sim: boolean) => marcar(chave, sim), [chave, marcar]);
  return <AvisarResposta.Provider value={avisar}>{children}</AvisarResposta.Provider>;
}

/** Para a conversa avisar a sua aba. Fora de uma aba (o `Chat` nos testes), não faz nada. */
export const useAvisarResposta = () => use(AvisarResposta);
