import { describe, expect, it } from "vitest";
import {
  colunasDeTipos,
  contagem,
  formatarData,
  nomeDoEstado,
  resumoTipos,
  tipoLabel,
  totalDocumentos,
  zipHref,
  type DownloadEntry,
} from "./downloads";

const entrada = (
  estado: string,
  tipos: [string, number][],
  tamanho_bytes = 1000,
): DownloadEntry => ({
  estado,
  uf: estado.toUpperCase(),
  nome: `SEFAZ-${estado.toUpperCase()}`,
  arquivo: `${estado}.zip`,
  tamanho_bytes,
  tamanho: "1,0 KB",
  sha256: "abc",
  atualizado_em: "2026-07-29",
  gerado_em: "2026-07-29T00:00:00Z",
  tipos: tipos.map(([tipo, arquivos]) => ({ tipo, arquivos })),
});

describe("colunasDeTipos", () => {
  it("usa a ordem preferida, não a ordem do dado", () => {
    const entries = [entrada("rs", [["pareceres", 372], ["faqs", 1945], ["servicos", 586]])];
    expect(colunasDeTipos(entries)).toEqual(["servicos", "faqs", "pareceres"]);
  });

  it("une os tipos de todos os estados", () => {
    const entries = [
      entrada("ac", [["servicos", 17]]),
      entrada("rs", [["servicos", 586], ["faqs", 1945]]),
      entrada("sp", [["servicos", 518], ["pareceres", 15605]]),
    ];
    expect(colunasDeTipos(entries)).toEqual(["servicos", "faqs", "pareceres"]);
  });

  it("põe tipo desconhecido depois dos conhecidos, em ordem alfabética", () => {
    // O bundle é genérico: uma coleção nova no data/ vira coluna sem tocar neste arquivo.
    const entries = [entrada("rs", [["zetas", 1], ["servicos", 2], ["alfas", 3]])];
    expect(colunasDeTipos(entries)).toEqual(["servicos", "alfas", "zetas"]);
  });

  it("devolve vazio quando não há entrada nenhuma", () => {
    expect(colunasDeTipos([])).toEqual([]);
  });
});

describe("contagem", () => {
  const rs = entrada("rs", [["servicos", 586], ["faqs", 1945]]);

  it("devolve o número do tipo presente", () => {
    expect(contagem(rs, "servicos")).toBe(586);
  });

  it("devolve undefined para tipo ausente — a célula vira travessão, não zero", () => {
    expect(contagem(rs, "pareceres")).toBeUndefined();
  });
});

describe("totalDocumentos", () => {
  it("soma todos os tipos", () => {
    const rs = entrada("rs", [["servicos", 586], ["faqs", 1945], ["pareceres", 372]]);
    expect(totalDocumentos(rs)).toBe(2903);
  });

  it("é zero quando o pacote não tem tipo nenhum", () => {
    expect(totalDocumentos(entrada("xx", []))).toBe(0);
  });
});

describe("resumoTipos", () => {
  it("formata em pt-BR e na ordem das colunas", () => {
    const rs = entrada("rs", [["pareceres", 372], ["servicos", 586], ["faqs", 1945]]);
    expect(resumoTipos(rs)).toBe("586 serviços · 1.945 faqs · 372 pareceres");
  });
});

describe("nomeDoEstado", () => {
  it("resolve pelo ENTITIES", () => {
    expect(nomeDoEstado("rs")).toBe("Rio Grande do Sul");
  });

  it("cai na sigla quando o estado não está no ENTITIES", () => {
    expect(nomeDoEstado("zz")).toBe("ZZ");
  });
});

describe("zipHref", () => {
  it("faz cache-bust pelos 8 primeiros do sha256, não pela versão do app", () => {
    expect(zipHref({ arquivo: "rs.zip", sha256: "abcdef1234567890" })).toBe(
      "/downloads/rs.zip?v=abcdef12",
    );
  });

  it("muda a URL quando o conteúdo muda — é o que impede o CDN de servir zip velho", () => {
    const antes = zipHref({ arquivo: "rs.zip", sha256: "1111111122222222" });
    const depois = zipHref({ arquivo: "rs.zip", sha256: "3333333344444444" });
    expect(antes).not.toBe(depois);
  });
});

describe("formatarData", () => {
  it("converte ISO para pt-BR", () => {
    expect(formatarData("2026-07-29")).toBe("29/07/2026");
  });

  it("não volta um dia em fuso a oeste — o parse é manual, não via Date", () => {
    // `new Date("2026-01-01")` é meia-noite UTC, que em Brasília é 31/12 do ano anterior.
    expect(formatarData("2026-01-01")).toBe("01/01/2026");
  });

  it("devolve a string crua quando o formato não é o esperado", () => {
    expect(formatarData("ontem")).toBe("ontem");
  });
});

describe("tipoLabel", () => {
  it("traduz os conhecidos e devolve o próprio nome nos demais", () => {
    expect(tipoLabel("servicos")).toBe("Serviços");
    expect(tipoLabel("faqs")).toBe("FAQs");
    expect(tipoLabel("pareceres")).toBe("Pareceres");
    expect(tipoLabel("notas")).toBe("Notas");
    expect(tipoLabel("conteudos")).toBe("Conteúdos");
    expect(tipoLabel("outra-coisa")).toBe("outra-coisa");
  });
});
