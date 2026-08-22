import { Box } from "@chakra-ui/react";
import { ehPlaceholder, type ItemBase } from "./types";

/**
 * O selo "exemplo" da D-TRIB-7. Renderiza para todo item que **não** se declarou publicado — o
 * default é o selo, não a ausência dele (ver `ehPlaceholder`).
 *
 * **Não é o `Badge` do Chakra**, e a divergência é deliberada: o componente não é usado em lugar
 * nenhum deste frontend, e o `colorPalette` dele puxa a paleta do Chakra em vez dos tokens
 * semânticos do `theme/system.js` — o que daria um selo que ignora o modo escuro do app. A casa
 * monta tudo com `Box`/`Flex`/`Text` sobre tokens, e é o que se faz aqui.
 *
 * O par `bg.highlight`/`fg.highlight` é o âmbar translúcido que o tema reserva para chamar atenção
 * sem sugerir clique — `accent` é a cor de link neste app, e um selo em accent leria como botão.
 * Ele já é o realce da busca; aqui a forma (pílula, fora do fluxo do texto) é o que distingue os
 * dois usos.
 */
export function PlaceholderBadge({ item }: { item: ItemBase }) {
  if (!ehPlaceholder(item)) return null;
  return (
    <Box
      as="span"
      flexShrink={0}
      bg="bg.highlight"
      color="fg.highlight"
      fontSize="0.7rem"
      fontWeight="600"
      textTransform="uppercase"
      letterSpacing="0.04em"
      px={2}
      py="1px"
      borderRadius="full"
      title="Item de exemplo — ainda não passou pela revisão da curadoria"
    >
      exemplo
    </Box>
  );
}
