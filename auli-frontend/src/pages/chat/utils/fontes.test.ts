import { describe, it, expect } from "vitest";
import { agruparFontes } from "./fontes";
import type { Fonte } from "../../../types/chat";

const f = (rotulo: string, triagem?: Fonte["triagem"]): Fonte => ({ rotulo, titulo: rotulo, link: "", triagem });

describe("agruparFontes", () => {
  it("separa citadas, consideradas e descartadas, preservando a ordem", () => {
    const fontes = [f("Serviço 1", "aplica"), f("Serviço 2", "condicional"), f("Serviço 3", "descarta"), f("FAQ 1"), f("FAQ 2", "aplica")];
    const g = agruparFontes(fontes, new Set(["FAQ 2", "Serviço 1"]));
    expect(g.citadas.map((x) => x.rotulo)).toEqual(["Serviço 1", "FAQ 2"]);
    expect(g.consideradas.map((x) => x.rotulo)).toEqual(["Serviço 2", "FAQ 1"]);
    expect(g.descartadas.map((x) => x.rotulo)).toEqual(["Serviço 3"]);
  });

  it("citada vence descartada (não deveria acontecer, mas o texto é a verdade do que foi dito)", () => {
    const g = agruparFontes([f("Serviço 1", "descarta")], new Set(["Serviço 1"]));
    expect(g.citadas).toHaveLength(1);
    expect(g.descartadas).toHaveLength(0);
  });
});
