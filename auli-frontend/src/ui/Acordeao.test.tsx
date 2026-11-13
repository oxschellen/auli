// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { useState } from "react";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { renderWithProvider } from "../test/render";
import { comoBotao, PainelRevelado } from "./Acordeao";

describe("comoBotao", () => {
  it("dá papel de botão e foco, e age com clique, Enter e Espaço — e só com eles", () => {
    const acao = vi.fn();
    renderWithProvider(<div {...comoBotao(acao)}>Abrir</div>);
    const el = screen.getByRole("button", { name: "Abrir" });
    expect(el).toHaveAttribute("tabindex", "0");

    fireEvent.click(el);
    fireEvent.keyDown(el, { key: "Enter" });
    // `fireEvent` devolve `false` quando o handler chamou `preventDefault`: o Espaço não rola a página.
    expect(fireEvent.keyDown(el, { key: " " })).toBe(false);
    fireEvent.keyDown(el, { key: "a" });
    fireEvent.keyDown(el, { key: "Tab" });
    expect(acao).toHaveBeenCalledTimes(3);
  });

  it("inativo, o elemento é só texto: sem papel, sem foco e sem ação", () => {
    const acao = vi.fn();
    renderWithProvider(<div {...comoBotao(acao, false)}>Rótulo</div>);
    expect(screen.queryByRole("button")).toBeNull();
    const el = screen.getByText("Rótulo");
    expect(el).not.toHaveAttribute("tabindex");
    fireEvent.click(el);
    fireEvent.keyDown(el, { key: "Enter" });
    expect(acao).not.toHaveBeenCalled();
  });
});

describe("PainelRevelado", () => {
  function Item() {
    const [aberto, setAberto] = useState(false);
    return (
      <>
        <div {...comoBotao(() => setAberto((a) => !a))} aria-expanded={aberto}>
          Seção
        </div>
        <PainelRevelado aberto={aberto}>Conteúdo da seção</PainelRevelado>
      </>
    );
  }

  it("fechado não desenha o conteúdo; abre e fecha pelo cabeçalho", async () => {
    renderWithProvider(<Item />);
    expect(screen.queryByText("Conteúdo da seção")).toBeNull();

    fireEvent.keyDown(screen.getByRole("button", { name: "Seção" }), { key: "Enter" });
    expect(screen.getByText("Conteúdo da seção")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Seção" }));
    await waitFor(() => expect(screen.queryByText("Conteúdo da seção")).toBeNull());
  });

  it("o painel contém o próprio layout (é o que a dispensa do react-doctor exige)", () => {
    renderWithProvider(<PainelRevelado aberto>Conteúdo</PainelRevelado>);
    const painel = screen.getByText("Conteúdo");
    expect(painel.style.contain).toBe("layout paint");
    expect(painel.style.overflow).toBe("hidden");
  });
});
