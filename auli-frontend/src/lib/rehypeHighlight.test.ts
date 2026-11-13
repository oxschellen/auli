import { describe, it, expect } from "vitest";
import { rehypeHighlight, type HastNode } from "./rehypeHighlight";
import { parseQuery } from "./textSearch";

const texto = (value: string): HastNode => ({ type: "text", value });
const elemento = (tagName: string, ...children: HastNode[]): HastNode => ({
  type: "element",
  tagName,
  properties: {},
  children,
});

/** Roda o plugin sobre a árvore (in place) e devolve ela. */
const marcar = (tree: HastNode, query: string): HastNode => {
  rehypeHighlight(parseQuery(query))()(tree);
  return tree;
};

/** Serializa a árvore com `«»` em volta do que foi marcado — legível no assert. */
function serializar(node: HastNode): string {
  if (node.type === "text") return node.value ?? "";
  const dentro = (node.children ?? []).map(serializar).join("");
  return node.tagName === "mark" ? `«${dentro}»` : dentro;
}

describe("rehypeHighlight", () => {
  it("marca dentro de um nó de texto simples", () => {
    const tree = elemento("p", texto("Crédito de ICMS"));
    expect(serializar(marcar(tree, "icms"))).toBe("Crédito de «ICMS»");
  });

  it("marca com acento na árvore e query sem acento", () => {
    const tree = elemento("p", texto("Substituição tributária"));
    expect(serializar(marcar(tree, "substituicao"))).toBe("«Substituição» tributária");
  });

  it("desce em elementos aninhados (o termo dentro de um <strong> num <li>)", () => {
    const tree = elemento(
      "ul",
      elemento("li", texto("chave: "), elemento("strong", texto("eletroposto"))),
    );
    expect(serializar(marcar(tree, "eletroposto"))).toBe("chave: «eletroposto»");
  });

  it("marca em nós irmãos independentes", () => {
    const tree = elemento(
      "div",
      elemento("h3", texto("Palavras Chave")),
      elemento("p", texto("sobre chave de acesso")),
    );
    expect(serializar(marcar(tree, "chave"))).toBe("Palavras «Chave»sobre «chave» de acesso");
  });

  it("NÃO marca dentro de <code> nem de <pre> (conteúdo literal)", () => {
    const tree = elemento(
      "p",
      texto("use icms aqui"),
      elemento("code", texto("icms")),
      elemento("pre", elemento("code", texto("icms"))),
    );
    expect(serializar(marcar(tree, "icms"))).toBe("use «icms» aquiicmsicms");
  });

  it("query vazia não altera a árvore", () => {
    const tree = elemento("p", texto("Crédito de ICMS"));
    const antes = JSON.stringify(tree);
    marcar(tree, "   ");
    expect(JSON.stringify(tree)).toBe(antes);
  });

  it("termo ausente não altera a árvore", () => {
    const tree = elemento("p", texto("Crédito de ICMS"));
    const antes = JSON.stringify(tree);
    marcar(tree, "inexistente");
    expect(JSON.stringify(tree)).toBe(antes);
  });

  it("marca vários termos e várias ocorrências", () => {
    const tree = elemento("p", texto("ICMS: crédito de ICMS na cesta"));
    expect(serializar(marcar(tree, "icms cesta"))).toBe("«ICMS»: crédito de «ICMS» na «cesta»");
  });

  it("o texto visível é preservado exatamente (nada some no recorte)", () => {
    const original = "Emissão de segunda via da Nota Fiscal — NF-e/NFC-e";
    const tree = elemento("p", texto(original));
    marcar(tree, "nota emissao");
    expect(serializar(tree).replace(/[«»]/g, "")).toBe(original);
  });
});
