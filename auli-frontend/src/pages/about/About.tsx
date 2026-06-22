import { Box, Flex, Spinner, Stack, Text, Alert } from "@chakra-ui/react";
import ReactMarkdown from "react-markdown";
import useSWR from "swr";
import { textFetcher, SWR_OPTS, versioned } from "../../shared/fetchers";
import { proseMarkdownComponents } from "../../shared/markdown";

const FILE_ENDPOINT = versioned("/about.md");

export const About = () => {
  const { data: manifesto, error, isLoading } = useSWR(FILE_ENDPOINT, textFetcher, SWR_OPTS);

  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.app">
      {isLoading && (
        <Flex justify="center" align="center" minH="200px" w="full">
          <Stack gap={4} align="center">
            <Spinner size="lg" color="accent" />
            <Text color="fg" fontSize="md">Carregando…</Text>
          </Stack>
        </Flex>
      )}
      {error && (
        <Alert.Root status="error" m={4}>
          <Alert.Indicator />
          <Alert.Content>
            <Alert.Title>Erro</Alert.Title>
            <Alert.Description>Não foi possível carregar o manifesto.</Alert.Description>
          </Alert.Content>
        </Alert.Root>
      )}
      {!isLoading && !error && <Box
        w="100%"
        maxW="720px"
        mx="auto"
        px={5}
        pt={5}
        pb={10}
        fontSize="1rem"
        lineHeight="1.7"
        color="fg"
        fontFamily="body"
        className="markdown-body"
      >
        <ReactMarkdown components={proseMarkdownComponents}>
          {manifesto ?? ""}
        </ReactMarkdown>
      </Box>}
    </Flex>
  );
};
