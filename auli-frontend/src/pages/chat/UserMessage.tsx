import { Flex, Text, Button } from "@chakra-ui/react";
import { Tooltip } from "./ui/tooltip";
import { MdCopyAll } from "react-icons/md";
import { utilsCopyTextToClipboard } from "./utils/utils";
import type { SetPrompt } from "../../types/chat";

interface UserMessageProps {
  messageText: string;
  setPrompt: SetPrompt;
}

export const UserMessage = ({ messageText, setPrompt }: UserMessageProps) => {
  return (
    <Flex px={3} py={0} w="100%" justify="flex-end">
      <Flex
        flexDirection="column"
        bg="bubble.user"
        color="fg"
        borderRadius="18px 18px 4px 18px"
        padding="14px 16px 8px"
        minW="200px"
        maxW="85%"
        my={0}
        border="1px solid var(--chakra-colors-border)"
      >
        <Text
          fontSize="16px"
          lineHeight="1.4"
          whiteSpace="pre-wrap"
          color="fg"
          fontFamily="body"
        >
          {messageText}
        </Text>

        <Flex justify="flex-end" mt={0}>
          <Tooltip content="Copiar pergunta" bg="bg.inverted">
            <Button
              borderRadius="full"
              aria-label="Copiar a pergunta para a área de entrada"
              size="xs"
              minW="22px"
              h="22px"
              color="var(--chakra-colors-fg-muted)"
              bg="transparent"
              _hover={{ bg: "bg.overlay" }}
              transition="all 0.15s ease"
              onClick={() => {
                setPrompt(messageText);
                utilsCopyTextToClipboard(
                  messageText,
                  "A pergunta foi copiada para a área de input",
                );
              }}
            >
              <MdCopyAll size={13} color="var(--chakra-colors-fg-muted)" />
            </Button>
          </Tooltip>
        </Flex>
      </Flex>
    </Flex>
  );
};
