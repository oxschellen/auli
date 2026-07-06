import { useState } from "react";
import { Box, useToken } from "@chakra-ui/react";
import { getEntityByUf } from "../../shared/entities";
import { BRAZIL_UF_PATHS, BRAZIL_VIEWBOX } from "./brazilPaths";

interface BrazilMapProps {
  /** Called with the entity id when an available (selectable) state is clicked. */
  onSelect: (entityId: string) => void;
}

/**
 * Outlined map of Brazil's 27 states. States backed by an available entity (RS, SC) are filled with
 * the accent color and are clickable; all other states are greyed out and inert. Geometry lives in
 * the generated `brazilPaths.ts` (keyed by UF code).
 *
 * Native SVG elements are used instead of Chakra's `Box as="path"` because the style factory drops
 * the `d`/`viewBox` attributes. Colors are resolved from the theme via `useToken` (concrete values),
 * so the map still follows light/dark mode without relying on Chakra emitting each CSS var.
 */
// Active states use the live `accent`/`brand.600` CSS variables (same as the entity cards) so the
// selectable blue matches the cards and tracks light/dark mode. `useToken` is only used for colors
// that Chakra doesn't emit as a standalone CSS var (the inert fill and the stroke).
// Selectable-state fill: a lighter blue than the app accent (map-specific).
const FILL_ACTIVE = "#3f9df2";

type Point = { x: number; y: number };

/** Parse a path into its rings (sub-polygons). Paths are absolute-coordinate
 *  polylines; each `M…Z` segment is one ring. */
function parseRings(d: string): Point[][] {
  const rings: Point[][] = [];
  for (const seg of d.split(/[MZ]/)) {
    const nums = seg.match(/-?\d+\.?\d*/g)?.map(Number) ?? [];
    if (nums.length < 6) continue; // need ≥3 points for a polygon
    const ring: Point[] = [];
    for (let i = 0; i + 1 < nums.length; i += 2) ring.push({ x: nums[i], y: nums[i + 1] });
    rings.push(ring);
  }
  return rings;
}

/** Signed area × 2 of a ring (shoelace); sign indicates winding, magnitude the size. */
function ringArea2(r: Point[]): number {
  let a = 0;
  for (let i = 0, j = r.length - 1; i < r.length; j = i++) {
    a += r[j].x * r[i].y - r[i].x * r[j].y;
  }
  return a;
}

/** Area-weighted centroid of a state's largest ring (its main landmass) plus a
 *  font size that fits the state — a much better label anchor than the bounding-box
 *  center for irregular shapes. Optional per-UF nudge corrects concave outliers. */
function centerOf(d: string, nudge?: Point): Point & { size: number } {
  const rings = parseRings(d);
  let ring = rings[0] ?? [];
  let best = 0;
  for (const r of rings) {
    const area = Math.abs(ringArea2(r));
    if (area > best) { best = area; ring = r; }
  }
  const a2 = ringArea2(ring);
  let cx = 0, cy = 0;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const f = ring[j].x * ring[i].y - ring[i].x * ring[j].y;
    cx += (ring[j].x + ring[i].x) * f;
    cy += (ring[j].y + ring[i].y) * f;
  }
  const c = a2 !== 0
    ? { x: cx / (3 * a2), y: cy / (3 * a2) }
    : ring[0] ?? { x: 0, y: 0 };
  // best = 2×area of the largest ring; shrink the label for small states so it fits.
  const area = best / 2;
  const size = area < 3000 ? 18 : area < 6000 ? 25 : 31;
  return { x: c.x + (nudge?.x ?? 0), y: c.y + (nudge?.y ?? 0), size };
}

/** Per-UF label offsets (viewBox units) for states whose plain centroid lands
 *  awkwardly. SC's mass leans toward the PR border, so its centroid sits high;
 *  nudge it down to the visual center. */
const LABEL_NUDGE: Record<string, Point> = {
  SC: { x: 0, y: 10 },
};
// Highlight (hover/focus) fill: teal, distinct from the accent-blue selectable
// states and the grey inactive ones.
const FILL_ACTIVE_HOVER = "#2dd4bf";

export function BrazilMap({ onSelect }: BrazilMapProps) {
  const [hoveredUf, setHoveredUf] = useState<string | null>(null);
  const [fillInactive, stroke] = useToken("colors", ["bg.mapInactive", "bg.canvas"]);

  return (
    <Box w="100%" maxW="506px" mx="auto">
      <svg
        viewBox={BRAZIL_VIEWBOX}
        role="group"
        aria-label="Mapa do Brasil — selecione um estado disponível"
        width="100%"
        height="auto"
        style={{ display: "block", width: "100%", height: "auto" }}
      >
        {Object.entries(BRAZIL_UF_PATHS).map(([uf, d]) => {
          const entity = getEntityByUf(uf);
          const available = Boolean(entity);
          const hovered = hoveredUf === uf;
          const fill = !available ? fillInactive : hovered ? FILL_ACTIVE_HOVER : FILL_ACTIVE;
          const center = available ? centerOf(d, LABEL_NUDGE[uf]) : null;

          return (
            <g key={uf}>
            <path
              d={d}
              fill={fill}
              stroke={stroke}
              strokeWidth={1.2}
              style={{ cursor: available ? "pointer" : "default", transition: "fill 0.15s ease" }}
              role={available ? "button" : undefined}
              tabIndex={available ? 0 : undefined}
              aria-label={available ? `Selecionar ${entity?.name} (${entity?.state})` : undefined}
              aria-disabled={available ? undefined : true}
              onClick={available ? () => entity && onSelect(entity.id) : undefined}
              onMouseEnter={available ? () => setHoveredUf(uf) : undefined}
              onMouseLeave={available ? () => setHoveredUf(null) : undefined}
              onFocus={available ? () => setHoveredUf(uf) : undefined}
              onBlur={available ? () => setHoveredUf(null) : undefined}
              onKeyDown={
                available
                  ? (e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        if (entity) onSelect(entity.id);
                      }
                    }
                  : undefined
              }
            >
              <title>{available ? entity?.name : uf}</title>
            </path>
            {hovered && center && (
              <text
                x={center.x}
                y={center.y}
                textAnchor="middle"
                dominantBaseline="central"
                fontSize={center.size}
                fontWeight={900}
                // Dark ink for contrast against the teal highlight fill.
                fill="#083344"
                style={{ pointerEvents: "none", userSelect: "none" }}
              >
                {uf}
              </text>
            )}
            </g>
          );
        })}
      </svg>
    </Box>
  );
}
