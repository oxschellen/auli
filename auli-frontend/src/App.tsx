import { useEffect, useState } from "react";
import { Box, Flex } from "@chakra-ui/react";
import { LazyMotion, domAnimation } from "framer-motion";
import { Provider } from "./pages/chat/ui/provider";
import { Toaster } from "./pages/chat/ui/toaster";
import { ErrorBoundary } from "./shared/ErrorBoundary";
import { AppHeader } from "./shared/AppHeader";
import { Home } from "./pages/home/Home";
import { StateSelection } from "./pages/stateselection/StateSelection";
import { EntityProvider } from "./shared/EntityContext";
import { AbasDeEstado } from "./shared/AbasDeEstado";
import { idDaAba, idDoPainel, useAbas } from "./shared/abas";
import { RespostaDaAbaProvider, useAbasRespondendo } from "./shared/RespostaDaAba";
import { getEntity } from "./shared/entities";
import { idDaSidebar } from "./shared/layout";

// Sourced from package.json at build time (see vite.config.js `define`).
const APP_VERSION = __APP_VERSION__;

const SIDEBAR_KEY = "auli.sidebar";

/** A sidebar começa como foi deixada; sem preferência salva (ou sem localStorage), aberta. */
function lerSidebarAberta(): boolean {
  try {
    return localStorage.getItem(SIDEBAR_KEY) !== "recolhida";
  } catch {
    return true;
  }
}

/**
 * App shell: o cabeçalho com as abas de estado e, embaixo, um painel por aba — o mapa enquanto a
 * aba não tem estado, depois o `Home` dele.
 *
 * As abas inativas ficam MONTADAS e escondidas (D-ABAS-5), como as seções dentro do `Home`: é o
 * que deixa cada uma com a sua conversa, a sua seção, a sua rolagem e até uma pergunta em
 * andamento. Cada aba tem o seu `EntityProvider`, então o que está dentro dela lê o estado dela.
 */
const AppShell = () => {
  const { abas, abrirNova, ativar, fechar, escolher } = useAbas();
  const { respondendo, novas, marcarRespondendo } = useAbasRespondendo(abas.ativa);
  const entity = getEntity(abas.lista.find((a) => a.chave === abas.ativa)?.entityId) ?? null;
  // Aqui e não no Home: o botão mora no cabeçalho, ao lado do logo, e a sidebar no Home. Uma
  // escolha só para todas as abas.
  const [sidebarAberta, setSidebarAberta] = useState(lerSidebarAberta);

  useEffect(() => {
    try {
      localStorage.setItem(SIDEBAR_KEY, sidebarAberta ? "aberta" : "recolhida");
    } catch {
      // Sem persistência, o botão segue valendo nesta sessão.
    }
  }, [sidebarAberta]);

  // Ctrl+B (⌘B no Mac), o atalho de sidebar dos editores. Só com estado escolhido na aba ativa:
  // no mapa não há sidebar, e o Ctrl+B fica com o navegador. Com Shift é outra coisa (o
  // Ctrl+Shift+B do Chrome mostra a barra de favoritos), então fica de fora.
  const temSidebar = entity !== null;
  useEffect(() => {
    if (!temSidebar) return;
    const aoTeclar = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.shiftKey || e.altKey || e.key.toLowerCase() !== "b") return;
      e.preventDefault();
      setSidebarAberta((v) => !v);
    };
    document.addEventListener("keydown", aoTeclar);
    return () => document.removeEventListener("keydown", aoTeclar);
  }, [temSidebar]);

  return (
    <Flex direction="column" h="100dvh" w="100%">
      <AppHeader
        subtitle={APP_VERSION}
        sidebarAberta={sidebarAberta}
        onToggleSidebar={entity ? () => setSidebarAberta((v) => !v) : undefined}
        sidebarId={entity ? idDaSidebar(entity.id) : undefined}
      >
        <AbasDeEstado
          abas={abas}
          respondendo={respondendo}
          novas={novas}
          onAtivar={ativar}
          onFechar={fechar}
          onNova={abrirNova}
        />
      </AppHeader>
      {/* Clip here; Home owns the single scroll container so the chat's
          scroll-to-bottom has exactly one scrollable ancestor. */}
      <Box flex="1" minH={0} overflow="hidden">
        {abas.lista.map((aba) => {
          const daAba = getEntity(aba.entityId);
          return (
            <Box
              key={aba.chave}
              role="tabpanel"
              id={idDoPainel(aba.chave)}
              aria-labelledby={idDaAba(aba.chave)}
              h="100%"
              hidden={aba.chave !== abas.ativa}
            >
              {daAba ? (
                <EntityProvider entity={daAba}>
                  <RespostaDaAbaProvider
                    chave={aba.chave}
                    aVista={aba.chave === abas.ativa}
                    marcar={marcarRespondendo}
                  >
                    <Home sidebarAberta={sidebarAberta} />
                  </RespostaDaAbaProvider>
                </EntityProvider>
              ) : (
                <StateSelection onSelect={(id) => escolher(aba.chave, id)} />
              )}
            </Box>
          );
        })}
      </Box>
    </Flex>
  );
};

const App = () => {
  return (
    <Provider>
      {/* Single global toaster for the whole app (each bubble used to mount its
          own, creating one portal per message). */}
      <Toaster />
      <ErrorBoundary>
        <LazyMotion features={domAnimation}>
          {/* Single-page app: one shared header above the state tabs, each with
              its own Home. Sections are tabs inside Home, not routes, so there's
              no router. Each tab's entity (state) scopes its data + chat. */}
          <AppShell />
        </LazyMotion>
      </ErrorBoundary>
    </Provider>
  );
};

export default App;
