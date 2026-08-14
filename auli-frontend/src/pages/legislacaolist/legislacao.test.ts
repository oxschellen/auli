import { describe, it, expect } from "vitest";
import {
  agruparPorLei,
  buildLegislacaoIndex,
  caminhoNaLei,
  leiDe,
  searchLegislacao,
  type Dispositivo,
} from "./legislacao";

const LEI = "Lei 6.537-1973 — Processo Tributário Administrativo";

const d = (ementa: string, titulo: string, trilha = `${LEI} | Título I`): Dispositivo => ({
  titulo,
  trilha,
  ementa,
  link: "http://legislacao.sefaz.rs.gov.br/x",
});

describe("leiDe / caminhoNaLei", () => {
  it("parte a trilha no primeiro separador", () => {
    // O 1º segmento é a lei E o nome da subpasta na árvore — é o contrato do D-LEG-4, e o mesmo
    // que o `lei_da_trilha` do contrato em Rust aplica do outro lado.
    expect(leiDe(`${LEI} | Título II | Capítulo I`)).toBe(LEI);
    expect(caminhoNaLei(`${LEI} | Título II | Capítulo I`)).toBe("Título II | Capítulo I");
  });

  it("trilha sem separador é toda ela a lei, e o caminho fica vazio", () => {
    expect(leiDe(LEI)).toBe(LEI);
    expect(caminhoNaLei(LEI)).toBe("");
  });

  it("o separador é ` | ` com espaços — pipe colado é texto do segmento", () => {
    expect(leiDe("Lei A|B | Título I")).toBe("Lei A|B");
  });
});

describe("agruparPorLei", () => {
  it("agrupa preservando a ordem de chegada, que é a do índice derivado", () => {
    // O derivador já ordenou (leis alfabéticas, dispositivos na ordem do texto legal — D-LEG-12).
    // Reordenar aqui desfaria o trabalho dele, que é onde a ordem é decidida e testada.
    const itens = [
      d("Art. 1º", "Primeira?"),
      d("Art. 17-A", "Segunda?"),
      d("Art. 5º", "Outra lei?", "Lei 8.820-1989 — ICMS | Título I"),
      d("Art. 28", "Terceira?"),
    ];
    const grupos = agruparPorLei(itens);
    expect(grupos.map(([lei]) => lei)).toEqual([LEI, "Lei 8.820-1989 — ICMS"]);
    expect(grupos[0][1].map((x) => x.ementa)).toEqual(["Art. 1º", "Art. 17-A", "Art. 28"]);
    expect(grupos[1][1]).toHaveLength(1);
  });

  it("lista vazia não vira grupo nenhum", () => {
    expect(agruparPorLei([])).toEqual([]);
  });
});

describe("searchLegislacao", () => {
  const itens = [
    d("Art. 28", "Qual o prazo e a forma da impugnação ao Auto de Lançamento?", `${LEI} | Título II — Do Procedimento Tributário Administrativo | Capítulo II — Do Processo Contencioso`),
    d("Art. 11, § 3º", "Existe teto para as multas por infração formal?", `${LEI} | Título I — Das Infrações à Legislação Tributária | Capítulo III — Das Infrações Formais`),
    d("Art. 96", "Como é composto o TARF?", `${LEI} | Título III — Do Tribunal Administrativo de Recursos Fiscais`),
  ];

  it("acha pela pergunta, ignorando acento e caixa", () => {
    expect(searchLegislacao(itens, "IMPUGNACAO").map((x) => x.ementa)).toEqual(["Art. 28"]);
  });

  it("acha pelo dispositivo — é como o analista busca", () => {
    expect(searchLegislacao(itens, "art. 11").map((x) => x.titulo)).toEqual([
      "Existe teto para as multas por infração formal?",
    ]);
  });

  it("acha pela trilha: o vocabulário estrutural da lei não está na pergunta", () => {
    // "infrações formais" é o nome do capítulo; a pergunta diz "infração formal", no singular.
    expect(searchLegislacao(itens, "infrações formais").map((x) => x.ementa)).toEqual([
      "Art. 11, § 3º",
    ]);
  });

  it("multi-termo é E entre os termos", () => {
    expect(searchLegislacao(itens, "prazo impugnação")).toHaveLength(1);
    expect(searchLegislacao(itens, "prazo tarf")).toHaveLength(0);
  });

  it("query vazia devolve tudo", () => {
    expect(searchLegislacao(itens, "   ")).toHaveLength(3);
  });

  it("o índice pré-normalizado dá o MESMO resultado que o caminho sem índice", () => {
    // A trava do atalho de desempenho: se as duas formas divergirem, a busca da tela deixa de ser
    // a busca testada.
    const index = buildLegislacaoIndex(itens);
    for (const q of ["impugnacao", "art. 11", "infrações formais", "tarf"]) {
      expect(searchLegislacao(itens, q, index)).toEqual(searchLegislacao(itens, q));
    }
  });
});
