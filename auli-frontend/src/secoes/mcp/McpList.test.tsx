// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { renderWithProvider, screen, within, fireEvent } from "../../test/render";
import { McpList } from "./McpList";

// O leitor busca o .md por axios; sem servidor no jsdom, fica pendente em vez de dar erro de rede.
vi.mock("../../api/estaticos", async (orig) => ({
  ...(await orig<typeof import("../../api/estaticos")>()),
  textFetcher: () => new Promise<string>(() => {}),
}));

const BLOCO = { name: "O que dá para pedir" };

describe("McpList — O que dá para pedir", () => {
  it("lista as cinco ferramentas, na ordem", () => {
    renderWithProvider(<McpList />);
    expect(screen.getByRole("heading", BLOCO)).toBeInTheDocument();
    // Escopo na tabela: três dos nomes também aparecem no texto do bloco, e um `getAllByText`
    // continuaria passando com uma linha a menos.
    const tabela = screen.getByRole("table", { name: "Ferramentas do servidor MCP" });
    const nomes = within(tabela)
      .getAllByRole("row")
      .slice(1)
      .map((linha) => within(linha).getAllByRole("cell")[0].textContent);
    expect(nomes).toEqual([
      "listar_entidades",
      "buscar_pareceres",
      "obter_parecer",
      "consultar_servicos_faqs",
      "buscar_legislacao",
    ]);
  });

  it("não toca a tabela de manuais e some no leitor", () => {
    renderWithProvider(<McpList />);
    expect(screen.getAllByRole("button", { name: /^Ler o manual do / })).toHaveLength(4);
    fireEvent.click(screen.getByRole("button", { name: "Ler o manual do Claude (web)" }));
    expect(screen.queryByRole("heading", BLOCO)).toBeNull();
  });
});
