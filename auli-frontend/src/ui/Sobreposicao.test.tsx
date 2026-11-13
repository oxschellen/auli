// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { useRef, useState } from "react";
import { fireEvent, renderWithProvider, screen } from "../test/render";
import { Sobreposicao } from "./Sobreposicao";

/** Um botão que abre um painel por cima, como os drawers: dois focáveis dentro, e um botão fora. */
function Abridor() {
  const [aberto, setAberto] = useState(false);
  const botao = useRef<HTMLButtonElement>(null);
  return (
    <>
      <button ref={botao} onClick={() => setAberto(true)}>
        Abrir
      </button>
      <button>Fora</button>
      {aberto && (
        <Sobreposicao onFechar={() => setAberto(false)} devolverFocoA={botao}>
          <div role="dialog" aria-label="Painel">
            <button>Primeiro</button>
            <button onClick={() => setAberto(false)}>Fechar</button>
            <button tabIndex={-1}>Fora da ordem</button>
          </div>
        </Sobreposicao>
      )}
    </>
  );
}

describe("Sobreposicao", () => {
  it("o Esc fecha e devolve o foco ao botão que abriu", () => {
    renderWithProvider(<Abridor />);
    fireEvent.click(screen.getByRole("button", { name: "Abrir" }));
    expect(screen.getByRole("dialog", { name: "Painel" })).toBeInTheDocument();

    fireEvent.keyDown(document, { key: "Enter" });
    expect(screen.getByRole("dialog", { name: "Painel" })).toBeInTheDocument();

    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByRole("button", { name: "Abrir" })).toHaveFocus();
  });

  it("ao abrir, o foco vai para o primeiro focável do painel", () => {
    renderWithProvider(<Abridor />);
    fireEvent.click(screen.getByRole("button", { name: "Abrir" }));
    expect(screen.getByRole("button", { name: "Primeiro" })).toHaveFocus();
  });

  it("o Tab dá a volta nas pontas, pulando o que está fora da ordem (tabindex=-1)", () => {
    renderWithProvider(<Abridor />);
    fireEvent.click(screen.getByRole("button", { name: "Abrir" }));
    const primeiro = screen.getByRole("button", { name: "Primeiro" });
    const ultimo = screen.getByRole("button", { name: "Fechar" });

    // No meio, o Tab é do navegador: a camada não o segura (`fireEvent` devolve `true`).
    expect(fireEvent.keyDown(primeiro, { key: "Tab" })).toBe(true);
    // Shift+Tab no primeiro vai ao último; Tab no último volta ao primeiro.
    expect(fireEvent.keyDown(primeiro, { key: "Tab", shiftKey: true })).toBe(false);
    expect(ultimo).toHaveFocus();
    expect(fireEvent.keyDown(ultimo, { key: "Tab" })).toBe(false);
    expect(primeiro).toHaveFocus();
  });

  it("o foco que escapa para fora volta para dentro", () => {
    renderWithProvider(<Abridor />);
    fireEvent.click(screen.getByRole("button", { name: "Abrir" }));
    screen.getByRole("button", { name: "Fora" }).focus();
    expect(screen.getByRole("button", { name: "Primeiro" })).toHaveFocus();
  });

  it("fechar por um botão do painel também devolve o foco a quem abriu", () => {
    renderWithProvider(<Abridor />);
    fireEvent.click(screen.getByRole("button", { name: "Abrir" }));
    fireEvent.click(screen.getByRole("button", { name: "Fechar" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByRole("button", { name: "Abrir" })).toHaveFocus();
    // Fechada, a camada já não segura o foco.
    screen.getByRole("button", { name: "Fora" }).focus();
    expect(screen.getByRole("button", { name: "Fora" })).toHaveFocus();
  });

  it("clicar no véu fecha; clicar no painel, não", () => {
    const onFechar = vi.fn();
    renderWithProvider(
      <Sobreposicao onFechar={onFechar} data-testid="camada">
        <div>painel</div>
      </Sobreposicao>,
    );
    fireEvent.click(screen.getByText("painel"));
    expect(onFechar).not.toHaveBeenCalled();
    const veu = screen.getByTestId("camada").firstElementChild!;
    fireEvent.click(veu);
    expect(onFechar).toHaveBeenCalledTimes(1);
  });

  it("fechada, não deixa ouvinte de Esc para trás", () => {
    const onFechar = vi.fn();
    const { unmount } = renderWithProvider(
      <Sobreposicao onFechar={onFechar}>
        <div>painel</div>
      </Sobreposicao>,
    );
    unmount();
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onFechar).not.toHaveBeenCalled();
  });
});
