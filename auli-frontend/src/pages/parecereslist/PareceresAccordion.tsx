import { useMemo, useState } from "react";
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

/** Uma linha de parecer: cabeçalho (número + assunto) que abre para a sinopse + link do portal. */
/** Renderiza o `<mark>` que o `rehypeHighlight` injeta, com o mesmo visual do `Highlight`. */
const markdownComponents = {
  ...compactMarkdownComponents,
  mark: ({ children }: { children?: ReactNode }) => (
    <Box as="mark" bg="bg.highlight" color="fg.highlight" px="0.1em" borderRadius="2px">
      {children}
    </Box>
  ),
};

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

export function PareceresAccordion({
  pareceres,
  searchQuery,
}: {
  pareceres: Parecer[];
  searchQuery: string;
}) {
  // Uma vez por query, não por parecer: `parseQuery` devolve array novo a cada chamada.
  const terms = useMemo(() => parseQuery(searchQuery), [searchQuery]);

  return (
    <Box border="1px solid var(--chakra-colors-border)" borderRadius="6px" overflow="hidden">
      {pareceres.map((p) => (
        <ParecerItem key={p.numero} p={p} terms={terms} />
      ))}
    </Box>
  );
}
