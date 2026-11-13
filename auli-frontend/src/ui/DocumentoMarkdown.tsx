import { Box } from "@chakra-ui/react";
import ReactMarkdown from "react-markdown";
import useSWR from "swr";
import { SWR_OPTS, textFetcher, versioned } from "../api/estaticos";
import { markdownPlugins, proseMarkdownComponents } from "../lib/markdown";
import { AsyncContent } from "./AsyncContent";

/**
 * Um `.md` de `public/` mostrado como prosa (I5 do docs/ESTUDO-frontend-organizacao.md): o manifesto
 * da aba Sobre, os Critérios do Tributum e os manuais de "Conectar sua IA". Busca com o `?v=` do
 * build e passa pelos estados de carga e erro do `AsyncContent`.
 *
 * Sem `erro`, a falha mostra a própria mensagem dela.
 */
export function DocumentoMarkdown({
  arquivo,
  carregando = "Carregando…",
  tituloDoErro = "Erro",
  erro,
}: {
  /** Caminho a partir da raiz de `public/` (ex.: `/about.md`). */
  arquivo: string;
  carregando?: string;
  tituloDoErro?: string;
  erro?: string;
}) {
  const { data, error, isLoading } = useSWR(versioned(arquivo), textFetcher, SWR_OPTS);
  return (
    <AsyncContent
      loading={isLoading}
      error={error}
      loadingText={carregando}
      errorTitle={tituloDoErro}
      errorDescription={erro ?? (error instanceof Error ? error.message : undefined)}
    >
      <Box fontSize="1rem" lineHeight="1.7" color="fg" fontFamily="body">
        <ReactMarkdown remarkPlugins={markdownPlugins} components={proseMarkdownComponents}>
          {data ?? ""}
        </ReactMarkdown>
      </Box>
    </AsyncContent>
  );
}
