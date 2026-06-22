import { Box, Button, Flex, Text, VStack } from "@chakra-ui/react";
import { ColorModeButton } from "../pages/chat/ui/color-mode.jsx";
import type { Entity } from "./entities";

interface AppHeaderProps {
  subtitle?: string;
  /** The active entity (state), if one is selected. */
  entity?: Entity | null;
  /** Return to the state-selection page. When omitted, no switcher is shown. */
  onChangeEntity?: () => void;
}

export const AppHeader = ({ subtitle, entity, onChangeEntity }: AppHeaderProps) => {
  return (
    <Box
      as="header"
      w="100%"
      py={3}
      px={3}
      bg="bg.inverted"
      borderBottom="1px solid var(--chakra-colors-border-inverted)"
      position="sticky"
      top={0}
      zIndex={100}
    >
      <Flex maxW="1440px" mx="auto" alignItems="center" justifyContent="space-between">
        {/* Left: active-state switcher (the rs/sc selector). */}
        <Box minW="96px" display="flex" justifyContent="flex-start">
          {entity && onChangeEntity && (
            <Button
              size="xs"
              variant="outline"
              onClick={onChangeEntity}
              aria-label={`Estado atual: ${entity.name}. Trocar de estado.`}
              title="Trocar de estado"
              color="fg.inverted"
              borderColor="border.inverted"
              _hover={{ bg: "whiteAlpha.200" }}
              borderRadius="full"
              px={3}
              gap={1.5}
            >
              <Box
                as="span"
                fontWeight="700"
                fontSize="0.7rem"
                bg="accent"
                color="accent.fg"
                borderRadius="full"
                px={1.5}
                py={0.5}
                lineHeight="1"
              >
                {entity.uf}
              </Box>
              <Text as="span" fontSize="0.72rem" color="fg.invertedMuted" fontWeight="500">
                trocar
              </Text>
            </Button>
          )}
        </Box>

        {/* Center */}
        <VStack gap={0} align="center">
          <Text
            fontSize="2rem"
            fontWeight="600"
            color="fg.inverted"
            lineHeight="1.2"
            letterSpacing="-0.01em"
            fontFamily='"SF Pro Display", system-ui, -apple-system, BlinkMacSystemFont, sans-serif'
          >
            Auli
          </Text>
          {subtitle && (
            <Text fontSize="0.75rem" color="fg.invertedMuted" lineHeight="1" fontWeight="400">
              {entity ? `${subtitle} · ${entity.name}` : subtitle}
            </Text>
          )}
        </VStack>

        {/* Right */}
        <Box minW="96px" display="flex" justifyContent="flex-end">
          <ColorModeButton />
        </Box>
      </Flex>
    </Box>
  );
};
