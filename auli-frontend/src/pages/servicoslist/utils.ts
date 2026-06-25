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
  return [
    { tipo: "Empresas",     filename: "rs-servicos-a-empresas" },
    { tipo: "Cidadãos",     filename: "rs-servicos-ao-cidadao" },
    { tipo: "Fornecedores", filename: "rs-servicos-a-fornecedores" },
    { tipo: "Agentes",      filename: "rs-servicos-a-agentes-publicos" },
    { tipo: "Servidores",   filename: "rs-servicos-a-servidores-publicos" },
  ];
}
