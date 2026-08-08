import { useMemo, useRef, useState, useDeferredValue } from "react";
import { Box, Flex, Text } from "@chakra-ui/react";
import useSWR from "swr";
import { PareceresAccordion } from "./PareceresAccordion";
import { buildPareceresIndex, searchPareceres, type Parecer } from "./pareceres";
import { jsonFetcher, SWR_OPTS, entityPath } from "../../shared/fetchers";
import { SearchInput } from "../../shared/SearchInput";
import { AsyncContent } from "../../shared/AsyncContent";
import { useSelectedEntity } from "../../shared/EntityContext";
import { hasCollection, type Collection } from "../../shared/entities";
import { CollectionEmpty } from "../../shared/CollectionEmpty";

/**
 * O vocabulário de UMA coleção de jurisprudência. É tudo o que difere entre as listas — o resto
 * (busca, índice preguiçoso, acordeão, estados de carga) é idêntico, porque o `.md` de origem tem o
 * mesmo formato desde a unificação do vocabulário das árvores.
 */
export interface Jurisprudencia {
  /** Coleção no registry: decide se a aba está disponível E o nome do índice a buscar. */
  colecao: Collection;
  /** Rótulo humano, para o estado vazio e as mensagens de carga/erro. */
  titulo: string;
  /** Como um documento é chamado ("parecer", "acórdão"). */
  singular: string;
  /** O plural ("pareceres", "acórdãos") — não derivável do singular em português. */
  plural: string;
  /**
   * Aviso curto sob a contagem, quando o acervo publicado é menor que o do portal.
   *
   * Publicar um recorte é legítimo; o recorte passar por acervo, não. Quem vê "3.800 acórdãos"
   * numa aba chamada "Acórdãos TARF" assume que são os 3.800 que existem. Ausente = acervo
   * completo, e nada é renderizado.
   */
  nota?: string;
}

/**
 * Lista de uma coleção de jurisprudência: busca no topo, acordeão embaixo.
 *
 * Genérica porque as duas coleções (pareceres e acórdãos do TARF) consomem o MESMO artefato — o
 * `<id>-<colecao>-index.json` derivado por `auli-collections <id> indice <colecao>` — com o mesmo
 * shape. Duplicar a componente significaria manter duas cópias da lógica de busca e da renderização
 * progressiva, que é onde moram os cuidados de desempenho.
 */
export const JurisprudenciaList = ({
  colecao,
  titulo,
  singular,
  plural,
  nota,
}: Jurisprudencia) => {
  const entity = useSelectedEntity();
  const available = hasCollection(entity, colecao);
  const {
    data: documentos = [],
    error,
    isLoading,
  } = useSWR<Parecer[]>(
    available ? entityPath(entity.id, `${colecao}-index.json`) : null,
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
  // busca paga uma vez em vez de a cada letra. Vale ainda mais no TARF, que é o maior acervo
  // (22,5 mil acórdãos) e cujo `resumo` é a fundamentação inteira, não uma sinopse curta.
  const searchIndex = useMemo(
    () => (isSearching ? buildPareceresIndex(documentos) : undefined),
    [documentos, isSearching],
  );

  const filtered = useMemo(
    () => searchPareceres(documentos, deferredQuery, searchIndex),
    [documentos, deferredQuery, searchIndex],
  );

  function clearSearch() {
    setSearchQuery("");
    inputRef.current?.focus();
  }

  if (!available) return <CollectionEmpty entity={entity} label={titulo} />;

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
          placeholder={`Pesquisar ${plural}...`}
        />
        {documentos.length > 0 && (
          <Box pt={2}>
            <Text fontSize="0.8rem" color="fg.muted" fontWeight="500">
              {filtered.length} {filtered.length !== 1 ? plural : singular}
              {isSearching ? ` de ${documentos.length}` : ""}
            </Text>
            {nota && (
              <Text fontSize="0.75rem" color="fg.muted" pt={1}>
                {nota}
              </Text>
            )}
          </Box>
        )}
      </Box>

      {/* Scrollable content */}
      <Box px={4} pt={3} pb={6}>
        <AsyncContent
          loading={isLoading}
          error={error}
          loadingText={`Aguarde enquanto os ${titulo} são carregados…`}
          errorTitle={`Erro ao carregar ${plural}`}
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
