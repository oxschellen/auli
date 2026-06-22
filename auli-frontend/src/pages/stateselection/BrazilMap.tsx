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
const FILL_ACTIVE = "var(--chakra-colors-accent)";
const FILL_ACTIVE_HOVER = "var(--chakra-colors-brand-600)";

export function BrazilMap({ onSelect }: BrazilMapProps) {
  const [hoveredUf, setHoveredUf] = useState<string | null>(null);
  const [fillInactive, stroke] = useToken("colors", ["bg.mapInactive", "bg.canvas"]);

  return (
    <Box w="100%" maxW="460px" mx="auto">
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

          return (
            <path
              key={uf}
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
          );
        })}
      </svg>
    </Box>
  );
}
