// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import axios from "axios";
import App from "./App";
import { getEntity } from "./shared/entities";

// Nada responde: os testes olham as abas, não os dados das seções nem o chat (o do spinner
// responde a pergunta quando quer, com `mockImplementationOnce`).
vi.mock("axios", () => ({
  default: {
    get: vi.fn(() => new Promise(() => {})),
    post: vi.fn(() => new Promise(() => {})),
    isCancel: vi.fn(() => false),
  },
}));

const faixa = () => within(screen.getByRole("tablist", { name: "Estados abertos" }));
const nomes = () => faixa().getAllByRole("tab").map((t) => t.getAttribute("aria-label"));
const aba = (nome: string) => faixa().getByRole("tab", { name: nome });
const ativa = () => faixa().getByRole("tab", { selected: true }).getAttribute("aria-label");
/** O painel VISÍVEL da aba (os escondidos não entram no `getByRole`). */
const painel = (nome: string) => screen.getByRole("tabpanel", { name: nome });
const caixa = (nome: string) => within(painel(nome)).getByRole("textbox", { name: "Sua pergunta" });
const fecharPeloX = (nome: string) =>
  fireEvent.click(aba(nome).querySelector('[title="Fechar (Delete)"]')!);

const escolherNoMapa = (entityId: string) => {
  const e = getEntity(entityId)!;
  fireEvent.click(screen.getByRole("button", { name: `Selecionar ${e.name} (${e.state})` }));
};
const abrir = (entityId: string) => {
  fireEvent.click(screen.getByRole("button", { name: "Abrir outro estado" }));
  escolherNoMapa(entityId);
};

beforeEach(() => {
  localStorage.removeItem("auli.abas");
  localStorage.removeItem("auli.entity");
  localStorage.removeItem("auli.historicoPainel");
});

describe("abas de estado", () => {
  it("sem nada salvo, começa numa aba no mapa, sem ×; escolher o estado transforma a aba", () => {
    render(<App />);
    expect(nomes()).toEqual(["Novo estado"]);
    expect(aba("Novo estado").querySelector('[title="Fechar (Delete)"]')).toBeNull();

    escolherNoMapa("rs");
    expect(nomes()).toEqual(["SEFAZ-RS"]);
    expect(painel("SEFAZ-RS")).toBeInTheDocument();
  });

  it("cada aba guarda a sua conversa e a sua seção", () => {
    localStorage.setItem("auli.entity", "rs");
    render(<App />);
    fireEvent.change(caixa("SEFAZ-RS"), { target: { value: "Pergunta do RS" } });
    fireEvent.click(document.getElementById("rs-tab-faqs")!);

    abrir("sp");
    expect(nomes()).toEqual(["SEFAZ-RS", "SEFAZ-SP"]);
    expect(ativa()).toBe("SEFAZ-SP");
    expect(caixa("SEFAZ-SP")).toHaveValue("");
    expect(screen.queryByRole("tabpanel", { name: "SEFAZ-RS" })).toBeNull(); // escondido

    fireEvent.click(aba("SEFAZ-RS"));
    expect(document.getElementById("rs-tab-faqs")).toHaveAttribute("aria-selected", "true");
    fireEvent.click(document.getElementById("rs-tab-chat")!);
    expect(caixa("SEFAZ-RS")).toHaveValue("Pergunta do RS");
  });

  it("com duas abas abertas, nenhum id se repete", () => {
    localStorage.setItem("auli.entity", "rs");
    render(<App />);
    abrir("sp");
    const todos = [...document.querySelectorAll("[id]")].map((e) => e.id);
    expect(new Set(todos).size).toBe(todos.length);
    expect(document.getElementById("rs-tab-chat")).not.toBeNull();
    expect(document.getElementById("sp-tab-chat")).not.toBeNull();
  });

  it("um estado já aberto não abre de novo: ativa a aba dele", () => {
    localStorage.setItem("auli.entity", "rs");
    render(<App />);
    abrir("sp");
    abrir("rs");
    expect(nomes()).toEqual(["SEFAZ-RS", "SEFAZ-SP"]);
    expect(ativa()).toBe("SEFAZ-RS");
  });

  it("× fecha; fechar a ativa ativa a vizinha; fechar a última volta ao mapa", () => {
    localStorage.setItem("auli.entity", "rs");
    render(<App />);
    abrir("sp");
    abrir("mg");
    fireEvent.click(aba("SEFAZ-SP"));

    fecharPeloX("SEF-MG"); // inativa: a ativa não muda
    expect(nomes()).toEqual(["SEFAZ-RS", "SEFAZ-SP"]);
    expect(ativa()).toBe("SEFAZ-SP");

    fecharPeloX("SEFAZ-SP"); // ativa e última: vai para a da esquerda
    expect(ativa()).toBe("SEFAZ-RS");

    fecharPeloX("SEFAZ-RS");
    expect(nomes()).toEqual(["Novo estado"]);
    expect(screen.getByRole("group", { name: /Mapa do Brasil/ })).toBeInTheDocument();
  });

  it("teclado: setas trocam (dando a volta), Delete fecha e o foco vai para a nova ativa", () => {
    localStorage.setItem("auli.entity", "rs");
    render(<App />);
    abrir("sp");
    aba("SEFAZ-SP").focus();

    fireEvent.keyDown(aba("SEFAZ-SP"), { key: "ArrowLeft" });
    expect(ativa()).toBe("SEFAZ-RS");
    expect(document.activeElement).toBe(aba("SEFAZ-RS"));

    fireEvent.keyDown(aba("SEFAZ-RS"), { key: "ArrowLeft" });
    expect(ativa()).toBe("SEFAZ-SP");

    fireEvent.keyDown(aba("SEFAZ-SP"), { key: "Delete" });
    expect(nomes()).toEqual(["SEFAZ-RS"]);
    expect(document.activeElement).toBe(aba("SEFAZ-RS"));
  });

  it("as abas voltam depois de recarregar, na ordem e com a mesma ativa; a do mapa não", () => {
    localStorage.setItem("auli.entity", "rs");
    const { unmount } = render(<App />);
    abrir("sp");
    abrir("mg");
    fireEvent.click(aba("SEFAZ-SP"));
    fireEvent.click(screen.getByRole("button", { name: "Abrir outro estado" })); // fica no mapa
    unmount();

    render(<App />);
    expect(nomes()).toEqual(["SEFAZ-RS", "SEFAZ-SP", "SEF-MG"]);
    expect(ativa()).toBe("SEFAZ-RS"); // a ativa era a do mapa, que não volta: a primeira
  });

  it("o botão da barra lateral aponta para a sidebar da aba ativa e some na aba do mapa", () => {
    localStorage.setItem("auli.entity", "rs");
    render(<App />);
    abrir("sp");
    const botao = () => screen.queryByRole("button", { name: /barra lateral/ });
    expect(botao()).toHaveAttribute("aria-controls", "sp-sidebar-secoes");

    fireEvent.click(aba("SEFAZ-RS"));
    expect(botao()).toHaveAttribute("aria-controls", "rs-sidebar-secoes");

    fireEvent.click(screen.getByRole("button", { name: "Abrir outro estado" }));
    expect(botao()).toBeNull();
  });

  it("fechar o painel Minhas perguntas numa aba fecha nas outras, e abrir também", () => {
    localStorage.setItem("auli.abas", JSON.stringify({ abas: ["rs", "sp"], ativa: "rs" }));
    render(<App />);
    const doPainel = (nome: string) =>
      within(painel(nome)).queryByRole("complementary", { name: "Minhas perguntas" });
    expect(doPainel("SEFAZ-RS")).not.toBeNull();

    fireEvent.click(within(painel("SEFAZ-RS")).getByRole("button", { name: "Fechar minhas perguntas" }));
    fireEvent.click(aba("SEFAZ-SP"));
    expect(doPainel("SEFAZ-SP")).toBeNull();

    fireEvent.click(within(painel("SEFAZ-SP")).getByRole("button", { name: "Mostrar minhas perguntas" }));
    fireEvent.click(aba("SEFAZ-RS"));
    expect(doPainel("SEFAZ-RS")).not.toBeNull();
  });

  it("a aba que espera resposta tem spinner, escondida também; a resposta chega nela", async () => {
    let responder!: (r: unknown) => void;
    vi.mocked(axios.post).mockImplementationOnce(() => new Promise((r) => (responder = r)));
    localStorage.setItem("auli.abas", JSON.stringify({ abas: ["rs", "sp"], ativa: "rs" }));
    render(<App />);
    const spinner = (nome: string) => aba(nome).querySelector(".chakra-spinner");

    fireEvent.change(caixa("SEFAZ-RS"), { target: { value: "Qual o prazo do ITCD?" } });
    fireEvent.click(within(painel("SEFAZ-RS")).getByRole("button", { name: "Enviar pesquisa" }));
    expect(nomes()).toEqual(["SEFAZ-RS, respondendo", "SEFAZ-SP"]);
    expect(spinner("SEFAZ-RS, respondendo")).not.toBeNull();
    expect(spinner("SEFAZ-SP")).toBeNull();

    fireEvent.click(aba("SEFAZ-SP"));
    expect(nomes()).toEqual(["SEFAZ-RS, respondendo", "SEFAZ-SP"]); // escondida, segue esperando

    await act(async () => responder({ data: { answer: "Resposta do RS" } }));
    expect(nomes()).toEqual(["SEFAZ-RS", "SEFAZ-SP"]);
    expect(spinner("SEFAZ-RS")).toBeNull();
    fireEvent.click(aba("SEFAZ-RS"));
    expect(within(painel("SEFAZ-RS")).getByText("Resposta do RS")).toBeInTheDocument();
  });
});
