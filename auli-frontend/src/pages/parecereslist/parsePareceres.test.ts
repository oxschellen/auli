import { describe, it, expect } from "vitest";
import { parsePareceres, searchPareceres } from "./parsePareceres";

const SAMPLE = `// 1
## pergunta:
descricao: PARECER Nº 26164
assunto  : ICMS – recarga de energia em automóveis elétricos.
link: http://legislacao/299748
## resposta:
PARECER Nº 26164

É o parecer.

// 2
## pergunta:
descricao: PARECER Nº 25148
assunto  : ICMS – crédito fiscal na cesta básica.
link: http://legislacao/297905
## resposta:
Corpo do 25148.
`;

describe("parsePareceres", () => {
  it("parses blocks into numero/assunto/link/corpo", () => {
    const ps = parsePareceres(SAMPLE);
    expect(ps).toHaveLength(2);
    expect(ps[0]).toMatchObject({
      id: 0,
      numero: "PARECER Nº 26164",
      assunto: "ICMS – recarga de energia em automóveis elétricos.",
      link: "http://legislacao/299748",
    });
    expect(ps[0].corpo).toBe("PARECER Nº 26164\n\nÉ o parecer.");
    expect(ps[1].numero).toBe("PARECER Nº 25148");
    expect(ps[1].corpo).toBe("Corpo do 25148.");
  });

  it("ignores empty input and content-less blocks", () => {
    expect(parsePareceres("")).toHaveLength(0);
    expect(parsePareceres("// 1\n## pergunta:\ndescricao:\n## resposta:\n")).toHaveLength(0);
  });

  it("searches by numero and assunto (case-insensitive)", () => {
    const ps = parsePareceres(SAMPLE);
    expect(searchPareceres(ps, "cesta")).toHaveLength(1);
    expect(searchPareceres(ps, "26164")).toHaveLength(1);
    expect(searchPareceres(ps, "ICMS")).toHaveLength(2);
    expect(searchPareceres(ps, "")).toHaveLength(2);
  });

  it("é insensível a acento nos dois sentidos", () => {
    const ps = parsePareceres(SAMPLE);
    // "automoveis" acha "automóveis"; "eletricos" acha "elétricos".
    expect(searchPareceres(ps, "automoveis")).toHaveLength(1);
    expect(searchPareceres(ps, "eletricos")).toHaveLength(1);
    // E o inverso: query acentuada achando texto sem acento no número.
    expect(searchPareceres(ps, "parecer nº 26164")).toHaveLength(1);
  });

  it("multi-termo com E lógico, ordem irrelevante, cruzando numero e assunto", () => {
    const ps = parsePareceres(SAMPLE);
    // "26164" está no numero; "recarga" no assunto — termos casam em campos diferentes.
    expect(searchPareceres(ps, "26164 recarga")).toHaveLength(1);
    expect(searchPareceres(ps, "recarga 26164")).toHaveLength(1);
    // Termo ausente derruba.
    expect(searchPareceres(ps, "26164 inexistente")).toHaveLength(0);
    // Dois termos que existem, mas em pareceres DIFERENTES: nenhum item tem os dois.
    expect(searchPareceres(ps, "recarga cesta")).toHaveLength(0);
  });

  it("query vazia ou só espaços devolve a lista inteira (mesma referência)", () => {
    const ps = parsePareceres(SAMPLE);
    expect(searchPareceres(ps, "   ")).toBe(ps);
  });

  it("não busca no corpo (só numero e assunto)", () => {
    const ps = parsePareceres(SAMPLE);
    // "corpo" só aparece no corpo do 25148 ("Corpo do 25148."), em nenhum numero/assunto.
    expect(searchPareceres(ps, "corpo")).toHaveLength(0);
  });

  it("termos de 1–2 letras casam largo (característica do substring-AND)", () => {
    const ps = parsePareceres(SAMPLE);
    // "e"/"o" são substrings de quase tudo ("parecer", "nº"), então uma query só com termos
    // curtos não discrimina. É inerente ao desenho (E lógico + substring por termo), não um bug —
    // documentado aqui para que a escolha seja consciente se um dia virar incômodo.
    expect(searchPareceres(ps, "e o")).toHaveLength(2);
  });
});
