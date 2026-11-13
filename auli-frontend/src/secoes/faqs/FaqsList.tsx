import { useMemo } from "react";
import { Box, Flex } from "@chakra-ui/react";
import useSWR from "swr";
import { FaqsAccordion } from "./FaqsAccordion";
import {
  buildNodesFromJson,
  buildAnswerMap,
  buildPageTypeMap,
  contarPerguntas,
  searchNodes,
  type RawFaqNode,
} from "./faqs";
import { jsonFetcher, SWR_OPTS, entityPath } from "../../api/estaticos";
import { BarraDeBusca, useBusca } from "../../ui/busca";
import { AsyncContent } from "../../ui/AsyncContent";
import { useSelectedEntity } from "../../contexto/EntityContext";
import { hasCollection } from "../../contexto/entities";
import { CollectionEmpty } from "../../ui/CollectionEmpty";

export function FaqsList() {
  const entity = useSelectedEntity();
  const available = hasCollection(entity, "faqs");
  const { data, error, isLoading } = useSWR(
    available ? entityPath(entity.id, "faqs-tree.json") : null,
    jsonFetcher<RawFaqNode>,
    SWR_OPTS,
  );
  const nodes = useMemo(() => (data ? buildNodesFromJson(data) : []), [data]);
  const answerMap = useMemo(
    () => (data ? buildAnswerMap(data) : new Map<string, string>()),
    [data],
  );
  const pageTypeMap = useMemo(
    () => (data ? buildPageTypeMap(data) : new Map<string, string>()),
    [data],
  );
  const totalPerguntas = useMemo(() => (data ? contarPerguntas(data) : 0), [data]);
  const busca = useBusca();

  // A busca roda AQUI, e não no acordeão, porque o contador da barra fixa e a lista de resultados
  // precisam do mesmo número — computar nos dois lugares varreria a árvore duas vezes por tecla.
  const results = useMemo(
    () => (busca.buscando ? searchNodes(nodes, busca.adiado.trim()) : []),
    [nodes, busca.buscando, busca.adiado],
  );

  if (!available) return <CollectionEmpty entity={entity} label="FAQs" />;

  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.app">
      <BarraDeBusca busca={busca} placeholder="Pesquisar FAQs...">
        {totalPerguntas > 0 &&
          (busca.buscando
            ? // Sem "de N": um acerto pode ser uma pergunta OU o título de uma página/menu,
              // então não há denominador honesto — as duas unidades não são a mesma coisa.
              `${results.length} resultado${results.length !== 1 ? "s" : ""}`
            : `${totalPerguntas} pergunta${totalPerguntas !== 1 ? "s" : ""}`)}
      </BarraDeBusca>

      {/* Scrollable content */}
      <Box w="100%" px={4} pt={3} pb={6}>
        <AsyncContent
          loading={isLoading}
          error={error}
          loadingText="Aguarde enquanto as FAQs são carregadas…"
          errorDescription="Aconteceu um erro ao carregar as FAQs."
        >
          <FaqsAccordion
            nodes={nodes}
            searchQuery={busca.adiado}
            results={results}
            answerMap={answerMap}
            pageTypeMap={pageTypeMap}
          />
        </AsyncContent>
      </Box>
    </Flex>
  );
}
