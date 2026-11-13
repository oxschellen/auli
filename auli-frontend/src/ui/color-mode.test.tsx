// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { renderWithProvider } from "../test/render";
import { ColorModeButton } from "./color-mode";

describe("ColorModeButton", () => {
  it("é um <button type=button> com rótulo, e alterna o tema da página ao clicar", async () => {
    renderWithProvider(<ColorModeButton />);
    const botao = screen.getByRole("button", { name: /Ativar modo/ });
    // `type="button"`: dentro de um <form>, um <button> sem tipo enviaria o formulário.
    expect(botao.tagName).toBe("BUTTON");
    expect(botao).toHaveAttribute("type", "button");

    const escuroAntes = document.documentElement.classList.contains("dark");
    fireEvent.click(botao);
    await waitFor(() =>
      expect(document.documentElement.classList.contains("dark")).toBe(!escuroAntes),
    );
  });
});
