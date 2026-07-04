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
