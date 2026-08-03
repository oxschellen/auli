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
export const SIDEBAR_WIDTH = "210px";
