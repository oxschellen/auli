import { useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { Flex, Box, chakra } from "@chakra-ui/react";
import { Chat } from "../chat/Chat";
import { ServicosList } from "../servicoslist/ServicosList";
import { FaqsList } from "../faqslist/FaqsList";
import { PareceresList } from "../parecereslist/PareceresList";
import { NotasList } from "../notaslist/NotasList";
import { ConteudosList } from "../conteudoslist/ConteudosList";
import { About } from "../about/About";

const menuItems = [
  { id: "chat", label: "Chat", Component: Chat },
  { id: "servicos", label: "Serviços", Component: ServicosList },
  { id: "faqs", label: "FAQs", Component: FaqsList },
  { id: "pareceres", label: "Pareceres", Component: PareceresList },
  { id: "notas", label: "Notas", Component: NotasList },
  { id: "conteudos", label: "Conteúdos", Component: ConteudosList },
  { id: "about", label: "Sobre", Component: About },
];

export const Home = () => {
  const [selectedId, setSelectedId] = useState("chat");
  // Mount a tab's content only once it's first activated, then keep it mounted
  // so its state (scroll, search, chat history) survives tab switches. This
  // avoids fetching every section's data — some of it megabytes — on load.
  const [mountedIds, setMountedIds] = useState<Set<string>>(() => new Set([selectedId]));
  const tabRefs = useRef<Record<string, HTMLButtonElement | null>>({});

  const selectTab = (id: string) => {
    setSelectedId(id);
    setMountedIds((prev) => (prev.has(id) ? prev : new Set(prev).add(id)));
  };

  // Arrow-key navigation across the tablist (WAI-ARIA tabs pattern): move focus
  // and activate the neighbouring tab, wrapping at the ends.
  const onTabKeyDown = (e: KeyboardEvent<HTMLButtonElement>) => {
    const delta = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (!delta) return;
    e.preventDefault();
    const i = menuItems.findIndex((m) => m.id === selectedId);
    const next = menuItems[(i + delta + menuItems.length) % menuItems.length];
    selectTab(next.id);
    tabRefs.current[next.id]?.focus();
  };

  return (
    <Flex direction="column" h="100%" minH={0} bg="bg.canvas">
      {/* Tab bar */}
      <Box w="100%" bg="bg.app" px={1} mb={1}>
        <Flex pt="6px" gap="2px" role="tablist" aria-label="Seções">
          {menuItems.map((item) => {
            const isActive = item.id === selectedId;
            return (
              <chakra.button
                key={item.id}
                ref={(el: HTMLButtonElement | null) => (tabRefs.current[item.id] = el)}
                type="button"
                role="tab"
                id={`tab-${item.id}`}
                aria-selected={isActive}
                aria-controls={`tabpanel-${item.id}`}
                // Roving tabindex: only the active tab is in the tab order; the
                // rest are reached via the arrow keys.
                tabIndex={isActive ? 0 : -1}
                onClick={() => selectTab(item.id)}
                onKeyDown={onTabKeyDown}
                flex="1 1 0"
                maxW="100px"
                minW={0}
                px="2px"
                cursor="pointer"
                textAlign="center"
                overflow="hidden"
                textOverflow="ellipsis"
                whiteSpace="nowrap"
                fontSize="clamp(0.75rem, 2.64vw, 0.92rem)"
                fontFamily="body"
                fontWeight={isActive ? "700" : "500"}
                color={isActive ? "accent" : "fg.muted"}
                pt={isActive ? "5px" : "6px"}
                pb="5px"
                bg={isActive ? "bg.canvas" : "bg.subtle"}
                borderTop={isActive ? "3px solid var(--chakra-colors-accent)" : "1px solid var(--chakra-colors-border)"}
                borderLeft="1px solid var(--chakra-colors-border)"
                borderRight="1px solid var(--chakra-colors-border)"
                borderBottom="none"
                borderRadius="5px 5px 0 0"
                transition="color 0.15s ease, background 0.15s ease"
              >
                {item.label}
              </chakra.button>
            );
          })}
        </Flex>
      </Box>

      {/* Tab content — mounted on first activation, then kept mounted to
          preserve state, hidden when inactive. */}
      <Box flex={1} overflowY="auto">
        {menuItems.map((item) =>
          mountedIds.has(item.id) ? (
            <Flex
              key={item.id}
              direction="column"
              minH="100%"
              role="tabpanel"
              id={`tabpanel-${item.id}`}
              aria-labelledby={`tab-${item.id}`}
              hidden={selectedId !== item.id}
              display={selectedId === item.id ? "flex" : "none"}
            >
              <item.Component />
            </Flex>
          ) : null,
        )}
      </Box>
    </Flex>
  );
};
