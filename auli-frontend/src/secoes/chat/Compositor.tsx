import { useEffect, useRef } from "react";
import type { ChangeEvent, ReactNode, RefObject } from "react";
import { Box, Flex, Text, chakra } from "@chakra-ui/react";
import { MdEdit, MdKeyboardArrowDown, MdKeyboardArrowUp } from "react-icons/md";
import { LuPanelRight } from "react-icons/lu";
import { Input } from "./Input";
import { useIsKeyboardVisible } from "./hooks/useIsKeyboardVisible";
import { HISTORICO_WIDTH, SIDEBAR_VAR, SIDEBAR_WIDTH, Z } from "../../ui/layout";

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

/**
 * A barra de composição do chat (I8 do docs/ESTUDO-frontend-organizacao.md): a linha dos botões
 * ("Minhas perguntas" e, no celular, "Recolher"), a barrinha que traz a caixa de volta, a caixa de
 * mensagem com o seletor de tipo (`children`) e o aviso de protótipo.
 *
 * `fixed` é o que permite subi-la acima do teclado virtual — mas `fixed` se ancora na VIEWPORT, não
 * na área de conteúdo, então ela precisa recuar a largura da sidebar. Sem isso invade o menu, que é
 * o que acontecia desde que a navegação virou sidebar. `right={0}` em vez de `width="100%"`: com o
 * recuo à esquerda, 100% transbordaria à direita exatamente a largura da sidebar. Abaixo de `md` a
 * sidebar vira drawer e não ocupa espaço, daí o `base: 0`. O recuo vem da variável que o `Home`
 * define (a faixa de ícones com a sidebar recolhida); `SIDEBAR_WIDTH` é o reserva para o Chat
 * montado fora dele.
 *
 * Como cobre o fim da lista, a barra MEDE a própria altura e a informa (`onAltura`): o recuo era
 * fixo, e no celular a barra passa dele — o seletor de tipo quebra em duas linhas —, escondendo a
 * última faixa de cada resposta (os botões do log e de cópia).
 */
export function Compositor({
  textareaRef,
  prompt,
  onDigitar,
  esperando,
  onEnviar,
  placeholder,
  children,
  painelAberto,
  onAlternarPainel,
  drawerAberto,
  onAbrirDrawer,
  botaoDrawerRef,
  recolhido,
  onRecolher,
  onExpandir,
  onAltura,
}: {
  textareaRef: RefObject<HTMLTextAreaElement | null>;
  prompt: string;
  onDigitar: (e: ChangeEvent<HTMLTextAreaElement>) => void;
  esperando: boolean;
  onEnviar: (pergunta: string) => void;
  placeholder: string;
  /** O seletor de tipo de consulta, que mora DENTRO da caixa de mensagem. */
  children: ReactNode;
  /** A coluna "Minhas perguntas" (a partir de `xl`): a barra recua a largura dela à direita. */
  painelAberto: boolean;
  onAlternarPainel: () => void;
  drawerAberto: boolean;
  onAbrirDrawer: () => void;
  botaoDrawerRef: RefObject<HTMLButtonElement | null>;
  /** Só no celular: a caixa recolhida vira uma barrinha, liberando a leitura da resposta. */
  recolhido: boolean;
  onRecolher: () => void;
  onExpandir: () => void;
  onAltura: (px: number) => void;
}) {
  const { isKeyboardVisible: isKeyboardOpen, keyboardHeight } = useIsKeyboardVisible();

  const barraRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = barraRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => onAltura(el.offsetHeight));
    ro.observe(el);
    return () => ro.disconnect();
  }, [onAltura]);

  return (
    <Box
      ref={barraRef}
      position="fixed"
      // Lift the input above the on-screen keyboard when it's open. The
      // keyboard height is measured from visualViewport, so this needs no
      // device/UA check — desktops simply report 0.
      bottom={isKeyboardOpen ? `${keyboardHeight}px` : 0}
      left={{ base: 0, md: `var(${SIDEBAR_VAR}, ${SIDEBAR_WIDTH})` }}
      // Com o painel "Minhas perguntas" aberto (>= xl), recua também à direita.
      right={painelAberto ? { base: 0, xl: HISTORICO_WIDTH } : 0}
      zIndex={Z.compositor}
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
          onClick={onAlternarPainel}
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
          onClick={onAbrirDrawer}
        >
          <LuPanelRight size={14} aria-hidden="true" />
          Minhas perguntas
        </chakra.button>
        <chakra.button
          type="button"
          aria-label="Recolher a caixa de mensagem"
          aria-expanded={true}
          onClick={onRecolher}
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
        onClick={onExpandir}
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

      <Input
        textareaRef={textareaRef}
        prompt={prompt}
        updatePrompt={onDigitar}
        loading={esperando}
        onEnviar={onEnviar}
        recolhido={recolhido}
        placeholder={placeholder}
      >
        {children}
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
  );
}
