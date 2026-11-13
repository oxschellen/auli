import { useMemo } from "react";
import type { ReactNode } from "react";
import { Box, Flex, Text } from "@chakra-ui/react";
import { BarraDeBusca, SemResultados, useBusca } from "../../ui/busca";
import { AsyncContent } from "../../ui/AsyncContent";
import { buildHaystack, haystackMatches } from "../../lib/textSearch";
import { useTributum } from "./useTributum";
import { RodapeCurador } from "./RodapeCurador";
import type { Estante, ItemBase } from "./types";

interface Props<T extends ItemBase> {
  /** Título da seção, e o que entra no placeholder da busca e nas mensagens de vazio. */
  titulo: string;
  /** Qual das cinco estantes esta aba mostra. */
  secao: Estante;
  /** Substantivo do contador, singular e plural ("artigo"/"artigos"). */
  substantivo: [string, string];
  /** Campos que a busca enxerga. Um array por item, na ordem que fizer sentido para o leitor. */
  campos: (item: T) => (string | number | undefined)[];
  /** O cartão do item. `terms` já vem normalizado, pronto para o `Highlight`. */
  children: (item: T, terms: string[]) => ReactNode;
}

/**
 * O esqueleto das cinco listas do Tributum: barra de busca fixa no topo com contador, o gate de
 * carga/erro e a lista de cartões.
 *
 * **Componente compartilhado, e não cinco cópias**, porque aqui as cinco abas compartilham tudo
 * menos os campos exibidos — é o oposto do caso da `LegislacaoList`, que documenta por que NÃO
 * reusou a `JurisprudenciaList` (lá os dois modos não compartilhavam nada além da caixa de busca).
 * O que varia vem por prop; o que é igual mora uma vez só.
 *
 * A busca é a util da casa (`lib/textSearch`), não uma cópia local: mesma normalização de
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
  const curador = catalogo.curadores?.[secao];

  const busca = useBusca();
  const haystacks = useMemo(
    () =>
      busca.buscando
        ? itens.map((i) => buildHaystack(campos(i).map((c) => (c == null ? "" : String(c)))))
        : undefined,
    // `campos` é uma closure recriada a cada render pelo chamador; incluí-la faria o índice
    // refazer por tecla. O que ela lê é `itens`, que já está nas dependências.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [itens, busca.buscando],
  );
  const filtrados = useMemo(
    () => (haystacks ? itens.filter((_, n) => haystackMatches(haystacks[n], busca.termos)) : itens),
    [itens, haystacks, busca.termos],
  );

  const [singular, plural] = substantivo;

  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.app">
      <BarraDeBusca busca={busca} placeholder={`Pesquisar em ${titulo.toLowerCase()}...`}>
        {itens.length > 0 && (
          <>
            {filtrados.length} {filtrados.length !== 1 ? plural : singular}
            {busca.buscando ? ` de ${itens.length}` : ""}
          </>
        )}
      </BarraDeBusca>

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
          {filtrados.length === 0 && busca.buscando ? (
            <SemResultados consulta={busca.adiado} />
          ) : filtrados.length === 0 ? (
            <Box py={10} textAlign="center">
              <Text color="fg.muted" fontSize="0.95rem">
                Nada em {titulo.toLowerCase()} por enquanto.
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
                  {children(item, busca.termos)}
                </Box>
              ))}
            </Flex>
          )}
          {/* Fora do gate de vazio: a estante sem itens continua tendo curador, e é justamente
              onde saber a quem propor um item vale mais. */}
          <RodapeCurador curador={curador} />
        </AsyncContent>
      </Box>
    </Flex>
  );
}
