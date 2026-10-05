// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
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
    const seletor = screen.getByRole("button", { name: /^Tipo de consulta:/ });
    expect(caixa).toContainElement(seletor);
    expect(caixa).toContainElement(screen.getByLabelText("Sua pergunta"));
  });

  it("o seletor vem ANTES do campo dentro da caixa", () => {
    montar();
    const seletor = screen.getByRole("button", { name: /^Tipo de consulta:/ });
    const campo = screen.getByLabelText("Sua pergunta");
    // `DOCUMENT_POSITION_FOLLOWING` = o campo vem depois do seletor na ordem do documento, que é
    // a ordem que o teclado e o leitor de tela seguem.
    expect(seletor.compareDocumentPosition(campo) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  // A troca de tipo em si é do RadioGroup do Chakra, que o jsdom não aciona por clique; ela é
  // conferida no navegador (ver a TAREFA-CHAT-REDESENHO, Fase 2). Aqui: abrir, listar, fechar.
  it("o seletor abre a lista dos tipos (radiogroup) e fecha com Esc", () => {
    montar();
    const botao = screen.getByRole("button", { name: /^Tipo de consulta:/ });
    expect(botao).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(botao);
    expect(botao).toHaveAttribute("aria-expanded", "true");
    const grupo = screen.getByRole("radiogroup", { name: "Tipo de consulta" });
    expect(grupo).toHaveTextContent("Serviços + FAQs");
    expect(grupo).toHaveTextContent("Pareceres");
    fireEvent.keyDown(grupo, { key: "Escape" });
    expect(screen.queryByRole("radiogroup")).toBeNull();
  });

  it("antes da primeira pergunta, mostra a apresentação do tipo e os exemplos vão para a caixa", () => {
    localStorage.setItem("auli.questionType", "1");
    montar();
    expect(screen.getByRole("heading", { name: "Como posso ajudar?" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Empresa nova: como apurar e recolher o ICMS?" }));
    expect(screen.getByLabelText("Sua pergunta")).toHaveValue("Empresa nova: como apurar e recolher o ICMS?");
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

/**
 * Recolher a caixa no celular. O esconder é CSS responsivo (`base: none`, `md: block`), então o
 * teste lê a regra que o Chakra emite para a classe da caixa — o mesmo método do bloco acima.
 */
describe("recolher a caixa de mensagem (celular)", () => {
  const regrasDe = (el: HTMLElement) => {
    const classes = el.className.split(/\s+/).filter(Boolean);
    const css = [...document.querySelectorAll("style")].map((s) => s.textContent).join("\n");
    return css
      .split("}")
      .filter((r) => classes.some((c) => r.includes(`.${c}`)))
      .join("}\n");
  };
  // Pelo `aria-label`, e não pelo papel: recolhida, a caixa sai da árvore de acessibilidade (o
  // jsdom aplica o `none` da base) e o `getByRole` deixa de achá-la.
  const caixa = () => screen.getByLabelText("Caixa de mensagem");

  it("Recolher esconde a caixa só abaixo de md", () => {
    montar();
    expect(regrasDe(caixa())).not.toMatch(/display:\s*none/);
    fireEvent.click(screen.getByRole("button", { name: "Recolher a caixa de mensagem" }));
    expect(regrasDe(caixa())).toMatch(/display:\s*none/);
    expect(regrasDe(caixa())).toMatch(/display:\s*block/); // o `md: block` segue valendo
  });

  it("a barrinha traz a caixa de volta com o foco no campo", async () => {
    montar();
    fireEvent.click(screen.getByRole("button", { name: "Recolher a caixa de mensagem" }));
    fireEvent.click(screen.getByRole("button", { name: "Mostrar a caixa de mensagem" }));
    expect(regrasDe(caixa())).not.toMatch(/display:\s*none/);
    await waitFor(() =>
      expect(document.activeElement).toBe(screen.getByLabelText("Sua pergunta")),
    );
  });
});
