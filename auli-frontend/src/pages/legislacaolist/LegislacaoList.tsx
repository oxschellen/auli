import { useDeferredValue, useMemo, useRef, useState } from "react";
import { Box, Flex, Link, Text } from "@chakra-ui/react";
import { MdExpandLess, MdExpandMore, MdOpenInNew } from "react-icons/md";
import useSWR from "swr";
import {
  agruparPorLei,
  buildLegislacaoIndex,
  caminhoNaLei,
  searchLegislacao,
  type Dispositivo,
} from "./legislacao";
import { jsonFetcher, SWR_OPTS, entityPath } from "../../shared/fetchers";
import { SearchInput } from "../../shared/SearchInput";
import { AsyncContent } from "../../shared/AsyncContent";
import { useSelectedEntity } from "../../shared/EntityContext";
import { hasCollection } from "../../shared/entities";
import { CollectionEmpty } from "../../shared/CollectionEmpty";
import { Highlight } from "../../shared/highlight";
import { parseQuery } from "../../shared/textSearch";

const TITULO = "Legislação";

/** Uma linha: o dispositivo à esquerda, a pergunta e o caminho na lei à direita. */
function ItemDispositivo({ d, terms }: { d: Dispositivo; terms: string[] }) {
  const caminho = caminhoNaLei(d.trilha);
  return (
    <Flex
      align="flex-start"
      gap={3}
      py={2}
      px={4}
      borderBottom="1px solid var(--chakra-colors-border)"
      _hover={{ bg: "bg.subtle" }}
      style={{ transition: "background 0.15s ease" }}
    >
      {/* O dispositivo é a identidade da linha e é por ele que o analista procura — daí vir
          primeiro, numa coluna de largura FIXA (`w`, não `minW`): com largura mínima, um
          "Art. 1º, parágrafo único" empurra a pergunta para a direita e o alinhamento da lista
          se perde linha a linha. Fixa, o texto longo quebra dentro da própria coluna. */}
      <Text
        fontSize="0.8rem"
        fontWeight="700"
        color="accent.fg.muted"
        w="7.5rem"
        flexShrink={0}
        pt="1px"
      >
        <Highlight text={d.ementa} terms={terms} />
      </Text>
      <Box flex={1} minW={0}>
        <Text fontSize="0.95rem" fontWeight="500" color="fg" lineHeight="1.3">
          <Highlight text={d.titulo} terms={terms} />
        </Text>
        {caminho && (
          <Text fontSize="0.75rem" color="fg.muted" lineHeight="1.35" mt={0.5}>
            <Highlight text={caminho} terms={terms} />
          </Text>
        )}
      </Box>
      {d.link && (
        <Link
          href={d.link}
          target="_blank"
          rel="noopener noreferrer"
          color="fg.muted"
          flexShrink={0}
          _hover={{ color: "accent" }}
          aria-label={`Abrir ${d.ementa} no portal oficial`}
        >
          <MdOpenInNew size={16} />
        </Link>
      )}
    </Flex>
  );
}

/** Um grupo: a lei, aberta por padrão, com os dispositivos dela na ordem do texto. */
function GrupoLei({
  lei,
  itens,
  terms,
}: {
  lei: string;
  itens: Dispositivo[];
  terms: string[];
}) {
  const [isOpen, setIsOpen] = useState(true);
  return (
    <Box mb={4}>
      <Flex
        align="center"
        justify="space-between"
        gap={2}
        py={2}
        px={4}
        bg="bg.canvas"
        borderRadius="8px"
        cursor="pointer"
        _hover={{ bg: "bg.subtle" }}
        onClick={() => setIsOpen((o) => !o)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            setIsOpen((o) => !o);
          }
        }}
        role="button"
        tabIndex={0}
        aria-expanded={isOpen}
      >
        <Text fontSize="0.95rem" fontWeight="700" color="fg" lineHeight="1.25">
          <Highlight text={lei} terms={terms} />
        </Text>
        <Flex align="center" gap={2} flexShrink={0}>
          <Text fontSize="0.8rem" color="fg.muted">
            {itens.length}
          </Text>
          <Box color="fg.muted" display="flex">
            {isOpen ? <MdExpandLess size={20} /> : <MdExpandMore size={20} />}
          </Box>
        </Flex>
      </Flex>
      {isOpen && (
        <Box mt={1}>
          {itens.map((d) => (
            <ItemDispositivo key={`${d.ementa}|${d.titulo}`} d={d} terms={terms} />
          ))}
        </Box>
      )}
    </Box>
  );
}

/**
 * Lista da coleção de legislação: busca no topo, um acordeão por lei embaixo.
 *
 * **Componente próprio, e não o `JurisprudenciaList`** (D-LEG-13): lá a lista é cronológica e plana,
 * e cada item abre para o `resumo`; aqui é uma hierarquia por lei, na ordem do texto legal, e não
 * há resumo para abrir — a resposta é o corpo, que vive no chat e no portal. Reusar significaria
 * um componente com dois modos que não compartilham nada além da caixa de busca.
 */
export const LegislacaoList = () => {
  const entity = useSelectedEntity();
  const available = hasCollection(entity, "legislacao");
  const {
    data: itens = [],
    error,
    isLoading,
  } = useSWR<Dispositivo[]>(
    available ? entityPath(entity.id, "legislacao-index.json") : null,
    jsonFetcher,
    SWR_OPTS,
  );

  const [searchQuery, setSearchQuery] = useState("");
  const deferredQuery = useDeferredValue(searchQuery);
  const inputRef = useRef<HTMLInputElement>(null);
  const isSearching = deferredQuery.trim().length > 0;

  // Índice preguiçoso, como nas irmãs: quem só folheia não paga a normalização.
  const searchIndex = useMemo(
    () => (isSearching ? buildLegislacaoIndex(itens) : undefined),
    [itens, isSearching],
  );
  const filtered = useMemo(
    () => searchLegislacao(itens, deferredQuery, searchIndex),
    [itens, deferredQuery, searchIndex],
  );
  const grupos = useMemo(() => agruparPorLei(filtered), [filtered]);
  const terms = useMemo(() => parseQuery(deferredQuery), [deferredQuery]);

  function clearSearch() {
    setSearchQuery("");
    inputRef.current?.focus();
  }

  if (!available) return <CollectionEmpty entity={entity} label={TITULO} />;

  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.app">
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
          placeholder="Pesquisar na legislação..."
        />
        {itens.length > 0 && (
          <Box pt={2}>
            <Text fontSize="0.8rem" color="fg.muted" fontWeight="500">
              {filtered.length} {filtered.length !== 1 ? "dispositivos" : "dispositivo"}
              {isSearching ? ` de ${itens.length}` : ""}
            </Text>
          </Box>
        )}
      </Box>

      <Box px={4} pt={3} pb={6}>
        <AsyncContent
          loading={isLoading}
          error={error}
          loadingText="Aguarde enquanto a legislação é carregada…"
          errorTitle="Erro ao carregar a legislação"
          errorDescription={error instanceof Error ? error.message : undefined}
        >
          {isSearching && filtered.length === 0 ? (
            <Box py={10} textAlign="center">
              <Text color="fg.muted" fontSize="0.95rem">
                Nenhum resultado encontrado para &ldquo;{deferredQuery}&rdquo;
              </Text>
            </Box>
          ) : (
            grupos.map(([lei, doLei]) => (
              <GrupoLei key={lei} lei={lei} itens={doLei} terms={terms} />
            ))
          )}
        </AsyncContent>
      </Box>
    </Flex>
  );
};
