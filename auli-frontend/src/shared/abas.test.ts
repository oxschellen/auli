// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { abrirNova, escolher, fechar, lerAbas } from "./abas";
import type { Abas } from "./abas";

/** Abas a partir dos ids das entidades (`null` = no mapa); a chave é o id, ou "mapa". */
const abas = (ids: (string | null)[], ativa: string): Abas => ({
  lista: ids.map((id) => ({ chave: id ?? "mapa", entityId: id })),
  ativa,
});
const ids = (s: Abas) => s.lista.map((a) => a.entityId);
const ativa = (s: Abas) => s.lista.find((a) => a.chave === s.ativa)?.entityId;

describe("abrirNova (+)", () => {
  it("abre uma aba no mapa, no fim, e a ativa", () => {
    const s = abrirNova(abas(["rs"], "rs"));
    expect(ids(s)).toEqual(["rs", null]);
    expect(ativa(s)).toBeNull();
  });

  it("com uma aba no mapa já aberta, volta a ela em vez de empilhar outra", () => {
    const s = abrirNova(abas([null, "rs"], "rs"));
    expect(ids(s)).toEqual([null, "rs"]);
    expect(s.ativa).toBe("mapa");
  });
});

describe("escolher (no mapa da aba)", () => {
  it("dá o estado à aba, no mesmo lugar", () => {
    const s = escolher(abas(["rs", null], "mapa"), "mapa", "sp");
    expect(ids(s)).toEqual(["rs", "sp"]);
    expect(s.ativa).toBe("mapa");
  });

  it("um estado já aberto não abre de novo: ativa a aba dele e descarta a do mapa", () => {
    const s = escolher(abas(["rs", "sp", null], "mapa"), "mapa", "rs");
    expect(ids(s)).toEqual(["rs", "sp"]);
    expect(ativa(s)).toBe("rs");
  });
});

describe("fechar", () => {
  it("fechar uma inativa não muda a ativa", () => {
    const s = fechar(abas(["rs", "sp", "mg"], "sp"), "mg");
    expect(ids(s)).toEqual(["rs", "sp"]);
    expect(ativa(s)).toBe("sp");
  });

  it("fechar a ativa ativa a vizinha da direita", () => {
    expect(ativa(fechar(abas(["rs", "sp", "mg"], "sp"), "sp"))).toBe("mg");
  });

  it("fechar a ativa que é a última ativa a da esquerda", () => {
    expect(ativa(fechar(abas(["rs", "sp", "mg"], "mg"), "mg"))).toBe("sp");
  });

  it("fechar a única deixa uma aba no mapa", () => {
    const s = fechar(abas(["rs"], "rs"), "rs");
    expect(ids(s)).toEqual([null]);
    expect(s.ativa).toBe(s.lista[0].chave);
  });

  it("chave que não existe não muda nada", () => {
    const antes = abas(["rs"], "rs");
    expect(fechar(antes, "xx")).toBe(antes);
  });
});

describe("lerAbas (partida)", () => {
  beforeEach(() => {
    localStorage.removeItem("auli.abas");
    localStorage.removeItem("auli.entity");
  });

  it("sem nada salvo, começa com uma aba no mapa", () => {
    const s = lerAbas();
    expect(ids(s)).toEqual([null]);
    expect(s.ativa).toBe(s.lista[0].chave);
  });

  it("migra o `auli.entity` de antes das abas", () => {
    localStorage.setItem("auli.entity", "sp");
    expect(ids(lerAbas())).toEqual(["sp"]);
  });

  it("com `auli.abas` salvo, o `auli.entity` antigo não conta", () => {
    localStorage.setItem("auli.entity", "sp");
    localStorage.setItem("auli.abas", JSON.stringify({ abas: ["rs"], ativa: "rs" }));
    expect(ids(lerAbas())).toEqual(["rs"]);
  });

  it("devolve as abas na ordem e com a ativa; ids desconhecidos e repetidos caem", () => {
    localStorage.setItem(
      "auli.abas",
      JSON.stringify({ abas: ["rs", "xx", "sp", "rs", 7, "mg"], ativa: "sp" }),
    );
    const s = lerAbas();
    expect(ids(s)).toEqual(["rs", "sp", "mg"]);
    expect(ativa(s)).toBe("sp");
  });

  it("ativa que não voltou (a aba do mapa, que não persiste) cai na primeira", () => {
    localStorage.setItem("auli.abas", JSON.stringify({ abas: ["rs", "sp"], ativa: null }));
    expect(ativa(lerAbas())).toBe("rs");
  });

  it.each(["{quebrado", "null", '"rs"', '{"abas":"rs"}', '{"abas":[]}'])(
    "salvo inválido ou vazio (%s) começa no mapa",
    (bruto) => {
      localStorage.setItem("auli.abas", bruto);
      expect(ids(lerAbas())).toEqual([null]);
    },
  );
});
