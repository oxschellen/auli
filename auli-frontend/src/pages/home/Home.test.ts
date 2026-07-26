import { describe, expect, it } from "vitest";
import { proximaHabilitada } from "./Home";

/** Barra de abas em miniatura: `-` habilitada, `x` inativa. */
const barra = (spec: string) =>
  spec.split("").map((c, i) => ({ id: `t${i}`, disabled: c === "x" }));

describe("proximaHabilitada", () => {
  it("anda para a vizinha quando ela está habilitada", () => {
    const itens = barra("---");
    expect(proximaHabilitada(itens, "t0", 1)?.id).toBe("t1");
    expect(proximaHabilitada(itens, "t2", -1)?.id).toBe("t1");
  });

  it("salta uma corrida de abas inativas", () => {
    // O caso real: entidade com só `servicos` desativa faqs/pareceres/notas/conteudos, então
    // ArrowRight de "servicos" tem de pular quatro e cair em "about".
    const itens = barra("--xxxx-");
    expect(proximaHabilitada(itens, "t1", 1)?.id).toBe("t6");
    expect(proximaHabilitada(itens, "t6", -1)?.id).toBe("t1");
  });

  it("dá a volta nas duas pontas", () => {
    const itens = barra("---");
    expect(proximaHabilitada(itens, "t2", 1)?.id).toBe("t0"); // direita → volta ao início
    expect(proximaHabilitada(itens, "t0", -1)?.id).toBe("t2"); // esquerda → vai ao fim
  });

  it("dá a volta SALTANDO inativas — o caso do delta negativo com módulo", () => {
    // `t0` habilitada, `t1`/`t2` inativas: ArrowLeft de t0 envolve para t2, que é inativa, e tem de
    // continuar até achar t0 de novo. É aqui que `% n` negativo do JS quebraria.
    const itens = barra("-xx");
    expect(proximaHabilitada(itens, "t0", -1)?.id).toBe("t0");
    expect(proximaHabilitada(itens, "t0", 1)?.id).toBe("t0");
  });

  it("devolve undefined se nada está habilitado, em vez de laçar", () => {
    expect(proximaHabilitada(barra("xxx"), "t0", 1)).toBeUndefined();
    expect(proximaHabilitada(barra("xxx"), "t0", -1)).toBeUndefined();
  });

  it("devolve undefined se o id de partida não existe", () => {
    expect(proximaHabilitada(barra("---"), "inexistente", 1)).toBeUndefined();
  });

  it("nunca devolve uma aba inativa, varrendo todas as posições de partida", () => {
    const itens = barra("-x-xx-x");
    for (const partida of itens) {
      for (const delta of [1, -1] as const) {
        const r = proximaHabilitada(itens, partida.id, delta);
        expect(r?.disabled).toBe(false);
      }
    }
  });
});
