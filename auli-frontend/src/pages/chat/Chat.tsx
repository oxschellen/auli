import { Flex } from "@chakra-ui/react";
import { useRef, useEffect, useState } from "react";
import { useIsKeyboardVisible } from "./utils/useIsKeyboardVisible";
import { Messages } from "./Messages";
import { Input } from "./Input";
import { usePrompt } from "./utils/usePrompt";
import { useMessages } from "./utils/useMessages";
import { callServerAPI } from "./utils/callServerAPI";
import { isPromptValid } from "./utils/prompt";
import { useSelectedEntity } from "../../shared/EntityContext";

// Override per environment with VITE_API_URL (e.g. a staging endpoint); falls
// back to production so the app works with no .env present.
const API_URL =
  import.meta.env.VITE_API_URL ?? "https://api.auli.com.br/v1/question";

export const Chat = () => {
  const entity = useSelectedEntity();
  const { prompt, setPrompt, updatePrompt } = usePrompt();
  const { messages, setMessages } = useMessages();
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
      });
    } finally {
      isSubmittingRef.current = false;
    }
  };

  return (
    <Flex flexDirection="column" w="100%" flex={1} bg="bg.app" pb="150px">
      <Messages messages={messages} setPrompt={setPrompt} />
      <div ref={messagesEndRef} />

      <div
        style={{
          position: "fixed",
          // Lift the input above the on-screen keyboard when it's open. The
          // keyboard height is measured from visualViewport, so this needs no
          // device/UA check — desktops simply report 0.
          bottom: isKeyboardOpen ? `${keyboardHeight}px` : 0,
          width: "100%",
          left: 0,
          zIndex: 20,
          backgroundColor: "var(--chakra-colors-bg-canvas)",
        }}
      >
        <Input
          textareaRef={textareaRef}
          prompt={prompt}
          updatePrompt={updatePrompt}
          loading={loading}
          callServerAPI={handleCallServerAPI}
        />
      </div>
    </Flex>
  );
};
