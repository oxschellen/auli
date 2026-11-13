// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import App from "./App";
import { SIDEBAR_RAIL_WIDTH, SIDEBAR_VAR, SIDEBAR_WIDTH } from "../ui/layout";

/** O valor de `prop` no CSS que o Chakra emitiu para as classes de `el`, fora de media query
 *  (jsdom não calcula estilo). */
const declaracao = (el: Element, prop: string) => {
  const classes = el.className.split(/\s+/).filter(Boolean);
  const css = [...document.querySelectorAll("style")].map((s) => s.textContent).join("\n");
  const padrao = new RegExp(`[{;]\\s*${prop}:\\s*([^;}]+)`);
  for (const regra of css.split("}")) {
    if (regra.includes("@media") || !classes.some((c) => regra.includes(`.${c}{`))) continue;
    const valor = regra.match(padrao)?.[1].trim();
    if (valor) return valor;
  }
  return undefined;
};

const sidebar = () => document.getElementById("rs-sidebar-secoes")!;
/** Largura da sidebar e a que a barra fixa do chat lê (definida na raiz do Home). */
const larguras = () => [
  declaracao(sidebar(), "width"),
  declaracao(sidebar().parentElement!, SIDEBAR_VAR),
];
const botao = () => screen.getByRole("button", { name: /barra lateral/ });
const ctrlB = (extra: Partial<KeyboardEventInit> = {}) =>
  fireEvent.keyDown(document, { key: "b", ctrlKey: true, ...extra });

describe("sidebar recolhível (botão ao lado do logo)", () => {
  beforeEach(() => {
    // A aba do RS vem da migração do `auli.entity` (sem `auli.abas` salvo).
    localStorage.removeItem("auli.abas");
    localStorage.setItem("auli.entity", "rs");
    localStorage.removeItem("auli.sidebar");
  });

  it("recolhe para a faixa de ícones e expande de volta, e a barra do chat acompanha", () => {
    render(<App />);
    expect(botao()).toHaveAccessibleName("Recolher a barra lateral");
    expect(botao()).toHaveAttribute("aria-expanded", "true");
    expect(botao()).toHaveAttribute("aria-controls", "rs-sidebar-secoes");
    expect(larguras()).toEqual([SIDEBAR_WIDTH, SIDEBAR_WIDTH]);

    fireEvent.click(botao());
    expect(botao()).toHaveAccessibleName("Expandir a barra lateral");
    expect(botao()).toHaveAttribute("aria-expanded", "false");
    expect(larguras()).toEqual([SIDEBAR_RAIL_WIDTH, SIDEBAR_RAIL_WIDTH]);

    fireEvent.click(botao());
    expect(larguras()).toEqual([SIDEBAR_WIDTH, SIDEBAR_WIDTH]);
  });

  it("recolhida, as abas seguem nomeadas pelo rótulo e os títulos de grupo saem", () => {
    render(<App />);
    fireEvent.click(botao());
    const lista = within(sidebar());
    // O rótulo nomeia a aba e, pelo `aria-labelledby`, o painel — sumir com ele apagaria os dois.
    expect(lista.getByRole("tab", { name: "Serviços" })).toBeInTheDocument();
    expect(screen.getByRole("tabpanel", { name: "Chat" })).toBeInTheDocument();
    expect(lista.queryByText("Acervo")).toBeNull();
  });

  it("a escolha sobrevive ao recarregar", () => {
    const { unmount } = render(<App />);
    fireEvent.click(botao());
    unmount();

    render(<App />);
    expect(botao()).toHaveAccessibleName("Expandir a barra lateral");
    expect(larguras()).toEqual([SIDEBAR_RAIL_WIDTH, SIDEBAR_RAIL_WIDTH]);
  });

  it("acompanha a escolha feita em outra janela do navegador", () => {
    render(<App />);
    expect(botao()).toHaveAccessibleName("Recolher a barra lateral");
    act(() => {
      localStorage.setItem("auli.sidebar", "recolhida");
      window.dispatchEvent(new StorageEvent("storage", { key: "auli.sidebar" }));
    });
    expect(botao()).toHaveAccessibleName("Expandir a barra lateral");
    expect(larguras()).toEqual([SIDEBAR_RAIL_WIDTH, SIDEBAR_RAIL_WIDTH]);
  });

  it("Ctrl+B e ⌘B alternam; com Shift ou Alt, a tecla fica com o navegador", () => {
    render(<App />);
    // `fireEvent` devolve `false` quando o handler chamou `preventDefault`.
    expect(ctrlB()).toBe(false);
    expect(botao()).toHaveAccessibleName("Expandir a barra lateral");
    expect(ctrlB({ ctrlKey: false, metaKey: true })).toBe(false);
    expect(botao()).toHaveAccessibleName("Recolher a barra lateral");

    expect(ctrlB({ shiftKey: true })).toBe(true); // Ctrl+Shift+B: barra de favoritos do Chrome
    expect(ctrlB({ altKey: true })).toBe(true);
    expect(fireEvent.keyDown(document, { key: "b" })).toBe(true);
    expect(botao()).toHaveAccessibleName("Recolher a barra lateral");
  });
});

describe("sem estado escolhido", () => {
  beforeEach(() => {
    localStorage.removeItem("auli.abas");
    localStorage.removeItem("auli.entity");
  });

  it("não há sidebar: nem botão, nem Ctrl+B", () => {
    render(<App />);
    expect(screen.queryByRole("button", { name: /barra lateral/ })).toBeNull();
    expect(ctrlB()).toBe(true);
  });
});
