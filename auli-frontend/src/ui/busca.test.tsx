// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { fireEvent, screen } from "@testing-library/react";
import { renderWithProvider } from "../test/render";
import { BarraDeBusca, SemResultados, useBusca } from "./busca";

/** Uma lista mínima: a barra com um contador que expõe o estado da busca. */
function Lista({ comContador = true }: { comContador?: boolean }) {
  const busca = useBusca();
  return (
    <BarraDeBusca busca={busca} placeholder="Pesquisar itens...">
      {comContador &&
        `buscando=${busca.buscando} termos=${busca.termos.join("+")} adiado=${busca.adiado}`}
    </BarraDeBusca>
  );
}

const campo = () => screen.getByPlaceholderText("Pesquisar itens...");

describe("useBusca e BarraDeBusca", () => {
  it("o que se digita vira texto adiado e termos normalizados", () => {
    renderWithProvider(<Lista />);
    fireEvent.change(campo(), { target: { value: "Alíquota ICMS" } });
    expect(
      screen.getByText("buscando=true termos=aliquota+icms adiado=Alíquota ICMS"),
    ).toBeVisible();
  });

  it("uma letra isolada não vira termo, mas a busca está ativa", () => {
    renderWithProvider(<Lista />);
    fireEvent.change(campo(), { target: { value: "a" } });
    expect(screen.getByText("buscando=true termos= adiado=a")).toBeVisible();
  });

  it("limpar pelo botão esvazia o campo e devolve o foco a ele", () => {
    renderWithProvider(<Lista />);
    fireEvent.change(campo(), { target: { value: "ipva" } });
    fireEvent.click(screen.getByLabelText("Limpar busca"));
    expect(campo()).toHaveValue("");
    expect(campo()).toHaveFocus();
    expect(screen.getByText("buscando=false termos= adiado=")).toBeVisible();
  });

  it("sem contador, a linha dele não aparece", () => {
    const { container } = renderWithProvider(<Lista comContador={false} />);
    expect(container.querySelectorAll("p")).toHaveLength(0);
  });

  it("SemResultados diz o que foi buscado", () => {
    renderWithProvider(<SemResultados consulta="xyz" />);
    expect(screen.getByText("Nenhum resultado encontrado para “xyz”")).toBeVisible();
  });
});
