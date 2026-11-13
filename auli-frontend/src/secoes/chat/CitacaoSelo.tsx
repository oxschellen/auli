import { chakra } from "@chakra-ui/react";

interface CitacaoSeloProps {
  rotulo: string;
  /** Título da fonte, quando a lista de fontes a tem — vira a dica do selo. */
  titulo?: string;
  onClick: () => void;
}

/**
 * O selo de uma citação no texto (`[Serviço 2]` → "Serviço 2" clicável, D-UI-7). Clicar leva à
 * fonte na lista abaixo da resposta e a realça; a dica nativa (`title`) mostra o título. É um
 * `<button>` de verdade — Tab chega nele e o leitor de tela anuncia o que ele faz.
 */
export const CitacaoSelo = ({ rotulo, titulo, onClick }: CitacaoSeloProps) => (
  <chakra.button
    type="button"
    onClick={onClick}
    title={titulo}
    aria-label={titulo ? `${rotulo}: ${titulo}. Ver nas fontes` : `${rotulo}. Ver nas fontes`}
    display="inline-flex"
    alignItems="center"
    h="20px"
    px="6px"
    mx="1px"
    verticalAlign="1px"
    borderRadius="6px"
    border="1px solid"
    borderColor="border"
    bg="bg.subtle"
    color="fg"
    fontSize="12px"
    fontWeight="600"
    lineHeight="1"
    whiteSpace="nowrap"
    cursor="pointer"
    transition="background-color 0.15s ease, color 0.15s ease"
    _hover={{ bg: "accent", color: "accent.fg", borderColor: "accent" }}
  >
    {rotulo}
  </chakra.button>
);
