/**
 * GERADO de data/registry.toml por scripts/gen-frontend-entities.mjs — NÃO EDITE À MÃO.
 * Rode `node scripts/gen-frontend-entities.mjs` após mudar o registry.
 *
 * Cada entidade é uma secretaria estadual da fazenda. O app é escopo de uma entidade por vez,
 * escolhida na landing de seleção de estado e persistida no localStorage. Os dados são servidos
 * por entidade de `public/<id>/…` (ver `entityPath` em fetchers).
 */

/** As abas de coleção que uma entidade pode ter. `chat` está sempre disponível. */
export type Collection = "conteudos" | "faqs" | "notas" | "pareceres" | "servicos";

export interface Entity {
  /** Id estável; também a pasta `public/<id>/` e a chave de tenant no backend. */
  id: string;
  /** Nome completo exibido no header / seletor (ex.: "SEFAZ-RS"). */
  name: string;
  /** Sigla curta para UI compacta (ex.: "RS"). */
  uf: string;
  /** Nome do estado por extenso (ex.: "Rio Grande do Sul"). */
  state: string;
  /** Quais coleções esta entidade tem dados hoje. */
  collections: Collection[];
}

export const ENTITIES: Entity[] = [
  {
    id: "rs",
    name: "SEFAZ-RS",
    uf: "RS",
    state: "Rio Grande do Sul",
    collections: ["servicos", "faqs", "pareceres", "notas", "conteudos"],
  },
  {
    id: "sc",
    name: "SEF-SC",
    uf: "SC",
    state: "Santa Catarina",
    collections: ["servicos"],
  },
  {
    id: "pr",
    name: "SEFA-PR",
    uf: "PR",
    state: "Paraná",
    collections: [],
  },
  {
    id: "sp",
    name: "SEFAZ-SP",
    uf: "SP",
    state: "São Paulo",
    collections: [],
  },
];

export const DEFAULT_ENTITY_ID = "rs";

export function getEntity(id: string | null | undefined): Entity | undefined {
  return ENTITIES.find((e) => e.id === id);
}

/** Entidade cujo estado é a UF dada (ex.: "RS"), se houver. Usado pelo mapa do Brasil. */
export function getEntityByUf(uf: string): Entity | undefined {
  return ENTITIES.find((e) => e.uf === uf);
}

/** Se uma entidade tem dados para uma coleção (dirige o estado vazio das abas). */
export function hasCollection(entity: Entity, collection: Collection): boolean {
  return entity.collections.includes(collection);
}
