import useSWR from "swr";
import { jsonFetcher, SWR_OPTS, versioned } from "../shared/fetchers";
import type { TributumCatalogo } from "./types";

/**
 * Catálogo GLOBAL, como o `downloads.json` — e diferente dos dados por-entidade em
 * `/<id>/<id>-*.json`. D-TRIB-2: o mesmo conteúdo em todas as entidades, um arquivo só.
 *
 * Mora na RAIZ de `public/` (com o `about.md`), não numa subpasta: o `auli-frontend/.gitignore`
 * ignora TODAS as subpastas de `public/`, porque tudo que vive numa delas é GERADO
 * (`build-frontend-public.sh`, `auli bundle`). Este catálogo é autorado à mão (D-TRIB-5) e precisa
 * ser versionado — a raiz é o único lugar de `public/` onde isso vale.
 *
 * (O padrão do gitignore não é citado aqui de propósito: ele contém a sequência que FECHA um
 * comentário de bloco, e colá-lo quebra o arquivo. Está no `.gitignore`, com o porquê ao lado.)
 */
const CATALOGO_PATH = "/tributum.json";

/** Catálogo vazio: mantém as quatro listas renderizáveis enquanto carrega ou depois de um erro. */
const VAZIO: TributumCatalogo = {
  artigos: [],
  instituicoes: [],
  dados: [],
  analises: [],
};

/**
 * Normaliza o que veio do disco para o contrato de quatro arrays.
 *
 * Cada chave é conferida com `Array.isArray` **isoladamente**: o catálogo é mantido à mão, e a
 * falha realista é uma seção faltando ou com o tipo errado — não o arquivo inteiro quebrado. Assim
 * um `dados` malformado não derruba `artigos`, que está bom.
 */
function normalizar(bruto: unknown): TributumCatalogo {
  const j = (bruto ?? {}) as Partial<Record<keyof TributumCatalogo, unknown>>;
  const arr = <T,>(v: unknown): T[] => (Array.isArray(v) ? (v as T[]) : []);
  const curadores = j.curadores;
  return {
    artigos: arr(j.artigos),
    instituicoes: arr(j.instituicoes),
    dados: arr(j.dados),
    analises: arr(j.analises),
    // Objeto, não array — e a mesma disciplina: forma errada vira ausência, não exceção.
    curadores:
      curadores && typeof curadores === "object" && !Array.isArray(curadores)
        ? (curadores as TributumCatalogo["curadores"])
        : undefined,
  };
}

/**
 * Carrega o catálogo do Tributum.
 *
 * Usa o `jsonFetcher` da casa, e **não** um `fetch` cru com `r.ok`: o Apache serve o `index.html`
 * com **200** para qualquer estático ausente (`FallbackResource /index.html`), então um arquivo que
 * não subiu chega como HTML bem-sucedido. O `jsonFetcher` transforma isso em erro; `r.ok` o
 * aceitaria e o `JSON.parse` estouraria longe daqui, num lugar difícil de ler.
 *
 * `versioned()` pelo mesmo motivo das outras listas: o `?v=` do build invalida o cache no deploy e
 * deixa recarregar de graça dentro do mesmo build.
 */
export function useTributum() {
  const { data, error, isLoading } = useSWR<unknown>(
    versioned(CATALOGO_PATH),
    jsonFetcher,
    SWR_OPTS,
  );

  return {
    catalogo: data ? normalizar(data) : VAZIO,
    erro: error as Error | undefined,
    carregando: isLoading,
  };
}
