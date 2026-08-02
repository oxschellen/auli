import { useMemo, useRef, useState } from "react";
import type { ComponentType, KeyboardEvent } from "react";
import { Flex, Box, chakra } from "@chakra-ui/react";
import { Chat } from "../chat/Chat";
import { ServicosList } from "../servicoslist/ServicosList";
import { FaqsList } from "../faqslist/FaqsList";
import { PareceresList } from "../parecereslist/PareceresList";
import { NotasList } from "../notaslist/NotasList";
import { ConteudosList } from "../conteudoslist/ConteudosList";
import { DownloadsList } from "../downloadslist/DownloadsList";
import { McpList } from "../mcplist/McpList";
import { About } from "../about/About";
import { useSelectedEntity } from "../../shared/EntityContext";
import { hasCollection } from "../../shared/entities";
import type { Collection } from "../../shared/entities";

/** As nove abas, na ordem da barra. `collection: null` = sempre disponível (`chat` fala com o RAG
 *  da entidade, `about` é estático, `downloads` lista TODOS os estados, `mcp` são manuais
 *  globais); as demais dependem de a entidade ter a coleção. */
const TABS: { id: string; label: string; Component: ComponentType; collection: Collection | null }[] = [
  { id: "chat", label: "Chat", Component: Chat, collection: null },
  { id: "servicos", label: "Serviços", Component: ServicosList, collection: "servicos" },
  { id: "faqs", label: "FAQs", Component: FaqsList, collection: "faqs" },
  { id: "pareceres", label: "Pareceres", Component: PareceresList, collection: "pareceres" },
  { id: "notas", label: "Notas", Component: NotasList, collection: "notas" },
  { id: "conteudos", label: "Conteúdos", Component: ConteudosList, collection: "conteudos" },
  { id: "downloads", label: "Downloads", Component: DownloadsList, collection: null },
  { id: "mcp", label: "MCP", Component: McpList, collection: null },
  { id: "about", label: "Sobre", Component: About, collection: null },
];

/**
 * Próxima aba HABILITADA a partir de `fromId`, andando `delta` (+1/−1) e dando a volta nas pontas.
 * Devolve `undefined` só se nenhuma estiver habilitada — impossível na prática, porque `chat` e
 * `about` nunca são desativados, mas é o que torna a função total e o laço garantidamente finito.
 *
 * Pura e exportada por causa do envolvimento de índice: `(((i + delta*passo) % n) + n) % n` é onde
 * mora o off-by-one, e delta negativo em `%` do JS devolve resto negativo.
 */
export function proximaHabilitada<T extends { id: string; disabled: boolean }>(
  itens: T[],
  fromId: string,
  delta: 1 | -1,
): T | undefined {
  const n = itens.length;
  const i = itens.findIndex((t) => t.id === fromId);
  if (i < 0) return undefined;
  for (let passo = 1; passo <= n; passo++) {
    const candidata = itens[(((i + delta * passo) % n) + n) % n];
    if (!candidata.disabled) return candidata;
  }
  return undefined;
}

export const Home = () => {
  const entity = useSelectedEntity();
  // Todas as abas continuam VISÍVEIS — a que a entidade não tem fica inerte, não desaparece: some
  // com a aba e o usuário não sabe que a seção existe; apagada, ele vê que existe e não vale aqui.
  // 23 das 27 entidades declaram só `servicos`, então isso desativa 4 das 7 na maioria dos estados.
  const menuItems = useMemo(
    () =>
      TABS.map((t) => ({
        ...t,
        disabled: t.collection !== null && !hasCollection(entity, t.collection),
      })),
    [entity],
  );

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
  // and activate the neighbouring tab, wrapping at the ends. Abas inativas são
  // SALTADAS — a seta anda até a próxima habilitada. A varredura termina sempre:
  // `chat` e `about` nunca são desativados, então há no mínimo dois destinos.
  const onTabKeyDown = (e: KeyboardEvent<HTMLButtonElement>) => {
    const delta = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (!delta) return;
    e.preventDefault();
    const proxima = proximaHabilitada(menuItems, selectedId, delta);
    if (!proxima) return;
    selectTab(proxima.id);
    tabRefs.current[proxima.id]?.focus();
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
                // `aria-disabled`, não o atributo `disabled`: o segundo tira o botão da árvore de
                // acessibilidade, e aí o leitor de tela não anuncia que a seção existe — que é
                // exatamente o que queremos comunicar. Com `aria-disabled` ele é anunciado como
                // indisponível. (Padrão WAI-ARIA para tabs.)
                aria-disabled={item.disabled || undefined}
                // Roving tabindex: only the active tab is in the tab order; the
                // rest are reached via the arrow keys.
                tabIndex={isActive ? 0 : -1}
                onClick={item.disabled ? undefined : () => selectTab(item.id)}
                onKeyDown={onTabKeyDown}
                flex="1 1 0"
                maxW="100px"
                minW={0}
                px="2px"
                cursor={item.disabled ? "not-allowed" : "pointer"}
                textAlign="center"
                overflow="hidden"
                textOverflow="ellipsis"
                whiteSpace="nowrap"
                fontSize="clamp(0.75rem, 2.64vw, 0.92rem)"
                fontFamily="body"
                fontWeight={isActive ? "700" : "500"}
                color={item.disabled ? "fg.disabled" : isActive ? "accent" : "fg.muted"}
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
