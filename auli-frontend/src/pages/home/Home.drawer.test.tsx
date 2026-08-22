// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { fireEvent, screen } from "@testing-library/react";
import { renderWithProvider } from "../../test/render";
import { EntityProvider } from "../../shared/EntityContext";
import { Home } from "./Home";

/** O Home exige uma entidade escolhida; o provider a lê do localStorage no primeiro render. */
beforeEach(() => localStorage.setItem("auli.entity", "rs"));

const abrirDrawer = () => fireEvent.click(screen.getByLabelText("Abrir menu de seções"));

const montar = () =>
  renderWithProvider(
    <EntityProvider>
      <Home />
    </EntityProvider>,
  );

describe("drawer de seções (mobile)", () => {
  // Em jsdom o `hideBelow`/`hideFrom` não recorta nada — as duas listas renderizam. É justamente
  // o cenário que interessa aqui: é assim que elas coexistem no navegador real com o drawer aberto.
  it("Esc fecha o drawer e devolve o foco ao botão que o abriu", () => {
    montar();
    const botao = screen.getByLabelText("Abrir menu de seções");
    expect(screen.getAllByRole("tablist")).toHaveLength(1);

    abrirDrawer();
    expect(screen.getAllByRole("tablist")).toHaveLength(2);

    // No documento, não no painel: ao abrir, o foco ainda está no botão, FORA do drawer.
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.getAllByRole("tablist")).toHaveLength(1);
    expect(document.activeElement).toBe(botao);
  });

  it("as duas listas não repetem nenhum id com o drawer aberto", () => {
    // A razão de a lista do drawer usar o prefixo `tabm`: sem isso haveria dois `id="tab-chat"`.
    const { container } = montar();
    abrirDrawer();
    const ids = [...container.querySelectorAll("[id]")].map((e) => e.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("do tablist até cada tab só há elementos presentacionais no caminho", () => {
    // O `tablist` exige `tab` como filhos possuídos, e o agrupamento visual mete um div (o
    // invólucro), um p (o título) e mais um div (o espaçador do rodapé) no meio. Todos os três
    // levam `role="none"`. A descida para no `tab` — o que está DENTRO do botão (ícone, span) é
    // conteúdo dele, não filho do tablist.
    montar();
    const intrusos = (el: Element): string[] =>
      [...el.children].flatMap((filho) => {
        const papel = filho.getAttribute("role") ?? "";
        if (papel === "tab") return [];
        if (papel !== "none") return [filho.tagName.toLowerCase()];
        return intrusos(filho);
      });

    const lista = screen.getAllByRole("tablist")[0];
    expect(intrusos(lista)).toEqual([]);
    // Contagem literal de propósito: é o que pega uma aba entrando ou saindo sem intenção. Foi 9
    // até ago/2026, quando "Acórdãos TARF" entrou no grupo Acervo, 10 até "Legislação" entrar no
    // mesmo grupo (D-LEG-13, auli_code.md §3.13.1), 11 até o grupo "Tributum" entrar com as suas
    // quatro (D-TRIB-3/4) — o primeiro grupo cujas abas NÃO dependem da entidade — e 15 até a aba
    // "Critérios" entrar como a quinta dele (D-TRIB-10).
    expect(lista.querySelectorAll('[role="tab"]')).toHaveLength(16);
  });
});
