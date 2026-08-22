import { Box, Flex } from "@chakra-ui/react";
import ReactMarkdown from "react-markdown";
import useSWR from "swr";
import { textFetcher, SWR_OPTS, versioned } from "../shared/fetchers";
import { AsyncContent } from "../shared/AsyncContent";
import { proseMarkdownComponents, markdownPlugins } from "../shared/markdown";

/** Mesmo lugar e mesma disciplina do `about.md`: prosa autorada, versionada, na raiz de `public/`. */
const ARQUIVO = versioned("/tributum-criterios.md");

/**
 * A aba **Critérios**: o que entra no Tributum, o que não entra e quem seleciona.
 *
 * **É uma aba, e não um arquivo em `docs/`, porque o próprio texto se declara público** — ele
 * explica ao autor de fora como propor um item e por que a curadoria não é chancela institucional.
 * Uma declaração de conflito de interesse guardada onde só o mantenedor lê não cumpre a função que
 * ela mesma se atribui.
 *
 * Renderiza como a aba Sobre — `textFetcher` + `proseMarkdownComponents` —, mas pelo `AsyncContent`
 * em vez do trio de estados à mão que o `About.tsx` ainda carrega de antes de ele existir.
 */
export function CriteriosTributum() {
  const { data, error, isLoading } = useSWR(ARQUIVO, textFetcher, SWR_OPTS);

  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.app">
      <Box w="100%" maxW="720px" mx="auto" px={5} pt={5} pb={10}>
        <AsyncContent
          loading={isLoading}
          error={error}
          loadingText="Carregando os critérios…"
          errorTitle="Erro ao carregar os critérios"
          errorDescription="Não foi possível carregar o texto agora."
        >
          <Box
            fontSize="1rem"
            lineHeight="1.7"
            color="fg"
            fontFamily="body"
            className="markdown-body"
          >
            <ReactMarkdown remarkPlugins={markdownPlugins} components={proseMarkdownComponents}>
              {data ?? ""}
            </ReactMarkdown>
          </Box>
        </AsyncContent>
      </Box>
    </Flex>
  );
}
