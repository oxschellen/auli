import { Box, Flex, Text, chakra } from "@chakra-ui/react";
import type { ReactNode } from "react";
import { LuPanelLeft } from "react-icons/lu";
import { ColorModeButton } from "../ui/color-mode.jsx";
import { Tooltip } from "../ui/tooltip";
import { HEADER_HEIGHT, SIDEBAR_WIDTH } from "../ui/layout";

interface AppHeaderProps {
  /** A versão do app. Saiu da vista (D-UI-4): vira a dica do logo — era informação de quem
   *  desenvolve ocupando a área mais nobre da tela. */
  subtitle?: string;
  /** Se a sidebar de seções está expandida; `false` = recolhida em faixa de ícones (só conta
   *  com `onToggleSidebar`). */
  sidebarAberta?: boolean;
  /** Expande/recolhe a sidebar. Sem ele (aba no mapa, que não tem sidebar), sem botão. */
  onToggleSidebar?: () => void;
  /** Id da sidebar da aba ativa, para o `aria-controls` do botão. */
  sidebarId?: string;
  /** A faixa de abas de estado, logo depois do logo. */
  children?: ReactNode;
}

/**
 * Cabeçalho compacto (D-UI-4): 52px na cor de superfície, com borda — era uma faixa preta de 77px
 * com o logo centralizado e a versão embaixo. A marca e os estados vão para a esquerda, onde a
 * leitura começa. O seletor "SEFAZ-RS ▾" deu lugar às abas de estado (`children`), uma por estado
 * aberto.
 */
export const AppHeader = ({
  subtitle,
  sidebarAberta,
  onToggleSidebar,
  sidebarId,
  children,
}: AppHeaderProps) => {
  const rotuloSidebar = sidebarAberta ? "Recolher a barra lateral" : "Expandir a barra lateral";
  // Com a sidebar aberta, as abas começam na linha dela: a marca ocupa a largura da sidebar menos
  // o padding do cabeçalho e o gap até as abas. Recolhida (64px), a faixa é mais estreita que
  // botão + logo, e as abas seguem o logo; sem sidebar (aba no mapa, ou abaixo de md), idem.
  const inicioDasAbas =
    onToggleSidebar && sidebarAberta
      ? `calc(${SIDEBAR_WIDTH} - var(--chakra-spacing-4) - var(--chakra-spacing-3))`
      : undefined;
  return (
    <Box
      as="header"
      w="100%"
      h={HEADER_HEIGHT}
      flex="none"
      px={{ base: 2, md: 4 }}
      bg="bg.canvas"
      borderBottom="1px solid"
      borderColor="border"
      position="sticky"
      top={0}
      zIndex={100}
    >
      <Flex h="100%" alignItems="center" justifyContent="space-between" gap={3}>
        <Flex alignItems="center" gap={3} flex="1" minW={0}>
          <Flex alignItems="center" gap={3} flexShrink={0} minW={{ md: inicioDasAbas }}>
            {/* Só a partir de md: abaixo dele a sidebar já é o drawer, com o seu botão de menu.
                O atalho (Ctrl+B) é ouvido no AppShell; aqui só é anunciado. */}
            {onToggleSidebar && (
              <Tooltip content={`${rotuloSidebar} (Ctrl+B)`}>
                <chakra.button
                  type="button"
                  hideBelow="md"
                  aria-label={rotuloSidebar}
                  aria-expanded={sidebarAberta}
                  aria-controls={sidebarId}
                  aria-keyshortcuts="Control+B"
                  onClick={onToggleSidebar}
                  display="flex"
                  alignItems="center"
                  p="6px"
                  borderRadius="8px"
                  color="fg.muted"
                  _hover={{ bg: "bg.overlay", color: "fg" }}
                >
                  <LuPanelLeft size={20} aria-hidden="true" />
                </chakra.button>
              </Tooltip>
            )}
            <Text
              as="span"
              fontSize="20px"
              fontWeight="700"
              letterSpacing="-0.02em"
              color="fg"
              pl={{ base: 1, md: 0 }}
              flexShrink={0}
              title={subtitle}
              fontFamily='"SF Pro Display", system-ui, -apple-system, BlinkMacSystemFont, sans-serif'
            >
              Auli
            </Text>
          </Flex>
          {children}
        </Flex>
        <ColorModeButton color="fg" _hover={{ bg: "bg.overlay" }} borderRadius="8px" />
      </Flex>
    </Box>
  );
};
