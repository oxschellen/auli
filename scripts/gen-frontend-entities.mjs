#!/usr/bin/env node
// gen-frontend-entities.mjs — gera auli-frontend/src/shared/entities.ts a partir de data/registry.toml.
//
// O frontend deixa de manter sua própria lista de entidades (4ª cópia da §6): a fonte da verdade é
// data/registry.toml. Rode após editar o registry:
//   node scripts/gen-frontend-entities.mjs
//
// Zero dependências: parser TOML mínimo, suficiente para o schema do registry (array de tabelas
// `[[entities]]` com strings e um array `collections`).

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const REGISTRY = join(ROOT, "data", "registry.toml");
const OUT = join(ROOT, "auli-frontend", "src", "shared", "entities.ts");

/** Minimal TOML parse for the registry's `[[entities]]` array-of-tables. */
function parseEntities(text) {
  const entities = [];
  let cur = null;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue; // full-line comment / blank
    if (line === "[[entities]]") {
      cur = {};
      entities.push(cur);
      continue;
    }
    const m = line.match(/^([A-Za-z_]+)\s*=\s*(.+)$/);
    if (!m || !cur) continue;
    const [, key, rhs] = m;
    cur[key] = rhs.startsWith("[")
      ? [...rhs.matchAll(/"([^"]*)"/g)].map((x) => x[1]) // array of quoted strings
      : rhs.replace(/^"(.*)"$/, "$1"); // quoted scalar
  }
  return entities;
}

const entities = parseEntities(readFileSync(REGISTRY, "utf8"));
if (entities.length === 0) {
  console.error("❌ Nenhuma entidade lida de", REGISTRY);
  process.exit(1);
}

// Union de coleções = conjunto distinto presente no registry (ordenado).
const allCollections = [...new Set(entities.flatMap((e) => e.collections ?? []))].sort();
const collType = allCollections.map((c) => JSON.stringify(c)).join(" | ");
const defaultId = entities[0].id;

const body = entities
  .map(
    (e) =>
      `  {\n` +
      `    id: ${JSON.stringify(e.id)},\n` +
      `    name: ${JSON.stringify(e.name)},\n` +
      `    uf: ${JSON.stringify(e.uf)},\n` +
      `    state: ${JSON.stringify(e.state)},\n` +
      `    collections: [${(e.collections ?? []).map((c) => JSON.stringify(c)).join(", ")}],\n` +
      `  },`,
  )
  .join("\n");

const out = `/**
 * GERADO de data/registry.toml por scripts/gen-frontend-entities.mjs — NÃO EDITE À MÃO.
 * Rode \`node scripts/gen-frontend-entities.mjs\` após mudar o registry.
 *
 * Cada entidade é uma secretaria estadual da fazenda. O app é escopo de uma entidade por vez,
 * escolhida na landing de seleção de estado e persistida no localStorage. Os dados são servidos
 * por entidade de \`public/<id>/…\` (ver \`entityPath\` em fetchers).
 */

/** As abas de coleção que uma entidade pode ter. \`chat\` está sempre disponível. */
export type Collection = ${collType};

export interface Entity {
  /** Id estável; também a pasta \`public/<id>/\` e a chave de tenant no backend. */
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
${body}
];

export const DEFAULT_ENTITY_ID = ${JSON.stringify(defaultId)};

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
`;

writeFileSync(OUT, out);
console.log(`✅ ${OUT} gerado de ${entities.length} entidades (${entities.map((e) => e.id).join(", ")}).`);
