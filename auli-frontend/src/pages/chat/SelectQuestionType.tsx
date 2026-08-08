import { Box, HStack, RadioGroup, Text, Flex } from "@chakra-ui/react";
import type { QuestionType } from "./utils/useQuestionType";

/** O rótulo de cada tipo. Os valores são enviados (como número) no campo `type` da requisição. */
const ROTULOS: Record<QuestionType, string> = {
  "1": "Serviços+FAQs",
  "2": "Pareceres",
  "3": "Acórdãos TARF",
};

interface SelectQuestionTypeProps {
  questionType: QuestionType;
  updateQuestionType: (value: string | null) => void;
  /** Os tipos que esta entidade tem, na ordem de exibição (ver `tiposDisponiveis`). */
  disponiveis: QuestionType[];
}

/**
 * Radio selector shown at the top of the chat message box: pick the kind of query to run. Themed
 * with the app's semantic tokens so the selected chip follows the accent color in both light and
 * dark mode.
 *
 * Não tem margem horizontal própria: quem espaça é o cartão que o contém (ver `Input`).
 */
export const SelectQuestionType = ({
  questionType,
  updateQuestionType,
  disponiveis,
}: SelectQuestionTypeProps) => {
  // Com uma opção só não há o que escolher, e um rádio solto lê como controle quebrado.
  if (disponiveis.length < 2) return null;
  return (
    <Box maxW="1440px" bg="bg.subtle" borderRadius="12px" px={2} py={1} mb={2}>
      <Text
        fontSize="0.525rem"
        fontWeight="600"
        color="fg.muted"
        mt={1}
        mb={1}
        textTransform="uppercase"
        letterSpacing="0.05em"
      >
        Selecione o tipo de consulta
      </Text>
      <RadioGroup.Root
        value={questionType}
        onValueChange={({ value }) => updateQuestionType(value)}
      >
        <HStack gap={2} flexWrap="wrap" justify="flex-start">
          {disponiveis.map((tipo) => {
            const selected = questionType === tipo;
            return (
              <RadioGroup.Item key={tipo} value={tipo} cursor="pointer">
                <RadioGroup.ItemHiddenInput />
                <RadioGroup.ItemIndicator display="none" />
                <Flex
                  direction="column"
                  align="flex-start"
                  bg={selected ? "accent" : "bg.canvas"}
                  color={selected ? "accent.fg" : "fg"}
                  px={2}
                  py={0.5}
                  borderRadius="7px"
                  border="2px solid"
                  borderColor={selected ? "accent" : "border"}
                  transition="all 0.2s ease"
                  _hover={{
                    borderColor: "accent",
                    transform: "translateY(-1px)",
                  }}
                >
                  <RadioGroup.ItemText fontWeight={selected ? "600" : "500"} fontSize="0.665rem">
                    {ROTULOS[tipo]}
                  </RadioGroup.ItemText>
                </Flex>
              </RadioGroup.Item>
            );
          })}
        </HStack>
      </RadioGroup.Root>
    </Box>
  );
};
