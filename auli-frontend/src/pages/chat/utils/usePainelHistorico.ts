import { useSyncExternalStore } from "react";

const PAINEL_KEY = "auli.historicoPainel";

/** Sem localStorage (bloqueado pelo navegador), a escolha vale só nesta sessão, aqui. */
let emMemoria = true;
const ouvintes = new Set<() => void>();

/** O painel começa como foi deixado; sem preferência salva (ou sem localStorage), aberto — a mesma
 *  regra da barra lateral esquerda (`App.tsx`). */
function lerAberto(): boolean {
  try {
    return localStorage.getItem(PAINEL_KEY) !== "recolhido";
  } catch {
    return emMemoria;
  }
}

function gravarAberto(aberto: boolean) {
  emMemoria = aberto;
  try {
    localStorage.setItem(PAINEL_KEY, aberto ? "aberto" : "recolhido");
  } catch {
    // Sem persistência, o botão segue valendo nesta sessão.
  }
  ouvintes.forEach((avisar) => avisar());
}

function assinar(avisar: () => void) {
  ouvintes.add(avisar);
  return () => {
    ouvintes.delete(avisar);
  };
}

/**
 * Se o painel "Minhas perguntas" está aberto (a partir de `xl`; abaixo dele é drawer, que não deixa
 * rastro). Uma escolha só para todas as abas de estado, como a da barra lateral esquerda: cada aba
 * tem o seu `Chat`, e com um `useState` em cada um, fechar o painel no RS o deixaria aberto no SP.
 * Daí um valor só, guardado em `auli.historicoPainel`, que todos os `Chat` leem.
 */
export function usePainelHistorico() {
  const aberto = useSyncExternalStore(assinar, lerAberto);
  return [aberto, gravarAberto] as const;
}
