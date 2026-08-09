import { Box, Button, Flex, Spinner, Text } from "@chakra-ui/react";
import { useEffect, useState } from "react";
import { MdClose, MdCopyAll } from "react-icons/md";
import { logUrl } from "./utils/api";
import { utilsCopyTextToClipboard } from "./utils/utils";

interface LogModalProps {
  /** UUID da consulta — a *capability* que abre o registro. */
  logId: string;
  onClose: () => void;
}

const MESSAGES = {
  titulo: "Log da pergunta",
  carregando: "Buscando o registro…",
  indisponivel: "Não foi possível obter o log agora.",
  copiado: "O log foi copiado para a área de transferência",
} as const;

/**
 * Mostra o registro de auditoria da resposta, **como está** (D-LOG-5): sem filtrar, resumir ou
 * reformatar. Isso inclui o par pergunta original / pergunta anonimizada — ver o mascaramento
 * funcionando é a prova de privacidade, não vazamento dela: quem lê é o autor da pergunta.
 *
 * Feito à mão (Box `fixed` + overlay) como o drawer do `Home.tsx`, para não depender da API de
 * Dialog da versão instalada do Chakra.
 */
export const LogModal = ({ logId, onClose }: LogModalProps) => {
  const [conteudo, setConteudo] = useState<string | null>(null);
  const [erro, setErro] = useState<string | null>(null);

  useEffect(() => {
    // `ignorar` evita escrever estado depois que o modal fechou — trocar de resposta rápido dispara
    // duas buscas, e a que chegar por último venceria sem isto.
    let ignorar = false;
    fetch(logUrl(logId))
      .then(async (r) => {
        const texto = await r.text();
        // O backend manda a mensagem amigável no corpo do 404/400 — usá-la é melhor que inventar
        // uma nossa, e é a mesma para "nunca existiu" e "podado", de propósito.
        if (!r.ok) throw new Error(texto || MESSAGES.indisponivel);
        return texto;
      })
      .then((t) => !ignorar && setConteudo(t))
      .catch((e: Error) => !ignorar && setErro(e.message || MESSAGES.indisponivel));
    return () => {
      ignorar = true;
    };
  }, [logId]);

  // Esc fecha, como em qualquer diálogo.
  useEffect(() => {
    const aoTeclar = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", aoTeclar);
    return () => window.removeEventListener("keydown", aoTeclar);
  }, [onClose]);

  return (
    <Box position="fixed" inset={0} zIndex={100}>
      <Box position="absolute" inset={0} bg="blackAlpha.600" onClick={onClose} />
      <Flex
        position="absolute"
        top="50%"
        left="50%"
        transform="translate(-50%, -50%)"
        flexDirection="column"
        w={{ base: "94vw", md: "min(900px, 90vw)" }}
        maxH="85vh"
        bg="bg.canvas"
        color="fg"
        border="1px solid var(--chakra-colors-border)"
        borderRadius="12px"
        overflow="hidden"
        role="dialog"
        aria-modal="true"
        aria-label={MESSAGES.titulo}
      >
        <Flex
          align="center"
          justify="space-between"
          px={4}
          py={3}
          borderBottom="1px solid var(--chakra-colors-border)"
        >
          <Text fontWeight="600" fontSize="15px">
            {MESSAGES.titulo}
          </Text>
          <Flex gap={1}>
            {conteudo && (
              <Button
                size="xs"
                variant="ghost"
                aria-label="Copiar o log inteiro"
                onClick={() => utilsCopyTextToClipboard(conteudo, MESSAGES.copiado)}
              >
                <MdCopyAll size={15} />
              </Button>
            )}
            <Button size="xs" variant="ghost" aria-label="Fechar" onClick={onClose}>
              <MdClose size={16} />
            </Button>
          </Flex>
        </Flex>

        <Box overflow="auto" px={4} py={3} flex={1}>
          {erro ? (
            <Text fontSize="14px" color="fg.muted">
              {erro}
            </Text>
          ) : conteudo === null ? (
            <Flex align="center" gap={2} color="fg.muted" fontSize="14px">
              <Spinner size="sm" />
              {MESSAGES.carregando}
            </Flex>
          ) : (
            // `pre` porque o registro JÁ é texto estruturado por seções — a v1 não parseia nada.
            <Box
              as="pre"
              fontFamily="mono"
              fontSize="12px"
              lineHeight="1.5"
              whiteSpace="pre-wrap"
              wordBreak="break-word"
            >
              {conteudo}
            </Box>
          )}
        </Box>
      </Flex>
    </Box>
  );
};
