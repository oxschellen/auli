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
