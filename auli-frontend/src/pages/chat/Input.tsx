import { Flex, Textarea, IconButton, Box } from "@chakra-ui/react";
import { MdArrowUpward } from "react-icons/md";
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
  /** No celular (< md), a caixa some e quem aparece é a barrinha "Escrever pergunta" do `Chat`.
   *  No desktop é ignorado: ali não há barrinha para trazê-la de volta. */
  recolhido?: boolean;
  /** Texto de exemplo do campo — muda com o tipo de consulta (D-UI-9). */
  placeholder?: string;
}

/**
 * A caixa de mensagem: um cartão ÚNICO com borda, contendo os controles (`children`), o campo e o
 * botão de enviar.
 *
 * A borda é do cartão, não do textarea — antes cada um tinha a sua, e aninhar o seletor deixaria
 * moldura dentro de moldura dentro de moldura. Pelo mesmo motivo o anel de foco subiu para o
 * cartão via `_focusWithin`: acende com o campo E com o seletor, que é o que "esta caixa está
 * ativa" quer dizer.
 *
 * Uma linha só (D-UI-4): seletor de tipo, campo e botão lado a lado; o campo começa com uma linha
 * e cresce até ~6. A dica de tamanho mínimo só aparece quando há texto curto demais — vazia, a
 * caixa não precisa explicar nada, e "Pronto para enviar" não informava (o botão aceso já diz).
 */
export const Input = ({
  textareaRef,
  prompt,
  updatePrompt,
  loading,
  callServerAPI,
  children,
  recolhido = false,
  placeholder = "Digite sua pergunta...",
}: InputProps) => {
  const valid = isPromptValid(prompt);
  const remaining = charsRemaining(prompt);
  const curto = prompt.trim().length > 0 && !valid;
  return (
    <Box
      role="group"
      aria-label="Caixa de mensagem"
      display={{ base: recolhido ? "none" : "block", md: "block" }}
      // Coluna de leitura (D-UI-4): no desktop a caixa alinha com as mensagens, em 760px.
      mx={{ base: 3, md: "auto" }}
      maxW="760px"
      mb={1.5}
      p={1.5}
      bg="bg.canvas"
      border="1px solid"
      borderColor="border"
      borderRadius="16px"
      boxShadow="composer"
      _focusWithin={{ borderColor: "accent", boxShadow: "focusRing" }}
      transition="border-color 0.15s ease, box-shadow 0.15s ease"
    >
      <Flex alignItems="flex-end" gap={2}>
        {children}

        <Textarea
          ref={textareaRef}
          id="prompt-id"
          name="prompt"
          aria-label="Sua pergunta"
          rows={1}
          autoresize
          maxH="160px"
          minH={{ base: "44px", md: "40px" }}
          px={2}
          py={{ base: "11px", md: "9px" }}
          fontSize="16px"
          lineHeight="1.4"
          placeholder={placeholder}
          border="none"
          bg="bg.canvas"
          color="fg"
          // Placeholder numa linha só: com `autoresize`, o texto de exemplo que quebrava no celular
          // fazia a caixa VAZIA crescer para 5 linhas.
          _placeholder={{ color: "fg.muted", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}
          _focus={{ outline: "none", boxShadow: "none" }}
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
          flex={1}
          minW={0}
          resize="none"
          fontFamily="body"
        />

        <IconButton
          flex="none"
          borderRadius="10px"
          h={{ base: "44px", md: "40px" }}
          minW={{ base: "44px", md: "40px" }}
          variant="solid"
          aria-label="Enviar pesquisa"
          bg="accent"
          color="accent.fg"
          _hover={{ bg: "brand.600" }}
          _disabled={{ bg: "border", color: "fg.disabled", cursor: "not-allowed" }}
          _active={{ bg: "brand.700" }}
          transition="background 0.15s ease"
          disabled={loading || !valid}
          onClick={() => callServerAPI(prompt)}
        >
          <MdArrowUpward size={20} />
        </IconButton>
      </Flex>

      {curto && (
        <Box fontSize="xs" color="fg.muted" textAlign="right" px={2} pt={1}>
          Mínimo {remaining} caracteres
        </Box>
      )}
    </Box>
  );
};
