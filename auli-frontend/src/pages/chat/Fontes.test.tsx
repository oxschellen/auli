// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { screen } from "@testing-library/react";
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
    expect(resumoFontes(fontes)).toBe("Fontes consultadas: 4 (1 descartada na triagem)");
    expect(resumoFontes([fontes[0]])).toBe("Fontes consultadas: 1");
    const duas = fontes.map((f) => ({ ...f, triagem: "descarta" as const }));
    expect(resumoFontes(duas)).toBe("Fontes consultadas: 4 (4 descartadas na triagem)");
  });
});

describe("Fontes", () => {
  it("mostra rótulo da citação, título em link e o selo do veredito", () => {
    const { container } = renderWithProvider(<Fontes fontes={fontes} />);
    // Recolhida por padrão.
    expect(container.querySelector("details")?.hasAttribute("open")).toBe(false);
    expect(screen.getByText("[Serviço 2]")).toBeInTheDocument();
    const link = screen.getByText("Emitir guia (Simples)").closest("a");
    expect(link).toHaveAttribute("href", "https://s/2");
    expect(link).toHaveAttribute("target", "_blank");
    expect(screen.getByText(/depende de condição/)).toBeInTheDocument();
    expect(screen.getByText(/descartada na triagem$/)).toBeInTheDocument();
    // Sem link: o título aparece como texto, não como âncora.
    expect(screen.getByText("Prazo").closest("a")).toBeNull();
  });
});

describe("SystemMessage + Fontes", () => {
  it("desenha a lista só quando há fontes", () => {
    const { rerender } = renderWithProvider(
      <SystemMessage messageText="Resposta" showButton fontes={fontes} />,
    );
    expect(screen.getByText(/Fontes consultadas: 4/)).toBeInTheDocument();
    rerender(<SystemMessage messageText="Resposta" showButton />);
    expect(screen.queryByText(/Fontes consultadas/)).toBeNull();
  });
});
