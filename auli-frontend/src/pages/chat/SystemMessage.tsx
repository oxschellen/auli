import { Flex, Button, Box } from "@chakra-ui/react";
import { Tooltip } from "./ui/tooltip";
import { MdCopyAll } from "react-icons/md";
import ReactMarkdown from "react-markdown";
import { utilsCopyTextToClipboard } from "./utils/utils";
import { compactMarkdownComponents } from "../../shared/markdown";

interface SystemMessageProps {
  messageText: string;
  showButton: boolean;
}

export const SystemMessage = ({ messageText, showButton }: SystemMessageProps) => {
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
          <ReactMarkdown components={compactMarkdownComponents}>
            {messageText}
          </ReactMarkdown>
        </Box>

        {showButton && (
          <Flex justify="flex-end" mt={0}>
            <Tooltip content="Copiar resposta" bg="bg.inverted">
              <Button
                borderRadius="full"
                aria-label="Copiar resposta para a área de transferência"
                size="xs"
                minW="22px"
                h="22px"
                color="var(--chakra-colors-fg-muted)"
                bg="transparent"
                _hover={{ bg: "bg.overlay" }}
                transition="all 0.15s ease"
                onClick={() => {
                  utilsCopyTextToClipboard(messageText, "A resposta foi copiada para a área de transferência");
                }}
              >
                <MdCopyAll size={13} color="var(--chakra-colors-fg-muted)" />
              </Button>
            </Tooltip>
          </Flex>
        )}
      </Flex>
    </Flex>
  );
};