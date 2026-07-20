// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { renderWithProvider } from "../../test/render";
import { PareceresAccordion } from "./PareceresAccordion";
import type { Parecer } from "./pareceres";

const gerar = (n: number): Parecer[] =>
  Array.from({ length: n }, (_, i) => ({
    numero: `PARECER Nº ${1000 + i}`,
    assunto: `Assunto ${i}`,
    resumo: `Sinopse do parecer ${i}`,
    link: `http://legislacao/${i}`,
  }));

/** Quantas linhas de parecer estão no DOM (uma por `aria-expanded`, o cabeçalho clicável). */
const linhasRenderizadas = () => document.querySelectorAll("[aria-expanded]").length;

/** Instala um IntersectionObserver que nunca dispara — simula o navegador antes de qualquer scroll. */
function stubIntersectionObserver() {
  class IOStub {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
  (globalThis as unknown as { IntersectionObserver: unknown }).IntersectionObserver = IOStub;
}

afterEach(() => {
  delete (globalThis as unknown as { IntersectionObserver?: unknown }).IntersectionObserver;
});

describe("PareceresAccordion — render progressivo", () => {
  it("renderiza só o primeiro lote quando a lista é grande", () => {
    // O acervo do SP são 15,6 mil linhas a ~9 elementos de DOM cada: montar tudo de uma vez trava a
    // thread principal por segundos e a barra de busca não aceita foco. Aqui entram 100 por vez.
    stubIntersectionObserver();
    renderWithProvider(<PareceresAccordion pareceres={gerar(2500)} searchQuery="" />);
    expect(linhasRenderizadas()).toBe(100);
  });

  it("lista menor que o lote entra inteira, sem sentinela", () => {
    stubIntersectionObserver();
    renderWithProvider(<PareceresAccordion pareceres={gerar(30)} searchQuery="" />);
    expect(linhasRenderizadas()).toBe(30);
  });

  it("sem IntersectionObserver, degrada para renderizar tudo (nunca trunca o acervo)", () => {
    // Melhor lento que inacessível: um parecer fora do DOM e fora do alcance do scroll seria um
    // documento que o usuário não tem como abrir.
    renderWithProvider(<PareceresAccordion pareceres={gerar(250)} searchQuery="" />);
    expect(linhasRenderizadas()).toBe(250);
  });

  it("o primeiro lote é o começo da lista, na ordem", () => {
    stubIntersectionObserver();
    renderWithProvider(<PareceresAccordion pareceres={gerar(2500)} searchQuery="" />);
    expect(document.body.textContent).toContain("PARECER Nº 1000");
    expect(document.body.textContent).toContain("PARECER Nº 1099");
    expect(document.body.textContent).not.toContain("PARECER Nº 1100");
  });
});
