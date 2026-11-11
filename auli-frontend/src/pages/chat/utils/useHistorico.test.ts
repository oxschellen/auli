// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { useHistorico } from "./useHistorico";

beforeEach(() => localStorage.clear());
afterEach(() => vi.restoreAllMocks());

describe("useHistorico", () => {
  it("grava numa chave por entidade, e uma não enxerga a outra", () => {
    const rs = renderHook(() => useHistorico("rs"));
    const sp = renderHook(() => useHistorico("sp"));
    act(() => rs.result.current.registrar("Como pagar o IPVA no estado?", "1"));
    expect(rs.result.current.itens.map((i) => i.pergunta)).toEqual(["Como pagar o IPVA no estado?"]);
    expect(sp.result.current.itens).toEqual([]);
    expect(localStorage.getItem("auli.historico.rs")).toContain("Como pagar o IPVA no estado?");
    expect(localStorage.getItem("auli.historico.sp")).toBeNull();
  });

  it("a lista sobrevive a remontar (é o que acontece ao recarregar a página)", () => {
    const primeiro = renderHook(() => useHistorico("rs"));
    act(() => primeiro.result.current.registrar("Como pagar o IPVA no estado?", "1"));
    primeiro.unmount();
    const segundo = renderHook(() => useHistorico("rs"));
    expect(segundo.result.current.itens).toHaveLength(1);
  });

  it("o que outra aba gravou aparece aqui e não é apagado pela próxima gravação", () => {
    const { result } = renderHook(() => useHistorico("rs"));
    act(() => result.current.registrar("Pergunta feita nesta aba", "1"));
    // A outra aba releu, acrescentou a dela e gravou; o navegador avisa esta com `storage`.
    const daOutra = JSON.parse(localStorage.getItem("auli.historico.rs")!);
    daOutra.itens.unshift({ id: "x", pergunta: "Pergunta feita na outra aba", tipo: "2", em: 9, favorita: false });
    act(() => {
      localStorage.setItem("auli.historico.rs", JSON.stringify(daOutra));
      window.dispatchEvent(new StorageEvent("storage", { key: "auli.historico.rs" }));
    });
    expect(result.current.itens).toHaveLength(2);
    act(() => result.current.alternarFavorita("x"));
    expect(result.current.itens.map((i) => i.pergunta)).toEqual([
      "Pergunta feita na outra aba",
      "Pergunta feita nesta aba",
    ]);
  });

  it("grava relendo o que está gravado, mesmo antes de o aviso da outra aba chegar", () => {
    const { result } = renderHook(() => useHistorico("rs"));
    act(() => result.current.registrar("Pergunta feita nesta aba", "1"));
    // A outra aba gravou, e o `storage` ainda não chegou: a tela não a mostra.
    const daOutra = JSON.parse(localStorage.getItem("auli.historico.rs")!);
    daOutra.itens.unshift({ id: "x", pergunta: "Pergunta feita na outra aba", tipo: "2", em: 9, favorita: false });
    localStorage.setItem("auli.historico.rs", JSON.stringify(daOutra));
    const daqui = result.current.itens[0].id;
    act(() => result.current.alternarFavorita(daqui));
    const gravado = JSON.parse(localStorage.getItem("auli.historico.rs")!);
    expect(gravado.itens.map((i: { id: string }) => i.id)).toEqual(["x", daqui]);
  });

  it("sem localStorage, a lista vale nesta sessão", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("bloqueado");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("bloqueado");
    });
    const { result } = renderHook(() => useHistorico("go"));
    act(() => result.current.registrar("Como pagar o IPVA no estado?", "1"));
    expect(result.current.itens).toHaveLength(1);
  });
});
