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
 * Rótulos que designam a audiência **empresa** nos 27 índices: `Empresa` (13 entidades), `Empresas`
 * (mg/pe/rs) e `Pessoa Jurídica` — o nome que o AM usa para a mesma audiência (o índice dele é
 * `Pessoa Física | Pessoa Jurídica | Órgãos Públicos`).
 *
 * Lista explícita, não regex: `/empresa/i` deixaria o AM de fora e pegaria por acidente qualquer
 * rótulo futuro que contivesse a palavra. Rótulo novo entra aqui à mão, de propósito.
 */
const PUBLICOS_EMPRESA = ["Empresa", "Empresas", "Pessoa Jurídica"];

/**
 * Põe a aba de empresa na FRENTE, preservando a ordem original das demais.
 *
 * **Só apresentação.** A ordem do dado (`publicos_ordem` no snapshot → `<id>-servicos-index.json`)
 * fica intocada, e é de propósito: no `auli-collections` essa mesma ordem decide a ocorrência
 * PRIMÁRIA de cada serviço, que vira o frontmatter do `docs/servicos/*.md` e a chave de embedding.
 * Mexer lá trocaria o `tipo` primário de 585 serviços e exigiria re-vetorizar 16 entidades. Aqui a
 * inversão acontece na leitura, depois do fetch — zero efeito sobre packs, chat e `docs_hash`.
 *
 * **Também define a aba inicial**, por consequência e por decisão: o `ServicosList` usa `[0]` como
 * padrão enquanto nada foi clicado, então a audiência de empresa passa a ser a entrada.
 *
 * A partição é **estável**, então as demais mantêm a sequência de chegada — o `pr` segue
 * `… | Município | Produtor rural | Receita/PR | Programas | Legislação` logo após o Empresa.
 * Entidade de público único (`Serviços`, `Cidadãos`) ou já começando com empresa sai equivalente.
 */
export function empresaPrimeiro(tipos: TipoServico[]): TipoServico[] {
  const ehEmpresa = (t: TipoServico) => PUBLICOS_EMPRESA.includes(t.tipo);
  return [...tipos.filter(ehEmpresa), ...tipos.filter((t) => !ehEmpresa(t))];
}

/**
 * Fallback audience tabs, used when `servicos-index.json` is absent (older deploys / RS-only data).
 * The scraper now emits `servicos-index.json` (`{ tipo, filename }[]`) so the tabs match whichever
 * entity's data was built — see `getTipoServicos`.
 */
export function getDefaultTipoServicos(): TipoServico[] {
  // Order mirrors the RS scraper / `servicos-index.json` (Cidadãos first) so the fallback tabs match
  // the real audience order when the index is absent. A ordem EXIBIDA não é esta: o chamador passa
  // o resultado por `empresaPrimeiro`, então "Empresas" aparece na frente — aqui fica a ordem do
  // dado, para o fallback continuar espelhando o índice real.
  return [
    { tipo: "Cidadãos",     filename: "servicos-ao-cidadao" },
    { tipo: "Empresas",     filename: "servicos-a-empresas" },
    { tipo: "Fornecedores", filename: "servicos-a-fornecedores" },
    { tipo: "Agentes",      filename: "servicos-a-agentes-publicos" },
    { tipo: "Servidores",   filename: "servicos-a-servidores-publicos" },
  ];
}
