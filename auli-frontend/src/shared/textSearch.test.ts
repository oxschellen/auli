import { describe, it, expect } from "vitest";
import {
  normalizeText,
  buildHaystack,
  parseQuery,
  haystackMatches,
  matchesQuery,
} from "./textSearch";

describe("normalizeText", () => {
  it("remove acentos, ç e caixa", () => {
    expect(normalizeText("Crédito de ICMS")).toBe("credito de icms");
    expect(normalizeText("Cabeçalho")).toBe("cabecalho");
    expect(normalizeText("SUBSTITUIÇÃO Tributária")).toBe("substituicao tributaria");
  });

  it("colapsa whitespace e apara as pontas", () => {
    expect(normalizeText("  crédito \t presumido \n exportação  ")).toBe(
      "credito presumido exportacao",
    );
    expect(normalizeText("   ")).toBe("");
  });
});

describe("parseQuery", () => {
  it("query vazia ou só espaços vira lista vazia", () => {
    expect(parseQuery("")).toEqual([]);
    expect(parseQuery("   \t ")).toEqual([]);
  });

  it("quebra em termos normalizados", () => {
    expect(parseQuery(" Crédito  Presumido ")).toEqual(["credito", "presumido"]);
  });
});

describe("matchesQuery", () => {
  const item = ["ICMS – Crédito presumido nas exportações", "Substituição Tributária"];

  it("acento na query e não no texto, e vice-versa", () => {
    expect(matchesQuery(["Crédito de ICMS"], "credito")).toBe(true);
    expect(matchesQuery(["credito de icms"], "crédito")).toBe(true);
    expect(matchesQuery(["Consultoria"], "consultoría")).toBe(true);
  });

  it("ç e c nos dois sentidos", () => {
    expect(matchesQuery(["Cabeçalho"], "cabecalho")).toBe(true);
    expect(matchesQuery(["cabecalho"], "cabeçalho")).toBe(true);
  });

  it("é insensível a caixa", () => {
    expect(matchesQuery(["crédito PRESUMIDO"], "CRÉDITO presumido")).toBe(true);
  });

  it("dois termos casando em campos diferentes do mesmo item casam", () => {
    expect(matchesQuery(item, "presumido substituição")).toBe(true);
  });

  it("E lógico: termo ausente derruba o item", () => {
    expect(matchesQuery(item, "presumido importação")).toBe(false);
  });

  it("ordem dos termos é irrelevante", () => {
    expect(matchesQuery(item, "presumido credito")).toBe(
      matchesQuery(item, "credito presumido"),
    );
    expect(matchesQuery(item, "presumido credito")).toBe(true);
  });

  it("query vazia ou só espaços casa tudo", () => {
    expect(matchesQuery(item, "")).toBe(true);
    expect(matchesQuery(item, "   ")).toBe(true);
    expect(matchesQuery([""], "")).toBe(true);
  });

  it("campo vazio, null ou undefined não quebra nem casa termo não-vazio", () => {
    expect(matchesQuery(["", null, undefined], "credito")).toBe(false);
    expect(matchesQuery(["", "Crédito"], "credito")).toBe(true);
  });
});

describe("haystackMatches + buildHaystack (caminho memoizado)", () => {
  it("equivale ao matchesQuery", () => {
    const fields = ["PARECER Nº 26164", "ICMS – recarga de energia em automóveis elétricos"];
    const haystack = buildHaystack(fields);
    for (const q of ["26164", "recarga elétricos", "energia automoveis", "diferimento"]) {
      expect(haystackMatches(haystack, parseQuery(q))).toBe(matchesQuery(fields, q));
    }
  });

  it("terms vazio casa qualquer haystack", () => {
    expect(haystackMatches(buildHaystack(["qualquer coisa"]), [])).toBe(true);
    expect(haystackMatches(buildHaystack([]), [])).toBe(true);
  });
});

describe("invariante de superset (guarda de não-regressão)", () => {
  // Amostra de corpus + queries de UM termo: todo item aceito pelo filtro antigo
  // (`toLowerCase().includes`) tem de ser aceito pelo novo. Sempre dinâmico sobre a amostra —
  // nunca uma lista hardcoded de esperados.
  const corpus: string[][] = [
    ["ICMS – Crédito presumido nas exportações"],
    ["Substituição Tributária", "Autopeças"],
    ["IPVA – Isenção para PCD"],
    ["ITCD – doação de imóvel rural"],
    ["Nota Fiscal Eletrônica", "cancelamento fora do prazo"],
    ["Cesta básica", "crédito fiscal"],
  ];
  const queries = ["credito", "Crédito", "ICMS", "substituição", "cesta", "prazo", "imóvel"];

  function filtroAntigo(fields: string[], q: string): boolean {
    const query = q.trim().toLowerCase();
    if (!query) return true;
    return fields.some((f) => f.toLowerCase().includes(query));
  }

  it("nenhum resultado do filtro antigo se perde", () => {
    for (const q of queries) {
      for (const fields of corpus) {
        if (filtroAntigo(fields, q)) {
          expect(matchesQuery(fields, q), `query=${JSON.stringify(q)} fields=${fields[0]}`).toBe(
            true,
          );
        }
      }
    }
  });
});
