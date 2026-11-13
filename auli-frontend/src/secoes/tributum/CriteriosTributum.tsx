import { Box, Flex } from "@chakra-ui/react";
import { DocumentoMarkdown } from "../../ui/DocumentoMarkdown";

/**
 * A aba **Critérios**: o que entra no Tributum, o que não entra e quem seleciona.
 *
 * **É uma aba, e não um arquivo em `docs/`, porque o próprio texto se declara público** — ele
 * explica ao autor de fora como propor um item e por que a curadoria não é chancela institucional.
 * Uma declaração de conflito de interesse guardada onde só o mantenedor lê não cumpre a função que
 * ela mesma se atribui.
 *
 * O texto mora no mesmo lugar e segue a mesma disciplina do `about.md` (prosa autorada,
 * versionada, na raiz de `public/`), e se mostra pelo mesmo `DocumentoMarkdown`.
 */
export function CriteriosTributum() {
  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.app">
      <Box w="100%" maxW="720px" mx="auto" px={5} pt={5} pb={10}>
        <DocumentoMarkdown
          arquivo="/tributum-criterios.md"
          carregando="Carregando os critérios…"
          tituloDoErro="Erro ao carregar os critérios"
          erro="Não foi possível carregar o texto agora."
        />
      </Box>
    </Flex>
  );
}
