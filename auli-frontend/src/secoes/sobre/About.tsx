import { Box, Flex } from "@chakra-ui/react";
import { DocumentoMarkdown } from "../../ui/DocumentoMarkdown";

export const About = () => (
  <Flex direction="column" flex={1} w="100%" bg="bg.app">
    <Box w="100%" maxW="720px" mx="auto" px={5} pt={5} pb={10}>
      <DocumentoMarkdown arquivo="/about.md" erro="Não foi possível carregar o manifesto." />
    </Box>
  </Flex>
);
