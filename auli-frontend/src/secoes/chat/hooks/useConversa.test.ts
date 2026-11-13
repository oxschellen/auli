// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import axios from "axios";
import { useConversa } from "./useConversa";
import { API_URL } from "../../../api/chat";

vi.mock("axios", () => ({ default: { post: vi.fn(), isCancel: vi.fn(() => false) } }));

const post = vi.mocked(axios.post);
const isCancel = vi.mocked(axios.isCancel);

const PERGUNTA = "Uma pergunta longa o suficiente";

/** Envia e espera a resposta chegar (a espera sair da conversa). */
async function enviarEEsperar(tipo: "1" | "2" = "1") {
  const hook = renderHook(() => useConversa("rs"));
  act(() => {
    hook.result.current.enviar(PERGUNTA, tipo);
  });
  await waitFor(() => expect(hook.result.current.esperando).toBe(false));
  return hook;
}

beforeEach(() => {
  vi.clearAllMocks();
  isCancel.mockReturnValue(false);
});

describe("useConversa", () => {
  it("começa vazia e sem espera", () => {
    const { result } = renderHook(() => useConversa("rs"));
    expect(result.current.mensagens).toEqual([]);
    expect(result.current.esperando).toBe(false);
  });

  it("no envio, mostra a pergunta e a espera (pendente), e depois troca a espera pela resposta", async () => {
    let responder!: (v: unknown) => void;
    post.mockImplementation(() => new Promise((r) => (responder = r)));
    const { result } = renderHook(() => useConversa("rs"));

    act(() => {
      result.current.enviar(PERGUNTA, "1");
    });
    expect(result.current.esperando).toBe(true);
    const [usuario, espera] = result.current.mensagens;
    expect(usuario).toMatchObject({ from: "user", text: PERGUNTA });
    expect(espera).toMatchObject({ from: "server", pendente: true, showButton: false });
    expect(espera.text).toMatch(/pensando/i);

    await act(async () => responder({ data: { answer: "Resposta do servidor", log_id: "abc" } }));
    expect(result.current.esperando).toBe(false);
    expect(result.current.mensagens).toHaveLength(2);
    expect(result.current.mensagens[1]).toMatchObject({
      from: "server",
      text: "Resposta do servidor",
      showButton: true,
      logId: "abc",
      tipo: "1",
    });
    expect(result.current.mensagens[1].pendente).toBeUndefined();

    // Com a resposta na tela, a trava se solta: a próxima pergunta é aceita.
    let aceita = false;
    act(() => {
      aceita = result.current.enviar(PERGUNTA, "1");
    });
    expect(aceita).toBe(true);
  });

  it("dois envios seguidos, antes da resposta: só o primeiro vale", () => {
    post.mockImplementation(() => new Promise(() => {}));
    const { result } = renderHook(() => useConversa("rs"));
    let aceitos: boolean[] = [];
    act(() => {
      aceitos = [result.current.enviar(PERGUNTA, "1"), result.current.enviar(PERGUNTA, "1")];
    });
    expect(aceitos).toEqual([true, false]);
    expect(post).toHaveBeenCalledTimes(1);
    expect(result.current.mensagens).toHaveLength(2);
  });

  it("manda pergunta, entidade e tipo, e leva as fontes para a mensagem (D-SF-10)", async () => {
    const fontes = [{ rotulo: "Serviço 1", titulo: "Emitir guia", link: "https://s/1" }];
    post.mockResolvedValue({ data: { answer: "ok", fontes } });
    const { result } = await enviarEEsperar("2");
    expect(post).toHaveBeenCalledWith(
      API_URL,
      { question: PERGUNTA, entity: "rs", type: 2 },
      expect.anything(),
    );
    expect(result.current.mensagens.at(-1)?.fontes).toEqual(fontes);
  });

  it("resposta vazia vira a mensagem de que não houve resposta", async () => {
    post.mockResolvedValue({ data: {} });
    const { result } = await enviarEEsperar();
    expect(result.current.mensagens.at(-1)?.text).toMatch(/obter resposta/i);
  });

  it("tempo esgotado, servidor fora e limite: cada falha com a sua mensagem", async () => {
    post.mockRejectedValue(new Error("aborted"));
    isCancel.mockReturnValue(true);
    expect((await enviarEEsperar()).result.current.mensagens.at(-1)?.text).toMatch(/tempo limite/i);

    isCancel.mockReturnValue(false);
    post.mockRejectedValue(new Error("network"));
    expect((await enviarEEsperar()).result.current.mensagens.at(-1)?.text).toMatch(
      /não está disponível/i,
    );

    post.mockRejectedValue({ response: { status: 429, data: { error: "Calma, aguarde." } } });
    expect((await enviarEEsperar()).result.current.mensagens.at(-1)?.text).toBe("Calma, aguarde.");

    post.mockRejectedValue({ response: { status: 429 } });
    expect((await enviarEEsperar()).result.current.mensagens.at(-1)?.text).toMatch(
      /muitas requisições/i,
    );
  });
});
