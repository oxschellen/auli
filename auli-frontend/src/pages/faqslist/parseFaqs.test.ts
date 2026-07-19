import { describe, it, expect } from "vitest";
import {
  buildNodesFromJson,
  searchNodes,
  getEffectiveUrl,
  buildAnswerMap,
  buildPageTypeMap,
} from "./parseFaqs";

const sampleJson = {
  children: [
    {
      title: "Impostos",
      url: "https://portal/impostos/",
      page_type: "Menu",
      children: [
        {
          title: "ICMS",
          url: "https://portal/icms",
          page_type: "Faq",
          faq_items: [
            { pergunta: "O que é ICMS?", resposta: "É um imposto estadual." },
            { pergunta: "Como pagar ICMS?", resposta: "Via guia." },
          ],
        },
      ],
    },
    {
      title: "Contato",
      url: "https://portal/contato",
      page_type: "Geral",
      children: [],
    },
  ],
};

describe("buildNodesFromJson", () => {
  it("maps top-level children into normalized nodes", () => {
    const nodes = buildNodesFromJson(sampleJson);
    expect(nodes).toHaveLength(2);
    expect(nodes[0].text).toBe("Impostos");
    expect(nodes[1].text).toBe("Contato");
  });

  it("turns faq_items into leaf children carrying the parent url", () => {
    const nodes = buildNodesFromJson(sampleJson);
    const icms = nodes[0].children[0];
    expect(icms.text).toBe("ICMS");
    expect(icms.children.map((c) => c.text)).toEqual([
      "O que é ICMS?",
      "Como pagar ICMS?",
    ]);
    expect(icms.children[0].url).toBe("https://portal/icms");
  });

  it("assigns stable, unique ids and resets the counter per build", () => {
    const first = buildNodesFromJson(sampleJson);
    const second = buildNodesFromJson(sampleJson);
    expect(first[0].id).toBe(second[0].id);

    const ids = new Set<string>();
    const collect = (n: { id: string; children: typeof n[] }) => {
      ids.add(n.id);
      n.children.forEach(collect);
    };
    first.forEach(collect);
    // Impostos + Contato (top-level) + ICMS + its 2 faq leaves = 5 nodes total
    expect(ids.size).toBe(5);
  });

  it("defaults a missing title to an empty string", () => {
    const nodes = buildNodesFromJson({ children: [{ url: "x" }] });
    expect(nodes[0].text).toBe("");
  });
});

describe("searchNodes", () => {
  it("finds nodes whose text matches, case-insensitively", () => {
    const nodes = buildNodesFromJson(sampleJson);
    const hits = searchNodes(nodes, "icms");
    expect(hits.map((h) => h.node.text)).toEqual([
      "ICMS",
      "O que é ICMS?",
      "Como pagar ICMS?",
    ]);
  });

  it("records the ancestor chain for each hit", () => {
    const nodes = buildNodesFromJson(sampleJson);
    const hit = searchNodes(nodes, "O que é").find((h) => h.node.text === "O que é ICMS?");
    expect(hit?.ancestors.map((a) => a.text)).toEqual(["Impostos", "ICMS"]);
  });

  it("returns nothing for a non-matching query", () => {
    const nodes = buildNodesFromJson(sampleJson);
    expect(searchNodes(nodes, "zzz")).toHaveLength(0);
  });

  it("é insensível a acento (query sem acento acha texto acentuado)", () => {
    // "É um imposto…" não é texto de nó, mas as perguntas têm acento em "é".
    const nodes = buildNodesFromJson(sampleJson);
    const hit = searchNodes(nodes, "o que e").find((h) => h.node.text === "O que é ICMS?");
    expect(hit).toBeTruthy();
  });

  it("multi-termo com E lógico: todos os termos precisam aparecer", () => {
    const nodes = buildNodesFromJson(sampleJson);
    expect(searchNodes(nodes, "pagar icms").map((h) => h.node.text)).toEqual(["Como pagar ICMS?"]);
    expect(searchNodes(nodes, "pagar zzz")).toHaveLength(0);
  });
});

describe("getEffectiveUrl", () => {
  it("prefers the node's own url", () => {
    const node = { id: "1", text: "x", url: "https://own", children: [] };
    expect(getEffectiveUrl(node, [])).toBe("https://own");
  });

  it("falls back to the nearest ancestor url", () => {
    const node = { id: "2", text: "x", url: null, children: [] };
    const ancestors = [
      { id: "a", text: "root", url: "https://root", children: [] },
      { id: "b", text: "mid", url: null, children: [] },
    ];
    expect(getEffectiveUrl(node, ancestors)).toBe("https://root");
  });

  it("returns null when nothing has a url", () => {
    const node = { id: "3", text: "x", url: null, children: [] };
    expect(getEffectiveUrl(node, [{ id: "a", text: "r", url: null, children: [] }])).toBeNull();
  });
});

describe("buildAnswerMap", () => {
  it("indexes answers by trimmed, lowercased question", () => {
    const map = buildAnswerMap(sampleJson);
    expect(map.get("o que é icms?")).toBe("É um imposto estadual.");
    expect(map.get("como pagar icms?")).toBe("Via guia.");
  });
});

describe("buildPageTypeMap", () => {
  it("indexes page_type by url with the trailing slash stripped", () => {
    const map = buildPageTypeMap(sampleJson);
    expect(map.get("https://portal/impostos")).toBe("Menu");
    expect(map.get("https://portal/icms")).toBe("Faq");
    expect(map.get("https://portal/contato")).toBe("Geral");
  });
});
