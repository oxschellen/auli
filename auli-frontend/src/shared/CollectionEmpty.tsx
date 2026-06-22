import { Flex, Stack, Text } from "@chakra-ui/react";
import { MdInbox } from "react-icons/md";
import type { Entity } from "./entities";

interface CollectionEmptyProps {
  entity: Entity;
  /** Human label of the collection, e.g. "Pareceres". */
  label: string;
}

/**
 * Friendly placeholder shown when the selected state has no data for a collection yet (e.g. SC has
 * only Serviços scraped). Replaces an error/404 with a clear "coming soon for this state" message.
 */
export function CollectionEmpty({ entity, label }: CollectionEmptyProps) {
  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.app" justify="center" align="center" py={16}>
      <Stack gap={3} align="center" maxW="360px" textAlign="center" px={4}>
        <MdInbox size={40} color="var(--chakra-colors-fg-muted)" />
        <Text fontSize="1.05rem" fontWeight="600" color="fg">
          {label} ainda não disponível para {entity.uf}
        </Text>
        <Text fontSize="0.9rem" color="fg.muted" lineHeight="1.6">
          Os dados de {label} para {entity.name} ({entity.state}) ainda não foram coletados. Em breve
          esta seção estará disponível.
        </Text>
      </Stack>
    </Flex>
  );
}
