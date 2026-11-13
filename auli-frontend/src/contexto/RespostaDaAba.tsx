import { createContext, use, useCallback, useState } from "react";
import type { ReactNode } from "react";

/**
 * Os sinais de resposta nas abas de estado. Uma pergunta segue rodando com a aba escondida
 * (D-ABAS-5), e sem isso nada diz quando ela terminou: a conversa de cada aba avisa quando começa
 * e quando para de esperar a resposta. A faixa põe um spinner nas abas que estão esperando e um
 * ponto nas que receberam resposta escondidas, até alguém abri-las.
 */

const AvisarResposta = createContext<(respondendo: boolean) => void>(() => {});

interface Avisos {
  respondendo: ReadonlySet<string>;
  novas: ReadonlySet<string>;
}

const com = (s: ReadonlySet<string>, chave: string, sim: boolean): ReadonlySet<string> => {
  const novo = new Set(s);
  if (sim) novo.add(chave);
  else novo.delete(chave);
  return novo;
};

/** As abas esperando resposta e as com resposta nova, pela chave. Mora no `App`, que tem a faixa e
 *  os painéis; `ativa` é a chave da aba à vista. */
export function useAbasRespondendo(ativa: string) {
  const [avisos, setAvisos] = useState<Avisos>(() => ({
    respondendo: new Set(),
    novas: new Set(),
  }));

  // Abrir a aba é ver a resposta nova dela: o ponto sai quando ela vira a ativa. Ajustado no
  // render, comparando com a última ativa vista, e não num efeito, que pintaria o ponto uma vez.
  const [ativaVista, setAtivaVista] = useState(ativa);
  if (ativa !== ativaVista) {
    setAtivaVista(ativa);
    if (avisos.novas.has(ativa)) setAvisos((a) => ({ ...a, novas: com(a.novas, ativa, false) }));
  }

  // Estável: o aviso de cada aba é memoizado por ele, e a conversa o chama num efeito.
  const marcarRespondendo = useCallback(
    (chave: string, sim: boolean, aVista: boolean) =>
      setAvisos((a) => {
        if (a.respondendo.has(chave) === sim) return a;
        return {
          respondendo: com(a.respondendo, chave, sim),
          // Terminou com a aba escondida: a resposta é nova até alguém abrir a aba.
          novas: !sim && !aVista ? com(a.novas, chave, true) : a.novas,
        };
      }),
    [],
  );
  return { respondendo: avisos.respondendo, novas: avisos.novas, marcarRespondendo };
}

/** Liga a conversa da aba `chave` ao `marcarRespondendo` do `App`. `aVista`: se a aba é a ativa. */
export function RespostaDaAbaProvider({
  chave,
  aVista,
  marcar,
  children,
}: {
  chave: string;
  aVista: boolean;
  marcar: (chave: string, sim: boolean, aVista: boolean) => void;
  children: ReactNode;
}) {
  const avisar = useCallback((sim: boolean) => marcar(chave, sim, aVista), [chave, aVista, marcar]);
  return <AvisarResposta.Provider value={avisar}>{children}</AvisarResposta.Provider>;
}

/** Para a conversa avisar a sua aba. Fora de uma aba (o `Chat` nos testes), não faz nada. */
export const useAvisarResposta = () => use(AvisarResposta);
