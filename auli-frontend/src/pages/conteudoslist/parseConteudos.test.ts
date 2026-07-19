import { describe, it, expect } from "vitest";
import { searchCategories, type ConteudoCategory } from "./parseConteudos";

const categories: ConteudoCategory[] = [
  {
    name: "Manuais",
    items: [
      { title: "Manual do ICMS", url: "a", type: "pdf" },
      { title: "Guia rápido", url: "b", type: "pdf" },
    ],
  },
  {
    name: "Vídeos",
    items: [{ title: "Como emitir nota", url: "c", type: "video" }],
  },
];

describe("searchCategories", () => {
  it("returns all categories unchanged for an empty query", () => {
    expect(searchCategories(categories, "")).toBe(categories);
    expect(searchCategories(categories, "   ")).toBe(categories);
  });

  it("keeps only items whose title matches, dropping empty categories", () => {
    const result = searchCategories(categories, "icms");
    expect(result).toHaveLength(1);
    expect(result[0].name).toBe("Manuais");
    expect(result[0].items.map((i) => i.title)).toEqual(["Manual do ICMS"]);
  });

  it("matches case-insensitively", () => {
    const result = searchCategories(categories, "NOTA");
    expect(result).toHaveLength(1);
    expect(result[0].name).toBe("Vídeos");
  });

  it("does not mutate the source categories", () => {
    searchCategories(categories, "guia");
    expect(categories[0].items).toHaveLength(2);
  });

  it("returns an empty array when nothing matches", () => {
    expect(searchCategories(categories, "zzz")).toEqual([]);
  });

  it("é insensível a acento (query sem acento acha item acentuado)", () => {
    const cats: ConteudoCategory[] = [
      { name: "Manuais", items: [{ title: "Substituição Tributária", url: "a" }] },
    ];
    const r = searchCategories(cats, "substituicao");
    expect(r).toHaveLength(1);
    expect(r[0].items.map((i) => i.title)).toEqual(["Substituição Tributária"]);
  });

  it("multi-termo com E lógico, ordem irrelevante", () => {
    // "Manual do ICMS" tem os dois termos; ordem não importa.
    expect(searchCategories(categories, "manual icms")[0].items.map((i) => i.title)).toEqual([
      "Manual do ICMS",
    ]);
    expect(searchCategories(categories, "icms manual")[0].items.map((i) => i.title)).toEqual([
      "Manual do ICMS",
    ]);
    // termo ausente derruba
    expect(searchCategories(categories, "manual zzz")).toEqual([]);
  });
});
