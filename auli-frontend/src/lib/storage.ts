import { useSyncExternalStore } from "react";

/**
 * A única porta do app para o `localStorage` (I1 do docs/ESTUDO-frontend-organizacao.md): as chaves
 * ficam juntas, o `try/catch` existe uma vez, e todo valor gravado é reativo.
 *
 * Quando o navegador bloqueia o armazenamento (ou ele está cheio), o valor vale só nesta sessão, na
 * memória — o app segue funcionando, só não lembra depois do reload. Com login, o que mora no
 * servidor deixa de passar por aqui; quem lê não precisa saber.
 */

/** Todas as chaves do app. Os valores NÃO mudam: é o que já está gravado nos navegadores. */
export const CHAVES = {
  abas: "auli.abas",
  /** A de antes das abas de estado, lida uma vez para migrar (ver `abas.ts`). */
  entidadeLegada: "auli.entity",
  sidebar: "auli.sidebar",
  painelHistorico: "auli.historicoPainel",
  tipoDeConsulta: "auli.questionType",
  /** Uma por entidade (D-HIST-1): a pergunta feita no RS não aparece em SP. */
  historico: (entityId: string) => `auli.historico.${entityId}`,
} as const;

const ouvintes = new Set<() => void>();
/** O que não coube no `localStorage` (bloqueado ou cheio): vale só nesta sessão. */
const soNaMemoria = new Map<string, string>();

export function lerTexto(chave: string): string | null {
  const local = soNaMemoria.get(chave);
  if (local !== undefined) return local;
  try {
    return localStorage.getItem(chave);
  } catch {
    return null;
  }
}

export function gravarTexto(chave: string, valor: string): void {
  try {
    localStorage.setItem(chave, valor);
    soNaMemoria.delete(chave);
  } catch {
    soNaMemoria.set(chave, valor);
  }
  ouvintes.forEach((avisar) => avisar());
}

function assinar(avisar: () => void) {
  ouvintes.add(avisar);
  // `storage` dispara nas OUTRAS abas do navegador quando uma grava: as duas mostram o mesmo.
  window.addEventListener("storage", avisar);
  return () => {
    ouvintes.delete(avisar);
    window.removeEventListener("storage", avisar);
  };
}

/** O texto gravado em `chave`, reativo. É string, então a referência é estável entre renders,
 *  como o `useSyncExternalStore` exige. */
export function useTextoGravado(chave: string): string | null {
  return useSyncExternalStore(assinar, () => lerTexto(chave));
}
