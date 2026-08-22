import { useDeferredValue, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Box, Flex, Text } from "@chakra-ui/react";
import { SearchInput } from "../shared/SearchInput";
import { AsyncContent } from "../shared/AsyncContent";
import { buildHaystack, haystackMatches, parseQuery } from "../shared/textSearch";
import { useTributum } from "./useTributum";
import type { ItemBase, TributumCatalogo } from "./types";

interface Props<T extends ItemBase> {
  /** Título da seção, e o que entra no placeholder da busca e nas mensagens de vazio. */
  titulo: string;
  /** Qual das quatro listas do catálogo esta aba mostra. */
  secao: keyof TributumCatalogo;
  /** Substantivo do contador, singular e plural ("artigo"/"artigos"). */
  substantivo: [string, string];
  /** Campos que a busca enxerga. Um array por item, na ordem que fizer sentido para o leitor. */
  campos: (item: T) => (string | number | undefined)[];
  /** O cartão do item. `terms` já vem normalizado, pronto para o `Highlight`. */
  children: (item: T, terms: string[]) => ReactNode;
}

/**
 * O esqueleto das quatro listas do Tributum: barra de busca fixa no topo com contador, o gate de
 * carga/erro e a lista de cartões.
 *
 * **Componente compartilhado, e não quatro cópias**, porque aqui as quatro abas compartilham tudo
 * menos os campos exibidos — é o oposto do caso da `LegislacaoList`, que documenta por que NÃO
 * reusou a `JurisprudenciaList` (lá os dois modos não compartilhavam nada além da caixa de busca).
 * O que varia vem por prop; o que é igual mora uma vez só.
 *
 * A busca é a util da casa (`shared/textSearch`), não uma cópia local: mesma normalização de
 * acentos, mesmo E entre termos, e o mesmo descarte de letra isolada da pendência 37. O haystack é
 * pré-computado por item num `useMemo` e só quando há busca — quem só folheia não paga a
 * normalização, como nas listas do Acervo.
 */
export function TributumShell<T extends ItemBase>({
  titulo,
  secao,
  substantivo,
  campos,
  children,
}: Props<T>) {
  const { catalogo, erro, carregando } = useTributum();
  const itens = catalogo[secao] as unknown as T[];

  const [searchQuery, setSearchQuery] = useState("");
  const deferredQuery = useDeferredValue(searchQuery);
  const inputRef = useRef<HTMLInputElement>(null);
  const isSearching = deferredQuery.trim().length > 0;

  const terms = useMemo(() => parseQuery(deferredQuery), [deferredQuery]);
  const haystacks = useMemo(
    () =>
      isSearching
        ? itens.map((i) => buildHaystack(campos(i).map((c) => (c == null ? "" : String(c)))))
        : undefined,
    // `campos` é uma closure recriada a cada render pelo chamador; incluí-la faria o índice
    // refazer por tecla. O que ela lê é `itens`, que já está nas dependências.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [itens, isSearching],
  );
  const filtrados = useMemo(
    () => (haystacks ? itens.filter((_, n) => haystackMatches(haystacks[n], terms)) : itens),
    [itens, haystacks, terms],
  );

  function clearSearch() {
    setSearchQuery("");
    inputRef.current?.focus();
  }

  const [singular, plural] = substantivo;

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
          placeholder={`Pesquisar em ${titulo.toLowerCase()}...`}
        />
        {itens.length > 0 && (
          <Box pt={2}>
            <Text fontSize="0.8rem" color="fg.muted" fontWeight="500">
              {filtrados.length} {filtrados.length !== 1 ? plural : singular}
              {isSearching ? ` de ${itens.length}` : ""}
            </Text>
          </Box>
        )}
      </Box>

      <Box px={4} pt={3} pb={6}>
        <AsyncContent
          loading={carregando}
          error={erro}
          loadingText="Aguarde enquanto o Tributum é carregado…"
          errorTitle="Erro ao carregar o Tributum"
          errorDescription={
            erro instanceof Error
              ? erro.message
              : "O catálogo não pôde ser carregado. Tente novamente mais tarde."
          }
        >
          {filtrados.length === 0 ? (
            <Box py={10} textAlign="center">
              <Text color="fg.muted" fontSize="0.95rem">
                {isSearching
                  ? `Nenhum resultado encontrado para “${deferredQuery}”`
                  : `Nada em ${titulo.toLowerCase()} por enquanto.`}
              </Text>
            </Box>
          ) : (
            <Flex direction="column" gap={3}>
              {filtrados.map((item) => (
                <Box
                  key={item.id}
                  borderWidth="1px"
                  borderColor="border"
                  borderRadius="10px"
                  bg="bg.canvas"
                  px={4}
                  py={3}
                >
                  {children(item, terms)}
                </Box>
              ))}
            </Flex>
          )}
        </AsyncContent>
      </Box>
    </Flex>
  );
}
