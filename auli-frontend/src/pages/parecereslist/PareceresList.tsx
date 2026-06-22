import { Flex, Box, Text } from "@chakra-ui/react";
import useSWR from "swr";
import { editLinks } from "../../shared/linkify";
import { textFetcher, SWR_OPTS, entityPath } from "../../shared/fetchers";
import { AsyncContent } from "../../shared/AsyncContent";
import { useSelectedEntity } from "../../shared/EntityContext";
import { hasCollection } from "../../shared/entities";
import { CollectionEmpty } from "../../shared/CollectionEmpty";

export const PareceresList = () => {
  const entity = useSelectedEntity();
  const available = hasCollection(entity, "pareceres");
  const { data: content = "", error, isLoading } = useSWR(
    available ? entityPath(entity.id, "portal-pareceres.txt") : null,
    textFetcher,
    SWR_OPTS,
  );

  if (!available) return <CollectionEmpty entity={entity} label="Pareceres" />;

  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.canvas">
      <Box w="100%" px={4} pt={3} pb={6}>
        <AsyncContent
          loading={isLoading}
          error={error}
          loadingText="Aguarde enquanto os Pareceres são carregados…"
        >
          <Text fontFamily="mono" whiteSpace="pre-wrap" color="fg" fontSize="sm">
            {editLinks(content)}
          </Text>
        </AsyncContent>
      </Box>
    </Flex>
  );
};

