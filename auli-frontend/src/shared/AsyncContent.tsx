import type { ReactNode } from "react";
import { Alert, Flex, Spinner, Stack, Text } from "@chakra-ui/react";

interface AsyncContentProps {
  loading: boolean;
  error: unknown;
  loadingText?: string;
  errorTitle?: string;
  errorDescription?: ReactNode;
  children: ReactNode;
}

/**
 * Three-state gate for SWR-backed pages: shows a centered spinner while
 * loading, an error alert on failure, and the children once data is ready.
 * Replaces the loading/error/content scaffold that was hand-rolled in each
 * list and text page. Callers keep their own surrounding layout (sticky bars,
 * padding); this only renders the swappable content region.
 */
export function AsyncContent({
  loading,
  error,
  loadingText,
  errorTitle = "Erro",
  errorDescription = "Aconteceu um erro.",
  children,
}: AsyncContentProps) {
  if (loading) {
    return (
      <Flex justify="center" align="center" minH="200px" w="full">
        <Stack gap={4} align="center">
          <Spinner size="lg" color="accent" />
          <Text color="fg" fontSize="md">
            {loadingText}
          </Text>
        </Stack>
      </Flex>
    );
  }

  if (error) {
    return (
      <Alert.Root status="error">
        <Alert.Indicator />
        <Alert.Content>
          <Alert.Title>{errorTitle}</Alert.Title>
          <Alert.Description>{errorDescription}</Alert.Description>
        </Alert.Content>
      </Alert.Root>
    );
  }

  return children;
}
