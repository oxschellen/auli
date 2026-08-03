// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithProvider } from "../../test/render";
import { EntityProvider } from "../../shared/EntityContext";
import { Chat } from "./Chat";

/** O Chat exige uma entidade escolhida; o provider a lê do localStorage no primeiro render. */
beforeEach(() => localStorage.setItem("auli.entity", "rs"));

const montar = () =>
  renderWithProvider(
    <EntityProvider>
      <Chat />
    </EntityProvider>,
  );

describe("Chat", () => {
  // A entrega 2 em uma linha: o seletor de tipo de consulta era IRMÃO da caixa de mensagem, na
  // barra fixa, e passou a ser filho dela. Sem isto, um refactor que o tirasse de volta para fora
  // passaria por todos os outros testes — nenhum deles olha a composição.
  it("monta o seletor de tipo de consulta DENTRO da caixa de mensagem", () => {
    montar();
    const caixa = screen.getByRole("group", { name: "Caixa de mensagem" });
    expect(caixa).toContainElement(screen.getByRole("radiogroup"));
    expect(caixa).toContainElement(screen.getByPlaceholderText("Digite sua pergunta..."));
  });

  it("o seletor vem ANTES do campo dentro da caixa", () => {
    montar();
    const grupo = screen.getByRole("radiogroup");
    const campo = screen.getByPlaceholderText("Digite sua pergunta...");
    // `DOCUMENT_POSITION_FOLLOWING` = o campo vem depois do seletor na ordem do documento, que é
    // a ordem que o teclado e o leitor de tela seguem.
    expect(grupo.compareDocumentPosition(campo) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("as duas opções de consulta continuam disponíveis", () => {
    montar();
    expect(screen.getByText("Serviços+FAQs")).toBeInTheDocument();
    expect(screen.getByText("Pareceres")).toBeInTheDocument();
  });
});
