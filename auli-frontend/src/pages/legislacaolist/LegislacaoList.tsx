import { useDeferredValue, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Box, Flex, Link, Text } from "@chakra-ui/react";
import { m, AnimatePresence, useReducedMotion } from "framer-motion";
import { MdExpandLess, MdExpandMore, MdOpenInNew } from "react-icons/md";
import ReactMarkdown from "react-markdown";
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
import { compactMarkdownComponents, markdownPlugins } from "../../shared/markdown";
import { rehypeHighlight } from "../../shared/rehypeHighlight";

const TITULO = "Legislação";

/** Renderiza o `<mark>` que o `rehypeHighlight` injeta, com o mesmo visual do `Highlight`. */
const markdownComponents = {
  ...compactMarkdownComponents,
  mark: ({ children }: { children?: ReactNode }) => (
    <Box as="mark" bg="bg.highlight" color="fg.highlight" px="0.1em" borderRadius="2px">
      {children}
    </Box>
  ),
};

/**
 * Uma linha: o dispositivo à esquerda, a pergunta e o caminho na lei à direita — e que **abre para
 * a resposta**, o `## corpo` do `.md` da árvore, que vem no índice (D-LEG-11a).
 *
 * O link do portal mudou de lugar quando a linha virou clicável: antes era um ícone `↗` na própria
 * linha, agora vive dentro do painel aberto, como no `ParecerItem`. Dois alvos de clique
 * concorrentes na mesma linha exigiriam `stopPropagation` e ainda deixariam o alvo do teclado
 * ambíguo — o portal continua a um clique, só que depois de abrir.
 */
function ItemDispositivo({ d, terms }: { d: Dispositivo; terms: string[] }) {
  const caminho = caminhoNaLei(d.trilha);
  const reduceMotion = useReducedMotion();
  const [isOpen, setIsOpen] = useState(false);
  // Sem termos o plugin é um no-op, mas evitamos até instanciá-lo no caso comum (sem busca).
  const rehypePlugins = useMemo(() => (terms.length ? [rehypeHighlight(terms)] : []), [terms]);

  return (
    <Box borderBottom="1px solid var(--chakra-colors-border)">
      <Flex
        align="flex-start"
        gap={3}
        py={2}
        px={4}
        bg={isOpen ? "bg.app" : undefined}
        cursor="pointer"
        _hover={{ bg: isOpen ? "bg.app" : "bg.subtle" }}
        style={{ transition: "background 0.15s ease" }}
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
        <Box color="fg.muted" display="flex" flexShrink={0} pt="1px">
          {isOpen ? <MdExpandLess size={20} /> : <MdExpandMore size={20} />}
        </Box>
      </Flex>

      <AnimatePresence initial={false}>
        {isOpen && (
          <m.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={reduceMotion ? { duration: 0 } : { duration: 0.22, ease: "easeInOut" }}
            style={{ overflow: "hidden" }}
          >
            <Box bg="bg.app" px={4} py={3} pl="calc(1rem + 7.5rem + 0.75rem)">
              {d.link && (
                <Link
                  href={d.link}
                  target="_blank"
                  rel="noopener noreferrer"
                  color="accent"
                  fontSize="0.85rem"
                  display="inline-flex"
                  alignItems="center"
                  gap={1}
                  mb={2}
                  aria-label={`Abrir ${d.ementa} no portal oficial`}
                >
                  <MdOpenInNew size={14} /> Abrir no portal
                </Link>
              )}
              <Box color="fg" fontSize="0.9rem" lineHeight="1.7">
                <ReactMarkdown
                  remarkPlugins={markdownPlugins}
                  rehypePlugins={rehypePlugins}
                  components={markdownComponents}
                >
                  {d.corpo}
                </ReactMarkdown>
              </Box>
            </Box>
          </m.div>
        )}
      </AnimatePresence>
    </Box>
  );
}

/**
 * Um grupo: a lei, **fechada por padrão**, com os dispositivos dela na ordem do texto.
 *
 * Fechada como nas irmãs (`PareceresAccordion`, `FaqsAccordion`, `ServicosAccordion`): a tela abre
 * mostrando o acervo inteiro em uma altura, e não uma lei despejada por cima das outras.
 *
 * Durante a BUSCA o grupo é forçado aberto e o toggle é inerte — mesmo tratamento do `ServicosList`.
 * Sem isso, quem pesquisa veria só cabeçalhos com contagem, com o resultado escondido atrás deles.
 */
function GrupoLei({
  lei,
  itens,
  terms,
  isSearching,
}: {
  lei: string;
  itens: Dispositivo[];
  terms: string[];
  isSearching: boolean;
}) {
  const [isOpen, setIsOpen] = useState(false);
  const aberto = isSearching || isOpen;
  const alternar = () => !isSearching && setIsOpen((o) => !o);
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
        onClick={alternar}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            alternar();
          }
        }}
        role="button"
        tabIndex={0}
        aria-expanded={aberto}
      >
        <Text fontSize="0.95rem" fontWeight="700" color="fg" lineHeight="1.25">
          <Highlight text={lei} terms={terms} />
        </Text>
        <Flex align="center" gap={2} flexShrink={0}>
          <Text fontSize="0.8rem" color="fg.muted">
            {itens.length}
          </Text>
          <Box color="fg.muted" display="flex">
            {aberto ? <MdExpandLess size={20} /> : <MdExpandMore size={20} />}
          </Box>
        </Flex>
      </Flex>
      {aberto && (
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
              <GrupoLei
                key={lei}
                lei={lei}
                itens={doLei}
                terms={terms}
                isSearching={isSearching}
              />
            ))
          )}
        </AsyncContent>
      </Box>
    </Flex>
  );
};
