import { useMemo, useRef, useState, useDeferredValue } from "react";
import { Box, Button, Flex, Text } from "@chakra-ui/react";
import useSWR from "swr";
import { AccordionItem } from "./ServicosAccordion";
import {
  getDefaultTipoServicos,
  empresaPrimeiro,
  filterServicoGroups,
  contarServicosDistintos,
  type Servico,
  type TipoServico,
  type ServicoGroup,
} from "./utils";
import { jsonFetcher, SWR_OPTS, entityPath } from "../../shared/fetchers";
import { SearchInput } from "../../shared/SearchInput";
import { AsyncContent } from "../../shared/AsyncContent";
import { useSelectedEntity } from "../../shared/EntityContext";
import { hasCollection } from "../../shared/entities";
import { CollectionEmpty } from "../../shared/CollectionEmpty";
import { parseQuery } from "../../shared/textSearch";

export function ServicosList() {
  const entity = useSelectedEntity();
  const available = hasCollection(entity, "servicos");
  // Audience tabs are driven by `servicos-index.json` (emitted by the scraper per entity); fall back
  // to the hardcoded default list when the manifest is missing (older / RS-only deploys). Gated on
  // `available` so states without Serviços data never fire the fetch (no 404) — see CollectionEmpty.
  const { data: tipoIndex } = useSWR<TipoServico[]>(
    available ? entityPath(entity.id, "servicos-index.json") : null,
    jsonFetcher<TipoServico[]>,
    SWR_OPTS
  );
  // `empresaPrimeiro` reordena SÓ para exibição: a aba de empresa vai para a frente (e, por
  // consequência do `[0]` abaixo, vira a aba inicial). O JSON em disco e a ordem do dado não mudam.
  const tipoServicos = empresaPrimeiro(
    tipoIndex && tipoIndex.length > 0 ? tipoIndex : getDefaultTipoServicos(),
  );

  const [active, setActive] = useState("");
  const [openGroups, setOpenGroups] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState("");
  // Input stays instant; grouping/filtering derive from the deferred value.
  const deferredQuery = useDeferredValue(searchQuery);
  const inputRef = useRef<HTMLInputElement>(null);

  // Default to the first tab once the manifest (or fallback) is known.
  const activeIndex = Math.max(
    0,
    tipoServicos.findIndex((ts) => ts.tipo === active),
  );
  // The tab currently rendered as selected (falls back to the first tab before any click).
  const activeName = tipoServicos[activeIndex]?.tipo ?? "";

  // TODAS as abas de uma vez, não só a ativa: o contador em repouso é o tamanho da coleção
  // inteira, e sem as outras abas não há como saber quem se repete entre públicos. O custo é
  // pequeno — 413 KB no SP, 580 KB no RS, contra os 33 MB do índice de pareceres da aba vizinha —,
  // e trocar de aba passa a ser instantâneo, porque o dado já está aqui.
  const { data: abas, error, isLoading: loading } = useSWR(
    available && tipoServicos.length > 0
      ? ["servicos", entity.id, ...tipoServicos.map((ts) => ts.filename)]
      : null,
    ([, id, ...filenames]: string[]) =>
      Promise.all(
        filenames.map((f) => jsonFetcher<Servico[]>(entityPath(id, `${f}.json`))),
      ),
    SWR_OPTS,
  );
  // Memoizado porque o `?? []` devolveria um array novo a cada render, e ele alimenta o `grouped`.
  const activeServicos = useMemo(() => abas?.[activeIndex] ?? [], [abas, activeIndex]);
  const totalDistintos = useMemo(
    () => (abas ? contarServicosDistintos(abas) : 0),
    [abas],
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

  const filteredGroups = useMemo<ServicoGroup[]>(
    () => filterServicoGroups(grouped, deferredQuery),
    [grouped, deferredQuery]
  );

  // Uma vez por query, não por grupo: `parseQuery` devolve array novo a cada chamada.
  const terms = useMemo(() => parseQuery(deferredQuery), [deferredQuery]);

  const isSearching = deferredQuery.trim().length > 0;
  const totalResults = isSearching ? filteredGroups.reduce((sum, [, items]) => sum + items.length, 0) : 0;

  if (!available) return <CollectionEmpty entity={entity} label="Serviços" />;

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

        {totalDistintos > 0 && (
          <Box pt={2}>
            <Text fontSize="0.8rem" color="fg.muted" fontWeight="500">
              {isSearching
                ? // O denominador é a ABA, não a coleção: a busca só varre a aba aberta, e dizer
                  // "de 518" no SP contaria 145 serviços que ninguém pesquisou. O nome da aba
                  // explica por que o número encolheu em relação ao de repouso — e some quando a
                  // entidade tem público único, porque aí não há nada a explicar.
                  `${totalResults} de ${activeServicos.length}${tipoServicos.length > 1 ? ` em ${activeName}` : ""}`
                : `${totalDistintos} ${totalDistintos !== 1 ? "serviços" : "serviço"}`}
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
                    terms={terms}
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
