import { useMemo, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { Flex, Box, Text, chakra } from "@chakra-ui/react";
import { MdMenu, MdClose } from "react-icons/md";
import { useSelectedEntity } from "../../contexto/EntityContext";
import { SIDEBAR_RAIL_WIDTH, SIDEBAR_VAR, SIDEBAR_WIDTH, idDaSidebar } from "../../ui/layout";
import { Sobreposicao } from "../../ui/Sobreposicao";
import { secoesDe } from "../registro";
import {
  NavList,
  PISTA_DE_ROLAGEM,
  proximaHabilitada,
  type Lista,
  type MenuItem,
} from "./Navegacao";

/** `sidebarAberta` vem do botão ao lado do logo (ou do Ctrl+B); `false` recolhe a sidebar numa
 *  faixa de ícones. Só vale a partir de `md`. */
export const Home = ({ sidebarAberta = true }: { sidebarAberta?: boolean }) => {
  const entity = useSelectedEntity();
  const largura = sidebarAberta ? SIDEBAR_WIDTH : SIDEBAR_RAIL_WIDTH;
  // Visíveis todas, desativadas as que a entidade não tem (ver `secoesDe`).
  const menuItems: MenuItem[] = useMemo(() => secoesDe(entity), [entity]);

  const [selectedId, setSelectedId] = useState("chat");
  // Painel montado só na primeira ativação e mantido montado depois, para o estado (scroll,
  // busca, histórico do chat) sobreviver à troca — e para não baixar megabytes na carga.
  const [mountedIds, setMountedIds] = useState<Set<string>>(() => new Set([selectedId]));
  const [drawerOpen, setDrawerOpen] = useState(false);
  const tabRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const drawerRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const menuBtnRef = useRef<HTMLButtonElement | null>(null);

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
    <Flex h="100%" minH={0} bg="bg.canvas" css={{ [SIDEBAR_VAR]: largura }}>
      {/* Sidebar (>= md). Scroll próprio se a janela for baixa; o conteúdo mantém o seu — ela não
          é ancestral do chat, então o "único container de scroll" do App.tsx segue valendo. */}
      <Box
        id={idDaSidebar(entity.id)}
        hideBelow="md"
        // Compartilhada com a barra de composição do chat, que é `fixed` e precisa saber onde a
        // sidebar termina para não invadi-la — pela variável acima. Ver `ui/layout.ts`.
        w={largura}
        flexShrink={0}
        {...PISTA_DE_ROLAGEM}
        borderRight="1px solid var(--chakra-colors-border)"
        p="10px 8px"
        overflowY="auto"
      >
        <NavList
          menuItems={menuItems}
          selectedId={selectedId}
          escopo={entity.id}
          idPrefix="tab"
          onSelect={selectTab}
          onKeyDown={onTabKeyDown("tab")}
          tabRefs={tabRefs}
          recolhida={!sidebarAberta}
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

        {/* Drawer mobile: o véu + painel com a MESMA NavList. Esc fecha e devolve o foco ao
            botão de menu. Acima do AppHeader (`Z`): abaixo dele, o cabeçalho cobria o topo do
            painel e o "Chat", primeiro item, sumia atrás dele. */}
        {drawerOpen && (
          <Sobreposicao
            hideFrom="md"
            onFechar={() => setDrawerOpen(false)}
            devolverFocoA={menuBtnRef}
          >
            <Box
              position="absolute"
              top={0}
              bottom={0}
              left={0}
              w="240px"
              {...PISTA_DE_ROLAGEM}
              borderRight="1px solid var(--chakra-colors-border)"
              p="10px 8px"
              overflowY="auto"
            >
              <NavList
                menuItems={menuItems}
                selectedId={selectedId}
                escopo={entity.id}
                idPrefix="tabm"
                onSelect={selectTab}
                onKeyDown={onTabKeyDown("tabm")}
                tabRefs={drawerRefs}
              />
            </Box>
          </Sobreposicao>
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
                id={`${entity.id}-tabpanel-${item.id}`}
                aria-labelledby={`${entity.id}-tab-${item.id}`}
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
