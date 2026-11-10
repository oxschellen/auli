import { useEffect, useState } from "react";
import { Box, Flex } from "@chakra-ui/react";
import { LazyMotion, domAnimation } from "framer-motion";
import { Provider } from "./pages/chat/ui/provider";
import { Toaster } from "./pages/chat/ui/toaster";
import { ErrorBoundary } from "./shared/ErrorBoundary";
import { AppHeader } from "./shared/AppHeader";
import { Home } from "./pages/home/Home";
import { StateSelection } from "./pages/stateselection/StateSelection";
import { EntityProvider, useEntity } from "./shared/EntityContext";

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

/** App shell: the state-selection landing page until an entity is chosen, then the tabbed app. */
const AppShell = () => {
  const { entity, clearEntity } = useEntity();
  // Aqui e não no Home: o botão mora no cabeçalho, ao lado do logo, e a sidebar no Home.
  const [sidebarAberta, setSidebarAberta] = useState(lerSidebarAberta);

  useEffect(() => {
    try {
      localStorage.setItem(SIDEBAR_KEY, sidebarAberta ? "aberta" : "recolhida");
    } catch {
      // Sem persistência, o botão segue valendo nesta sessão.
    }
  }, [sidebarAberta]);

  // Ctrl+B (⌘B no Mac), o atalho de sidebar dos editores. Só com estado escolhido: na seleção
  // de estado não há sidebar, e o Ctrl+B fica com o navegador. Com Shift é outra coisa (o
  // Ctrl+Shift+B do Chrome mostra a barra de favoritos), então fica de fora.
  useEffect(() => {
    if (!entity) return;
    const aoTeclar = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.shiftKey || e.altKey || e.key.toLowerCase() !== "b") return;
      e.preventDefault();
      setSidebarAberta((v) => !v);
    };
    document.addEventListener("keydown", aoTeclar);
    return () => document.removeEventListener("keydown", aoTeclar);
  }, [entity]);

  return (
    <Flex direction="column" h="100dvh" w="100%">
      <AppHeader
        subtitle={APP_VERSION}
        entity={entity}
        onChangeEntity={entity ? clearEntity : undefined}
        sidebarAberta={sidebarAberta}
        onToggleSidebar={entity ? () => setSidebarAberta((v) => !v) : undefined}
      />
      {/* Clip here; Home owns the single scroll container so the chat's
          scroll-to-bottom has exactly one scrollable ancestor. */}
      <Box flex="1" minH={0} overflow="hidden">
        {entity ? <Home sidebarAberta={sidebarAberta} /> : <StateSelection />}
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
          {/* Single-page app: one shared header above the tabbed Home. Sections
              are tabs (see Home), not routes, so there's no router. The active
              entity (state) scopes all data + chat; chosen on the landing page. */}
          <EntityProvider>
            <AppShell />
          </EntityProvider>
        </LazyMotion>
      </ErrorBoundary>
    </Provider>
  );
};

export default App;
