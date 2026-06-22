import { chakra, Flex, SimpleGrid, Stack, Text } from "@chakra-ui/react";
import { ENTITIES, type Entity } from "../../shared/entities";
import { useEntity } from "../../shared/EntityContext";
import { BrazilMap } from "./BrazilMap";

/**
 * Landing page: pick the state (tax authority / entity) before entering the app. The whole session
 * is scoped to the chosen entity (data files + chat), so this is chosen once up front rather than
 * toggled mid-session.
 *
 * Layout: an interactive outlined Brazil map (available states in accent, others greyed) beside the
 * explicit entity cards. Both call the same `selectEntity`.
 */
export function StateSelection() {
  const { selectEntity } = useEntity();

  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.canvas" overflowY="auto">
      <Flex direction="column" align="center" maxW="1100px" mx="auto" w="100%" px={5} py={10} flex={1}>
        <Stack gap={2} align="center" textAlign="center" mb={10}>
          <Text fontSize="1.6rem" fontWeight="700" color="fg" letterSpacing="-0.01em">
            Escolha o estado
          </Text>
          <Text fontSize="0.95rem" color="fg.muted" maxW="520px" lineHeight="1.6">
            Selecione a Secretaria da Fazenda estadual no mapa ou na lista para consultar serviços,
            perguntas frequentes e conversar com a assistente Auli.
          </Text>
        </Stack>

        <Flex
          w="100%"
          gap={{ base: 8, md: 10 }}
          align="center"
          justify="center"
          direction={{ base: "column", md: "row" }}
        >
          {/* Interactive Brazil map */}
          <Flex flex="1" justify="center" w="100%" maxW="460px">
            <BrazilMap onSelect={selectEntity} />
          </Flex>

          {/* Entity cards */}
          <SimpleGrid columns={{ base: 1, sm: 2, md: 1 }} gap={4} flex="1" maxW="420px" w="100%">
            {ENTITIES.map((entity) => (
              <StateCard key={entity.id} entity={entity} onSelect={() => selectEntity(entity.id)} />
            ))}
          </SimpleGrid>
        </Flex>
      </Flex>
    </Flex>
  );
}

interface StateCardProps {
  entity: Entity;
  onSelect: () => void;
}

function StateCard({ entity, onSelect }: StateCardProps) {
  const collectionCount = entity.collections.length;

  return (
    <chakra.button
      type="button"
      onClick={onSelect}
      textAlign="left"
      p={5}
      borderRadius="12px"
      border="1px solid var(--chakra-colors-border)"
      bg="bg.app"
      cursor="pointer"
      transition="border-color 0.15s ease, transform 0.1s ease, box-shadow 0.15s ease"
      _hover={{
        borderColor: "accent",
        boxShadow: "cardHover",
        transform: "translateY(-2px)",
      }}
      _focusVisible={{ outline: "2px solid var(--chakra-colors-accent)", outlineOffset: "2px" }}
    >
      <Flex align="center" gap={3} mb={3}>
        <Flex
          align="center"
          justify="center"
          w="44px"
          h="44px"
          borderRadius="10px"
          bg="accent"
          color="accent.fg"
          fontWeight="700"
          fontSize="1.05rem"
          flexShrink={0}
        >
          {entity.uf}
        </Flex>
        <Stack gap={0}>
          <Text fontSize="1.05rem" fontWeight="600" color="fg" lineHeight="1.2">
            {entity.name}
          </Text>
          <Text fontSize="0.85rem" color="fg.muted" lineHeight="1.3">
            {entity.state}
          </Text>
        </Stack>
      </Flex>
      <Text fontSize="0.8rem" color="fg.muted">
        {collectionCount} {collectionCount === 1 ? "coleção disponível" : "coleções disponíveis"}
      </Text>
    </chakra.button>
  );
}
