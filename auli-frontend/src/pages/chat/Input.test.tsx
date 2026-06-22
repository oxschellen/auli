// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { createRef } from "react";
import { fireEvent, screen } from "@testing-library/react";
import { renderWithProvider } from "../../test/render";
import { Input } from "./Input";

const SHORT = "oi"; // < 10 chars → invalid
const VALID = "uma pergunta de verdade"; // >= 10 chars → valid

function setup(prompt: string, overrides: Partial<Parameters<typeof Input>[0]> = {}) {
  const props = {
    textareaRef: createRef<HTMLTextAreaElement>(),
    prompt,
    updatePrompt: vi.fn(),
    loading: false,
    callServerAPI: vi.fn(),
    ...overrides,
  };
  renderWithProvider(<Input {...props} />);
  return props;
}

describe("Input", () => {
  it("disables send and shows the minimum-chars hint for a short prompt", () => {
    setup(SHORT);
    expect(screen.getByLabelText("Enviar pesquisa")).toBeDisabled();
    expect(screen.getByText(/Mínimo \d+ caracteres/)).toBeInTheDocument();
  });

  it("enables send and shows the ready hint for a valid prompt", () => {
    setup(VALID);
    expect(screen.getByLabelText("Enviar pesquisa")).toBeEnabled();
    expect(screen.getByText("Pronto para enviar")).toBeInTheDocument();
  });

  it("submits on click with a valid prompt", () => {
    const { callServerAPI } = setup(VALID);
    fireEvent.click(screen.getByLabelText("Enviar pesquisa"));
    expect(callServerAPI).toHaveBeenCalledWith(VALID);
  });

  it("submits on Enter (without Shift) when valid", () => {
    const { callServerAPI } = setup(VALID);
    fireEvent.keyDown(screen.getByPlaceholderText("Digite sua pergunta..."), {
      key: "Enter",
    });
    expect(callServerAPI).toHaveBeenCalledWith(VALID);
  });

  it("does not submit on Enter when the prompt is too short", () => {
    const { callServerAPI } = setup(SHORT);
    fireEvent.keyDown(screen.getByPlaceholderText("Digite sua pergunta..."), {
      key: "Enter",
    });
    expect(callServerAPI).not.toHaveBeenCalled();
  });

  it("does not submit on Shift+Enter (newline)", () => {
    const { callServerAPI } = setup(VALID);
    fireEvent.keyDown(screen.getByPlaceholderText("Digite sua pergunta..."), {
      key: "Enter",
      shiftKey: true,
    });
    expect(callServerAPI).not.toHaveBeenCalled();
  });
});
