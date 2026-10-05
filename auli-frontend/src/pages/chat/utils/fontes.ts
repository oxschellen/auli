import type { Fonte } from "../../../types/chat";

/**
 * A lista de fontes em três grupos (D-UI-8), na ordem do contexto dentro de cada um:
 *
 * - **citadas**: as que o texto cita — o que sustenta a resposta;
 * - **consideradas**: chegaram à redação mas não foram citadas — inclui as `condicional`, que são
 *   justamente a alternativa que o texto às vezes omite (REGISTRO-triagem §7);
 * - **descartadas**: a triagem tirou da redação (e o texto não cita).
 */
export interface FontesAgrupadas {
  citadas: Fonte[];
  consideradas: Fonte[];
  descartadas: Fonte[];
}

export function agruparFontes(fontes: Fonte[], citados: Set<string>): FontesAgrupadas {
  const g: FontesAgrupadas = { citadas: [], consideradas: [], descartadas: [] };
  for (const f of fontes) {
    if (citados.has(f.rotulo)) g.citadas.push(f);
    else if (f.triagem === "descarta") g.descartadas.push(f);
    else g.consideradas.push(f);
  }
  return g;
}
