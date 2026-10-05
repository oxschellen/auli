import { Box, Button, Flex, Text } from "@chakra-ui/react";
import { MdExpandMore } from "react-icons/md";
import { ColorModeButton } from "../pages/chat/ui/color-mode.jsx";
import type { Entity } from "./entities";

interface AppHeaderProps {
  /** A versão do app. Saiu da vista (D-UI-4): vira a dica do logo — era informação de quem
   *  desenvolve ocupando a área mais nobre da tela. */
  subtitle?: string;
  /** The active entity (state), if one is selected. */
  entity?: Entity | null;
  /** Return to the state-selection page. When omitted, no switcher is shown. */
  onChangeEntity?: () => void;
}

/**
 * Cabeçalho compacto (D-UI-4): 52px na cor de superfície, com borda — era uma faixa preta de 77px
 * com o logo centralizado e a versão embaixo. A marca e o estado vão para a esquerda, onde a
 * leitura começa; o estado vira um seletor que diz o que é ("SEFAZ-RS ▾"), não "RS trocar".
 */
export const AppHeader = ({ subtitle, entity, onChangeEntity }: AppHeaderProps) => {
  return (
    <Box
      as="header"
      w="100%"
      h="52px"
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
        <Flex alignItems="center" gap={3} minW={0}>
          <Text
            as="span"
            fontSize="20px"
            fontWeight="700"
            letterSpacing="-0.02em"
            color="fg"
            pl={{ base: 1, md: 0 }}
            title={subtitle}
            fontFamily='"SF Pro Display", system-ui, -apple-system, BlinkMacSystemFont, sans-serif'
          >
            Auli
          </Text>
          {entity && onChangeEntity && (
            <Button
              size="sm"
              variant="outline"
              onClick={onChangeEntity}
              aria-label={`Estado atual: ${entity.name}. Trocar de estado.`}
              title="Trocar de estado"
              h="32px"
              px={2}
              gap={1.5}
              borderRadius="8px"
              borderColor="border"
              color="fg"
              fontWeight="500"
              _hover={{ bg: "bg.overlay" }}
            >
              <Box as="span" fontWeight="700" fontSize="11px" bg="accent" color="accent.fg" borderRadius="5px" px={1.5} lineHeight="20px">
                {entity.uf}
              </Box>
              <Text as="span" fontSize="14px" display={{ base: "none", sm: "inline" }}>
                {entity.name}
              </Text>
              <MdExpandMore size={16} color="var(--chakra-colors-fg-muted)" />
            </Button>
          )}
        </Flex>
        <ColorModeButton color="fg" _hover={{ bg: "bg.overlay" }} borderRadius="8px" />
      </Flex>
    </Box>
  );
};
