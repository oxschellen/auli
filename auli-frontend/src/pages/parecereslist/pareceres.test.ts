import { describe, it, expect } from "vitest";
import { buildPareceresIndex, searchPareceres, type Parecer } from "./pareceres";

/** Duas entradas no formato do `<id>-pareceres-index.json` (derivado da árvore `.md`). */
const PS: Parecer[] = [
  {
    numero: "PARECER Nº 26164",
    assunto: "ICMS – recarga de energia em automóveis elétricos.",
    resumo:
      "### Descrição Resumida do Assunto\nTrata da incidência na recarga.\n\n### Palavras Chave do Tema\n- **eletroposto**\n- **ICMS**",
    link: "http://legislacao/299748",
  },
  {
    numero: "PARECER Nº 25148",
    assunto: "ICMS – crédito fiscal na cesta básica.",
    resumo:
      "### Descrição Resumida do Assunto\nTrata do creditamento.\n\n### Palavras Chave do Tema\n- **cesta básica**\n- **ICMS**",
    link: "http://legislacao/297905",
  },
];

describe("searchPareceres", () => {
  it("busca por numero e assunto (insensível a caixa)", () => {
    expect(searchPareceres(PS, "cesta")).toHaveLength(1);
    expect(searchPareceres(PS, "26164")).toHaveLength(1);
    expect(searchPareceres(PS, "ICMS")).toHaveLength(2);
    expect(searchPareceres(PS, "")).toHaveLength(2);
  });

  it("busca também no resumo — o ganho da migração para o índice leve", () => {
    // "eletroposto" só existe nas palavras-chave da sinopse: nem o número nem a ementa o trazem.
    // Antes da migração o campo não chegava ao navegador e esta query voltava vazia.
    expect(searchPareceres(PS, "eletroposto")).toHaveLength(1);
    expect(searchPareceres(PS, "creditamento")).toHaveLength(1);
  });

  it("é insensível a acento nos dois sentidos", () => {
    expect(searchPareceres(PS, "automoveis")).toHaveLength(1);
    expect(searchPareceres(PS, "eletricos")).toHaveLength(1);
    expect(searchPareceres(PS, "parecer nº 26164")).toHaveLength(1);
  });

  it("multi-termo com E lógico, ordem irrelevante, cruzando os campos", () => {
    // "26164" está no numero; "recarga" no assunto; "eletroposto" no resumo — três campos.
    expect(searchPareceres(PS, "26164 recarga")).toHaveLength(1);
    expect(searchPareceres(PS, "recarga 26164")).toHaveLength(1);
    expect(searchPareceres(PS, "25148 creditamento")).toHaveLength(1);
    // Termo ausente derruba.
    expect(searchPareceres(PS, "26164 inexistente")).toHaveLength(0);
    // Dois termos que existem, mas em pareceres DIFERENTES: nenhum item tem os dois.
    expect(searchPareceres(PS, "eletroposto cesta")).toHaveLength(0);
  });

  it("query vazia ou só espaços devolve a lista inteira (mesma referência)", () => {
    expect(searchPareceres(PS, "   ")).toBe(PS);
  });

  it("documento pendente (resumo vazio) continua buscável por numero e assunto", () => {
    const pendente: Parecer[] = [{ ...PS[0], resumo: "" }];
    expect(searchPareceres(pendente, "recarga")).toHaveLength(1);
    expect(searchPareceres(pendente, "eletroposto")).toHaveLength(0);
  });

  it("o índice pré-computado não muda NENHUM resultado (invariante da otimização)", () => {
    // O índice existe só por custo: normalizar o resumo a cada tecla custava ~400 ms no acervo do
    // SP. Se ele mudasse algum resultado, seria bug — então as duas formas têm de coincidir sempre.
    const index = buildPareceresIndex(PS);
    for (const q of ["", "  ", "icms", "eletroposto", "credito basica", "26164 recarga", "xyz"]) {
      expect(searchPareceres(PS, q, index)).toEqual(searchPareceres(PS, q));
    }
  });

  it("termos de 1–2 letras casam largo (característica do substring-AND)", () => {
    // "e"/"o" são substrings de quase tudo, então uma query só com termos curtos não discrimina.
    // É inerente ao desenho (E lógico + substring por termo), não um bug — documentado para que a
    // escolha seja consciente se um dia virar incômodo.
    expect(searchPareceres(PS, "e o")).toHaveLength(2);
  });
});
