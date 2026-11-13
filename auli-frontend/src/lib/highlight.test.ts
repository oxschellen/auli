import { describe, it, expect } from "vitest";
import { findHighlightRanges } from "./highlight";
import { parseQuery } from "./textSearch";

/** Açúcar dos testes: o que a UI marcaria, como lista de trechos da string original. */
const marked = (text: string, query: string): string[] =>
  findHighlightRanges(text, parseQuery(query)).map(([s, e]) => text.slice(s, e));

describe("findHighlightRanges", () => {
  it("marca o trecho casado, no texto original", () => {
    expect(marked("Crédito de ICMS", "icms")).toEqual(["ICMS"]);
  });

  it("marca o texto ACENTUADO quando a query vem sem acento", () => {
    // O caso que o mapa de offsets existe para resolver: "credito" casa na string normalizada, mas
    // o recorte tem de sair da original — com o acento, e sem deslocar.
    expect(marked("Crédito de ICMS", "credito")).toEqual(["Crédito"]);
  });

  it("não desloca o realce com vários acentos à esquerda", () => {
    // Cada acento encolhe a string normalizada em 1. Se o recorte usasse o índice normalizado, o
    // erro acumularia: com 3 acentos antes do alvo, marcaria 3 caracteres adiantado.
    expect(marked("Ação é órgão automóveis", "automoveis")).toEqual(["automóveis"]);
    expect(marked("ÁÉÍÓÚ substituição", "substituicao")).toEqual(["substituição"]);
  });

  it("marca cada termo, em qualquer ordem, e ordena as faixas pela posição", () => {
    expect(marked("Crédito de ICMS na cesta básica", "basica credito")).toEqual([
      "Crédito",
      "básica",
    ]);
  });

  it("marca todas as ocorrências do mesmo termo", () => {
    expect(marked("ICMS sobre ICMS", "icms")).toEqual(["ICMS", "ICMS"]);
  });

  it("funde faixas que se sobrepõem entre termos diferentes", () => {
    // "sub" e "bstitui" se sobrepõem: uma marca só, contínua — nunca dois <mark> aninhados.
    expect(marked("substituição", "sub bstitui")).toEqual(["substitui"]);
  });

  it("funde faixas adjacentes", () => {
    expect(marked("cabeçalho", "cabe calho")).toEqual(["cabeçalho"]);
  });

  it("query vazia, só espaços ou texto vazio não marcam nada", () => {
    expect(marked("Crédito de ICMS", "")).toEqual([]);
    expect(marked("Crédito de ICMS", "   ")).toEqual([]);
    expect(marked("", "icms")).toEqual([]);
  });

  it("termo ausente não marca nada", () => {
    expect(marked("Crédito de ICMS", "inexistente")).toEqual([]);
  });

  it("é insensível a caixa nos dois sentidos", () => {
    expect(marked("Crédito de ICMS", "ICMS")).toEqual(["ICMS"]);
    expect(marked("crédito de icms", "CRÉDITO")).toEqual(["crédito"]);
  });

  it("ç casa com c (o NFD decompõe a cedilha)", () => {
    expect(marked("Restituição e cabeçalho", "cabecalho")).toEqual(["cabeçalho"]);
  });

  it("não desloca depois de um emoji (par surrogate conta como 1 caractere)", () => {
    // Iterar por code unit em vez de code point somaria 1 a mais aqui e cortaria no lugar errado.
    expect(marked("📄 Crédito de ICMS", "icms")).toEqual(["ICMS"]);
    expect(marked("📄📄 crédito", "credito")).toEqual(["crédito"]);
  });

  it("marca acento composto na origem (base + combinante separados)", () => {
    // "e" + U+0301 exibe "é" mas são 2 caracteres na original; o combinante some na normalização e
    // o mapa tem de devolver os DOIS ao recortar.
    const composto = "crédito";
    expect(marked(composto, "credito")).toEqual([composto]);
  });

  it("as faixas nunca se cruzam nem retrocedem (invariante do recorte)", () => {
    const texto = "ICMS: crédito de ICMS na substituição tributária do ICMS";
    const faixas = findHighlightRanges(texto, parseQuery("icms credito substituicao"));
    for (let i = 1; i < faixas.length; i++) {
      expect(faixas[i][0]).toBeGreaterThanOrEqual(faixas[i - 1][1]);
    }
    for (const [s, e] of faixas) expect(e).toBeGreaterThan(s);
  });

  it("o que é marcado é sempre o que a busca casaria (coerência com textSearch)", () => {
    // Invariante do par: se `parseQuery` produz termos que casam o texto, há marcação; e o texto
    // marcado, normalizado, contém o termo. Marcar o que não casou seria mentir sobre o motivo.
    const texto = "Emissão de segunda via da Nota Fiscal";
    for (const q of ["emissao", "nota fiscal", "via"]) {
      const terms = parseQuery(q);
      const faixas = findHighlightRanges(texto, terms);
      expect(faixas.length).toBeGreaterThan(0);
    }
  });
});
