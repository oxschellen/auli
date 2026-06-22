import { Flex, Textarea, IconButton, Box } from "@chakra-ui/react";
import { MdSend } from "react-icons/md";
import type { ChangeEvent, RefObject } from "react";
import { isPromptValid, charsRemaining } from "./utils/prompt";

interface InputProps {
  textareaRef: RefObject<HTMLTextAreaElement | null>;
  prompt: string;
  updatePrompt: (e: ChangeEvent<HTMLTextAreaElement>) => void;
  loading: boolean;
  callServerAPI: (prompt: string) => void;
}

export const Input = ({ textareaRef, prompt, updatePrompt, loading, callServerAPI }: InputProps) => {
  const valid = isPromptValid(prompt);
  const remaining = charsRemaining(prompt);
  return (
    <Box py={1} bg="bg.canvas" borderColor="border">
      <Flex
        mx="auto"
        flexDirection="row"
        alignItems="flex-start"
      >
        <Textarea
          ref={textareaRef}
          id="prompt-id"
          name="prompt"
          size="md"
          fontSize="1rem"
          mt={2}
          mb={2}
          px={4}
          py={3}
          rows={3}
          minH="80px"
          ml={1}
          variant="outline"
          placeholder="Digite sua pergunta..."
          border="1px solid"
          borderColor="border"
          bg="bg.canvas"
          color="fg"
          _placeholder={{ color: "fg.muted" }}
          _focus={{
            borderColor: "accent",
            boxShadow: "focusRing",
            outline: "none",
          }}
          _hover={{
            borderColor: "brand.400",
          }}
          borderRadius="12px"
          value={prompt}
          onChange={updatePrompt}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              if (!loading && valid) {
                callServerAPI(prompt);
              }
            }
          }}
          width="100%"
          flex={1}
          resize="none"
          fontFamily="body"
        />

        <IconButton
          borderRadius="full"
          ml={2}
          mr={1}
          mt={7}
          px={4}
          h="48px"
          minW="48px"
          variant="solid"
          aria-label="Enviar pesquisa"
          bg="accent"
          color="accent.fg"
          _hover={{
            bg: "brand.600",
            transform: "translateY(-2px)",
          }}
          _disabled={{
            bg: "border",
            cursor: "not-allowed",
            transform: "none",
          }}
          _active={{
            bg: "brand.700",
            transform: "translateY(0)"
          }}
          transition="all 0.2s ease"
          disabled={loading || !valid}
          onClick={() => callServerAPI(prompt)}
        >
          <MdSend size={20} />
        </IconButton>
      </Flex>
      
      {/* Character count hint */}
      <Flex justify="flex-end" px={2} mt={1}>
        <Box fontSize="xs" color={valid ? "accent" : "fg.muted"} fontWeight={valid ? "500" : "400"}>
          {valid ? "Pronto para enviar" : `Mínimo ${remaining} caracteres`}
        </Box>
      </Flex>
    </Box>
  );
};
