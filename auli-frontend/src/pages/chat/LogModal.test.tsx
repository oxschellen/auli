// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { renderWithProvider } from "../../test/render";
import { SystemMessage } from "./SystemMessage";

const ROTULO_LOG = "Ver o log de auditoria desta resposta";
const ROTULO_COPIA = "Copiar resposta para a área de transferência";

const REGISTRO =
  "----- PERGUNTA (ORIGINAL) -----\nquanto é a alíquota?\n\n" +
  "----- PERGUNTA (ANONIMIZADA) -----\nquanto é a alíquota?\n";

function respostaFalsa(body: string, ok = true) {
  return vi.fn().mockResolvedValue({ ok, text: () => Promise.resolve(body) });
}

// Re-stub a cada caso; **nada de `vi.unstubAllGlobals()` aqui.** O `src/test/setup.ts` instala
// `matchMedia` e `ResizeObserver` com `stubGlobal` uma única vez, no carregamento do módulo —
// limpar todos os stubs os leva junto, e a partir do segundo caso o Chakra/next-themes quebra
// com "window.matchMedia is not a function".
beforeEach(() => {
  vi.stubGlobal("fetch", respostaFalsa(REGISTRO));
});

describe("ícone do log", () => {
  it("aparece ao lado do de cópia quando há log_id", () => {
    renderWithProvider(<SystemMessage messageText="Depende." showButton logId="abc" />);
    expect(screen.getByLabelText(ROTULO_LOG)).toBeInTheDocument();
    expect(screen.getByLabelText(ROTULO_COPIA)).toBeInTheDocument();
  });

  /** Sem `log_id` não há registro para abrir — oferecer o ícone levaria a um 404. */
  it("some quando a gravação do log falhou (sem log_id)", () => {
    renderWithProvider(<SystemMessage messageText="Depende." showButton />);
    expect(screen.queryByLabelText(ROTULO_LOG)).toBeNull();
    expect(screen.getByLabelText(ROTULO_COPIA)).toBeInTheDocument();
  });

  /** A saudação chega com `showButton=false` e sem `logId`: nenhum dos dois botões. */
  it("não aparece na saudação", () => {
    renderWithProvider(
      <SystemMessage messageText="Olá! Como posso ajudar?" showButton={false} />,
    );
    expect(screen.queryByLabelText(ROTULO_LOG)).toBeNull();
    expect(screen.queryByLabelText(ROTULO_COPIA)).toBeNull();
  });

  /** O mesmo tamanho do ícone de cópia é decisão de UI registrada na D-LOG-4. */
  it("tem o mesmo tamanho do botão de cópia", () => {
    renderWithProvider(<SystemMessage messageText="Depende." showButton logId="abc" />);
    const log = screen.getByLabelText(ROTULO_LOG);
    const copia = screen.getByLabelText(ROTULO_COPIA);
    expect(log.className).toEqual(copia.className);
  });
});

describe("modal do log", () => {
  it("mostra o registro como está, sem reformatar", async () => {
    renderWithProvider(<SystemMessage messageText="Depende." showButton logId="abc" />);
    fireEvent.click(screen.getByLabelText(ROTULO_LOG));

    // `textContent` do <pre>: preserva as quebras e as seções exatamente como vieram (D-LOG-5).
    await waitFor(() => {
      expect(screen.getByRole("dialog").textContent).toContain(REGISTRO);
    });
  });

  /** O corpo do 404 do backend é a mensagem exibida — a MESMA para inexistente e para podado. */
  it("mostra a mensagem do backend quando o log não está mais lá", async () => {
    vi.stubGlobal("fetch", respostaFalsa("Log expirado ou inexistente.", false));
    renderWithProvider(<SystemMessage messageText="Depende." showButton logId="abc" />);
    fireEvent.click(screen.getByLabelText(ROTULO_LOG));

    await waitFor(() => {
      expect(screen.getByText("Log expirado ou inexistente.")).toBeInTheDocument();
    });
  });

  it("fecha e reabre", async () => {
    renderWithProvider(<SystemMessage messageText="Depende." showButton logId="abc" />);

    fireEvent.click(screen.getByLabelText(ROTULO_LOG));
    await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());

    fireEvent.click(screen.getByLabelText("Fechar"));
    expect(screen.queryByRole("dialog")).toBeNull();

    fireEvent.click(screen.getByLabelText(ROTULO_LOG));
    await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());
  });

  /** A URL pedida é a do UUID daquela resposta — é o que liga o ícone ao registro certo. */
  it("busca pelo uuid da própria resposta", async () => {
    renderWithProvider(<SystemMessage messageText="Depende." showButton logId="uuid-da-vez" />);
    fireEvent.click(screen.getByLabelText(ROTULO_LOG));

    await waitFor(() => {
      const url = vi.mocked(fetch).mock.calls[0][0] as string;
      expect(url).toMatch(/\/v1\/log\/uuid-da-vez$/);
    });
  });
});
