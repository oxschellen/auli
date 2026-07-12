import { Box, HStack, RadioGroup, Text, Flex } from "@chakra-ui/react";
import type { QuestionType } from "./utils/useQuestionType";

/** The two query types, in display order. Values are sent (as numbers) in the request `type` field.
 *  Pareceres is disabled until the backend handles type 2 (later refactor). */
const questionOptions: { label: string; value: QuestionType; disabled?: boolean }[] = [
  { label: "Serviços+FAQs", value: "1" },
  { label: "Pareceres", value: "2", disabled: true },
];

interface SelectQuestionTypeProps {
  questionType: QuestionType;
  updateQuestionType: (value: string | null) => void;
}

/**
 * Radio selector shown above the chat input: pick the kind of query to run. Themed with the app's
 * semantic tokens so the selected chip follows the accent color in both light and dark mode.
 */
export const SelectQuestionType = ({ questionType, updateQuestionType }: SelectQuestionTypeProps) => {
  return (
    <Box mx={3} maxW="1440px" bg="bg.subtle" borderRadius="12px" px={2} py={1} mb={2}>
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
          {questionOptions.map((opt) => {
            const selected = questionType === opt.value;
            const disabled = Boolean(opt.disabled);
            return (
              <RadioGroup.Item
                key={opt.value}
                value={opt.value}
                disabled={disabled}
                cursor={disabled ? "not-allowed" : "pointer"}
              >
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
                  opacity={disabled ? 0.45 : 1}
                  transition="all 0.2s ease"
                  _hover={disabled ? undefined : {
                    borderColor: "accent",
                    transform: "translateY(-1px)",
                  }}
                >
                  <RadioGroup.ItemText fontWeight={selected ? "600" : "500"} fontSize="0.665rem">
                    {opt.label}
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
