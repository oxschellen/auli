import { useMemo, useRef, useState, useDeferredValue } from "react";
import { Box, Button, Flex, Text } from "@chakra-ui/react";
import useSWR from "swr";
import { AccordionItem } from "./ServicosAccordion";
import { getDefaultTipoServicos, type Servico, type TipoServico } from "./utils";
import { jsonFetcher, SWR_OPTS, entityPath } from "../../shared/fetchers";
import { SearchInput } from "../../shared/SearchInput";
import { AsyncContent } from "../../shared/AsyncContent";
import { useSelectedEntity } from "../../shared/EntityContext";

/** A class heading paired with its (possibly filtered) services. */
type ServicoGroup = [string, Servico[]];

export function ServicosList() {
  const entity = useSelectedEntity();
  // Audience tabs are driven by `servicos-index.json` (emitted by the scraper per entity); fall back
  // to the hardcoded default list when the manifest is missing (older / RS-only deploys).
  const { data: tipoIndex } = useSWR<TipoServico[]>(
    entityPath(entity.id, "servicos-index.json"),
    jsonFetcher<TipoServico[]>,
    SWR_OPTS
  );
  const tipoServicos =
    tipoIndex && tipoIndex.length > 0 ? tipoIndex : getDefaultTipoServicos();

  const [active, setActive] = useState("");
  const [openGroups, setOpenGroups] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState("");
  // Input stays instant; grouping/filtering derive from the deferred value.
  const deferredQuery = useDeferredValue(searchQuery);
  const inputRef = useRef<HTMLInputElement>(null);

  // Default to the first tab once the manifest (or fallback) is known.
  const activeTipo =
    tipoServicos.find((ts) => ts.tipo === active) ?? tipoServicos[0];
  // The tab currently rendered as selected (falls back to the first tab before any click).
  const activeName = activeTipo?.tipo ?? "";

  const { data: activeServicos = [], error, isLoading: loading } = useSWR(
    activeTipo ? entityPath(entity.id, `${activeTipo.filename}.json`) : null,
    jsonFetcher<Servico[]>,
    SWR_OPTS
  );

  function selectTipo(tipo: string) {
    if (tipo === active) return;
    setActive(tipo);
    setOpenGroups(new Set());
    setSearchQuery("");
  }

  function clearSearch() {
    setSearchQuery("");
    inputRef.current?.focus();
  }

  function toggleGroup(classe: string) {
    setOpenGroups((prev) => {
      const next = new Set(prev);
      if (next.has(classe)) next.delete(classe);
      else next.add(classe);
      return next;
    });
  }

  const grouped = useMemo<ServicoGroup[]>(() => {
    const map = new Map<string, Servico[]>();
    for (const svc of activeServicos) {
      const list = map.get(svc.classe) ?? [];
      map.set(svc.classe, [...list, svc]);
    }
    return Array.from(map.entries());
  }, [activeServicos]);

  const filteredGroups = useMemo<ServicoGroup[]>(() => {
    const query = deferredQuery.trim().toLowerCase();
    if (!query) return grouped;
    const result: ServicoGroup[] = [];
    for (const [classe, items] of grouped) {
      if (classe.toLowerCase().includes(query)) {
        result.push([classe, items]);
      } else {
        const matching = items.filter((s) => s.titulo.toLowerCase().includes(query));
        if (matching.length > 0) result.push([classe, matching]);
      }
    }
    return result;
  }, [grouped, deferredQuery]);

  const isSearching = deferredQuery.trim().length > 0;
  const totalResults = isSearching ? filteredGroups.reduce((sum, [, items]) => sum + items.length, 0) : 0;

  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.app">
      {/* Sticky controls: tabs + search + result count */}
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
        <Flex mb={3} gap={2} flexWrap="wrap">
          {tipoServicos.map((ts) => (
            <Button
              key={ts.filename}
              size="sm"
              h="30px"
              px={2}
              onClick={() => selectTipo(ts.tipo)}
              bg={activeName === ts.tipo ? "accent" : "transparent"}
              color={activeName === ts.tipo ? "accent.fg" : "fg"}
              border="1px solid"
              borderColor={activeName === ts.tipo ? "accent" : "border"}
              borderRadius="md"
              _hover={{ bg: activeName === ts.tipo ? "brand.600" : "bg.subtle" }}
            >
              {ts.tipo}
            </Button>
          ))}
        </Flex>

        <SearchInput
          ref={inputRef}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onClear={clearSearch}
          placeholder="Pesquisar serviços..."
        />

        {isSearching && (
          <Box pt={2}>
            <Text fontSize="0.8rem" color="fg.muted" fontWeight="500">
              {totalResults} resultado{totalResults !== 1 ? "s" : ""} encontrado{totalResults !== 1 ? "s" : ""}
            </Text>
          </Box>
        )}
      </Box>

      {/* Scrollable content */}
      <Box px={4} pt={3} pb={6}>
        <AsyncContent
          loading={loading}
          error={error}
          loadingText="Aguarde enquanto os serviços são carregados…"
          errorTitle="Erro ao carregar serviços"
          errorDescription={error?.message}
        >
          <Box>
            {isSearching && filteredGroups.length === 0 ? (
              <Box py={10} textAlign="center">
                <Text color="fg.muted" fontSize="0.95rem">
                  Nenhum resultado encontrado para &ldquo;{deferredQuery}&rdquo;
                </Text>
              </Box>
            ) : (
              <Box border="1px solid var(--chakra-colors-border)" borderRadius="6px">
                {filteredGroups.map(([classe, items], idx) => (
                  <AccordionItem
                    key={classe}
                    classe={classe}
                    items={items}
                    isOpen={isSearching || openGroups.has(classe)}
                    onToggle={() => !isSearching && toggleGroup(classe)}
                    isFirst={idx === 0}
                    isLast={idx === filteredGroups.length - 1}
                  />
                ))}
              </Box>
            )}
          </Box>
        </AsyncContent>
      </Box>
    </Flex>
  );
}
