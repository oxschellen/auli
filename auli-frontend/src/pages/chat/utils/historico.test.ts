import { describe, expect, it } from "vitest";
import {
  MAX_RECENTES,
  alternarFavorita,
  desserializar,
  limparRecentes,
  registrar,
  remover,
  serializar,
  type ItemHistorico,
} from "./historico";

const item = (id: string, pergunta: string, extra: Partial<ItemHistorico> = {}): ItemHistorico => ({
  id,
  pergunta,
  tipo: "1",
  em: 1,
  favorita: false,
  ...extra,
});

describe("registrar", () => {
  it("põe a pergunta nova no topo", () => {
    const lista = registrar(
      [item("a", "Como emitir a certidão?")],
      "Como pagar o IPVA?",
      "1",
      5,
      "b",
    );
    expect(lista.map((i) => i.id)).toEqual(["b", "a"]);
    expect(lista[0]).toEqual({
      id: "b",
      pergunta: "Como pagar o IPVA?",
      tipo: "1",
      em: 5,
      favorita: false,
    });
  });

  it("pergunta repetida sobe ao topo sem duplicar, mantendo o id e a estrela", () => {
    const antes = [
      item("a", "Outra pergunta qualquer"),
      item("b", "Como pagar o IPVA?", { favorita: true }),
    ];
    // Caixa e espaços não fazem dela outra pergunta.
    const lista = registrar(antes, "  como pagar   o IPVA? ", "1", 9, "novo");
    expect(lista.map((i) => i.id)).toEqual(["b", "a"]);
    expect(lista[0]).toMatchObject({ id: "b", favorita: true, em: 9 });
  });

  it("a mesma frase em outro tipo de consulta é outra entrada", () => {
    const lista = registrar([item("a", "Como pagar o IPVA?")], "Como pagar o IPVA?", "2", 9, "b");
    expect(lista.map((i) => [i.id, i.tipo])).toEqual([
      ["b", "2"],
      ["a", "1"],
    ]);
  });

  it("descarta a recente mais antiga ao passar do teto, e nunca uma favorita", () => {
    const cheias = Array.from({ length: MAX_RECENTES }, (_, n) =>
      item(`r${n}`, `Pergunta número ${n}`),
    );
    const antes = [...cheias, item("fav", "Favorita antiga", { favorita: true })];
    const lista = registrar(antes, "Pergunta nova", "1", 9, "nova");
    expect(lista.filter((i) => !i.favorita)).toHaveLength(MAX_RECENTES);
    expect(lista.map((i) => i.id)).not.toContain(`r${MAX_RECENTES - 1}`);
    expect(lista.map((i) => i.id)).toContain("fav");
  });
});

describe("favoritar, remover e limpar", () => {
  const lista = [item("a", "Primeira"), item("b", "Segunda", { favorita: true })];

  it("a estrela liga e desliga só o item pedido", () => {
    expect(alternarFavorita(lista, "a").map((i) => i.favorita)).toEqual([true, true]);
    expect(alternarFavorita(lista, "b").map((i) => i.favorita)).toEqual([false, false]);
  });

  it("remover tira só o item pedido", () => {
    expect(remover(lista, "a").map((i) => i.id)).toEqual(["b"]);
  });

  it("limpar recentes preserva as favoritas", () => {
    expect(limparRecentes(lista).map((i) => i.id)).toEqual(["b"]);
  });
});

describe("o que está gravado", () => {
  it("vai e volta sem perda", () => {
    const lista = [item("a", "Primeira"), item("b", "Segunda", { favorita: true, tipo: "3" })];
    expect(desserializar(serializar(lista))).toEqual(lista);
  });

  it.each([
    ["nada gravado", null],
    ["texto que não é JSON", "{{{"],
    ["JSON que não é objeto", "[]"],
    ["versão desconhecida", JSON.stringify({ v: 2, itens: [item("a", "Primeira")] })],
    ["itens que não são lista", JSON.stringify({ v: 1, itens: "x" })],
  ])("%s vira lista vazia", (_, bruto) => {
    expect(desserializar(bruto)).toEqual([]);
  });

  it("item fora da forma é descartado, os bons ficam", () => {
    const bruto = JSON.stringify({
      v: 1,
      itens: [
        item("a", "Boa"),
        { id: "b", pergunta: "Tipo inválido", tipo: "9", em: 1, favorita: false },
        null,
        { id: "c" },
      ],
    });
    expect(desserializar(bruto).map((i) => i.id)).toEqual(["a"]);
  });
});
