// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import ReactMarkdown from "react-markdown";
import { compactMarkdownComponents, markdownPlugins } from "./markdown";

function renderMd(md: string) {
  return render(
    <ReactMarkdown remarkPlugins={markdownPlugins} components={compactMarkdownComponents}>
      {md}
    </ReactMarkdown>,
  );
}

describe("markdown rendering", () => {
  it("renders GFM tables as a real <table> (rows on their own lines)", () => {
    const { container } = renderMd("| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |");
    expect(container.querySelector("table")).toBeTruthy();
    expect(container.querySelectorAll("tbody tr").length).toBe(2);
    expect(container.querySelectorAll("td").length).toBe(4);
  });

  it("autolinks a bare https URL and routes it through the target=_blank link renderer", () => {
    const url = "https://www.legislacao.sefaz.rs.gov.br/Site/DocumentView.aspx?inpKey=273534";
    const { container } = renderMd(`Veja ${url} para detalhes.`);
    const a = container.querySelector("a");
    expect(a).toBeTruthy();
    expect(a?.getAttribute("href")).toContain("inpKey=273534");
    expect(a?.getAttribute("target")).toBe("_blank");
    expect(a?.getAttribute("rel")).toContain("noopener");
  });

  it("renders a markdown link as a clickable, safe link", () => {
    const { container } = renderMd("[Ver parecer](https://exemplo/p/1)");
    const a = container.querySelector("a");
    expect(a?.getAttribute("href")).toBe("https://exemplo/p/1");
    expect(a?.getAttribute("target")).toBe("_blank");
    expect(a?.textContent).toBe("Ver parecer");
  });

  it("forces a scheme-less markdown-link target to an absolute http:// URL (so it can't resolve as relative)", () => {
    const { container } = renderMd(
      "[parecer](www.legislacao.sefaz.rs.gov.br/Site/DocumentView.aspx?inpKey=299748)",
    );
    const a = container.querySelector("a");
    expect(a?.getAttribute("href")).toBe(
      "http://www.legislacao.sefaz.rs.gov.br/Site/DocumentView.aspx?inpKey=299748",
    );
  });

  it("keeps an explicit http:// portal link exactly as given", () => {
    const url = "http://www.legislacao.sefaz.rs.gov.br/Site/DocumentView.aspx?inpKey=299748";
    const { container } = renderMd(`[parecer](${url})`);
    expect(container.querySelector("a")?.getAttribute("href")).toBe(url);
  });
});
