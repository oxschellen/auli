import { describe, it, expect } from "vitest";
import { filterServicoGroups, type Servico, type ServicoGroup } from "./utils";

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
