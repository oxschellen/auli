import { useEffect, useMemo, useRef, useState } from "react";
import type { ComponentType, KeyboardEvent, RefObject } from "react";
import { Flex, Box, Text, chakra } from "@chakra-ui/react";
import {
  MdOutlineChat,
  MdOutlineWorkOutline,
  MdHelpOutline,
  MdOutlineGavel,
  MdOutlineStickyNote2,
  MdOutlineLibraryBooks,
  MdOutlineFileDownload,
  MdOutlinePower,
  MdInfoOutline,
  MdMenu,
  MdClose,
} from "react-icons/md";
import type { IconType } from "react-icons";
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
import { SIDEBAR_WIDTH } from "../../shared/layout";

/** Grupo de navegação. `null` = topo (sem título); `rodape` cola no pé da sidebar. */
type Grupo = null | "acervo" | "integracoes" | "rodape";

/** As nove seções, agora agrupadas para a sidebar. `collection: null` = sempre disponível
 *  (`chat` fala com o RAG da entidade, `about` é estático, `downloads` lista TODOS os
 *  estados, `mcp` são manuais globais); as demais dependem de a entidade ter a coleção.
 *  O rótulo de `mcp` virou "Conectar sua IA" — o id NÃO muda (ancora tab-/tabpanel-). */
const TABS: {
  id: string;
  label: string;
  Component: ComponentType;
  collection: Collection | null;
  grupo: Grupo;
  Icone: IconType;
}[] = [
  { id: "chat", label: "Chat", Component: Chat, collection: null, grupo: null, Icone: MdOutlineChat },
  { id: "servicos", label: "Serviços", Component: ServicosList, collection: "servicos", grupo: "acervo", Icone: MdOutlineWorkOutline },
  { id: "faqs", label: "FAQs", Component: FaqsList, collection: "faqs", grupo: "acervo", Icone: MdHelpOutline },
  { id: "pareceres", label: "Pareceres", Component: PareceresList, collection: "pareceres", grupo: "acervo", Icone: MdOutlineGavel },
  { id: "notas", label: "Notas", Component: NotasList, collection: "notas", grupo: "acervo", Icone: MdOutlineStickyNote2 },
  { id: "conteudos", label: "Conteúdos", Component: ConteudosList, collection: "conteudos", grupo: "acervo", Icone: MdOutlineLibraryBooks },
  { id: "mcp", label: "Conectar sua IA", Component: McpList, collection: null, grupo: "integracoes", Icone: MdOutlinePower },
  { id: "downloads", label: "Downloads", Component: DownloadsList, collection: null, grupo: "integracoes", Icone: MdOutlineFileDownload },
  { id: "about", label: "Sobre", Component: About, collection: null, grupo: "rodape", Icone: MdInfoOutline },
];

/** Títulos e ordem dos grupos na sidebar (o `rodape` é tratado à parte, colado no pé). */
const GRUPOS: { id: Grupo; titulo: string | null }[] = [
  { id: null, titulo: null },
  { id: "acervo", titulo: "Acervo" },
  { id: "integracoes", titulo: "Integrações" },
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

type MenuItem = (typeof TABS)[number] & { disabled: boolean };
type TabRefs = RefObject<Record<string, HTMLButtonElement | null>>;
/** Qual das duas listas: a da sidebar (desktop) ou a do drawer (mobile). Serve de prefixo dos
 *  ids E de seletor do mapa de refs, para que os dois não possam divergir. */
type Lista = "tab" | "tabm";

/** Um item da sidebar/drawer. Botão largura total, ícone + rótulo, estados via cores do tema. */
const NavItem = ({
  item,
  idPrefix,
  isActive,
  onSelect,
  onKeyDown,
  tabRef,
}: {
  item: MenuItem;
  idPrefix: Lista;
  isActive: boolean;
  onSelect: () => void;
  onKeyDown: (e: KeyboardEvent<HTMLButtonElement>) => void;
  tabRef: (el: HTMLButtonElement | null) => void;
}) => (
  <chakra.button
    ref={tabRef}
    type="button"
    role="tab"
    id={`${idPrefix}-${item.id}`}
    aria-selected={isActive}
    aria-controls={`tabpanel-${item.id}`}
    // `aria-disabled`, não `disabled`: o segundo tira o botão da árvore de acessibilidade e o
    // leitor de tela nem anuncia que a seção existe — que é o que queremos comunicar.
    aria-disabled={item.disabled || undefined}
    // Roving tabindex: só a ativa está na ordem de Tab; as demais via setas.
    tabIndex={isActive ? 0 : -1}
    onClick={item.disabled ? undefined : onSelect}
    onKeyDown={onKeyDown}
    display="flex"
    alignItems="center"
    gap="10px"
    w="100%"
    px="12px"
    py="8px"
    borderRadius="8px"
    cursor={item.disabled ? "not-allowed" : "pointer"}
    fontSize="0.9rem"
    fontFamily="body"
    fontWeight={isActive ? "700" : "500"}
    textAlign="left"
    color={item.disabled ? "fg.disabled" : isActive ? "accent" : "fg.muted"}
    bg={isActive ? "bg.subtle" : "transparent"}
    _hover={item.disabled || isActive ? undefined : { bg: "bg.subtle", color: "fg" }}
    transition="color 0.15s ease, background 0.15s ease"
  >
    <item.Icone size="17px" aria-hidden="true" />
    <Box as="span" flex="1" overflow="hidden" textOverflow="ellipsis" whiteSpace="nowrap">
      {item.label}
    </Box>
  </chakra.button>
);

/**
 * A lista de navegação (grupos + rodapé). Usada na sidebar (desktop) e no drawer (mobile).
 *
 * `idPrefix` existe porque `hideBelow`/`hideFrom` escondem por CSS, não desmontam: com o drawer
 * aberto as DUAS listas ficam no DOM ao mesmo tempo, e sem prefixos distintos haveria dois
 * `id="tab-chat"`. O `aria-labelledby` dos painéis segue apontando para a lista da sidebar
 * (`tab-…`), que existe sempre — o accname lê elemento referenciado mesmo com `display:none`.
 * Pelo mesmo motivo cada lista tem seu próprio mapa de refs: um só faria o `.focus()` das setas
 * mirar a lista invisível, conforme a ordem em que os callbacks de ref rodam.
 */
const NavList = ({
  menuItems,
  selectedId,
  idPrefix,
  onSelect,
  onKeyDown,
  tabRefs,
}: {
  menuItems: MenuItem[];
  selectedId: string;
  idPrefix: Lista;
  onSelect: (id: string) => void;
  onKeyDown: (e: KeyboardEvent<HTMLButtonElement>) => void;
  tabRefs: TabRefs;
}) => (
  <Flex
    direction="column"
    h="100%"
    role="tablist"
    aria-label="Seções"
    aria-orientation="vertical"
    gap="2px"
  >
    {GRUPOS.map((g) => (
      // `role="none"` no invólucro e no título: o `tablist` exige `tab` como filhos possuídos, e
      // o agrupamento visual mete um div e um parágrafo no meio. Marcá-los como presentacionais
      // devolve os botões à posse do tablist sem perder o texto do título, que segue exposto.
      <Box key={g.id ?? "topo"} role="none">
        {g.titulo && (
          <Text
            role="none"
            fontSize="0.68rem"
            color="fg.muted"
            letterSpacing="0.06em"
            textTransform="uppercase"
            px="12px"
            mt="14px"
            mb="4px"
          >
            {g.titulo}
          </Text>
        )}
        {menuItems
          .filter((t) => t.grupo === g.id)
          .map((item) => (
            <NavItem
              key={item.id}
              item={item}
              idPrefix={idPrefix}
              isActive={item.id === selectedId}
              onSelect={() => onSelect(item.id)}
              onKeyDown={onKeyDown}
              tabRef={(el) => (tabRefs.current[item.id] = el)}
            />
          ))}
      </Box>
    ))}
    {/* Espaçador que empurra o rodapé para o pé — presentacional pelo mesmo motivo dos
        invólucros de grupo acima: é filho direto do tablist e não é um `tab`. */}
    <Box flex="1" role="none" />
    {menuItems
      .filter((t) => t.grupo === "rodape")
      .map((item) => (
        <NavItem
          key={item.id}
          item={item}
          idPrefix={idPrefix}
          isActive={item.id === selectedId}
          onSelect={() => onSelect(item.id)}
          onKeyDown={onKeyDown}
          tabRef={(el) => (tabRefs.current[item.id] = el)}
        />
      ))}
  </Flex>
);

export const Home = () => {
  const entity = useSelectedEntity();
  // Todas as seções continuam VISÍVEIS — a que a entidade não tem fica inerte, não desaparece:
  // some com a seção e o usuário não sabe que ela existe; apagada, ele vê que existe e não vale
  // aqui. 23 das 27 entidades declaram só `servicos`, então isso desativa 4 das 7 na maioria.
  const menuItems: MenuItem[] = useMemo(
    () =>
      TABS.map((t) => ({
        ...t,
        disabled: t.collection !== null && !hasCollection(entity, t.collection),
      })),
    [entity],
  );

  const [selectedId, setSelectedId] = useState("chat");
  // Painel montado só na primeira ativação e mantido montado depois, para o estado (scroll,
  // busca, histórico do chat) sobreviver à troca — e para não baixar megabytes na carga.
  const [mountedIds, setMountedIds] = useState<Set<string>>(() => new Set([selectedId]));
  const [drawerOpen, setDrawerOpen] = useState(false);
  const tabRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const drawerRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const menuBtnRef = useRef<HTMLButtonElement | null>(null);

  // Esc fecha o drawer e devolve o foco ao botão que o abriu — o par mínimo que um overlay
  // modal deve ao teclado.
  //
  // O ouvinte é do DOCUMENTO, não do painel: ao abrir, o foco continua no botão de menu, que
  // fica FORA do drawer, então um `onKeyDown` no container não veria a tecla no caso mais comum.
  // `globalThis.KeyboardEvent` porque `KeyboardEvent` aqui é o tipo sintético do React, importado
  // no topo — o `addEventListener` quer o do DOM.
  useEffect(() => {
    if (!drawerOpen) return;
    const aoTeclar = (e: globalThis.KeyboardEvent) => {
      if (e.key !== "Escape") return;
      setDrawerOpen(false);
      menuBtnRef.current?.focus();
    };
    document.addEventListener("keydown", aoTeclar);
    return () => document.removeEventListener("keydown", aoTeclar);
  }, [drawerOpen]);

  const selectTab = (id: string) => {
    setSelectedId(id);
    setMountedIds((prev) => (prev.has(id) ? prev : new Set(prev).add(id)));
    setDrawerOpen(false);
  };

  // Padrão WAI-ARIA para tabs verticais: ArrowDown/ArrowUp movem e ativam a vizinha HABILITADA,
  // com volta nas pontas (Left/Right como sinônimos). A varredura termina sempre: `chat` e
  // `about` nunca são desativados.
  //
  // Recebe QUAL lista disparou o evento, não o mapa de refs: passar o ref em si aqui seria
  // passá-lo durante o render, o que a regra `react-hooks/refs` recusa (e com razão — o valor de
  // um ref não pode influenciar o render). Assim o `.current` só é lido dentro do handler.
  const onTabKeyDown = (lista: Lista) => (e: KeyboardEvent<HTMLButtonElement>) => {
    const delta =
      e.key === "ArrowDown" || e.key === "ArrowRight"
        ? 1
        : e.key === "ArrowUp" || e.key === "ArrowLeft"
          ? -1
          : 0;
    if (!delta) return;
    e.preventDefault();
    const proxima = proximaHabilitada(menuItems, selectedId, delta as 1 | -1);
    if (!proxima) return;
    selectTab(proxima.id);
    const refs = lista === "tab" ? tabRefs : drawerRefs;
    refs.current[proxima.id]?.focus();
  };

  const ativa = menuItems.find((t) => t.id === selectedId);

  return (
    <Flex h="100%" minH={0} bg="bg.canvas">
      {/* Sidebar (>= md). Scroll próprio se a janela for baixa; o conteúdo mantém o seu — ela não
          é ancestral do chat, então o "único container de scroll" do App.tsx segue valendo. */}
      <Box
        hideBelow="md"
        // Compartilhada com a barra de composição do chat, que é `fixed` e precisa saber onde a
        // sidebar termina para não invadi-la. Ver `shared/layout.ts`.
        w={SIDEBAR_WIDTH}
        flexShrink={0}
        bg="bg.app"
        borderRight="1px solid var(--chakra-colors-border)"
        p="10px 8px"
        overflowY="auto"
      >
        <NavList
          menuItems={menuItems}
          selectedId={selectedId}
          idPrefix="tab"
          onSelect={selectTab}
          onKeyDown={onTabKeyDown("tab")}
          tabRefs={tabRefs}
        />
      </Box>

      <Flex direction="column" flex="1" minW={0}>
        {/* Faixa mobile (< md): botão de menu + seção atual. */}
        <Flex
          hideFrom="md"
          alignItems="center"
          gap="8px"
          px="10px"
          py="6px"
          bg="bg.app"
          borderBottom="1px solid var(--chakra-colors-border)"
        >
          <chakra.button
            ref={menuBtnRef}
            type="button"
            aria-label={drawerOpen ? "Fechar menu de seções" : "Abrir menu de seções"}
            aria-expanded={drawerOpen}
            onClick={() => setDrawerOpen((v) => !v)}
            display="flex"
            alignItems="center"
            p="6px"
            borderRadius="8px"
            color="fg"
            _hover={{ bg: "bg.subtle" }}
          >
            {drawerOpen ? <MdClose size="20px" /> : <MdMenu size="20px" />}
          </chakra.button>
          <Text fontSize="0.9rem" fontWeight="700" color="accent">
            {ativa?.label}
          </Text>
        </Flex>

        {/* Drawer mobile: overlay + painel com a MESMA NavList. Feito à mão (Box fixed) para
            não depender da API de Drawer da versão instalada do Chakra. */}
        {drawerOpen && (
          <Box hideFrom="md" position="fixed" inset={0} zIndex={90}>
            <Box
              position="absolute"
              inset={0}
              bg="blackAlpha.500"
              onClick={() => setDrawerOpen(false)}
            />
            <Box
              position="absolute"
              top={0}
              bottom={0}
              left={0}
              w="240px"
              bg="bg.app"
              borderRight="1px solid var(--chakra-colors-border)"
              p="10px 8px"
              overflowY="auto"
            >
              <NavList
                menuItems={menuItems}
                selectedId={selectedId}
                idPrefix="tabm"
                onSelect={selectTab}
                onKeyDown={onTabKeyDown("tabm")}
                tabRefs={drawerRefs}
              />
            </Box>
          </Box>
        )}

        {/* Conteúdo — único container de scroll (ver comentário no App.tsx). */}
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
    </Flex>
  );
};
