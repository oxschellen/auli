import { Flex, Textarea, IconButton, Box } from "@chakra-ui/react";
import { MdSend } from "react-icons/md";
import type { ChangeEvent, ReactNode, RefObject } from "react";
import { isPromptValid, charsRemaining } from "./utils/prompt";

interface InputProps {
  textareaRef: RefObject<HTMLTextAreaElement | null>;
  prompt: string;
  updatePrompt: (e: ChangeEvent<HTMLTextAreaElement>) => void;
  loading: boolean;
  callServerAPI: (prompt: string) => void;
  /** Controles que moram DENTRO da caixa de mensagem, acima do campo — hoje o seletor de tipo de
   *  consulta. Slot em vez de import direto para o `Input` não depender do que é composto nele
   *  (e para os testes dele seguirem montando o campo sozinho). */
  children?: ReactNode;
}

/**
 * A caixa de mensagem: um cartão ÚNICO com borda, contendo os controles (`children`), o campo e o
 * botão de enviar.
 *
 * A borda é do cartão, não do textarea — antes cada um tinha a sua, e aninhar o seletor deixaria
 * moldura dentro de moldura dentro de moldura. Pelo mesmo motivo o anel de foco subiu para o
 * cartão via `_focusWithin`: acende com o campo E com os chips, que é o que "esta caixa está
 * ativa" quer dizer.
 */
export const Input = ({
  textareaRef,
  prompt,
  updatePrompt,
  loading,
  callServerAPI,
  children,
}: InputProps) => {
  const valid = isPromptValid(prompt);
  const remaining = charsRemaining(prompt);
  return (
    <Box
      // O cartão passou a reunir controles heterogêneos (o seletor, o campo, o enviar), então
      // agrupá-los é a marcação honesta — e é o que o dá nome para o leitor de tela.
      role="group"
      aria-label="Caixa de mensagem"
      mx={3}
      mb={2}
      px={2}
      pt={2}
      pb={1}
      bg="bg.canvas"
      border="1px solid"
      borderColor="border"
      borderRadius="12px"
      _focusWithin={{ borderColor: "accent", boxShadow: "focusRing" }}
      transition="border-color 0.15s ease, box-shadow 0.15s ease"
    >
      {children}

      <Flex mx="auto" flexDirection="row" alignItems="flex-start">
        <Textarea
          ref={textareaRef}
          id="prompt-id"
          name="prompt"
          size="md"
          fontSize="1rem"
          px={2}
          py={2}
          rows={3}
          minH="80px"
          variant="outline"
          placeholder="Digite sua pergunta..."
          // Sem borda e sem anel próprios: quem os desenha é o cartão (ver o topo do arquivo).
          border="none"
          bg="bg.canvas"
          color="fg"
          _placeholder={{ color: "fg.muted" }}
          _focus={{ outline: "none", boxShadow: "none" }}
          borderRadius="8px"
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
          mt={4}
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
      <Flex justify="flex-end" px={2}>
        <Box fontSize="xs" color={valid ? "accent" : "fg.muted"} fontWeight={valid ? "500" : "400"}>
          {valid ? "Pronto para enviar" : `Mínimo ${remaining} caracteres`}
        </Box>
      </Flex>
    </Box>
  );
};
