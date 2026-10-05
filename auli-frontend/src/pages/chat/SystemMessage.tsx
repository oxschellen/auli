import { Flex, Button, Box, Heading, Text } from "@chakra-ui/react";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { Tooltip } from "./ui/tooltip";
import { MdCopyAll, MdReceiptLong } from "react-icons/md";
import ReactMarkdown from "react-markdown";
import type { Components } from "react-markdown";
import { utilsCopyTextToClipboard } from "./utils/utils";
import { compactMarkdownComponents, markdownPlugins, MarkdownLink } from "../../shared/markdown";
import { LogModal } from "./LogModal";
import { Fontes } from "./Fontes";
import { CitacaoSelo } from "./CitacaoSelo";
import { dividirSecoes } from "./utils/secoes";
import { ancora, marcarCitacoes, rotuloDoHref, rotulosCitados } from "./utils/citacoes";
import { TIPOS } from "./utils/tipos";
import type { QuestionType } from "./utils/useQuestionType";
import type { Fonte } from "../../types/chat";

interface SystemMessageProps {
  /** Id da mensagem: prefixa os ids das fontes (o mesmo rótulo existe em toda resposta) e é o
   *  alvo da rolagem do `Chat` quando a resposta chega. */
  id?: string;
  messageText: string;
  showButton: boolean;
  /** Quando presente, oferece o ícone que abre o registro de auditoria desta resposta. */
  logId?: string;
  /** Lista de fontes recuperadas (D-SF-10); ausente ou vazia = nada é desenhado. */
  fontes?: Fonte[];
  /** Tipo de consulta e duração — a linha de contexto no topo da resposta (D-UI-9). */
  tipo?: QuestionType;
  duracaoMs?: number;
}

/** Quanto tempo o realce da fonte clicada dura. */
const REALCE_MS = 2500;

const Markdown = ({ children, components }: { children: string; components: Components }) => (
  <Box fontSize="16px" lineHeight="1.6" color="fg" fontFamily="body" className="markdown-body">
    <ReactMarkdown remarkPlugins={markdownPlugins} components={components}>
      {children}
    </ReactMarkdown>
  </Box>
);

const Secao = ({ rotulo, children }: { rotulo: string; children: ReactNode }) => (
  <Box as="section" aria-label={rotulo}>
    <Heading as="h3" fontSize="15px" fontWeight="700" mb={1.5}>
      {rotulo}
    </Heading>
    {children}
  </Box>
);

export const SystemMessage = ({
  id,
  messageText,
  showButton,
  logId,
  fontes,
  tipo,
  duracaoMs,
}: SystemMessageProps) => {
  const [logAberto, setLogAberto] = useState(false);
  const [destaque, setDestaque] = useState<string | null>(null);
  const idBase = `msg-${id ?? "saudacao"}`;
  const comFontes = showButton && !!fontes && fontes.length > 0;

  // Citações só viram selos quando há lista de fontes para onde levar (D-UI-7).
  const texto = useMemo(() => (comFontes ? marcarCitacoes(messageText) : messageText), [comFontes, messageText]);
  const citados = useMemo(() => (comFontes ? rotulosCitados(messageText) : new Set<string>()), [comFontes, messageText]);
  const secoes = useMemo(() => dividirSecoes(texto), [texto]);
  const titulos = useMemo(() => new Map((fontes ?? []).map((f) => [f.rotulo, f.titulo])), [fontes]);

  const citar = useCallback(
    (rotulo: string) => {
      setDestaque(rotulo);
      document.getElementById(`${idBase}-${ancora(rotulo)}`)?.scrollIntoView({ behavior: "smooth", block: "center" });
    },
    [idBase],
  );
  useEffect(() => {
    if (!destaque) return;
    const t = setTimeout(() => setDestaque(null), REALCE_MS);
    return () => clearTimeout(t);
  }, [destaque]);

  const components = useMemo<Components>(
    () => ({
      ...compactMarkdownComponents,
      a: ({ href, children }) => {
        const rotulo = rotuloDoHref(href);
        if (rotulo && comFontes) {
          return <CitacaoSelo rotulo={rotulo} titulo={titulos.get(rotulo)} onClick={() => citar(rotulo)} />;
        }
        return <MarkdownLink href={href}>{children}</MarkdownLink>;
      },
    }),
    [comFontes, titulos, citar],
  );

  // Sem ações (`showButton` falso), a mensagem segue como balão simples. A resposta ocupa a
  // coluna — e as mensagens de erro também, porque o `callServerAPI` as cria com `showButton`.
  if (!showButton) {
    return (
      <Flex px={3} py={2} w="100%">
        <Box bg="bg.canvas" border="1px solid" borderColor="border" borderRadius="16px" px={4} py={3} maxW="85%">
          <Markdown components={compactMarkdownComponents}>{messageText}</Markdown>
        </Box>
      </Flex>
    );
  }

  return (
    <Flex id={idBase} px={3} py={2} w="100%" scrollMarginTop="72px">
      <Flex
        as="article"
        direction="column"
        gap={4}
        w="100%"
        bg="bg.canvas"
        color="fg"
        borderRadius="16px"
        border="1px solid"
        borderColor="border"
        px={{ base: 4, md: 6 }}
        pt={{ base: 4, md: 5 }}
        pb={2}
      >
        {tipo && (
          <Flex align="center" gap={2} fontSize="13px" color="fg.muted">
            <Text as="span" px={2} lineHeight="22px" borderRadius="6px" bg="bg.subtle" color="fg" fontWeight="600">
              {TIPOS[tipo].rotulo}
            </Text>
            {duracaoMs !== undefined && <Text as="span">respondida em {Math.max(1, Math.round(duracaoMs / 1000))} s</Text>}
          </Flex>
        )}

        {secoes.antes && <Markdown components={components}>{secoes.antes}</Markdown>}

        {secoes.resumo !== null && (
          <Box as="section" aria-label="Resumo" bg="bg.resumo" borderRadius="12px" px={{ base: 3.5, md: 4.5 }} pt={3} pb={1.5}>
            <Text fontSize="12px" fontWeight="700" letterSpacing="0.08em" color="accent.strong" mb={1}>
              RESUMO
            </Text>
            <Markdown components={components}>{secoes.resumo}</Markdown>
          </Box>
        )}

        {secoes.detalhes !== null && (
          <Secao rotulo="Detalhes">
            <Markdown components={components}>{secoes.detalhes}</Markdown>
          </Secao>
        )}

        {comFontes && <Fontes fontes={fontes!} citados={citados} idBase={idBase} destaque={destaque} />}

        <Flex justify="flex-end" gap={1} pt={2} borderTop="1px solid" borderColor="border">
          {/* Só existe quando o backend devolveu `log_id` — sem registro gravado não há o que
              abrir, e o botão some em vez de levar a um 404. */}
          {logId && (
            <Tooltip content="Ver o log desta resposta" bg="bg.inverted">
              <Button
                borderRadius="8px"
                aria-label="Ver o log de auditoria desta resposta"
                size="xs"
                h="32px"
                px={2.5}
                gap={1.5}
                fontSize="13px"
                fontWeight="normal"
                color="fg.muted"
                bg="transparent"
                _hover={{ bg: "bg.overlay" }}
                onClick={() => setLogAberto(true)}
              >
                <MdReceiptLong size={16} color="var(--chakra-colors-fg-muted)" />
                Log da consulta
              </Button>
            </Tooltip>
          )}
          <Tooltip content="Copiar resposta" bg="bg.inverted">
            <Button
              borderRadius="8px"
              aria-label="Copiar resposta para a área de transferência"
              size="xs"
              h="32px"
              px={2.5}
              gap={1.5}
              fontSize="13px"
              fontWeight="normal"
              color="fg.muted"
              bg="transparent"
              _hover={{ bg: "bg.overlay" }}
              onClick={() => {
                utilsCopyTextToClipboard(messageText, "A resposta foi copiada para a área de transferência");
              }}
            >
              <MdCopyAll size={16} color="var(--chakra-colors-fg-muted)" />
              Copiar resposta
            </Button>
          </Tooltip>
        </Flex>
      </Flex>

      {logAberto && logId && <LogModal logId={logId} onClose={() => setLogAberto(false)} />}
    </Flex>
  );
};
