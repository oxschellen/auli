// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useQuestionType, tiposDisponiveis } from "./useQuestionType";
import type { Entity } from "../../../shared/entities";

const STORAGE_KEY = "auli.questionType";

/** Entidade sintética: as coleções são o que importa aqui, o resto é preenchimento. */
const entidade = (collections: Entity["collections"]): Entity => ({
  id: "xx",
  name: "SEFAZ-XX",
  uf: "XX",
  state: "Estado",
  collections,
});

/** O acervo completo de hoje — o do RS, única entidade com TARF e com legislação. */
const COMPLETA = entidade(["servicos", "faqs", "pareceres", "tarf", "legislacao"]);
/** Só serviços: o caso de mg/rr, onde não há o que escolher. */
const SO_SERVICOS = entidade(["servicos"]);

beforeEach(() => {
  localStorage.clear();
});

describe("tiposDisponiveis", () => {
  it("liga cada tipo à coleção do registry", () => {
    // A regra que impede a interface de oferecer um caminho que só pode voltar como
    // "não está disponível para esta entidade".
    expect(tiposDisponiveis(COMPLETA)).toEqual(["1", "2", "3", "4"]);
    expect(tiposDisponiveis(entidade(["servicos", "pareceres"]))).toEqual(["1", "2"]);
    expect(tiposDisponiveis(SO_SERVICOS)).toEqual(["1"]);
    // Legislação sem pareceres nem TARF: o seletor mostra só o padrão e ela.
    expect(tiposDisponiveis(entidade(["servicos", "legislacao"]))).toEqual(["1", "4"]);
  });

  it("o tipo padrão existe mesmo sem coleção nenhuma", () => {
    // Serviços+FAQs tolera coleção ausente (a parte que falta sai vazia do contexto), então ele
    // nunca some — some o SELETOR, quando sobra uma opção só.
    expect(tiposDisponiveis(entidade([]))).toEqual(["1"]);
  });
});

describe("useQuestionType", () => {
  it("defaults to '1' when nothing is stored", () => {
    const { result } = renderHook(() => useQuestionType(COMPLETA));
    expect(result.current.questionType).toBe("1");
  });

  it("persists the selection to localStorage", () => {
    const { result } = renderHook(() => useQuestionType(COMPLETA));
    act(() => result.current.updateQuestionType("2"));
    expect(result.current.questionType).toBe("2");
    expect(localStorage.getItem(STORAGE_KEY)).toBe("2");
  });

  it("reads the persisted selection on init", () => {
    localStorage.setItem(STORAGE_KEY, "2");
    const { result } = renderHook(() => useQuestionType(COMPLETA));
    expect(result.current.questionType).toBe("2");
  });

  it("aceita o TARF onde a entidade tem a coleção", () => {
    const { result } = renderHook(() => useQuestionType(COMPLETA));
    act(() => result.current.updateQuestionType("3"));
    expect(result.current.questionType).toBe("3");
    expect(localStorage.getItem(STORAGE_KEY)).toBe("3");
  });

  it("aceita a legislação onde a entidade tem a coleção", () => {
    const { result } = renderHook(() => useQuestionType(COMPLETA));
    act(() => result.current.updateQuestionType("4"));
    expect(result.current.questionType).toBe("4");
    expect(localStorage.getItem(STORAGE_KEY)).toBe("4");
  });

  it("a legislação é recusada onde a entidade não a tem", () => {
    // Hoje é o caso das outras 26: a coleção existe no código e não no acervo delas.
    const { result } = renderHook(() => useQuestionType(entidade(["servicos", "faqs"])));
    act(() => result.current.updateQuestionType("4"));
    expect(result.current.questionType).toBe("1");
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it("ignores an unknown stored value and falls back to the default", () => {
    localStorage.setItem(STORAGE_KEY, "9");
    const { result } = renderHook(() => useQuestionType(COMPLETA));
    expect(result.current.questionType).toBe("1");
  });

  it("ignores an invalid update (no state change, nothing persisted)", () => {
    const { result } = renderHook(() => useQuestionType(COMPLETA));
    act(() => result.current.updateQuestionType("9"));
    expect(result.current.questionType).toBe("1");
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it("recusa um tipo que a entidade não tem, sem persistir nada", () => {
    const { result } = renderHook(() => useQuestionType(entidade(["servicos", "pareceres"])));
    act(() => result.current.updateQuestionType("3"));
    expect(result.current.questionType).toBe("1");
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it("a preferência guardada não some ao passar por uma entidade que não a tem", () => {
    // A chave do localStorage é global (a preferência é do usuário, não do estado). Escolher TARF
    // no RS e passar por SP tem de degradar para o padrão em SP — e voltar a valer no RS.
    localStorage.setItem(STORAGE_KEY, "3");
    const { result: emSp } = renderHook(() => useQuestionType(entidade(["servicos", "pareceres"])));
    expect(emSp.current.questionType).toBe("1");
    expect(localStorage.getItem(STORAGE_KEY)).toBe("3");

    const { result: emRs } = renderHook(() => useQuestionType(COMPLETA));
    expect(emRs.current.questionType).toBe("3");
  });
});
