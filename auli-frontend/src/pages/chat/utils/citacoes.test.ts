import { describe, it, expect } from "vitest";
import { ancora, marcarCitacoes, rotuloDoHref, rotulosCitados } from "./citacoes";

describe("citações", () => {
  it("vira link para a âncora da fonte, com rótulo canônico", () => {
    expect(marcarCitacoes("Use a GA [Serviço 1] e veja [FAQ 4].")).toBe(
      "Use a GA [Serviço 1](#fonte-servico-1) e veja [FAQ 4](#fonte-faq-4).",
    );
    expect(marcarCitacoes("[Servico 2]")).toBe("[Serviço 2](#fonte-servico-2)");
  });

  it("não mexe em link que o modelo já escreveu nem em outros colchetes", () => {
    const md = "[Serviço 1](https://portal/x) e [nota]";
    expect(marcarCitacoes(md)).toBe(md);
  });

  it("âncora e href fazem o caminho de ida e volta", () => {
    expect(ancora("Serviço 12")).toBe("fonte-servico-12");
    expect(rotuloDoHref("#fonte-servico-12")).toBe("Serviço 12");
    expect(rotuloDoHref("#fonte-faq-3")).toBe("FAQ 3");
    expect(rotuloDoHref("https://portal")).toBeNull();
    expect(rotuloDoHref(undefined)).toBeNull();
  });

  it("lista os rótulos citados, sem repetição", () => {
    expect([...rotulosCitados("[Serviço 1] [FAQ 2] [Serviço 1]")]).toEqual(["Serviço 1", "FAQ 2"]);
  });
});
