import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Box, Flex, Link, Text } from "@chakra-ui/react";
import { m, AnimatePresence, useReducedMotion } from "framer-motion";
import { MdExpandMore, MdExpandLess, MdOpenInNew } from "react-icons/md";
import ReactMarkdown from "react-markdown";
import type { Parecer } from "./pareceres";
import { compactMarkdownComponents, markdownPlugins } from "../../shared/markdown";
import { Highlight } from "../../shared/highlight";
import { parseQuery } from "../../shared/textSearch";
import { rehypeHighlight } from "../../shared/rehypeHighlight";

/** Renderiza o `<mark>` que o `rehypeHighlight` injeta, com o mesmo visual do `Highlight`. */
const markdownComponents = {
  ...compactMarkdownComponents,
  mark: ({ children }: { children?: ReactNode }) => (
    <Box as="mark" bg="bg.highlight" color="fg.highlight" px="0.1em" borderRadius="2px">
      {children}
    </Box>
  ),
};

/** Uma linha de parecer: cabeçalho (número + assunto) que abre para a sinopse + link do portal. */
function ParecerItem({ p, terms }: { p: Parecer; terms: string[] }) {
  const reduceMotion = useReducedMotion();
  const [isOpen, setIsOpen] = useState(false);
  // Sem termos o plugin é um no-op, mas evitamos até instanciá-lo no caso comum (sem busca).
  const rehypePlugins = useMemo(
    () => (terms.length ? [rehypeHighlight(terms)] : []),
    [terms],
  );

  return (
    <Box borderBottom="1px solid var(--chakra-colors-border)">
      <Flex
        align="center"
        justify="space-between"
        gap={2}
        py={2}
        px={4}
        bg={isOpen ? "bg.app" : "bg.canvas"}
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
        <Box flex={1} minW={0}>
          <Text fontSize="0.95rem" fontWeight="600" color="fg" lineHeight="1.25">
            <Highlight text={p.numero} terms={terms} />
          </Text>
          {p.assunto && (
            <Text fontSize="0.85rem" color="fg.muted" lineHeight="1.35" mt={0.5}>
              <Highlight text={p.assunto} terms={terms} />
            </Text>
          )}
        </Box>
        <Box color="fg.muted" display="flex" flexShrink={0}>
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
            <Box bg="bg.app" px={4} py={3}>
              {p.link && (
                <Link
                  href={p.link}
                  target="_blank"
                  rel="noopener noreferrer"
                  color="accent"
                  fontSize="0.85rem"
                  display="inline-flex"
                  alignItems="center"
                  gap={1}
                  mb={2}
                >
                  <MdOpenInNew size={14} /> Abrir no portal
                </Link>
              )}
              {p.resumo ? (
                <Box color="fg" fontSize="0.9rem" lineHeight="1.7">
                  <ReactMarkdown
                    remarkPlugins={markdownPlugins}
                    rehypePlugins={rehypePlugins}
                    components={markdownComponents}
                  >
                    {p.resumo}
                  </ReactMarkdown>
                </Box>
              ) : (
                // A árvore permite documento sem sinopse (pendente); o índice o traz assim mesmo,
                // para não sumir da listagem. O link acima continua sendo o caminho da íntegra.
                <Text color="fg.muted" fontSize="0.85rem" fontStyle="italic">
                  Sinopse ainda não disponível — abra no portal para o texto integral.
                </Text>
              )}
            </Box>
          </m.div>
        )}
      </AnimatePresence>
    </Box>
  );
}

/**
 * Quantas linhas entram por vez. Cada linha custa ~9 elementos de DOM, então o acervo do SP (15,6
 * mil) montava ~140 mil elementos num render síncrono — segundos de thread principal travada, com a
 * barra de busca sem aceitar foco. Renderizamos um lote e crescemos conforme o scroll chega perto do
 * fim: o que o usuário vê é idêntico, o custo inicial vira uma fração.
 */
const LOTE = 100;

export function PareceresAccordion({
  pareceres,
  searchQuery,
}: {
  pareceres: Parecer[];
  searchQuery: string;
}) {
  // Uma vez por query, não por parecer: `parseQuery` devolve array novo a cada chamada.
  const terms = useMemo(() => parseQuery(searchQuery), [searchQuery]);

  const [limite, setLimite] = useState(LOTE);
  const sentinela = useRef<HTMLDivElement>(null);

  // Lista nova (a busca mudou) volta ao primeiro lote — senão uma busca larga herdaria o limite alto
  // que o scroll anterior acumulou. Ajuste durante o render, não em effect: um `setState` em effect
  // provoca render em cascata (e a regra `react-hooks/set-state-in-effect` reclama, com razão).
  const [listaAnterior, setListaAnterior] = useState(pareceres);
  if (listaAnterior !== pareceres) {
    setListaAnterior(pareceres);
    setLimite(LOTE);
  }

  // Sem IntersectionObserver (jsdom, navegador antigo) degrada para renderizar tudo: melhor lento
  // que truncado — parecer fora do DOM E fora do alcance do scroll seria documento inacessível.
  // Derivado, não sincronizado por effect, pelo mesmo motivo acima.
  const suportaIO = typeof IntersectionObserver !== "undefined";
  const visiveis = suportaIO ? Math.min(limite, pareceres.length) : pareceres.length;

  useEffect(() => {
    if (!suportaIO || visiveis >= pareceres.length) return;
    const alvo = sentinela.current;
    if (!alvo) return;
    const obs = new IntersectionObserver(
      ([entrada]) => {
        if (entrada.isIntersecting) setLimite((l) => Math.min(l + LOTE, pareceres.length));
      },
      { rootMargin: "600px" }, // carrega antes de o usuário chegar ao fim
    );
    obs.observe(alvo);
    return () => obs.disconnect();
  }, [suportaIO, visiveis, pareceres.length]);

  return (
    <Box border="1px solid var(--chakra-colors-border)" borderRadius="6px" overflow="hidden">
      {pareceres.slice(0, visiveis).map((p) => (
        <ParecerItem key={p.numero} p={p} terms={terms} />
      ))}
      {visiveis < pareceres.length && <Box ref={sentinela} h="1px" />}
    </Box>
  );
}
