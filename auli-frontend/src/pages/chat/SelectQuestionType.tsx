import { Box, Flex, RadioGroup, Text, chakra } from "@chakra-ui/react";
import { useEffect, useRef, useState } from "react";
import { MdExpandMore } from "react-icons/md";
import type { QuestionType } from "./utils/useQuestionType";
import { TIPOS } from "./utils/tipos";

interface SelectQuestionTypeProps {
  questionType: QuestionType;
  updateQuestionType: (value: string | null) => void;
  /** Os tipos que esta entidade tem, na ordem de exibição (ver `tiposDisponiveis`). */
  disponiveis: QuestionType[];
}

/**
 * O seletor do tipo de consulta (D-UI-4): um botão compacto DENTRO da caixa de mensagem, à
 * esquerda do campo, que abre a lista dos tipos com uma linha de descrição cada. Era uma faixa de
 * chips com rótulo "SELECIONE O TIPO DE CONSULTA" acima do campo — no celular, quebrava em duas
 * linhas e a caixa passava de 220px.
 *
 * A lista é um `radiogroup` de verdade (setas, Tab e leitor de tela continuam funcionando), num
 * painel que abre para CIMA (a caixa fica no pé da tela). Fecha ao escolher, com Esc ou com clique
 * fora.
 */
export const SelectQuestionType = ({ questionType, updateQuestionType, disponiveis }: SelectQuestionTypeProps) => {
  const [aberto, setAberto] = useState(false);
  const raiz = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!aberto) return;
    const fora = (e: MouseEvent) => {
      if (raiz.current && !raiz.current.contains(e.target as Node)) setAberto(false);
    };
    document.addEventListener("mousedown", fora);
    return () => document.removeEventListener("mousedown", fora);
  }, [aberto]);

  // Com uma opção só não há o que escolher, e um seletor solto lê como controle quebrado.
  if (disponiveis.length < 2) return null;
  const atual = TIPOS[questionType];

  return (
    <Box ref={raiz} position="relative" flex="none" onKeyDown={(e) => e.key === "Escape" && setAberto(false)}>
      <chakra.button
        type="button"
        aria-expanded={aberto}
        aria-label={`Tipo de consulta: ${atual.rotulo}. Trocar`}
        onClick={() => setAberto((v) => !v)}
        display="flex"
        alignItems="center"
        gap={1}
        h={{ base: "44px", md: "40px" }}
        pl={3}
        pr={2}
        borderRadius="10px"
        bg={aberto ? "bg.resumo" : "bg.subtle"}
        boxShadow={aberto ? "inset 0 0 0 1.5px var(--chakra-colors-accent)" : "none"}
        color="fg"
        fontSize={{ base: "13.5px", md: "14px" }}
        fontWeight="600"
        whiteSpace="nowrap"
        cursor="pointer"
        _hover={{ bg: "bg.resumo" }}
      >
        {atual.rotulo}
        <MdExpandMore size={16} color="var(--chakra-colors-fg-muted)" />
      </chakra.button>

      {aberto && (
        <Box
          position="absolute"
          bottom="calc(100% + 10px)"
          left={0}
          w="min(340px, calc(100vw - 32px))"
          p={1.5}
          bg="bg.canvas"
          border="1px solid"
          borderColor="border"
          borderRadius="12px"
          boxShadow="popover"
          zIndex={30}
        >
          <RadioGroup.Root
            aria-label="Tipo de consulta"
            value={questionType}
            onValueChange={({ value }) => {
              updateQuestionType(value);
              setAberto(false);
            }}
          >
            <Flex direction="column" gap={0.5}>
              {disponiveis.map((tipo) => {
                const selected = questionType === tipo;
                return (
                  <RadioGroup.Item key={tipo} value={tipo} cursor="pointer" w="100%">
                    <RadioGroup.ItemHiddenInput />
                    <RadioGroup.ItemIndicator display="none" />
                    <Flex
                      direction="column"
                      align="flex-start"
                      gap={0.5}
                      w="100%"
                      px={3}
                      py={2}
                      borderRadius="8px"
                      bg={selected ? "bg.resumo" : "transparent"}
                      _hover={{ bg: selected ? "bg.resumo" : "bg.overlay" }}
                    >
                      <RadioGroup.ItemText fontSize="14.5px" fontWeight="600" color={selected ? "accent.strong" : "fg"}>
                        {TIPOS[tipo].rotulo}
                      </RadioGroup.ItemText>
                      <Text fontSize="13px" color="fg.muted">
                        {TIPOS[tipo].descricao}
                      </Text>
                    </Flex>
                  </RadioGroup.Item>
                );
              })}
            </Flex>
          </RadioGroup.Root>
        </Box>
      )}
    </Box>
  );
};
