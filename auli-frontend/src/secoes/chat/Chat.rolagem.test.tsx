// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import axios from "axios";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { renderWithProvider } from "../../test/render";
import { EntityProvider } from "../../contexto/EntityContext";
import { getEntity } from "../../contexto/entities";
import { Chat } from "./Chat";

// Aqui a consulta RESPONDE (o Chat.test a deixa pendente para olhar só o envio).
vi.mock("axios", () => ({ default: { post: vi.fn(), isCancel: vi.fn(() => false) } }));
const post = vi.mocked(axios.post);

const PERGUNTA = "Qual o prazo para pagar o ICMS da empresa?";

function perguntar() {
  renderWithProvider(
    <EntityProvider entity={getEntity("rs")!}>
      <Chat />
    </EntityProvider>,
  );
  fireEvent.change(screen.getByLabelText("Sua pergunta"), { target: { value: PERGUNTA } });
  fireEvent.click(screen.getByLabelText("Enviar pesquisa"));
}

/** A última rolagem pedida: em que elemento e com que opções. */
function ultimaRolagem(rolar: ReturnType<typeof vi.spyOn>) {
  const n = rolar.mock.calls.length - 1;
  return { el: rolar.mock.contexts[n] as Element, opcoes: rolar.mock.calls[n][0] };
}

afterEach(() => vi.restoreAllMocks());

describe("rolagem quando a mensagem chega (natureza)", () => {
  it("a resposta rola até o COMEÇO dela (D-UI-2)", async () => {
    post.mockResolvedValue({ data: { answer: "O prazo é o dia 20." } });
    const rolar = vi.spyOn(Element.prototype, "scrollIntoView");
    perguntar();
    await screen.findByText("O prazo é o dia 20.");
    await waitFor(() => expect(ultimaRolagem(rolar).opcoes).toMatchObject({ block: "start" }));
    expect(ultimaRolagem(rolar).el.id).toMatch(/^msg-/);
  });

  it("o erro é balão sem ações e rola até o fim, como a pergunta", async () => {
    let falhar!: (e: unknown) => void;
    post.mockImplementation(() => new Promise((_, rej) => (falhar = rej)));
    const rolar = vi.spyOn(Element.prototype, "scrollIntoView");
    perguntar();
    await screen.findByRole("status"); // a espera (Pensando) já está na tela
    // Só conta a rolagem que o ERRO pede: a do envio já aconteceu.
    await waitFor(() => expect(rolar).toHaveBeenCalled());
    rolar.mockClear();
    falhar(new Error("network"));
    await screen.findByText(/não está disponível/);
    expect(screen.queryByLabelText("Copiar resposta para a área de transferência")).toBeNull();
    await waitFor(() => expect(rolar).toHaveBeenCalled());
    const { el, opcoes } = ultimaRolagem(rolar);
    expect(opcoes).not.toHaveProperty("block");
    expect(el.id).toBe("");
  });
});
