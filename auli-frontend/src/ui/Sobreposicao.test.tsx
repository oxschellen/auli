// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { useRef, useState } from "react";
import { fireEvent, renderWithProvider, screen } from "../test/render";
import { Sobreposicao } from "./Sobreposicao";

/** Um botão que abre um painel por cima, como os drawers. */
function Abridor() {
  const [aberto, setAberto] = useState(false);
  const botao = useRef<HTMLButtonElement>(null);
  return (
    <>
      <button ref={botao} onClick={() => setAberto(true)}>
        Abrir
      </button>
      {aberto && (
        <Sobreposicao onFechar={() => setAberto(false)} devolverFocoA={botao}>
          <div role="dialog" aria-label="Painel">
            conteúdo
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
