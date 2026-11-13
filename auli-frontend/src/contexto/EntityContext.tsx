import { createContext, use } from "react";
import type { ReactNode } from "react";
import type { Entity } from "./entities";

/**
 * A entidade da aba de estado em que o componente está. Cada aba tem o seu provider (ver
 * `App.tsx`), então duas abas abertas — RS e SP — leem cada uma a sua, ao mesmo tempo. Quem abre,
 * fecha, escolhe e persiste as abas é o `useAbas` (`app/abas.ts`).
 */
const EntityContext = createContext<Entity | null>(null);

export function EntityProvider({ entity, children }: { entity: Entity; children: ReactNode }) {
  return <EntityContext.Provider value={entity}>{children}</EntityContext.Provider>;
}

/** A entidade da aba (use dentro de uma aba com estado escolhido). */
export function useSelectedEntity(): Entity {
  const entity = use(EntityContext);
  if (!entity) throw new Error("useSelectedEntity fora de uma aba de estado (EntityProvider)");
  return entity;
}
