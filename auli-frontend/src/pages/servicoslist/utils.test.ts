import { describe, it, expect } from "vitest";
import {
  contarServicosDistintos,
  empresaPrimeiro,
  filterServicoGroups,
  getDefaultTipoServicos,
  type Servico,
  type ServicoGroup,
  type TipoServico,
} from "./utils";

function svc(id: number, classe: string, titulo: string): Servico {
  return { id, classe, titulo, link: `http://x/${id}` };
}

const grouped: ServicoGroup[] = [
  [
    "Crédito Fiscal",
    [svc(1, "Crédito Fiscal", "Apropriação de crédito"), svc(2, "Crédito Fiscal", "Estorno de crédito")],
  ],
  [
    "Substituição Tributária",
    [svc(3, "Substituição Tributária", "ST de autopeças"), svc(4, "Substituição Tributária", "ST de bebidas")],
  ],
];

describe("filterServicoGroups", () => {
  it("query vazia ou só espaços devolve os grupos intactos (mesma referência)", () => {
    expect(filterServicoGroups(grouped, "")).toBe(grouped);
    expect(filterServicoGroups(grouped, "   ")).toBe(grouped);
  });

  it("classe casa → grupo inteiro entra, sem filtrar itens", () => {
    const r = filterServicoGroups(grouped, "credito");
    expect(r).toHaveLength(1);
    expect(r[0][0]).toBe("Crédito Fiscal");
    expect(r[0][1]).toHaveLength(2); // nada filtrado dentro do grupo
  });

  it("classe não casa → filtra itens pelo título", () => {
    const r = filterServicoGroups(grouped, "autopecas");
    expect(r).toHaveLength(1);
    expect(r[0][0]).toBe("Substituição Tributária");
    expect(r[0][1].map((s) => s.titulo)).toEqual(["ST de autopeças"]);
  });

  it("acento + multi-termo dentro do título (E lógico, sem acento na query)", () => {
    // Ambos os termos no mesmo título "ST de autopeças"; "autopecas" casa "autopeças".
    const r = filterServicoGroups(grouped, "st autopecas");
    expect(r).toHaveLength(1);
    expect(r[0][1].map((s) => s.titulo)).toEqual(["ST de autopeças"]);
  });

  it("termo do título isolado não casa via classe (regra preservada)", () => {
    // "substituição" só está na classe; "bebidas" só no título — nenhum campo tem os dois.
    expect(filterServicoGroups(grouped, "substituicao bebidas")).toEqual([]);
  });

  it("termo ausente derruba tudo", () => {
    expect(filterServicoGroups(grouped, "credito inexistente")).toEqual([]);
  });
});

/** Abas a partir dos rótulos; o `filename` não participa da ordenação, só acompanha. */
function abas(...tipos: string[]): TipoServico[] {
  return tipos.map((tipo) => ({ tipo, filename: `f-${tipo}` }));
}

const rotulos = (t: TipoServico[]) => t.map((x) => x.tipo);

describe("empresaPrimeiro", () => {
  it("traz Empresa para a frente preservando a ordem das demais", () => {
    // O índice real do `sp`.
    const r = empresaPrimeiro(abas("Cidadão", "Empresa", "Servidor Público", "Tributos"));
    expect(rotulos(r)).toEqual(["Empresa", "Cidadão", "Servidor Público", "Tributos"]);
  });

  it("preserva a cauda longa intacta — o caso do `pr`, com 7 abas", () => {
    const r = empresaPrimeiro(
      abas("Cidadão", "Empresa", "Município", "Produtor rural", "Receita/PR", "Programas", "Legislação"),
    );
    expect(rotulos(r)).toEqual([
      "Empresa",
      "Cidadão",
      "Município",
      "Produtor rural",
      "Receita/PR",
      "Programas",
      "Legislação",
    ]);
  });

  it("reconhece o plural `Empresas` — mg / pe / rs", () => {
    const r = empresaPrimeiro(abas("Cidadãos", "Empresas", "Fornecedores", "Agentes", "Servidores"));
    expect(rotulos(r)).toEqual(["Empresas", "Cidadãos", "Fornecedores", "Agentes", "Servidores"]);
  });

  it("reconhece `Pessoa Jurídica` — o rótulo do `am` para a mesma audiência", () => {
    const r = empresaPrimeiro(abas("Pessoa Física", "Pessoa Jurídica", "Órgãos Públicos"));
    expect(rotulos(r)).toEqual(["Pessoa Jurídica", "Pessoa Física", "Órgãos Públicos"]);
  });

  it("é idempotente e no-op quando já começa com empresa — al / ma", () => {
    const entrada = abas("Empresa", "Cidadão", "Órgão Público", "Certidões");
    const uma = empresaPrimeiro(entrada);
    expect(rotulos(uma)).toEqual(rotulos(entrada));
    expect(rotulos(empresaPrimeiro(uma))).toEqual(rotulos(entrada));
  });

  it("não altera entidade sem aba de empresa — os 10 de público único", () => {
    expect(rotulos(empresaPrimeiro(abas("Serviços")))).toEqual(["Serviços"]);
    expect(rotulos(empresaPrimeiro(abas("Cidadão", "Contabilista", "Poder Público")))).toEqual([
      "Cidadão",
      "Contabilista",
      "Poder Público",
    ]);
  });

  it("não casa por substring — 'Apoio Empresarial' não é a aba de empresa", () => {
    // A lista é de rótulos EXATOS: um /empresa/i pegaria isto por acidente.
    expect(rotulos(empresaPrimeiro(abas("Cidadão", "Apoio Empresarial")))).toEqual([
      "Cidadão",
      "Apoio Empresarial",
    ]);
  });

  it("aguenta array vazio e não muta a entrada", () => {
    expect(empresaPrimeiro([])).toEqual([]);
    const entrada = abas("Cidadão", "Empresa");
    const copia = rotulos(entrada);
    empresaPrimeiro(entrada);
    expect(rotulos(entrada)).toEqual(copia);
  });

  it("o fallback sem índice também abre em Empresas", () => {
    // getDefaultTipoServicos mantém a ordem do DADO (Cidadãos first); quem exibe passa por aqui.
    expect(rotulos(empresaPrimeiro(getDefaultTipoServicos()))[0]).toBe("Empresas");
  });
});

describe("contarServicosDistintos", () => {
  function s(titulo: string, link: string): Servico {
    return { id: `${titulo}|${link}`, classe: "Geral", titulo, link };
  }

  it("não conta duas vezes o serviço que atende mais de um público", () => {
    const cidadao = [s("Consulta de débitos", "http://x/deb"), s("Emissão de guia", "http://x/guia")];
    const empresa = [s("Consulta de débitos", "http://x/deb"), s("Credenciamento", "http://x/cred")];
    expect(contarServicosDistintos([cidadao, empresa])).toBe(3);
  });

  it("distingue serviços que compartilham a URL — o caso do SP", () => {
    // 30 links do SP servem mais de um serviço: a página do CADIN é a mesma para "Logon",
    // "Consulta Inscritos" e "Consulta Comunicados". Contar por link daria 1 para 3 documentos.
    const aba = [
      s("Logon", "http://x/cadin"),
      s("Consulta Inscritos CADIN", "http://x/cadin"),
      s("Consulta Comunicados", "http://x/cadin"),
    ];
    expect(contarServicosDistintos([aba])).toBe(3);
  });

  it("não junta pares diferentes que colidiriam se o separador fosse espaço", () => {
    const aba = [s("A B", "C"), s("A", "B C")];
    expect(contarServicosDistintos([aba])).toBe(2);
  });

  it("devolve 0 sem abas e ignora aba vazia", () => {
    expect(contarServicosDistintos([])).toBe(0);
    expect(contarServicosDistintos([[], [s("Único", "http://x/1")]])).toBe(1);
  });
});
