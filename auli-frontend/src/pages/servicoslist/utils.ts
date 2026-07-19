import { parseQuery, buildHaystack, haystackMatches } from "../../shared/textSearch";

/** A Serviços tab: the label shown on the button and the JSON file it loads. */
export interface TipoServico {
  tipo: string;
  filename: string;
}

/** A single service entry as stored in the per-tipo servicos JSON files. */
export interface Servico {
  id: string | number;
  classe: string;
  titulo: string;
  link: string;
}

/** A class heading paired with its (possibly filtered) services. */
export type ServicoGroup = [string, Servico[]];

/**
 * Filtra os grupos pela query, preservando a regra da lista: se a **classe** casa, o grupo inteiro
 * entra (sem filtrar itens); senão, entram só os itens cujo **título** casa. Busca via `textSearch`
 * (acentos + multi-termo, E entre termos). Query vazia devolve `grouped` intacto (mesma referência).
 *
 * Servicos é pequeno por aba (SP tem 537), então normalizar por tecla é irrelevante — o corpus
 * grande é Pareceres, tratado à parte. Helper puro (mesmo idioma de `searchNodes`/`searchCategories`)
 * para ser testável sem montar o componente.
 */
export function filterServicoGroups(grouped: ServicoGroup[], query: string): ServicoGroup[] {
  const terms = parseQuery(query);
  if (terms.length === 0) return grouped;
  const result: ServicoGroup[] = [];
  for (const [classe, items] of grouped) {
    if (haystackMatches(buildHaystack([classe]), terms)) {
      result.push([classe, items]);
    } else {
      const matching = items.filter((s) => haystackMatches(buildHaystack([s.titulo]), terms));
      if (matching.length > 0) result.push([classe, matching]);
    }
  }
  return result;
}

/**
 * Fallback audience tabs, used when `servicos-index.json` is absent (older deploys / RS-only data).
 * The scraper now emits `servicos-index.json` (`{ tipo, filename }[]`) so the tabs match whichever
 * entity's data was built — see `getTipoServicos`.
 */
export function getDefaultTipoServicos(): TipoServico[] {
  // Order mirrors the RS scraper / `servicos-index.json` (Cidadãos first) so the fallback tabs match
  // the real audience order when the index is absent.
  return [
    { tipo: "Cidadãos",     filename: "servicos-ao-cidadao" },
    { tipo: "Empresas",     filename: "servicos-a-empresas" },
    { tipo: "Fornecedores", filename: "servicos-a-fornecedores" },
    { tipo: "Agentes",      filename: "servicos-a-agentes-publicos" },
    { tipo: "Servidores",   filename: "servicos-a-servidores-publicos" },
  ];
}
