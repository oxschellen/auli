/**
 * Medidas de layout compartilhadas entre componentes que precisam concordar sobre elas.
 *
 * Existe por causa de UM caso concreto: a barra de composição do chat é `position: fixed` (para
 * conseguir subir acima do teclado virtual, medido pelo visualViewport), e `fixed` se ancora na
 * VIEWPORT — não na área de conteúdo. Ela precisa, então, saber onde a sidebar termina.
 *
 * O valor não pode ser exportado do `Home`: é o `Home` que importa o `Chat`, e o caminho de volta
 * fecharia um ciclo de imports.
 */

/** Largura da sidebar de seções (a partir do breakpoint `md`; abaixo dele vira drawer). */
export const SIDEBAR_WIDTH = "224px";

/** Largura da sidebar recolhida em faixa de ícones (botão ao lado do logo ou Ctrl+B). 64px põe
 *  o centro dos ícones no do botão do cabeçalho: 16px de margem + 6px de folga + 10px. */
export const SIDEBAR_RAIL_WIDTH = "64px";

/** Variável CSS com a largura que a sidebar OCUPA agora: `SIDEBAR_WIDTH` aberta,
 *  `SIDEBAR_RAIL_WIDTH` recolhida. O `Home` a define na sua raiz e a barra fixa do chat, que é
 *  descendente dela no DOM, a herda — `fixed` muda o bloco de contenção, não a herança. */
export const SIDEBAR_VAR = "--auli-sidebar-w";

/** Id da sidebar da aba de estado `entityId`: o `Home` o põe nela e o botão de recolher, no
 *  cabeçalho, aponta para ele (`aria-controls`). Com o estado no id, duas abas não colidem. */
export const idDaSidebar = (entityId: string) => `${entityId}-sidebar-secoes`;

/** Altura do cabeçalho. O painel "Minhas perguntas" gruda no topo da área que rola e precisa da
 *  altura que sobra abaixo dele; o `AppHeader` lê daqui para os dois não divergirem. */
export const HEADER_HEIGHT = "52px";

/** Largura do painel "Minhas perguntas", à direita do chat (a partir do breakpoint `xl`; abaixo
 *  dele vira drawer). A barra de composição, que é `fixed`, recua esta largura à direita. */
export const HISTORICO_WIDTH = "288px";

/** As camadas do app, da mais baixa à mais alta. Quem precisa de `zIndex` lê daqui: a relação entre
 *  elas morava em comentários espalhados por doze arquivos. */
export const Z = {
  /** Barra de busca das listas (`sticky`). */
  barraFixa: 10,
  /** Caixa de mensagem do chat (`fixed`). */
  compositor: 20,
  /** Coluna "Minhas perguntas": a barra de composição é `fixed`, não desconta a barra de rolagem
   *  e avança alguns pixels sobre a borda dela. */
  painelHistorico: 21,
  /** Seletor do tipo de consulta, que abre para cima, sobre a conversa. */
  menuSuspenso: 30,
  /** `AppHeader` (`sticky`). */
  cabecalho: 100,
  /** Drawers e modais: acima do cabeçalho, senão ele cobre o topo do painel. */
  sobreposicao: 110,
} as const;
