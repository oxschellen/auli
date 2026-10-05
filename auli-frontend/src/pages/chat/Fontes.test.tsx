// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { fireEvent, screen } from "@testing-library/react";
import { renderWithProvider } from "../../test/render";
import { Fontes, resumoFontes } from "./Fontes";
import { SystemMessage } from "./SystemMessage";
import type { Fonte } from "../../types/chat";

const fontes: Fonte[] = [
  { rotulo: "Serviço 1", titulo: "Emitir guia", link: "https://s/1", triagem: "aplica" },
  { rotulo: "Serviço 2", titulo: "Emitir guia (Simples)", link: "https://s/2", triagem: "condicional" },
  { rotulo: "FAQ 1", titulo: "Como pagar?", link: "https://f/1", triagem: "descarta" },
  { rotulo: "FAQ 2", titulo: "Prazo", link: "" },
];

describe("resumoFontes", () => {
  it("conta o total e, quando houver, os descartados — com o plural certo", () => {
    expect(resumoFontes(fontes)).toBe("4 recuperadas · 1 descartada na triagem");
    expect(resumoFontes([fontes[0]])).toBe("1 recuperada");
  });
});

describe("Fontes", () => {
  it("citadas à vista; consideradas e descartadas recolhidas, abrindo ao clicar", () => {
    renderWithProvider(<Fontes fontes={fontes} citados={new Set(["Serviço 1"])} idBase="msg-x" />);
    const link = screen.getByText("Emitir guia").closest("a");
    expect(link).toHaveAttribute("href", "https://s/1");
    expect(link).toHaveAttribute("target", "_blank");
    expect(screen.queryByText("Emitir guia (Simples)")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /2 também consideradas/ }));
    expect(screen.getByText("Emitir guia (Simples)")).toBeInTheDocument();
    expect(screen.getByText("depende de condição")).toBeInTheDocument();
    // Sem link: o título aparece como texto, não como âncora.
    expect(screen.getByText("Prazo").closest("a")).toBeNull();
    expect(screen.queryByText("Como pagar?")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /descartada na triagem/ }));
    expect(screen.getByText("Como pagar?")).toBeInTheDocument();
  });

  it("sem citação no texto, as consideradas já abrem — são tudo o que há", () => {
    renderWithProvider(<Fontes fontes={fontes} citados={new Set()} idBase="msg-x" />);
    expect(screen.getByText("Emitir guia")).toBeInTheDocument();
    expect(screen.getByText("Também consideradas")).toBeInTheDocument();
  });

  it("cada item tem id único por mensagem, alvo do selo de citação", () => {
    const { container } = renderWithProvider(<Fontes fontes={fontes} citados={new Set(["Serviço 1"])} idBase="msg-42" />);
    expect(container.querySelector("#msg-42-fonte-servico-1")).not.toBeNull();
  });
});

describe("SystemMessage + Fontes", () => {
  it("desenha a lista só quando há fontes, e a citação vira selo", () => {
    const { rerender } = renderWithProvider(
      <SystemMessage id="m1" messageText="**Resumo**\nUse a guia [Serviço 1]." showButton fontes={fontes} />,
    );
    expect(screen.getByRole("heading", { name: "Fontes" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Serviço 1: Emitir guia. Ver nas fontes" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Resumo" })).toHaveTextContent("Use a guia");
    rerender(<SystemMessage id="m1" messageText="Use a guia [Serviço 1]." showButton />);
    expect(screen.queryByRole("heading", { name: "Fontes" })).toBeNull();
    // Sem fontes, a citação fica como texto: não há para onde o selo levar.
    expect(screen.queryByRole("button", { name: /Ver nas fontes/ })).toBeNull();
    expect(screen.getByText(/\[Serviço 1\]/)).toBeInTheDocument();
  });
});
