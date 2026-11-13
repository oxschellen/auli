import { describe, expect, it } from "vitest";
import type { Entity } from "../contexto/entities";
import { GRUPOS, SECOES, secoesDe } from "./registro";

/** Uma entidade fictícia, para o teste não depender do que o registry declara hoje. */
const so = (...collections: Entity["collections"]): Entity => ({
  id: "xx",
  name: "SEFAZ-XX",
  uf: "XX",
  state: "Estado XX",
  collections,
});

describe("registro das seções", () => {
  it("os ids são únicos (ancoram tab- e tabpanel-) e todo grupo usado tem título na sidebar", () => {
    const ids = SECOES.map((s) => s.id);
    expect(new Set(ids).size).toBe(ids.length);
    const grupos = new Set(GRUPOS.map((g) => g.id));
    expect(SECOES.every((s) => grupos.has(s.grupo))).toBe(true);
  });

  it("todas as seções aparecem; as de coleção que a entidade não tem ficam desativadas", () => {
    const secoes = secoesDe(so("servicos"));
    expect(secoes.map((s) => s.id)).toEqual(SECOES.map((s) => s.id));
    const desativadas = secoes.filter((s) => s.disabled).map((s) => s.id);
    expect(desativadas).toEqual(["faqs", "pareceres", "tarf", "legislacao", "notas", "conteudos"]);
  });

  it("as seções sem coleção valem para qualquer entidade, mesmo sem coleção nenhuma", () => {
    const sempre = secoesDe(so())
      .filter((s) => !s.disabled)
      .map((s) => s.id);
    expect(sempre).toEqual(SECOES.filter((s) => s.collection === null).map((s) => s.id));
    expect(sempre).toContain("chat");
    expect(sempre).toContain("about");
  });
});
