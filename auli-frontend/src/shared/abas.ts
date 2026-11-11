import { useEffect, useState } from "react";
import { getEntity } from "./entities";

/**
 * Abas de estado (D-ABAS-5..7, docs/ESTUDO-abas-por-estado.md): vários estados abertos ao mesmo
 * tempo, cada um com a sua conversa, a sua seção e a sua rolagem.
 *
 * As operações são funções puras sobre `Abas` — é nelas que moram os casos de borda (fechar a
 * ativa, fechar a última, escolher um estado já aberto) — e o `useAbas` só as liga ao estado React
 * e ao `localStorage`.
 */

/** Uma aba. `entityId: null` = aba nova, ainda no mapa. A `chave` vale só nesta sessão: o que
 *  persiste são os ids das entidades. */
export interface Aba {
  chave: string;
  entityId: string | null;
}

/** Sempre com ao menos uma aba, e `ativa` sempre é a chave de uma delas. */
export interface Abas {
  lista: Aba[];
  ativa: string;
}

const STORAGE_KEY = "auli.abas";
/** A chave de antes das abas, quando só havia um estado. Lida uma vez, para quem já escolhera. */
const LEGADO_KEY = "auli.entity";

const novaAba = (entityId: string | null = null): Aba => ({ chave: crypto.randomUUID(), entityId });

/** Só uma aba, no mapa: a partida sem nada salvo e o que sobra ao fechar a última. */
const soOMapa = (): Abas => {
  const aba = novaAba();
  return { lista: [aba], ativa: aba.chave };
};

/** Ids do par aba ↔ painel, que o `AbasDeEstado` e o `App` precisam concordar. */
export const idDaAba = (chave: string) => `aba-${chave}`;
export const idDoPainel = (chave: string) => `painel-${chave}`;

/** "+": abre uma aba no mapa — ou volta à que já está lá, em vez de empilhar abas vazias. */
export function abrirNova(s: Abas): Abas {
  const vazia = s.lista.find((a) => a.entityId === null);
  if (vazia) return { ...s, ativa: vazia.chave };
  const aba = novaAba();
  return { lista: [...s.lista, aba], ativa: aba.chave };
}

/** O estado escolhido no mapa da aba `chave`. Se ele já está aberto em outra aba, ativa aquela e
 *  descarta esta: um estado, uma aba. */
export function escolher(s: Abas, chave: string, entityId: string): Abas {
  const aberta = s.lista.find((a) => a.entityId === entityId && a.chave !== chave);
  if (aberta) return { lista: s.lista.filter((a) => a.chave !== chave), ativa: aberta.chave };
  return { ...s, lista: s.lista.map((a) => (a.chave === chave ? { ...a, entityId } : a)) };
}

/** Fecha `chave`. Fechando a ativa, ativa a vizinha da direita (a da esquerda, se era a última),
 *  como o navegador. Fechando a única, sobra uma aba no mapa. */
export function fechar(s: Abas, chave: string): Abas {
  const i = s.lista.findIndex((a) => a.chave === chave);
  if (i < 0) return s;
  const lista = s.lista.filter((a) => a.chave !== chave);
  if (lista.length === 0) return soOMapa();
  if (s.ativa !== chave) return { ...s, lista };
  return { lista, ativa: lista[Math.min(i, lista.length - 1)].chave };
}

/**
 * As abas salvas. Só as que têm estado persistem (a aba no mapa é passageira); ids que o registry
 * não conhece mais e repetidos caem. Sem nada salvo, migra o `auli.entity`; sem ele, só o mapa.
 * A ativa volta se ainda existir; senão, a primeira.
 */
export function lerAbas(): Abas {
  let ids: unknown[] = [];
  let ativaId: unknown = null;
  try {
    const bruto = localStorage.getItem(STORAGE_KEY);
    if (bruto === null) {
      ids = [localStorage.getItem(LEGADO_KEY)];
    } else {
      const salvo = JSON.parse(bruto);
      ids = Array.isArray(salvo?.abas) ? salvo.abas : [];
      ativaId = salvo?.ativa;
    }
  } catch {
    // Sem localStorage ou JSON quebrado: começa do mapa.
  }
  const validos = [...new Set(ids)].filter(
    (id): id is string => typeof id === "string" && getEntity(id) !== undefined,
  );
  if (validos.length === 0) return soOMapa();
  const lista = validos.map((id) => novaAba(id));
  return { lista, ativa: (lista.find((a) => a.entityId === ativaId) ?? lista[0]).chave };
}

function salvarAbas(s: Abas) {
  const ativa = s.lista.find((a) => a.chave === s.ativa)?.entityId ?? null;
  const abas = s.lista.flatMap((a) => (a.entityId ? [a.entityId] : []));
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ abas, ativa }));
  } catch {
    // Sem persistência, as abas seguem valendo nesta sessão.
  }
}

/** As abas de estado do app, persistidas em `auli.abas`. */
export function useAbas() {
  const [abas, setAbas] = useState(lerAbas);

  useEffect(() => salvarAbas(abas), [abas]);

  return {
    abas,
    abrirNova: () => setAbas(abrirNova),
    ativar: (chave: string) => setAbas((s) => ({ ...s, ativa: chave })),
    fechar: (chave: string) => setAbas((s) => fechar(s, chave)),
    escolher: (chave: string, entityId: string) => setAbas((s) => escolher(s, chave, entityId)),
  };
}
