import { Box, Flex } from "@chakra-ui/react";
import { useRef, useEffect, useState } from "react";
import { useIsKeyboardVisible } from "./utils/useIsKeyboardVisible";
import { Messages } from "./Messages";
import { Input } from "./Input";
import { usePrompt } from "./utils/usePrompt";
import { useMessages } from "./utils/useMessages";
import { useQuestionType } from "./utils/useQuestionType";
import { callServerAPI } from "./utils/callServerAPI";
import { isPromptValid } from "./utils/prompt";
import { SelectQuestionType } from "./SelectQuestionType";
import { useSelectedEntity } from "../../shared/EntityContext";
import { SIDEBAR_WIDTH } from "../../shared/layout";

// Override per environment with VITE_API_URL (e.g. a staging endpoint); falls
// back to production so the app works with no .env present.
const API_URL =
  import.meta.env.VITE_API_URL ?? "https://api.auli.com.br/v1/question";

export const Chat = () => {
  const entity = useSelectedEntity();
  const { prompt, setPrompt, updatePrompt } = usePrompt();
  const { messages, setMessages } = useMessages();
  const { questionType, updateQuestionType } = useQuestionType();
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const isSubmittingRef = useRef(false);

  const { isKeyboardVisible: isKeyboardOpen, keyboardHeight } = useIsKeyboardVisible();
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    requestAnimationFrame(() => {
      messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
    });
  }, [messages]);

  const handleCallServerAPI = async (prompt: string) => {
    const trimmedPrompt = prompt.trim();
    if (isSubmittingRef.current || loading || !isPromptValid(trimmedPrompt)) return;
    isSubmittingRef.current = true;
    try {
      await callServerAPI({
        prompt: trimmedPrompt,
        messages,
        setMessages,
        setPrompt,
        setLoading,
        API_URL,
        entityId: entity.id,
        questionType,
      });
    } finally {
      isSubmittingRef.current = false;
    }
  };

  return (
    <Flex flexDirection="column" w="100%" flex={1} bg="bg.app" pb="185px">
      <Messages messages={messages} setPrompt={setPrompt} />
      <div ref={messagesEndRef} />

      {/* Barra de composição. `fixed` é o que permite subi-la acima do teclado virtual — mas
          `fixed` se ancora na VIEWPORT, não na área de conteúdo, então ela precisa recuar a
          largura da sidebar. Sem isso invade o menu, que é o que acontecia desde que a navegação
          virou sidebar (antes as abas eram uma faixa no topo e a barra cheia estava certa).

          `right={0}` em vez de `width="100%"`: com o recuo à esquerda, 100% transbordaria à
          direita exatamente a largura da sidebar. Abaixo de `md` a sidebar vira drawer e não
          ocupa espaço, daí o `base: 0`. */}
      <Box
        position="fixed"
        // Lift the input above the on-screen keyboard when it's open. The
        // keyboard height is measured from visualViewport, so this needs no
        // device/UA check — desktops simply report 0.
        bottom={isKeyboardOpen ? `${keyboardHeight}px` : 0}
        left={{ base: 0, md: SIDEBAR_WIDTH }}
        right={0}
        zIndex={20}
        bg="bg.canvas"
      >
        {/* O seletor mora DENTRO da caixa de mensagem: é filho do `Input`, não irmão. */}
        <Input
          textareaRef={textareaRef}
          prompt={prompt}
          updatePrompt={updatePrompt}
          loading={loading}
          callServerAPI={handleCallServerAPI}
        >
          <SelectQuestionType questionType={questionType} updateQuestionType={updateQuestionType} />
        </Input>
      </Box>
    </Flex>
  );
};
