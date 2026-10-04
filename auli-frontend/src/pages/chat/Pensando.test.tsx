// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithProvider } from "../../test/render";
import { Messages } from "./Messages";
import { Pensando, etapaEm } from "./Pensando";

describe("etapaEm", () => {
  it("troca de etapa nos limites de tempo", () => {
    expect(etapaEm(0)).toBe("Buscando nos documentos");
    expect(etapaEm(2)).toBe("Buscando nos documentos");
    expect(etapaEm(3)).toBe("Separando o que se aplica à sua pergunta");
    expect(etapaEm(8)).toBe("Redigindo a resposta");
    expect(etapaEm(60)).toMatch(/^Quase lá/);
  });
});

describe("Pensando", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("avança a etapa e o contador com o tempo", () => {
    renderWithProvider(<Pensando />);
    expect(screen.getByRole("status").textContent).toBe("Buscando nos documentos…");
    expect(screen.getByText("0 s")).toBeInTheDocument();
    act(() => vi.advanceTimersByTime(8000));
    expect(screen.getByRole("status").textContent).toBe("Redigindo a resposta…");
    expect(screen.getByText("8 s")).toBeInTheDocument();
  });
});

describe("Messages", () => {
  it("desenha a mensagem pendente como a animação, não como texto", () => {
    renderWithProvider(
      <Messages
        setPrompt={vi.fn()}
        messages={[
          {
            id: "1",
            from: "server",
            text: "Aguarde! Pensando...",
            showButton: false,
            pendente: true,
          },
        ]}
      />,
    );
    expect(screen.getByRole("status").textContent).toBe("Buscando nos documentos…");
    expect(screen.queryByText("Aguarde! Pensando...")).toBeNull();
  });
});
