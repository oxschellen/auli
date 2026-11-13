import { Box, Flex, Link, Stack, Text } from "@chakra-ui/react";
import { comoBotao, PainelRevelado } from "../../ui/Acordeao";
import { MdExpandLess, MdExpandMore } from "react-icons/md";
import type { Servico } from "./servicos";
import { Highlight } from "../../lib/highlight";

interface AccordionItemProps {
  classe: string;
  items: Servico[];
  isOpen: boolean;
  onToggle: () => void;
  isFirst: boolean;
  isLast: boolean;
  /** Termos já normalizados (`parseQuery`) — os MESMOS que filtraram, para marcar o que casou. */
  terms: string[];
}

export function AccordionItem({
  classe,
  items,
  isOpen,
  onToggle,
  isFirst,
  isLast,
  terms,
}: AccordionItemProps) {
  return (
    <Box>
      <Flex
        align="center"
        justify="space-between"
        py={2}
        px={4}
        bg={isOpen ? "bg.app" : "bg.canvas"}
        borderBottom="1px solid var(--chakra-colors-border)"
        cursor="pointer"
        _hover={{ bg: isOpen ? "bg.app" : "bg.subtle" }}
        style={{ transition: "background 0.15s ease" }}
        {...comoBotao(onToggle)}
        aria-expanded={isOpen}
        borderTopRadius={isFirst ? "6px" : undefined}
        borderBottomRadius={isLast && !isOpen ? "6px" : undefined}
      >
        <Text fontSize="1rem" fontWeight="600" color="fg" flex={1} mr={4} lineHeight="1">
          <Highlight text={classe} terms={terms} />
        </Text>
        <Flex align="center" gap={1} flexShrink={0}>
          <Box color="fg.muted" display="flex">
            {isOpen ? <MdExpandLess size={20} /> : <MdExpandMore size={20} />}
          </Box>
        </Flex>
      </Flex>

      <PainelRevelado aberto={isOpen}>
        <Box
          bg="bg.canvas"
          p={4}
          borderBottom="1px solid var(--chakra-colors-border)"
          borderBottomRadius={isLast ? "6px" : undefined}
        >
          <Stack gap={2}>
            {items.map((s) => (
              <Link
                key={s.id}
                href={s.link}
                target="_blank"
                rel="noopener noreferrer"
                color="accent"
                fontSize="1rem"
                lineHeight="1.75"
                display="block"
              >
                <Highlight text={s.titulo} terms={terms} />
              </Link>
            ))}
          </Stack>
        </Box>
      </PainelRevelado>
    </Box>
  );
}
