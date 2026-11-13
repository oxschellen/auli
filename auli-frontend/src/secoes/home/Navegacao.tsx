import type { KeyboardEvent, RefObject } from "react";
import { Flex, Box, Text, chakra } from "@chakra-ui/react";
import { Tooltip } from "../../ui/tooltip";
import { GRUPOS, type Secao } from "../registro";

/**
 * A navegação entre as seções (I7 do docs/ESTUDO-frontend-organizacao.md): a lista por grupos,
 * usada na sidebar e no drawer, a navegação por setas e a pista de rolagem dos dois containers.
 */

/**
 * Pista de rolagem dos dois containers da lista de seções (sidebar e drawer): uma sombra na borda
 * de baixo enquanto há itens abaixo, e na de cima depois de rolar. A lista passa da altura de uma
 * tela de notebook (num 768 px, o navegador deixa ~650), e sem a pista o último grupo ficava abaixo
 * da dobra sem nada dizer que ela rolava.
 *
 * CSS puro: as duas capas (`local`) rolam com o conteúdo e, ao chegar a uma ponta, tapam a sombra
 * (`scroll`, presa à borda) daquele lado. Sem rolagem, as capas tapam as duas e nada aparece. Cor
 * de fundo via `bgColor`, não `bg`: o `bg` é o atalho `background` e apagaria as camadas.
 */
export const PISTA_DE_ROLAGEM = {
  bgColor: "bg.app",
  backgroundImage: [
    "linear-gradient(var(--chakra-colors-bg-app) 30%, transparent)",
    "linear-gradient(transparent, var(--chakra-colors-bg-app) 70%)",
    "radial-gradient(farthest-side at 50% 0, var(--chakra-colors-bg-scroll-cue), transparent)",
    "radial-gradient(farthest-side at 50% 100%, var(--chakra-colors-bg-scroll-cue), transparent)",
  ].join(", "),
  backgroundPosition: "center top, center bottom, center top, center bottom",
  backgroundRepeat: "no-repeat",
  backgroundSize: "100% 40px, 100% 40px, 100% 14px, 100% 14px",
  backgroundAttachment: "local, local, scroll, scroll",
} as const;

/**
 * Próxima aba HABILITADA a partir de `fromId`, andando `delta` (+1/−1) e dando a volta nas pontas.
 * Devolve `undefined` só se nenhuma estiver habilitada — impossível na prática, porque `chat` e
 * `about` nunca são desativados, mas é o que torna a função total e o laço garantidamente finito.
 *
 * Pura e exportada por causa do envolvimento de índice: `(((i + delta*passo) % n) + n) % n` é onde
 * mora o off-by-one, e delta negativo em `%` do JS devolve resto negativo.
 */
export function proximaHabilitada<T extends { id: string; disabled: boolean }>(
  itens: T[],
  fromId: string,
  delta: 1 | -1,
): T | undefined {
  const n = itens.length;
  const i = itens.findIndex((t) => t.id === fromId);
  if (i < 0) return undefined;
  for (let passo = 1; passo <= n; passo++) {
    const candidata = itens[(((i + delta * passo) % n) + n) % n];
    if (!candidata.disabled) return candidata;
  }
  return undefined;
}

export type MenuItem = Secao & { disabled: boolean };
type TabRefs = RefObject<Record<string, HTMLButtonElement | null>>;
/** Qual das duas listas: a da sidebar (desktop) ou a do drawer (mobile). Serve de prefixo dos
 *  ids E de seletor do mapa de refs, para que os dois não possam divergir. */
export type Lista = "tab" | "tabm";

/** Um item da sidebar/drawer. Botão largura total, ícone + rótulo, estados via cores do tema. */
const NavItem = ({
  item,
  escopo,
  idPrefix,
  isActive,
  onSelect,
  onKeyDown,
  tabRef,
  recolhida,
}: {
  item: MenuItem;
  escopo: string;
  idPrefix: Lista;
  isActive: boolean;
  onSelect: () => void;
  onKeyDown: (e: KeyboardEvent<HTMLButtonElement>) => void;
  tabRef: (el: HTMLButtonElement | null) => void;
  recolhida: boolean;
}) => (
  // Recolhida, o rótulo sai da vista mas não do leitor de tela (`srOnly`: é ele que nomeia a aba
  // e, por ela, o painel) e passa a aparecer como dica ao lado do ícone.
  <Tooltip content={item.label} disabled={!recolhida} positioning={{ placement: "right" }}>
    <chakra.button
      ref={tabRef}
      type="button"
      role="tab"
      id={`${escopo}-${idPrefix}-${item.id}`}
      aria-selected={isActive}
      aria-controls={`${escopo}-tabpanel-${item.id}`}
      // `aria-disabled`, não `disabled`: o segundo tira o botão da árvore de acessibilidade e o
      // leitor de tela nem anuncia que a seção existe — que é o que queremos comunicar.
      aria-disabled={item.disabled || undefined}
      // Roving tabindex: só a ativa está na ordem de Tab; as demais via setas.
      tabIndex={isActive ? 0 : -1}
      onClick={item.disabled ? undefined : onSelect}
      onKeyDown={onKeyDown}
      display="flex"
      alignItems="center"
      justifyContent={recolhida ? "center" : undefined}
      gap="10px"
      w="100%"
      px={recolhida ? 0 : "12px"}
      // 6px dá 34px de item (era 8px → 38px): 10% mais baixo, para a lista caber melhor na altura
      // de um notebook.
      py="6px"
      borderRadius="8px"
      cursor={item.disabled ? "not-allowed" : "pointer"}
      fontSize="0.9rem"
      fontFamily="body"
      fontWeight={isActive ? "600" : "500"}
      textAlign="left"
      // Ativa: fundo azul (`bg.navActive`) e texto em `accent.strong`, sem barra lateral. O
      // `bg.subtle` de antes era quase da cor do `bg.app` da sidebar e o destaque sumia no claro.
      color={item.disabled ? "fg.disabled" : isActive ? "accent.strong" : "fg.muted"}
      bg={isActive ? "bg.navActive" : "transparent"}
      _hover={item.disabled || isActive ? undefined : { bg: "bg.subtle", color: "fg" }}
      transition="color 0.15s ease, background 0.15s ease"
    >
      <item.Icone size="17px" aria-hidden="true" />
      <Box
        as="span"
        flex="1"
        overflow="hidden"
        textOverflow="ellipsis"
        whiteSpace="nowrap"
        srOnly={recolhida || undefined}
      >
        {item.label}
      </Box>
    </chakra.button>
  </Tooltip>
);

/**
 * A lista de navegação, por grupos. Usada na sidebar (desktop) e no drawer (mobile).
 *
 * `idPrefix` existe porque `hideBelow`/`hideFrom` escondem por CSS, não desmontam: com o drawer
 * aberto as DUAS listas ficam no DOM ao mesmo tempo, e sem prefixos distintos haveria dois
 * `id="tab-chat"`. O `aria-labelledby` dos painéis segue apontando para a lista da sidebar
 * (`tab-…`), que existe sempre — o accname lê elemento referenciado mesmo com `display:none`.
 * Pelo mesmo motivo cada lista tem seu próprio mapa de refs: um só faria o `.focus()` das setas
 * mirar a lista invisível, conforme a ordem em que os callbacks de ref rodam.
 *
 * `escopo` (o id da entidade) prefixa todos os ids pelo mesmo motivo, um nível acima: cada aba de
 * estado tem o seu `Home` montado, e duas abas abertas teriam dois `tab-chat` (`rs-tab-chat` e
 * `sp-tab-chat` não colidem — um estado só abre numa aba).
 */
export const NavList = ({
  menuItems,
  selectedId,
  escopo,
  idPrefix,
  onSelect,
  onKeyDown,
  tabRefs,
  recolhida = false,
}: {
  menuItems: MenuItem[];
  selectedId: string;
  escopo: string;
  idPrefix: Lista;
  onSelect: (id: string) => void;
  onKeyDown: (e: KeyboardEvent<HTMLButtonElement>) => void;
  tabRefs: TabRefs;
  /** Faixa de ícones (só a sidebar; o drawer é sempre a lista cheia). */
  recolhida?: boolean;
}) => (
  <Flex
    direction="column"
    h="100%"
    role="tablist"
    aria-label="Seções"
    aria-orientation="vertical"
    gap="2px"
  >
    {GRUPOS.map((g) => (
      // `role="none"` no invólucro e no título: o `tablist` exige `tab` como filhos possuídos, e
      // o agrupamento visual mete um div e um parágrafo no meio. Marcá-los como presentacionais
      // devolve os botões à posse do tablist sem perder o texto do título, que segue exposto.
      <Box key={g.id ?? "topo"} role="none">
        {g.titulo &&
          (recolhida ? (
            // Na faixa de ícones o título não cabe; vira um traço, senão os grupos se fundem.
            <Box role="none" h="1px" bg="border" mx="12px" my="10px" />
          ) : (
            <Text
              role="none"
              fontSize="0.68rem"
              color="fg.muted"
              letterSpacing="0.06em"
              textTransform="uppercase"
              px="12px"
              mt="14px"
              mb="4px"
            >
              {g.titulo}
            </Text>
          ))}
        {menuItems
          .filter((t) => t.grupo === g.id)
          .map((item) => (
            <NavItem
              key={item.id}
              item={item}
              escopo={escopo}
              idPrefix={idPrefix}
              isActive={item.id === selectedId}
              onSelect={() => onSelect(item.id)}
              onKeyDown={onKeyDown}
              tabRef={(el) => (tabRefs.current[item.id] = el)}
              recolhida={recolhida}
            />
          ))}
      </Box>
    ))}
  </Flex>
);
