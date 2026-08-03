// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithProvider } from "../../test/render";
import { EntityProvider } from "../../shared/EntityContext";
import { Chat } from "./Chat";
import { SIDEBAR_WIDTH } from "../../shared/layout";

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

/**
 * Regressão que chegou a produção na v0.1.59: a barra de composição é `position: fixed` — o que é
 * o que permite subi-la acima do teclado virtual — e `fixed` se ancora na VIEWPORT, não na área de
 * conteúdo. Com `left: 0` ela cobria a sidebar inteira.
 *
 * O teste olha o CSS que o Chakra emite porque é onde a correção vive: não há atributo nem estado
 * de React para inspecionar, só a regra e a media query.
 */
describe("barra de composição não invade a sidebar", () => {
  const regrasDaBarra = () => {
    const caixa = screen.getByRole("group", { name: "Caixa de mensagem" });
    const classes = caixa.parentElement!.className.split(/\s+/);
    const css = [...document.querySelectorAll("style")].map((s) => s.textContent).join("\n");
    return css.split("\n").filter((l) => classes.some((c) => c && l.includes(c)));
  };

  it("ocupa a largura toda abaixo de md, onde a sidebar é drawer", () => {
    montar();
    const base = regrasDaBarra().find((r) => !r.startsWith("@media"));
    expect(base).toMatch(/position:fixed/);
    expect(base).toMatch(/left:0/);
    expect(base).toMatch(/right:0/); // `right`, não `width:100%` — senão transbordaria à direita
  });

  it("recua a largura da sidebar a partir de md", () => {
    montar();
    const media = regrasDaBarra().find((r) => r.startsWith("@media"));
    expect(media).toBeDefined();
    expect(media).toContain(`left:${SIDEBAR_WIDTH}`);
  });
});
