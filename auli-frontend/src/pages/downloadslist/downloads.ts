/**
 * Tipos e lógica pura da aba Downloads.
 *
 * O `downloads.json` é o ÚNICO arquivo global do frontend — todo o resto vive em `/<id>/<id>-*.json`
 * e é escopado à entidade selecionada. Ele é gerado pelo `auli bundle` a partir das mesmas árvores
 * `.md` que alimentam os packs, então é a verdade sobre o que foi empacotado; `entity.collections`
 * não serve aqui, porque declara coleções (notas, conteúdos) que não têm `.md` fonte e portanto
 * ficam de fora do zip.
 */

import { ENTITIES } from "../../shared/entities";

/** Uma linha do `downloads.json`, no formato que o `auli bundle` grava. */
export interface DownloadEntry {
  /** Id da entidade (`rs`), que casa com `Entity.id`. */
  estado: string;
  uf: string;
  /** Nome da secretaria ("SEFAZ-RS"). O nome do estado por extenso vem do `ENTITIES`. */
  nome: string;
  arquivo: string;
  tamanho_bytes: number;
  /** Já formatado em pt-BR pelo backend ("49,8 MB") — a UI não reformata. */
  tamanho: string;
  sha256: string;
  /** Data em que o conteúdo deste estado mudou pela última vez (`YYYY-MM-DD`). */
  atualizado_em: string;
  gerado_em: string;
  tipos: { tipo: string; arquivos: number }[];
}

/**
 * `2026-07-29` → `29/07/2026`. Parse manual em vez de `new Date(iso)`: a string sem hora é lida
 * como UTC, e num fuso a oeste de Greenwich — o nosso — a data exibida voltaria um dia.
 */
export function formatarData(iso: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso);
  return m ? `${m[3]}/${m[2]}/${m[1]}` : iso;
}

/**
 * Ordem preferida das colunas. Não é a lista fechada: um tipo novo no manifesto aparece depois
 * destes, em ordem alfabética. Hardcodar as três aqui desfaria no frontend a generalidade que o
 * `bundle` tem de propósito no backend.
 */
const ORDEM_TIPOS = ["servicos", "faqs", "pareceres"];

const TIPO_LABELS: Record<string, string> = {
  servicos: "Serviços",
  faqs: "FAQs",
  pareceres: "Pareceres",
  notas: "Notas",
  conteudos: "Conteúdos",
};

/** Rótulo de exibição de uma tabela; devolve a própria chave se for desconhecida. */
export function tipoLabel(tipo: string): string {
  return TIPO_LABELS[tipo] ?? tipo;
}

/**
 * As colunas da tabela: a união dos tipos presentes em QUALQUER estado, nos conhecidos primeiro.
 * Deriva do dado e não de uma constante, senão um estado que ganhe uma coleção nova fica com o
 * conteúdo invisível na tabela.
 */
export function colunasDeTipos(entries: DownloadEntry[]): string[] {
  const vistos = new Set<string>();
  for (const e of entries) for (const t of e.tipos) vistos.add(t.tipo);

  const conhecidos = ORDEM_TIPOS.filter((t) => vistos.has(t));
  const novos = [...vistos].filter((t) => !ORDEM_TIPOS.includes(t)).sort();
  return [...conhecidos, ...novos];
}

/** Quantos documentos aquele estado tem daquele tipo; `undefined` quando o tipo não está no zip. */
export function contagem(entry: DownloadEntry, tipo: string): number | undefined {
  return entry.tipos.find((t) => t.tipo === tipo)?.arquivos;
}

/** Total de documentos do pacote — o número que resume a linha em destaque. */
export function totalDocumentos(entry: DownloadEntry): number {
  return entry.tipos.reduce((soma, t) => soma + t.arquivos, 0);
}

/** "586 serviços · 1.945 FAQs · 372 pareceres", na ordem das colunas. */
export function resumoTipos(entry: DownloadEntry): string {
  return colunasDeTipos([entry])
    .map((tipo) => {
      const n = contagem(entry, tipo) ?? 0;
      return `${n.toLocaleString("pt-BR")} ${tipoLabel(tipo).toLowerCase()}`;
    })
    .join(" · ");
}

/** Nome do estado por extenso, do `ENTITIES` (gerado do mesmo `registry.toml` que o manifesto). */
export function nomeDoEstado(estadoId: string): string {
  return ENTITIES.find((e) => e.id === estadoId)?.state ?? estadoId.toUpperCase();
}

/**
 * URL do zip, com cache-bust pelo **hash do conteúdo**, não pela versão do app.
 *
 * As duas alternativas erram para lados opostos: o `versioned()` (`?v=ASSET_VERSION`) faria o CDN
 * rebaixar 70 MB a cada release, mesmo sem dado novo; já uma URL fixa serviria o zip VELHO do cache
 * do Cloudflare depois de um re-scrape, porque o nome do arquivo não muda quando o conteúdo muda.
 * O `sha256` do manifesto resolve os dois: a URL muda exatamente quando o zip muda.
 */
export function zipHref(entry: Pick<DownloadEntry, "arquivo" | "sha256">): string {
  return `/downloads/${entry.arquivo}?v=${entry.sha256.slice(0, 8)}`;
}
