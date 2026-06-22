// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithProvider } from "../test/render";
import { AsyncContent } from "./AsyncContent";

describe("AsyncContent", () => {
  it("shows the loading text and hides children while loading", () => {
    renderWithProvider(
      <AsyncContent loading error={null} loadingText="Carregando serviços">
        <div>conteúdo</div>
      </AsyncContent>,
    );
    expect(screen.getByText("Carregando serviços")).toBeInTheDocument();
    expect(screen.queryByText("conteúdo")).not.toBeInTheDocument();
  });

  it("shows the error title and description on error", () => {
    renderWithProvider(
      <AsyncContent
        loading={false}
        error={new Error("boom")}
        errorTitle="Falhou"
        errorDescription="Detalhe do erro"
      >
        <div>conteúdo</div>
      </AsyncContent>,
    );
    expect(screen.getByText("Falhou")).toBeInTheDocument();
    expect(screen.getByText("Detalhe do erro")).toBeInTheDocument();
    expect(screen.queryByText("conteúdo")).not.toBeInTheDocument();
  });

  it("renders children once ready (not loading, no error)", () => {
    renderWithProvider(
      <AsyncContent loading={false} error={null}>
        <div>conteúdo</div>
      </AsyncContent>,
    );
    expect(screen.getByText("conteúdo")).toBeInTheDocument();
  });
});
