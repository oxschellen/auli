// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { renderWithProvider, screen } from "../test/render";
import { DocumentoMarkdown } from "./DocumentoMarkdown";

// Cada caso usa um arquivo próprio: o SWR guarda as respostas por chave entre os casos.
vi.mock("../api/estaticos", async (orig) => ({
  ...(await orig<typeof import("../api/estaticos")>()),
  textFetcher: (url: string) =>
    url.includes("/ok.md")
      ? Promise.resolve("# Manifesto\n\nO Auli é um projeto aberto.")
      : Promise.reject(new Error("HTTP 404 em " + url.split("?")[0])),
}));

describe("DocumentoMarkdown", () => {
  it("busca o .md e o mostra como prosa", async () => {
    renderWithProvider(<DocumentoMarkdown arquivo="/ok.md" />);
    expect(await screen.findByRole("heading", { name: "Manifesto" })).toBeVisible();
    expect(screen.getByText("O Auli é um projeto aberto.")).toBeVisible();
  });

  it("na falha, mostra o título e a mensagem dados", async () => {
    renderWithProvider(
      <DocumentoMarkdown arquivo="/falha-1.md" tituloDoErro="Erro" erro="Não foi possível." />,
    );
    expect(await screen.findByText("Não foi possível.")).toBeVisible();
    expect(screen.getByText("Erro")).toBeVisible();
  });

  it("na falha sem mensagem dada, mostra a da própria falha", async () => {
    renderWithProvider(<DocumentoMarkdown arquivo="/falha-2.md" />);
    expect(await screen.findByText("HTTP 404 em /falha-2.md")).toBeVisible();
  });
});
