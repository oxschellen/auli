import { CHAVES, gravarTexto, useTextoGravado } from "../../../lib/storage";

/** Abre ou recolhe o painel. Fora do hook para ser a mesma função em todo render. */
const definirAberto = (aberto: boolean) =>
  gravarTexto(CHAVES.painelHistorico, aberto ? "aberto" : "recolhido");

/**
 * Se o painel "Minhas perguntas" está aberto (a partir de `xl`; abaixo dele é drawer, que não deixa
 * rastro). Uma escolha só para todas as abas de estado, como a da barra lateral esquerda: cada aba
 * tem o seu `Chat`, e com um `useState` em cada um, fechar o painel no RS o deixaria aberto no SP.
 * Daí um valor só, guardado em `auli.historicoPainel`, que todos os `Chat` leem. Sem preferência
 * salva, aberto.
 */
export function usePainelHistorico() {
  const aberto = useTextoGravado(CHAVES.painelHistorico) !== "recolhido";
  return [aberto, definirAberto] as const;
}
