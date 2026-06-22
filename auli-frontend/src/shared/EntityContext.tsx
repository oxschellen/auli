import { createContext, use, useCallback, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { ENTITIES, getEntity, type Entity } from "./entities";

const STORAGE_KEY = "auli.entity";

interface EntityContextValue {
  /** The selected entity, or null when no state has been chosen yet (show the selector). */
  entity: Entity | null;
  /** Choose / switch the active entity (persisted to localStorage). */
  selectEntity: (id: string) => void;
  /** Clear the selection and return to the state-selection page. */
  clearEntity: () => void;
}

const EntityContext = createContext<EntityContextValue | undefined>(undefined);

/** Reads the persisted entity id once at startup (ignored if it no longer maps to a known entity). */
function readStoredEntity(): Entity | null {
  try {
    const id = localStorage.getItem(STORAGE_KEY);
    return getEntity(id) ?? null;
  } catch {
    return null;
  }
}

export function EntityProvider({ children }: { children: ReactNode }) {
  const [entity, setEntity] = useState<Entity | null>(readStoredEntity);

  const selectEntity = useCallback((id: string) => {
    const next = ENTITIES.find((e) => e.id === id);
    if (!next) return;
    try {
      localStorage.setItem(STORAGE_KEY, next.id);
    } catch {
      // Non-fatal: selection still works for this session without persistence.
    }
    setEntity(next);
  }, []);

  const clearEntity = useCallback(() => {
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {
      // ignore
    }
    setEntity(null);
  }, []);

  const value = useMemo(
    () => ({ entity, selectEntity, clearEntity }),
    [entity, selectEntity, clearEntity],
  );

  return <EntityContext.Provider value={value}>{children}</EntityContext.Provider>;
}

/** Access the entity selection. Returns null `entity` until a state is chosen. */
export function useEntity(): EntityContextValue {
  const ctx = use(EntityContext);
  if (!ctx) throw new Error("useEntity must be used within an EntityProvider");
  return ctx;
}

/** Convenience: the chosen entity, asserting one is selected (use inside the app shell). */
export function useSelectedEntity(): Entity {
  const { entity } = useEntity();
  if (!entity) throw new Error("useSelectedEntity called before an entity was selected");
  return entity;
}
