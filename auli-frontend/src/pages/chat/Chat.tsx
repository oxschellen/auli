import { Box, Flex, chakra } from "@chakra-ui/react";
import { MdEdit, MdKeyboardArrowDown, MdKeyboardArrowUp } from "react-icons/md";
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
// O `API_URL` saiu daqui para `utils/api.ts` quando o modal do log passou a precisar da MESMA
// base — ver `logUrl`, que é derivada dele justamente para não divergir.
import { API_URL } from "./utils/api";

export const Chat = () => {
  const entity = useSelectedEntity();
  const { prompt, setPrompt, updatePrompt } = usePrompt();
  const { messages, setMessages } = useMessages();
  // O seletor de fonte é dirigido pelo registry: a entidade decide quais tipos existem.
  const { questionType, updateQuestionType, disponiveis } = useQuestionType(entity);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const isSubmittingRef = useRef(false);

  const { isKeyboardVisible: isKeyboardOpen, keyboardHeight } = useIsKeyboardVisible();
  const [loading, setLoading] = useState(false);
  // Só no celular: a caixa de mensagem ocupa ~220px da tela, e recolhê-la libera a leitura da
  // resposta. Fica uma barrinha que a traz de volta já com o foco no campo.
  const [recolhido, setRecolhido] = useState(false);
  const expandir = () => {
    setRecolhido(false);
    requestAnimationFrame(() => textareaRef.current?.focus());
  };

  // Altura MEDIDA da barra de composição, que é `fixed` e cobre o fim da lista. O recuo era fixo
  // (185px) e no celular a barra passa disso — o seletor de tipo quebra em duas linhas —, então a
  // última faixa de cada resposta (os botões do log e de cópia) ficava escondida atrás dela.
  // O valor inicial reproduz os 185px de antes até a primeira medição.
  const composerRef = useRef<HTMLDivElement>(null);
  const [composerHeight, setComposerHeight] = useState(169);
  useEffect(() => {
    const el = composerRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setComposerHeight(el.offsetHeight));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

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
    <Flex flexDirection="column" w="100%" flex={1} bg="bg.app" pb={`${composerHeight + 16}px`}>
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
        ref={composerRef}
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
        {/* Celular: o "Recolher" acima da caixa e, recolhida, a barrinha que a traz de volta. A
            caixa segue filha DIRETA desta barra fixa (o teste da sidebar sobe por `parentElement`),
            então quem a esconde é o próprio `Input`, pela prop `recolhido`. */}
        <Flex display={{ base: recolhido ? "none" : "flex", md: "none" }} justify="flex-end" mx={3}>
          <chakra.button
            type="button"
            aria-label="Recolher a caixa de mensagem"
            aria-expanded={true}
            onClick={() => setRecolhido(true)}
            display="flex"
            alignItems="center"
            gap={1}
            px={2}
            py={1}
            fontSize="11px"
            color="fg.muted"
            borderRadius="full"
            _hover={{ bg: "bg.overlay" }}
          >
            Recolher
            <MdKeyboardArrowDown size={16} />
          </chakra.button>
        </Flex>
        <chakra.button
          type="button"
          aria-label="Mostrar a caixa de mensagem"
          aria-expanded={false}
          onClick={expandir}
          display={{ base: recolhido ? "flex" : "none", md: "none" }}
          alignItems="center"
          justifyContent="space-between"
          w="calc(100% - 24px)"
          mx={3}
          mb={2}
          px={3}
          py={2}
          fontSize="0.9rem"
          color="fg.muted"
          bg="bg.canvas"
          border="1px solid"
          borderColor="border"
          borderRadius="12px"
        >
          <Flex as="span" alignItems="center" gap={2}>
            <MdEdit size={16} />
            Escrever pergunta
          </Flex>
          <MdKeyboardArrowUp size={20} />
        </chakra.button>

        {/* O seletor mora DENTRO da caixa de mensagem: é filho do `Input`, não irmão. */}
        <Input
          textareaRef={textareaRef}
          prompt={prompt}
          updatePrompt={updatePrompt}
          loading={loading}
          callServerAPI={handleCallServerAPI}
          recolhido={recolhido}
        >
          <SelectQuestionType
            questionType={questionType}
            updateQuestionType={updateQuestionType}
            disponiveis={disponiveis}
          />
        </Input>
      </Box>
    </Flex>
  );
};
