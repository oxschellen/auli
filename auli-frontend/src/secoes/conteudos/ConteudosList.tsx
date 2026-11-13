import { Box, Flex } from "@chakra-ui/react";
import useSWR from "swr";
import { ConteudosAccordion } from "./ConteudosAccordion";
import { jsonFetcher, SWR_OPTS, entityPath } from "../../api/estaticos";
import { BarraDeBusca, useBusca } from "../../ui/busca";
import { AsyncContent } from "../../ui/AsyncContent";
import { useSelectedEntity } from "../../contexto/EntityContext";
import { hasCollection } from "../../contexto/entities";
import { CollectionEmpty } from "../../ui/CollectionEmpty";
import type { ConteudoTree } from "./conteudos";

export function ConteudosList() {
  const entity = useSelectedEntity();
  const available = hasCollection(entity, "conteudos");
  const { data, error, isLoading } = useSWR(
    available ? entityPath(entity.id, "conteudo_site_tree.json") : null,
    jsonFetcher<ConteudoTree>,
    SWR_OPTS,
  );
  const categories = data?.categories || [];
  const busca = useBusca();

  if (!available) return <CollectionEmpty entity={entity} label="Conteúdos" />;

  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.app">
      {/* Fixa no topo, como nas outras listas (antes rolava junto com o conteúdo). */}
      <BarraDeBusca busca={busca} placeholder="Pesquisar conteúdos..." />

      <Box w="100%" px={4} pt={3} pb={6}>
        <AsyncContent
          loading={isLoading}
          error={error}
          loadingText="Aguarde enquanto os conteúdos são carregados…"
          errorDescription="Aconteceu um erro ao carregar os conteúdos."
        >
          <ConteudosAccordion categories={categories} searchQuery={busca.adiado} />
        </AsyncContent>
      </Box>
    </Flex>
  );
}
