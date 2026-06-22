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

/** App shell: the state-selection landing page until an entity is chosen, then the tabbed app. */
const AppShell = () => {
  const { entity, clearEntity } = useEntity();

  return (
    <Flex direction="column" h="100dvh" w="100%">
      <AppHeader
        subtitle={APP_VERSION}
        entity={entity}
        onChangeEntity={entity ? clearEntity : undefined}
      />
      {/* Clip here; Home owns the single scroll container so the chat's
          scroll-to-bottom has exactly one scrollable ancestor. */}
      <Box flex="1" minH={0} overflow="hidden">
        {entity ? <Home /> : <StateSelection />}
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
