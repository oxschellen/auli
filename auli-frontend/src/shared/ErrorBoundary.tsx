import { Component } from "react";
import type { ErrorInfo, ReactNode } from "react";
import { Box, Button, Flex, Heading, Text } from "@chakra-ui/react";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
}

/**
 * Top-level error boundary. Contains render errors to a friendly fallback
 * instead of unmounting the whole app to a blank screen.
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { hasError: false };

  static getDerivedStateFromError(): ErrorBoundaryState {
    return { hasError: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Erro inesperado na interface:", error, info);
  }

  handleReload = () => {
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      return (
        <Flex
          direction="column"
          align="center"
          justify="center"
          minH="100dvh"
          bg="bg.app"
          px={6}
          textAlign="center"
        >
          <Box maxW="420px">
            <Heading size="lg" color="fg" mb={3}>
              Algo deu errado
            </Heading>
            <Text color="fg.muted" mb={6}>
              Ocorreu um erro inesperado. Tente recarregar a página.
            </Text>
            <Button
              bg="accent"
              color="accent.fg"
              _hover={{ bg: "brand.600" }}
              onClick={this.handleReload}
            >
              Recarregar
            </Button>
          </Box>
        </Flex>
      );
    }

    return this.props.children;
  }
}
