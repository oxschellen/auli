// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { CHAVES, gravarTexto, lerTexto, useTextoGravado } from "./storage";

// O módulo guarda em memória o que não coube no localStorage, e essa memória vive enquanto o
// módulo vive (o arquivo de teste inteiro). Cada caso usa a sua chave para não herdar de outro.

afterEach(() => vi.restoreAllMocks());

describe("storage", () => {
  it("as chaves são as que já estão gravadas nos navegadores", () => {
    expect(CHAVES).toMatchObject({
      abas: "auli.abas",
      entidadeLegada: "auli.entity",
      sidebar: "auli.sidebar",
      painelHistorico: "auli.historicoPainel",
      tipoDeConsulta: "auli.questionType",
    });
    expect(CHAVES.historico("rs")).toBe("auli.historico.rs");
  });

  it("grava no localStorage e lê de volta", () => {
    gravarTexto("teste.ida-e-volta", "valor");
    expect(localStorage.getItem("teste.ida-e-volta")).toBe("valor");
    expect(lerTexto("teste.ida-e-volta")).toBe("valor");
    expect(lerTexto("teste.nunca-gravada")).toBeNull();
  });

  it("o hook acompanha o que se grava nesta aba", () => {
    const { result } = renderHook(() => useTextoGravado("teste.mesma-aba"));
    expect(result.current).toBeNull();
    act(() => gravarTexto("teste.mesma-aba", "novo"));
    expect(result.current).toBe("novo");
  });

  it("o hook acompanha o que outra janela do navegador grava (evento `storage`)", () => {
    const { result } = renderHook(() => useTextoGravado("teste.outra-janela"));
    act(() => {
      localStorage.setItem("teste.outra-janela", "da outra");
      window.dispatchEvent(new StorageEvent("storage", { key: "teste.outra-janela" }));
    });
    expect(result.current).toBe("da outra");
  });

  it("sem localStorage, o valor vale nesta sessão; quando ele volta, a gravação vai para ele", () => {
    const getItem = vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("bloqueado");
    });
    const setItem = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("bloqueado");
    });
    expect(lerTexto("teste.bloqueado")).toBeNull();
    gravarTexto("teste.bloqueado", "só na memória");
    expect(lerTexto("teste.bloqueado")).toBe("só na memória");

    getItem.mockRestore();
    setItem.mockRestore();
    gravarTexto("teste.bloqueado", "agora persistido");
    expect(localStorage.getItem("teste.bloqueado")).toBe("agora persistido");
    expect(lerTexto("teste.bloqueado")).toBe("agora persistido");
  });
});
