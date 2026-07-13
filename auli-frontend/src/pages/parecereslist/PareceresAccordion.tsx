import { useState } from "react";
import { Box, Flex, Link, Text } from "@chakra-ui/react";
import { m, AnimatePresence, useReducedMotion } from "framer-motion";
import { MdExpandMore, MdExpandLess, MdOpenInNew } from "react-icons/md";
import type { Parecer } from "./parsePareceres";

/** Uma linha de parecer: cabeçalho (número + assunto) que abre para o corpo integral + link do portal. */
function ParecerItem({ p }: { p: Parecer }) {
  const reduceMotion = useReducedMotion();
  const [isOpen, setIsOpen] = useState(false);

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
            {p.numero}
          </Text>
          {p.assunto && (
            <Text fontSize="0.85rem" color="fg.muted" lineHeight="1.35" mt={0.5}>
              {p.assunto}
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
              <Text
                fontFamily="mono"
                whiteSpace="pre-wrap"
                color="fg"
                fontSize="0.8rem"
                lineHeight="1.5"
              >
                {p.corpo}
              </Text>
            </Box>
          </m.div>
        )}
      </AnimatePresence>
    </Box>
  );
}

export function PareceresAccordion({ pareceres }: { pareceres: Parecer[] }) {
  return (
    <Box border="1px solid var(--chakra-colors-border)" borderRadius="6px" overflow="hidden">
      {pareceres.map((p) => (
        <ParecerItem key={p.id} p={p} />
      ))}
    </Box>
  );
}
