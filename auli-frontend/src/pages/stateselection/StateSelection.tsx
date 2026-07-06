import { Flex, Stack, Text } from "@chakra-ui/react";
import { useEntity } from "../../shared/EntityContext";
import { BrazilMap } from "./BrazilMap";

/**
 * Landing page: pick the state (tax authority / entity) before entering the app. The whole session
 * is scoped to the chosen entity (data files + chat), so this is chosen once up front rather than
 * toggled mid-session.
 *
 * Selection is via the interactive Brazil map (available states in accent, others greyed).
 */
export function StateSelection() {
  const { selectEntity } = useEntity();

  return (
    <Flex direction="column" h="100%" w="100%" bg="bg.canvas" overflowY="auto">
      <Flex direction="column" align="center" maxW="720px" mx="auto" w="100%" px={5} py={10} flex={1}>
        <Stack gap={2} align="center" textAlign="center" mb={10}>
          <Text fontSize="1.6rem" fontWeight="700" color="fg" letterSpacing="-0.01em">
            Escolha o estado
          </Text>
          <Text fontSize="0.95rem" color="fg.muted" maxW="520px" lineHeight="1.6">
            Selecione o Estado no mapa para consultar serviços, perguntas frequentes e conversar com
            a assistente Auli.
          </Text>
        </Stack>

        {/* Interactive Brazil map */}
        <Flex justify="center" w="100%" maxW="506px">
          <BrazilMap onSelect={selectEntity} />
        </Flex>
      </Flex>
    </Flex>
  );
}
