// @vitest-environment jsdom
import { afterEach, describe, it, expect, vi } from "vitest";
import { renderWithProvider, screen } from "../test/render";
import { abrirEmNovaAba, LinkExterno } from "./LinkExterno";

afterEach(() => vi.restoreAllMocks());

describe("LinkExterno", () => {
  it("abre fora, sem dar à página aberta acesso à nossa, e se nomeia pelo aria-label", () => {
    renderWithProvider(
      <LinkExterno href="https://x.gov.br/a" rotulo="Abrir no portal" ariaLabel="Abrir A" />,
    );
    const link = screen.getByRole("link", { name: "Abrir A" });
    expect(link).toHaveAttribute("href", "https://x.gov.br/a");
    expect(link).toHaveAttribute("target", "_blank");
    expect(link).toHaveAttribute("rel", "noopener noreferrer");
    expect(link).toHaveTextContent("Abrir no portal");
  });

  it("abrirEmNovaAba abre numa aba nova, sem window.opener", () => {
    const open = vi.spyOn(window, "open").mockImplementation(() => null);
    abrirEmNovaAba("https://x.gov.br/b");
    expect(open).toHaveBeenCalledWith("https://x.gov.br/b", "_blank", "noopener,noreferrer");
  });
});
