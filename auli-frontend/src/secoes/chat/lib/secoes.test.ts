import { describe, it, expect } from "vitest";
import { dividirSecoes } from "./secoes";

describe("dividirSecoes", () => {
  it("separa rótulo em negrito sozinho na linha", () => {
    const s = dividirSecoes("**Resumo**\nUse a GA [Serviço 1].\n\n**Detalhes**\n- item");
    expect(s).toEqual({ antes: "", resumo: "Use a GA [Serviço 1].", detalhes: "- item" });
  });

  it("aceita rótulo com dois-pontos e texto na mesma linha", () => {
    expect(dividirSecoes("**Resumo:** Use a GA.").resumo).toBe("Use a GA.");
    expect(dividirSecoes("**Resumo**: Use a GA.").resumo).toBe("Use a GA.");
    expect(dividirSecoes("Resumo: Use a GA.").resumo).toBe("Use a GA.");
  });

  it("aceita título markdown", () => {
    const s = dividirSecoes("### Resumo\nTexto.\n## Detalhes\nMais.");
    expect(s.resumo).toBe("Texto.");
    expect(s.detalhes).toBe("Mais.");
  });

  it("prosa que começa com a palavra NÃO é rótulo", () => {
    const s = dividirSecoes("Resumo do caso é este aqui.");
    expect(s).toEqual({ antes: "Resumo do caso é este aqui.", resumo: null, detalhes: null });
  });

  it("sem seções devolve tudo em `antes`", () => {
    expect(dividirSecoes("Só texto.\n\n- lista")).toEqual({
      antes: "Só texto.\n\n- lista",
      resumo: null,
      detalhes: null,
    });
  });

  it("não abre a mesma seção duas vezes, nem dentro de bloco de código", () => {
    const s = dividirSecoes("**Resumo**\nA.\n```\n**Detalhes**\n```\n**Resumo**\nB.");
    expect(s.detalhes).toBeNull();
    expect(s.resumo).toContain("**Resumo**\nB.");
  });
});
