import { Box, Flex, Text } from "@chakra-ui/react";
import { Highlight } from "../shared/highlight";
import { TributumShell } from "./TributumShell";
import { PlaceholderBadge } from "./PlaceholderBadge";
import { LinkExterno } from "./LinkExterno";
import type { AcessoLivro, Livro } from "./types";

/**
 * Rótulo do selo e do link, por forma de acesso (D-TRIB-24).
 *
 * Os dois andam juntos de propósito: dizer "Acesso aberto" e oferecer "Ver na editora" seria
 * prometer uma coisa e entregar outra. Uma tabela só, e o cartão lê as duas pontas dela.
 */
const ACESSO: Record<AcessoLivro, { selo: string; rotulo: string }> = {
  aberto: { selo: "Acesso aberto", rotulo: "Ler" },
  comercial: { selo: "Venda", rotulo: "Ver na editora" },
  biblioteca: { selo: "Biblioteca", rotulo: "Ver no catálogo" },
};

/**
 * A estante **Livros** (D-TRIB-22..26). Estrutura da `InstituicoesLista` — sem modal e sem PDF,
 * porque livro nunca é hospedado aqui: obra comercial não se hospeda, e a aberta já está publicada
 * por quem a publica.
 *
 * O selo de acesso é o que essa estante tem de próprio. Ele existe porque o custo de clicar é
 * assimétrico: quem procura um PDF e cai numa loja perdeu a viagem, e a lista sabe a resposta antes
 * do clique. É o papel que o `hospedado` cumpre nos artigos.
 *
 * **Sem capa** (D-TRIB-26): imagem de capa é obra protegida da editora, e a lista é texto.
 */
export function LivrosLista() {
  return (
    <TributumShell<Livro>
      titulo="Livros"
      secao="livros"
      substantivo={["livro", "livros"]}
      campos={(l) => [l.titulo, l.autores, l.editora, l.resumo]}
    >
      {(l, terms) => {
        const acesso = ACESSO[l.acesso] ?? ACESSO.comercial;
        return (
          <>
            <Flex justify="space-between" align="flex-start" gap={3}>
              <Text fontWeight="600" fontSize="0.95rem" color="fg">
                <Highlight text={l.titulo} terms={terms} />
              </Text>
              <PlaceholderBadge item={l} />
            </Flex>
            <Text fontSize="0.8rem" color="fg.muted" mt={1}>
              <Highlight text={`${l.autores} · ${l.editora}, ${l.ano}`} terms={terms} />
              {l.isbn ? ` · ISBN ${l.isbn}` : ""}
            </Text>
            <Text fontSize="0.9rem" color="fg" lineHeight="1.6" mt={2}>
              <Highlight text={l.resumo} terms={terms} />
            </Text>
            <Flex mt={3} align="center" gap={3} wrap="wrap">
              {/* Mesma pílula do `PlaceholderBadge`, e pelo mesmo motivo: o `Badge` do Chakra puxa
                  `colorPalette` em vez dos tokens do tema e ignoraria o modo escuro. Aqui em
                  `bg.subtle`, não no âmbar do selo de exemplo — isto informa, não alerta. */}
              <Box
                as="span"
                flexShrink={0}
                bg="bg.subtle"
                color="fg.muted"
                fontSize="0.7rem"
                fontWeight="600"
                textTransform="uppercase"
                letterSpacing="0.04em"
                px={2}
                py="1px"
                borderRadius="full"
                borderWidth="1px"
                borderColor="border"
              >
                {acesso.selo}
              </Box>
              <LinkExterno
                href={l.link}
                rotulo={acesso.rotulo}
                ariaLabel={`${acesso.rotulo}: ${l.titulo}`}
              />
            </Flex>
          </>
        );
      }}
    </TributumShell>
  );
}
