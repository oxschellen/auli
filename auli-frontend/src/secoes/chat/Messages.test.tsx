// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithProvider } from "../../test/render";
import { Messages } from "./Messages";
import type { Message } from "./tipos";

const messages: Message[] = [
  { id: "1", natureza: "pergunta", text: "Qual a alíquota do ICMS?" },
  { id: "2", natureza: "resposta", text: "Depende do produto." },
  { id: "3", natureza: "erro", text: "Desculpe! A requisição excedeu o tempo limite." },
];

describe("Messages", () => {
  it("renders the question, the answer and the error text", () => {
    renderWithProvider(<Messages messages={messages} setPrompt={vi.fn()} />);
    expect(screen.getByText("Qual a alíquota do ICMS?")).toBeInTheDocument();
    expect(screen.getByText("Depende do produto.")).toBeInTheDocument();
    expect(screen.getByText("Desculpe! A requisição excedeu o tempo limite.")).toBeInTheDocument();
  });

  it("a resposta é o cartão com ações; o erro, um balão sem nenhuma (C4)", () => {
    renderWithProvider(<Messages messages={messages} setPrompt={vi.fn()} />);
    const resposta = screen.getByText("Depende do produto.").closest("article");
    expect(resposta).not.toBeNull();
    // Um "Copiar resposta" só: o da resposta. O erro não tem o que copiar.
    expect(screen.getAllByLabelText("Copiar resposta para a área de transferência")).toHaveLength(
      1,
    );
    expect(
      screen.getByText("Desculpe! A requisição excedeu o tempo limite.").closest("article"),
    ).toBeNull();
  });

  it("renders nothing for an empty transcript", () => {
    const { container } = renderWithProvider(<Messages messages={[]} setPrompt={vi.fn()} />);
    // The Flex wrapper renders but has no message children.
    expect(container.querySelector("p")).toBeNull();
  });
});
