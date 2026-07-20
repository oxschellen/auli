import { useMemo, useRef, useState, useDeferredValue } from "react";
import { Box, Flex, Text } from "@chakra-ui/react";
import useSWR from "swr";
import { PareceresAccordion } from "./PareceresAccordion";
import { buildPareceresIndex, searchPareceres, type Parecer } from "./pareceres";
import { jsonFetcher, SWR_OPTS, entityPath } from "../../shared/fetchers";
import { SearchInput } from "../../shared/SearchInput";
import { AsyncContent } from "../../shared/AsyncContent";
import { useSelectedEntity } from "../../shared/EntityContext";
import { hasCollection } from "../../shared/entities";
import { CollectionEmpty } from "../../shared/CollectionEmpty";

export const PareceresList = () => {
  const entity = useSelectedEntity();
  const available = hasCollection(entity, "pareceres");
  const { data: pareceres = [], error, isLoading } = useSWR<Parecer[]>(
    available ? entityPath(entity.id, "pareceres-index.json") : null,
    jsonFetcher,
    SWR_OPTS,
  );

  const [searchQuery, setSearchQuery] = useState("");
  // Input stays instant; parsing/filtering derive from the deferred value.
  const deferredQuery = useDeferredValue(searchQuery);
  const inputRef = useRef<HTMLInputElement>(null);

  const isSearching = deferredQuery.trim().length > 0;

  // Índice de busca pré-normalizado, construído na PRIMEIRA busca e reusado nas teclas seguintes.
  // Preguiçoso de propósito: quem só folheia a lista nunca paga os ~400 ms do acervo do SP, e quem
  // busca paga uma vez em vez de a cada letra.
  const searchIndex = useMemo(
    () => (isSearching ? buildPareceresIndex(pareceres) : undefined),
    [pareceres, isSearching],
  );

  const filtered = useMemo(
    () => searchPareceres(pareceres, deferredQuery, searchIndex),
    [pareceres, deferredQuery, searchIndex],
  );

  function clearSearch() {
    setSearchQuery("");
    inputRef.current?.focus();
  }

  if (!available) return <CollectionEmpty entity={entity} label="Pareceres" />;

  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.app">
      {/* Sticky search + count */}
      <Box
        position="sticky"
        top={0}
        zIndex={10}
        bg="bg.app"
        px={4}
        pt={3}
        pb={2}
        borderBottom="1px solid var(--chakra-colors-border)"
      >
        <SearchInput
          ref={inputRef}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onClear={clearSearch}
          placeholder="Pesquisar pareceres..."
        />
        {pareceres.length > 0 && (
          <Box pt={2}>
            <Text fontSize="0.8rem" color="fg.muted" fontWeight="500">
              {filtered.length} parecer{filtered.length !== 1 ? "es" : ""}
              {isSearching ? ` de ${pareceres.length}` : ""}
            </Text>
          </Box>
        )}
      </Box>

      {/* Scrollable content */}
      <Box px={4} pt={3} pb={6}>
        <AsyncContent
          loading={isLoading}
          error={error}
          loadingText="Aguarde enquanto os Pareceres são carregados…"
          errorTitle="Erro ao carregar pareceres"
          errorDescription={error instanceof Error ? error.message : undefined}
        >
          {isSearching && filtered.length === 0 ? (
            <Box py={10} textAlign="center">
              <Text color="fg.muted" fontSize="0.95rem">
                Nenhum resultado encontrado para &ldquo;{deferredQuery}&rdquo;
              </Text>
            </Box>
          ) : (
            <PareceresAccordion pareceres={filtered} searchQuery={deferredQuery} />
          )}
        </AsyncContent>
      </Box>
    </Flex>
  );
};
