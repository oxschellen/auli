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
});
