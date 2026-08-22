import { Flex, Button, Box } from "@chakra-ui/react";
import { useState } from "react";
import { Tooltip } from "./ui/tooltip";
import { MdCopyAll, MdHistory } from "react-icons/md";
import ReactMarkdown from "react-markdown";
import { utilsCopyTextToClipboard } from "./utils/utils";
import { compactMarkdownComponents, markdownPlugins } from "../../shared/markdown";
import { LogModal } from "./LogModal";

interface SystemMessageProps {
  messageText: string;
  showButton: boolean;
  /** Quando presente, oferece o ícone que abre o registro de auditoria desta resposta. */
  logId?: string;
}

/** Aviso exibido no rodapé de toda resposta gerada (não aparece na saudação,
 *  que chega com showButton=false). Texto validado com quem atende — manter uma frase só. */
const DISCLAIMER =
  "A Auli é um protótipo experimental de inteligência artificial. As respostas são geradas " +
  "automaticamente, podem variar e conter imprecisões, e têm caráter apenas informativo — " +
  "confira sempre as fontes e os links indicados.";

export const SystemMessage = ({ messageText, showButton, logId }: SystemMessageProps) => {
  const [logAberto, setLogAberto] = useState(false);

  return (
    <Flex px={3} py={2} w="100%">
      <Flex
        flexDirection="column"
        bg="bg.canvas"
        color="fg"
        borderRadius="18px 18px 18px 4px"
        padding="14px 16px 8px"
        minW="200px"
        maxW="85%"
        my={0}
        border="1px solid var(--chakra-colors-border)"
      >
        <Box
          fontSize="16px"
          lineHeight="1.6"
          color="fg"
          fontFamily="body"
          className="markdown-body"
        >
          <ReactMarkdown remarkPlugins={markdownPlugins} components={compactMarkdownComponents}>
            {messageText}
          </ReactMarkdown>
        </Box>

        {showButton && (
          <Box
            mt={2}
            pt={2}
            borderTop="1px solid var(--chakra-colors-border)"
            fontSize="11px"
            lineHeight="1.4"
            color="fg.muted"
          >
            {DISCLAIMER}
          </Box>
        )}

        {showButton && (
          <Flex justify="flex-end" mt={0} gap={2}>
            {/* Só existe quando o backend devolveu `log_id` — sem registro gravado não há o que
                abrir, e o ícone some em vez de levar a um 404. Isso já o exclui da saudação. */}
            {logId && (
              <Tooltip content="Ver o log desta resposta" bg="bg.inverted">
                <Button
                  borderRadius="full"
                  aria-label="Ver o log de auditoria desta resposta"
                  size="xs"
                  minW="26px"
                  h="26px"
                  color="fg.muted"
                  bg="transparent"
                  _hover={{ bg: "bg.overlay" }}
                  transition="all 0.15s ease"
                  onClick={() => setLogAberto(true)}
                >
                  <MdHistory size={16} color="var(--chakra-colors-fg-muted)" />
                </Button>
              </Tooltip>
            )}
            <Tooltip content="Copiar resposta" bg="bg.inverted">
              <Button
                borderRadius="full"
                aria-label="Copiar resposta para a área de transferência"
                size="xs"
                minW="26px"
                h="26px"
                color="fg.muted"
                bg="transparent"
                _hover={{ bg: "bg.overlay" }}
                transition="all 0.15s ease"
                onClick={() => {
                  utilsCopyTextToClipboard(messageText, "A resposta foi copiada para a área de transferência");
                }}
              >
                <MdCopyAll size={16} color="var(--chakra-colors-fg-muted)" />
              </Button>
            </Tooltip>
          </Flex>
        )}
      </Flex>

      {logAberto && logId && <LogModal logId={logId} onClose={() => setLogAberto(false)} />}
    </Flex>
  );
};