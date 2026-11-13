import type { ComponentType } from "react";
import type { IconType } from "react-icons";
import {
  MdOutlineChat,
  MdOutlineWorkOutline,
  MdHelpOutline,
  MdOutlineGavel,
  MdOutlineBalance,
  MdOutlineStickyNote2,
  MdOutlineLibraryBooks,
  MdOutlineMenuBook,
  MdOutlineArticle,
  MdOutlineAccountBalance,
  MdOutlineDataset,
  MdOutlineInsights,
  MdOutlineAutoStories,
  MdOutlineRule,
  MdOutlineFileDownload,
  MdOutlinePower,
  MdInfoOutline,
} from "react-icons/md";
import { Chat } from "./chat/Chat";
import { ServicosList } from "./servicos/ServicosList";
import { FaqsList } from "./faqs/FaqsList";
import { PareceresList } from "./jurisprudencia/PareceresList";
import { TarfList } from "./jurisprudencia/TarfList";
import { LegislacaoList } from "./legislacao/LegislacaoList";
import { NotasList } from "./notas/NotasList";
import { ConteudosList } from "./conteudos/ConteudosList";
import { CriteriosTributum } from "./tributum/CriteriosTributum";
import { ArtigosLista } from "./tributum/ArtigosLista";
import { InstituicoesLista } from "./tributum/InstituicoesLista";
import { DadosLista } from "./tributum/DadosLista";
import { AnalisesLista } from "./tributum/AnalisesLista";
import { LivrosLista } from "./tributum/LivrosLista";
import { DownloadsList } from "./downloads/DownloadsList";
import { McpList } from "./mcp/McpList";
import { About } from "./sobre/About";
import { hasCollection } from "../contexto/entities";
import type { Collection, Entity } from "../contexto/entities";

/**
 * O registro das seções (I7 do docs/ESTUDO-frontend-organizacao.md): quais existem, com rótulo,
 * ícone, componente, coleção exigida e grupo. É dado, e mora fora do `Home`, que só o desenha.
 */

/** Grupo de navegação. `null` = topo (sem título). */
export type Grupo = null | "acervo" | "tributum" | "integracoes";

/** Uma seção da sidebar. O `id` ancora os ids `tab-` e `tabpanel-`: não muda. */
export interface Secao {
  id: string;
  label: string;
  Component: ComponentType;
  collection: Collection | null;
  grupo: Grupo;
  Icone: IconType;
}

/** As seções, agrupadas para a sidebar. `collection: null` = sempre disponível
 *  (`chat` fala com o RAG da entidade, `about` é estático, `downloads` lista TODOS os
 *  estados, `mcp` são manuais globais, e as seis do `tributum` são curadoria com catálogo
 *  ÚNICO compartilhado por todas as entidades, D-TRIB-2); as demais dependem de a entidade ter
 *  a coleção. **`Critérios` abre o grupo**: a régua editorial vem antes das listas que ela
 *  governa — quem chega à seção lê o que entra e o que não entra antes de ler a seleção. O rótulo de `mcp` virou "Conectar sua IA" — o id NÃO muda (ancora tab-/tabpanel-). */
export const SECOES: Secao[] = [
  {
    id: "chat",
    label: "Chat",
    Component: Chat,
    collection: null,
    grupo: null,
    Icone: MdOutlineChat,
  },
  {
    id: "servicos",
    label: "Serviços",
    Component: ServicosList,
    collection: "servicos",
    grupo: "acervo",
    Icone: MdOutlineWorkOutline,
  },
  {
    id: "faqs",
    label: "FAQs",
    Component: FaqsList,
    collection: "faqs",
    grupo: "acervo",
    Icone: MdHelpOutline,
  },
  {
    id: "pareceres",
    label: "Pareceres",
    Component: PareceresList,
    collection: "pareceres",
    grupo: "acervo",
    Icone: MdOutlineGavel,
  },
  {
    id: "tarf",
    label: "Acórdãos TARF",
    Component: TarfList,
    collection: "tarf",
    grupo: "acervo",
    Icone: MdOutlineBalance,
  },
  {
    id: "legislacao",
    label: "Legislação",
    Component: LegislacaoList,
    collection: "legislacao",
    grupo: "acervo",
    Icone: MdOutlineMenuBook,
  },
  {
    id: "notas",
    label: "Notas",
    Component: NotasList,
    collection: "notas",
    grupo: "acervo",
    Icone: MdOutlineStickyNote2,
  },
  {
    id: "conteudos",
    label: "Conteúdos",
    Component: ConteudosList,
    collection: "conteudos",
    grupo: "acervo",
    Icone: MdOutlineLibraryBooks,
  },
  // O Sobre fechava a sidebar, colado no pé; com o grupo Tributum ela passou da altura de tela de
  // notebook (768 px) e ele ficava abaixo da dobra, sem pista de que a lista rolava.
  {
    id: "about",
    label: "Sobre",
    Component: About,
    collection: null,
    grupo: "acervo",
    Icone: MdInfoOutline,
  },
  {
    id: "tributum-criterios",
    label: "Critérios",
    Component: CriteriosTributum,
    collection: null,
    grupo: "tributum",
    Icone: MdOutlineRule,
  },
  {
    id: "tributum-artigos",
    label: "Artigos e estudos",
    Component: ArtigosLista,
    collection: null,
    grupo: "tributum",
    Icone: MdOutlineArticle,
  },
  {
    id: "tributum-instituicoes",
    label: "Instituições",
    Component: InstituicoesLista,
    collection: null,
    grupo: "tributum",
    Icone: MdOutlineAccountBalance,
  },
  {
    id: "tributum-dados",
    label: "Dados",
    Component: DadosLista,
    collection: null,
    grupo: "tributum",
    Icone: MdOutlineDataset,
  },
  {
    id: "tributum-analises",
    label: "Análises",
    Component: AnalisesLista,
    collection: null,
    grupo: "tributum",
    Icone: MdOutlineInsights,
  },
  {
    id: "tributum-livros",
    label: "Livros",
    Component: LivrosLista,
    collection: null,
    grupo: "tributum",
    Icone: MdOutlineAutoStories,
  },
  {
    id: "mcp",
    label: "Conectar sua IA",
    Component: McpList,
    collection: null,
    grupo: "integracoes",
    Icone: MdOutlinePower,
  },
  {
    id: "downloads",
    label: "Downloads",
    Component: DownloadsList,
    collection: null,
    grupo: "integracoes",
    Icone: MdOutlineFileDownload,
  },
];

/** Títulos e ordem dos grupos na sidebar. */
export const GRUPOS: { id: Grupo; titulo: string | null }[] = [
  { id: null, titulo: null },
  { id: "acervo", titulo: "Acervo" },
  // D-TRIB-3: entre o Acervo e as Integrações. Sem gate por registry — o catálogo é o mesmo em
  // todas as entidades (D-TRIB-2), e cada lista se vira sozinha com erro e vazio.
  { id: "tributum", titulo: "Tributum" },
  { id: "integracoes", titulo: "Integrações" },
];

/**
 * As seções como a entidade as vê. Todas continuam VISÍVEIS — a que a entidade não tem fica inerte,
 * não desaparece: some com a seção e o usuário não sabe que ela existe; apagada, ele vê que existe
 * e não vale aqui. 23 das 27 entidades declaram só `servicos`, então isso desativa 4 das 7 na maioria.
 */
export const secoesDe = (entity: Entity) =>
  SECOES.map((s) => ({
    ...s,
    disabled: s.collection !== null && !hasCollection(entity, s.collection),
  }));
