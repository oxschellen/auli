/**
 * Guardas dos dois predicados de política do Tributum. Eles são pequenos e é justamente por isso
 * que precisam de teste: cada um inverte um default, e o modo de falha dos dois é **silencioso e
 * público** — conteúdo de rascunho servido como curadoria (D-TRIB-7) ou PDF de terceiro servido do
 * nosso domínio (D-TRIB-8).
 *
 * O catálogo real entra junto, e aqui isso é seguro: `public/tributum.json` é autorado à mão e
 * **versionado** (mora na raiz de `public/`, que o `.gitignore` não pega). Diferente do
 * `textSearch.corpus.test.ts`, que precisa se pular no CI porque lê artefato gerado, este roda em
 * qualquer máquina.
 */
/// <reference types="node" />
import { describe, it, expect } from "vitest";
import { existsSync, readFileSync } from "node:fs";

import { ehPlaceholder, temPdfHospedado, type TributumCatalogo } from "./types";

describe("ehPlaceholder (D-TRIB-7)", () => {
  it("só publica com `false` explícito", () => {
    expect(ehPlaceholder({ id: "x", placeholder: false })).toBe(false);
  });

  it("trata `true` como exemplo", () => {
    expect(ehPlaceholder({ id: "x", placeholder: true })).toBe(true);
  });

  /**
   * O caso que dá razão ao predicado existir. Verificação por mutação: trocar o corpo por
   * `!!item.placeholder` mata este teste e deixa os outros dois verdes — é o único que separa o
   * fail-safe do default ingênuo.
   */
  it("trata campo AUSENTE como exemplo — é o fail-safe", () => {
    expect(ehPlaceholder({ id: "x" })).toBe(true);
  });
});

describe("temPdfHospedado (D-TRIB-8)", () => {
  it("aceita com os três campos: `hospedado`, `pdf` e `autorizacao`", () => {
    expect(
      temPdfHospedado({ hospedado: true, pdf: "/tributum/a.pdf", autorizacao: "Autor X, 2026-08" }),
    ).toBe(true);
  });

  /**
   * A guarda de direitos propriamente dita: `pdf` preenchido num item de terceiro **não** autoriza
   * servir o arquivo. Sem esta linha, um caminho copiado por engano para um item alheio abriria o
   * visualizador — que é exatamente o que a D-TRIB-8 proíbe.
   */
  it("recusa `pdf` sem `hospedado`", () => {
    expect(temPdfHospedado({ hospedado: false, pdf: "/tributum/a.pdf", autorizacao: "x" })).toBe(
      false,
    );
    expect(temPdfHospedado({ pdf: "/tributum/a.pdf", autorizacao: "x" })).toBe(false);
  });

  it("recusa `hospedado` sem `pdf`", () => {
    expect(temPdfHospedado({ hospedado: true, autorizacao: "x" })).toBe(false);
  });

  /**
   * D-TRIB-10: o critério editorial diz que a autorização "fica registrada". Sem o campo, o item
   * não é servido — degrada para link. Espaço em branco não conta como registro: um `" "` deixado
   * para calar a guarda é exatamente o que ela existe para pegar.
   */
  it("recusa hospedagem sem `autorizacao` registrada", () => {
    expect(temPdfHospedado({ hospedado: true, pdf: "/tributum/a.pdf" })).toBe(false);
    expect(temPdfHospedado({ hospedado: true, pdf: "/tributum/a.pdf", autorizacao: "   " })).toBe(
      false,
    );
  });
});

describe("public/tributum.json", () => {
  const catalogo = JSON.parse(
    readFileSync(new URL("../../public/tributum.json", import.meta.url), "utf-8"),
  ) as TributumCatalogo & { _leia_me?: string };

  it("tem as quatro seções, todas arrays", () => {
    for (const secao of ["artigos", "instituicoes", "dados", "analises"] as const) {
      expect(Array.isArray(catalogo[secao]), secao).toBe(true);
    }
  });

  it("todo item tem `id`, e os ids não colidem dentro da seção", () => {
    for (const secao of ["artigos", "instituicoes", "dados", "analises"] as const) {
      const ids = catalogo[secao].map((i) => i.id);
      expect(ids.every(Boolean), `${secao}: item sem id`).toBe(true);
      expect(new Set(ids).size, `${secao}: id repetido`).toBe(ids.length);
    }
  });

  /**
   * D-TRIB-8 no DADO, não só na UI: nenhum item pode apontar um `pdf` sem se declarar hospedado.
   * A UI já não mostraria o botão, mas um caminho desses no catálogo é um erro de curadoria que
   * vale pegar na revisão, e não na leitura de código.
   */
  it("nenhum item traz `pdf` sem `hospedado: true`", () => {
    const suspeitos = [...catalogo.artigos, ...catalogo.analises]
      .filter((i) => i.pdf && i.hospedado !== true)
      .map((i) => i.id);
    expect(suspeitos, "pdf sem hospedado:true").toEqual([]);
  });

  /**
   * D-TRIB-10, do lado do DADO: item hospedado sem procedência não passa na revisão. A UI já
   * degradaria para link, mas isso esconderia o erro em vez de mostrá-lo — aqui ele falha, com o
   * id na mensagem. Esta suíte roda no CI do frontend (`npm test`), então é gate de verdade.
   */
  it("todo item hospedado registra a `autorizacao`", () => {
    const semProcedencia = [...catalogo.artigos, ...catalogo.analises]
      .filter((i) => i.hospedado === true && !i.autorizacao?.trim())
      .map((i) => i.id);
    expect(semProcedencia, "hospedado sem autorizacao").toEqual([]);
  });

  /** O critério editorial exige DATA em todo item que é texto — a instituição é a exceção. */
  it("artigos, análises e dados têm data", () => {
    expect(catalogo.artigos.filter((a) => !a.ano).map((a) => a.id)).toEqual([]);
    expect(catalogo.analises.filter((a) => !a.ano).map((a) => a.id)).toEqual([]);
    expect(catalogo.dados.filter((d) => !d.atualizado_em).map((d) => d.id)).toEqual([]);
  });

  /** Todo `pdf` do catálogo aponta para um arquivo que existe em `tributum/`, na raiz do repo. */
  it("todo `pdf` hospedado existe no disco", () => {
    for (const item of [...catalogo.artigos, ...catalogo.analises]) {
      if (!item.pdf) continue;
      const nome = item.pdf.replace(/^\/tributum\//, "");
      const caminho = new URL(`../../../tributum/${nome}`, import.meta.url);
      expect(existsSync(caminho), `${item.id}: ${item.pdf} não existe em tributum/`).toBe(true);
    }
  });

  /**
   * O critério editorial promete nome e contato no rodapé de CADA estante. O rodapé some quando a
   * estante não declara curador, o que é o desenho certo (nome não se inventa) — mas silencioso.
   * Este teste é o que torna a lacuna visível: se uma estante ficar sem curador, ele nomeia qual.
   */
  it("as quatro estantes declaram curador", () => {
    const semCurador = (["artigos", "instituicoes", "dados", "analises"] as const).filter(
      (e) => !catalogo.curadores?.[e]?.nome?.trim(),
    );
    expect(semCurador, "estante sem curador declarado").toEqual([]);
  });

  /** A regra de direitos viaja com o arquivo — quem abrir o JSON para editar lê antes de colar. */
  it("carrega a regra de direitos no `_leia_me`", () => {
    expect(catalogo._leia_me).toContain("D-TRIB-8");
  });
});
