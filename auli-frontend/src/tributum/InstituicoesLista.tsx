import { Box, Flex, Text } from "@chakra-ui/react";
import { Highlight } from "../shared/highlight";
import { TributumShell } from "./TributumShell";
import { PlaceholderBadge } from "./PlaceholderBadge";
import { LinkExterno } from "./LinkExterno";
import type { Instituicao } from "./types";

/** Instituições que produzem estudo tributário. Ação única: visitar o site. */
export function InstituicoesLista() {
  return (
    <TributumShell<Instituicao>
      titulo="Instituições"
      secao="instituicoes"
      substantivo={["instituição", "instituições"]}
      campos={(i) => [i.nome, i.descricao]}
    >
      {(i, terms) => (
        <>
          <Flex justify="space-between" align="flex-start" gap={3}>
            <Text fontWeight="600" fontSize="0.95rem" color="fg">
              <Highlight text={i.nome} terms={terms} />
            </Text>
            <PlaceholderBadge item={i} />
          </Flex>
          <Text fontSize="0.9rem" color="fg" lineHeight="1.6" mt={2}>
            <Highlight text={i.descricao} terms={terms} />
          </Text>
          <Box mt={3}>
            <LinkExterno
              href={i.homepage}
              rotulo="Visitar site"
              ariaLabel={`Visitar o site de ${i.nome}`}
            />
          </Box>
        </>
      )}
    </TributumShell>
  );
}
