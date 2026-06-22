/**
 * Multi-tenant entity registry (frontend mirror of the scraper's `domain::entities`).
 *
 * Each entity is a Brazilian state tax authority. The app is scoped to one entity at a time, chosen
 * on the state-selection landing page and persisted to localStorage. Data files are served per
 * entity from `public/<id>/…` (see `entityPath` in fetchers).
 */

/** The collection tabs an entity can have data for. `chat` is always available. */
export type Collection = "servicos" | "faqs" | "pareceres" | "notas" | "conteudos";

export interface Entity {
  /** Stable id; also the `public/<id>/` data folder and the backend tenant key. */
  id: string;
  /** Full name shown in the header / selector (e.g. "SEFAZ-RS"). */
  name: string;
  /** Short label for compact UI (e.g. "RS"). */
  uf: string;
  /** State name in Portuguese (e.g. "Rio Grande do Sul"). */
  state: string;
  /** Which collections this entity currently has scraped data for. */
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
    // Only servicos has been scraped for SC so far; other tabs show an empty state.
    collections: ["servicos"],
  },
];

export const DEFAULT_ENTITY_ID = "rs";

export function getEntity(id: string | null | undefined): Entity | undefined {
  return ENTITIES.find((e) => e.id === id);
}

/** Find the entity whose state is the given UF code (e.g. "RS"), if any. Used by the Brazil map. */
export function getEntityByUf(uf: string): Entity | undefined {
  return ENTITIES.find((e) => e.uf === uf);
}

/** Whether an entity has data for a given collection (drives the tabs' empty state). */
export function hasCollection(entity: Entity, collection: Collection): boolean {
  return entity.collections.includes(collection);
}
