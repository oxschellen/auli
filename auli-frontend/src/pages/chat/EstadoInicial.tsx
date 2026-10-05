import { Box, Flex, Heading, Text, chakra } from "@chakra-ui/react";
import { MdInfoOutline } from "react-icons/md";
import type { QuestionType } from "./utils/useQuestionType";
import { TIPOS } from "./utils/tipos";

interface EstadoInicialProps {
  tipo: QuestionType;
  /** Põe a pergunta de exemplo na caixa (não envia: quem pergunta ainda pode ajustar). */
  onExemplo: (pergunta: string) => void;
}

/**
 * O que o chat mostra antes da primeira pergunta (D-UI-10), no lugar do "Olá! Como posso ajudar?":
 * o que o tipo selecionado faz, três perguntas de exemplo e o que a Auli NÃO responde — situação de
 * contribuinte e erro de processamento de sistema, as perguntas que mais chegam e que o acervo
 * público não cobre. Muda com o tipo: é a apresentação dele.
 */
export const EstadoInicial = ({ tipo, onExemplo }: EstadoInicialProps) => {
  const t = TIPOS[tipo];
  return (
    <Flex w="100%" maxW="760px" mx="auto" direction="column" gap={6} px={3} pt={{ base: 6, md: 14 }} pb={6}>
      <Flex direction="column" gap={2.5}>
        <Heading as="h1" fontSize={{ base: "24px", md: "30px" }} lineHeight="1.2" fontWeight="700" letterSpacing="-0.02em">
          Como posso ajudar?
        </Heading>
        <Text fontSize="16.5px" lineHeight="1.6" color="fg.muted" maxW="640px">
          <Text as="strong" color="fg">
            {t.rotulo}
          </Text>{" "}
          {t.apresentacao}
        </Text>
      </Flex>

      <Flex direction="column" gap={2.5}>
        <Text fontSize="12px" fontWeight="700" letterSpacing="0.08em" color="fg.muted">
          EXPERIMENTE
        </Text>
        <Flex direction="column" align="flex-start" gap={2}>
          {t.exemplos.map((pergunta) => (
            <chakra.button
              key={pergunta}
              type="button"
              onClick={() => onExemplo(pergunta)}
              minH="44px"
              px={4}
              py={2.5}
              border="1px solid"
              borderColor="border"
              borderRadius="12px"
              bg="bg.canvas"
              color="fg"
              fontSize="15px"
              lineHeight="1.4"
              textAlign="left"
              cursor="pointer"
              transition="border-color 0.15s ease"
              _hover={{ borderColor: "accent" }}
            >
              {pergunta}
            </chakra.button>
          ))}
        </Flex>
      </Flex>

      <Flex gap={2.5} align="flex-start" px={3.5} py={3} borderRadius="12px" bg="bg.subtle" fontSize="14px" lineHeight="1.5" color="fg.muted">
        <Box flex="none" mt="2px">
          <MdInfoOutline size={16} />
        </Box>
        <Text>
          A Auli consulta só o acervo público. Situação de um contribuinte específico e erros de processamento dos
          sistemas exigem acesso aos dados — ela não responde.
        </Text>
      </Flex>
    </Flex>
  );
};
