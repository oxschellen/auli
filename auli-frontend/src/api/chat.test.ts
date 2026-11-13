import { afterEach, describe, expect, it, vi } from "vitest";
import axios from "axios";
import { API_URL, buscarLog, perguntar } from "./chat";

vi.mock("axios", () => ({ default: { post: vi.fn(), isCancel: vi.fn(() => false) } }));

const post = vi.mocked(axios.post);
const isCancel = vi.mocked(axios.isCancel);

afterEach(() => {
  post.mockReset();
  isCancel.mockReset().mockReturnValue(false);
});

describe("perguntar", () => {
  it("manda pergunta, entidade e tipo ao endpoint, e devolve resposta, log e fontes", async () => {
    const fontes = [{ rotulo: "Serviço 1", titulo: "IPVA", link: "https://x" }];
    post.mockResolvedValue({ data: { answer: "Resposta.", log_id: "abc", fontes } });

    const r = await perguntar("Como pagar o IPVA?", "rs", 1);

    expect(post).toHaveBeenCalledWith(
      API_URL,
      { question: "Como pagar o IPVA?", entity: "rs", type: 1 },
      expect.objectContaining({ signal: expect.any(AbortSignal) }),
    );
    expect(r).toEqual({ ok: true, texto: "Resposta.", logId: "abc", fontes });
  });

  it("sem entidade, o corpo não leva `entity`", async () => {
    post.mockResolvedValue({ data: {} });
    await perguntar("Pergunta", undefined, 2);
    // `toStrictEqual`: o `toEqual` aceitaria `entity: undefined` no lugar da ausência.
    expect(post.mock.calls[0][1]).toStrictEqual({ question: "Pergunta", type: 2 });
  });

  it("tempo esgotado (requisição abortada)", async () => {
    post.mockRejectedValue(new Error("canceled"));
    isCancel.mockReturnValue(true);
    expect(await perguntar("Pergunta", "rs", 1)).toEqual({ ok: false, falha: "tempo" });
  });

  it("HTTP 429 traz a mensagem do próprio servidor", async () => {
    post.mockRejectedValue({ response: { status: 429, data: { error: "Calma, aguarde." } } });
    expect(await perguntar("Pergunta", "rs", 1)).toEqual({
      ok: false,
      falha: "limite",
      mensagem: "Calma, aguarde.",
    });
  });

  it("qualquer outra falha é o servidor indisponível", async () => {
    post.mockRejectedValue({ response: { status: 500 } });
    expect(await perguntar("Pergunta", "rs", 1)).toEqual({ ok: false, falha: "indisponivel" });
    post.mockRejectedValue(new Error("Network Error"));
    expect(await perguntar("Pergunta", "rs", 1)).toEqual({ ok: false, falha: "indisponivel" });
  });
});

describe("buscarLog", () => {
  const resposta = (corpo: string, ok: boolean) =>
    vi.fn().mockResolvedValue({ ok, text: () => Promise.resolve(corpo) });

  afterEach(() => vi.unstubAllGlobals());

  it("pede o log ao lado do endpoint da pergunta e devolve o texto", async () => {
    vi.stubGlobal("fetch", resposta("REGISTRO", true));
    expect(await buscarLog("uuid-1")).toBe("REGISTRO");
    expect(vi.mocked(fetch).mock.calls[0][0]).toBe(API_URL.replace(/\/question$/, "/log/uuid-1"));
  });

  it("sem registro, lança com a mensagem do servidor", async () => {
    vi.stubGlobal("fetch", resposta("Log expirado ou inexistente.", false));
    await expect(buscarLog("uuid-2")).rejects.toThrow("Log expirado ou inexistente.");
  });
});
