import { Box, Flex, Text, chakra } from "@chakra-ui/react";
import { MdEdit, MdKeyboardArrowDown, MdKeyboardArrowUp } from "react-icons/md";
import { LuPanelRight } from "react-icons/lu";
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
import { EstadoInicial } from "./EstadoInicial";
import { Historico } from "./Historico";
import { useHistorico } from "./utils/useHistorico";
import { usePainelHistorico } from "./utils/usePainelHistorico";
import type { ItemHistorico } from "./utils/historico";
import { TIPOS } from "./utils/tipos";
import { useSelectedEntity } from "../../shared/EntityContext";
import { useAvisarResposta } from "../../shared/RespostaDaAba";
import { HEADER_HEIGHT, HISTORICO_WIDTH, SIDEBAR_VAR, SIDEBAR_WIDTH } from "../../shared/layout";
// O `API_URL` saiu daqui para `utils/api.ts` quando o modal do log passou a precisar da MESMA
// base — ver `logUrl`, que é derivada dele justamente para não divergir.
import { API_URL } from "./utils/api";

/** O botão de "Minhas perguntas", na linha acima da caixa de mensagem. São dois no DOM — um por
 *  modo, escolhidos por CSS (`hideBelow`/`hideFrom`), como os dois botões da barra lateral
 *  esquerda — com a mesma cara, que é a do "Recolher" vizinho. */
const BOTAO_HISTORICO = {
  type: "button",
  display: "flex",
  alignItems: "center",
  gap: 1,
  px: 2,
  py: 1,
  fontSize: "12px",
  color: "fg.muted",
  borderRadius: "full",
  cursor: "pointer",
  _hover: { bg: "bg.overlay" },
} as const;

export const Chat = () => {
  const entity = useSelectedEntity();
  const { prompt, setPrompt, updatePrompt } = usePrompt();
  const { messages, setMessages } = useMessages();
  // O seletor de fonte é dirigido pelo registry: a entidade decide quais tipos existem.
  const { questionType, updateQuestionType, disponiveis } = useQuestionType(entity);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const isSubmittingRef = useRef(false);

  const historico = useHistorico(entity.id);
  // Dois estados, como na barra lateral esquerda: a partir de `xl` o painel é uma coluna e a
  // escolha fica salva, e vale para todas as abas de estado; abaixo dele é um drawer, que abre e
  // fecha sem deixar rastro.
  const [painelAberto, setPainelAberto] = usePainelHistorico();
  const [drawerAberto, setDrawerAberto] = useState(false);
  const botaoDrawerRef = useRef<HTMLButtonElement>(null);

  // Esc fecha o drawer e devolve o foco ao botão que o abriu — o mesmo par do drawer de seções.
  useEffect(() => {
    if (!drawerAberto) return;
    const aoTeclar = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      setDrawerAberto(false);
      botaoDrawerRef.current?.focus();
    };
    document.addEventListener("keydown", aoTeclar);
    return () => document.removeEventListener("keydown", aoTeclar);
  }, [drawerAberto]);

  const { isKeyboardVisible: isKeyboardOpen, keyboardHeight } = useIsKeyboardVisible();
  const [loading, setLoading] = useState(false);
  // O spinner na aba do estado: é com a aba escondida, a pergunta ainda rodando, que ele conta.
  const avisarResposta = useAvisarResposta();
  useEffect(() => {
    avisarResposta(loading);
  }, [avisarResposta, loading]);
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

  // Resposta que chega rola até o COMEÇO dela, não até o fim (D-UI-2): a resposta longa terminava
  // com a tabela e as fontes na tela, e o Resumo — a parte mais lida — ficava acima, fora da vista
  // (no celular, sempre). A pergunta e a espera seguem rolando até o fim, como antes.
  useEffect(() => {
    const ultima = messages[messages.length - 1];
    requestAnimationFrame(() => {
      if (ultima?.from === "server" && ultima.showButton && !ultima.pendente) {
        document.getElementById(`msg-${ultima.id}`)?.scrollIntoView({ behavior: "smooth", block: "start" });
      } else {
        messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
      }
    });
  }, [messages]);

  // Antes da primeira pergunta, a apresentação do tipo selecionado no lugar da saudação (D-UI-10).
  const vazio = !messages.some((m) => m.from === "user");
  const usarExemplo = (pergunta: string) => {
    setPrompt(pergunta);
    requestAnimationFrame(() => textareaRef.current?.focus());
  };

  // Devolve a pergunta à caixa COM o tipo em que foi feita (D-HIST-6): sem isso, a pergunta de
  // Pareceres iria para Serviços + FAQs se o seletor tivesse mudado. O `updateQuestionType` já
  // ignora o tipo que a entidade não tem. Não envia — quem pergunta ainda pode ajustar.
  const usarDoHistorico = (item: ItemHistorico) => {
    updateQuestionType(item.tipo);
    setDrawerAberto(false);
    setRecolhido(false);
    usarExemplo(item.pergunta);
  };

  const painel = (aoFechar: () => void) => (
    <Historico
      itens={historico.itens}
      onUsar={usarDoHistorico}
      onFavoritar={historico.alternarFavorita}
      onRemover={historico.remover}
      onLimparRecentes={historico.limparRecentes}
      onFechar={aoFechar}
    />
  );

  const handleCallServerAPI = async (prompt: string) => {
    const trimmedPrompt = prompt.trim();
    if (isSubmittingRef.current || loading || !isPromptValid(trimmedPrompt)) return;
    isSubmittingRef.current = true;
    // No ENVIO, não na chegada (D-HIST-3): a pergunta que deu erro ou estourou o tempo é
    // justamente a que a pessoa vai querer reenviar.
    historico.registrar(trimmedPrompt, questionType);
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
    <Flex w="100%" flex={1} bg="bg.app">
      <Flex flexDirection="column" flex={1} minW={0} pb={`${composerHeight + 16}px`}>
        {vazio ? (
          <EstadoInicial tipo={questionType} onExemplo={usarExemplo} />
        ) : (
          <Messages messages={messages} setPrompt={setPrompt} />
        )}
        <div ref={messagesEndRef} />
      </Flex>

      {/* Painel (>= xl): coluna à direita, DENTRO do fluxo — o chat encolhe em vez de ser coberto,
          e a barra de rolagem da área de conteúdo segue à vista, à direita dele. `sticky` porque o
          Chat mora dentro do único container de scroll (ver `Home`): sem isso o painel subiria
          junto com as mensagens. A altura é a da área que rola, que nesta largura é a tela menos
          o cabeçalho. O zIndex fica ACIMA da barra de composição (20): ela é `fixed` e não desconta
          a barra de rolagem, então avança alguns pixels sobre a borda do painel. */}
      {painelAberto && (
        <Box
          as="aside"
          aria-label="Minhas perguntas"
          hideBelow="xl"
          position="sticky"
          top={0}
          alignSelf="flex-start"
          flexShrink={0}
          w={HISTORICO_WIDTH}
          h={`calc(100dvh - ${HEADER_HEIGHT})`}
          overflowY="auto"
          bg="bg.app"
          borderLeft="1px solid var(--chakra-colors-border)"
          zIndex={21}
        >
          {painel(() => setPainelAberto(false))}
        </Box>
      )}

      {/* Drawer (< xl): overlay + o MESMO painel, vindo da direita. Feito à mão e com o zIndex
          acima do AppHeader, pelos motivos do drawer de seções no `Home`. */}
      {drawerAberto && (
        <Box hideFrom="xl" position="fixed" inset={0} zIndex={110}>
          <Box position="absolute" inset={0} bg="blackAlpha.500" onClick={() => setDrawerAberto(false)} />
          <Box
            as="aside"
            aria-label="Minhas perguntas"
            position="absolute"
            top={0}
            bottom={0}
            right={0}
            w="min(320px, 88vw)"
            overflowY="auto"
            bg="bg.app"
            borderLeft="1px solid var(--chakra-colors-border)"
          >
            {painel(() => setDrawerAberto(false))}
          </Box>
        </Box>
      )}

      {/* Barra de composição. `fixed` é o que permite subi-la acima do teclado virtual — mas
          `fixed` se ancora na VIEWPORT, não na área de conteúdo, então ela precisa recuar a
          largura da sidebar. Sem isso invade o menu, que é o que acontecia desde que a navegação
          virou sidebar (antes as abas eram uma faixa no topo e a barra cheia estava certa).

          `right={0}` em vez de `width="100%"`: com o recuo à esquerda, 100% transbordaria à
          direita exatamente a largura da sidebar. Abaixo de `md` a sidebar vira drawer e não
          ocupa espaço, daí o `base: 0`. O recuo vem da variável que o `Home` define (a faixa de
          ícones com a sidebar recolhida); `SIDEBAR_WIDTH` é o reserva para o Chat montado fora
          dele. */}
      <Box
        ref={composerRef}
        position="fixed"
        // Lift the input above the on-screen keyboard when it's open. The
        // keyboard height is measured from visualViewport, so this needs no
        // device/UA check — desktops simply report 0.
        bottom={isKeyboardOpen ? `${keyboardHeight}px` : 0}
        left={{ base: 0, md: `var(${SIDEBAR_VAR}, ${SIDEBAR_WIDTH})` }}
        // Com o painel "Minhas perguntas" aberto (>= xl), recua também à direita.
        right={painelAberto ? { base: 0, xl: HISTORICO_WIDTH } : 0}
        zIndex={20}
        // Mesma cor do fundo: a caixa é que se destaca (borda e sombra), não uma faixa branca de
        // ponta a ponta (D-UI-4).
        bg="bg.app"
      >
        {/* A linha acima da caixa, alinhada com ela: à esquerda, "Minhas perguntas" (em toda
            largura); à direita, só no celular, o "Recolher" — e, recolhida, a barrinha que a traz
            de volta. A caixa segue filha DIRETA desta barra fixa (o teste da sidebar sobe por
            `parentElement`), então quem a esconde é o próprio `Input`, pela prop `recolhido`. */}
        <Flex
          display={{ base: recolhido ? "none" : "flex", md: "flex" }}
          justify="space-between"
          maxW="760px"
          mx={{ base: 3, md: "auto" }}
        >
          <chakra.button
            {...BOTAO_HISTORICO}
            hideBelow="xl"
            aria-label={painelAberto ? "Recolher minhas perguntas" : "Mostrar minhas perguntas"}
            aria-expanded={painelAberto}
            onClick={() => setPainelAberto(!painelAberto)}
          >
            <LuPanelRight size={14} aria-hidden="true" />
            Minhas perguntas
          </chakra.button>
          <chakra.button
            {...BOTAO_HISTORICO}
            ref={botaoDrawerRef}
            hideFrom="xl"
            aria-label="Abrir minhas perguntas"
            aria-expanded={drawerAberto}
            onClick={() => setDrawerAberto(true)}
          >
            <LuPanelRight size={14} aria-hidden="true" />
            Minhas perguntas
          </chakra.button>
          <chakra.button
            type="button"
            aria-label="Recolher a caixa de mensagem"
            aria-expanded={true}
            onClick={() => setRecolhido(true)}
            display={{ base: "flex", md: "none" }}
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
          placeholder={TIPOS[questionType].placeholder}
        >
          <SelectQuestionType
            questionType={questionType}
            updateQuestionType={updateQuestionType}
            disponiveis={disponiveis}
          />
        </Input>
        {/* O aviso de protótipo saiu de cada resposta (eram 3 linhas repetidas) e mora aqui, uma vez
            (D-UI-4). O texto completo, validado com quem atende, está no Sobre. */}
        <Text
          display={{ base: recolhido ? "none" : "block", md: "block" }}
          textAlign="center"
          fontSize="12px"
          color="fg.muted"
          px={3}
          pb={2}
        >
          A Auli é experimental e pode errar — confira as fontes.
        </Text>
      </Box>
    </Flex>
  );
};
