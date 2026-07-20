// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithProvider } from "../test/render";
import { Highlight } from "./highlight";
import { parseQuery } from "./textSearch";

/** Todos os `<mark>` do documento, na ordem, pelo texto. */
const marcados = (): string[] =>
  Array.from(document.querySelectorAll("mark")).map((m) => m.textContent ?? "");

describe("<Highlight>", () => {
  it("envolve em <mark> só o trecho casado, preservando o resto do texto", () => {
    renderWithProvider(<Highlight text="Crédito de ICMS" terms={parseQuery("icms")} />);
    expect(marcados()).toEqual(["ICMS"]);
    // O texto completo continua legível na tela — a marcação não fragmenta o conteúdo visível.
    expect(document.body.textContent).toContain("Crédito de ICMS");
  });

  it("marca o texto acentuado a partir de query sem acento", () => {
    renderWithProvider(<Highlight text="Substituição tributária" terms={parseQuery("substituicao")} />);
    expect(marcados()).toEqual(["Substituição"]);
  });

  it("marca cada termo de uma query multi-termo", () => {
    renderWithProvider(
      <Highlight text="Crédito de ICMS na cesta básica" terms={parseQuery("credito basica")} />,
    );
    expect(marcados()).toEqual(["Crédito", "básica"]);
  });

  it("sem termos não emite <mark> nenhum (caso comum: lista sem busca)", () => {
    renderWithProvider(<Highlight text="Crédito de ICMS" terms={parseQuery("")} />);
    expect(marcados()).toEqual([]);
    expect(screen.getByText("Crédito de ICMS")).toBeInTheDocument();
  });

  it("termo ausente não emite <mark>", () => {
    renderWithProvider(<Highlight text="Crédito de ICMS" terms={parseQuery("inexistente")} />);
    expect(marcados()).toEqual([]);
  });

  it("o <mark> usa o token de tema, não a cor padrão do browser", () => {
    // A cor padrão do <mark> é preto sobre amarelo puro — ilegível no tema escuro. O realce tem de
    // sair pelo `bg.highlight`/`fg.highlight` do system.js (há regra de lint contra cor literal).
    renderWithProvider(<Highlight text="Crédito de ICMS" terms={parseQuery("icms")} />);
    const mark = document.querySelector("mark");
    expect(mark).not.toBeNull();
    expect(mark?.className).toBeTruthy(); // Chakra emitiu classe própria (não é o <mark> cru)
  });
});
