import { Box, Flex, Text } from "@chakra-ui/react";
import { keyframes } from "@emotion/react";
import { useEffect, useState } from "react";

/**
 * O que aparece no lugar da resposta enquanto a consulta roda. Substitui o "Aguarde! Pensando..."
 * estático: a espera vai de 7 a 30 s (busca + triagem + redação), e texto parado nesse tempo lê
 * como app travado.
 *
 * As etapas são **por tempo decorrido, não progresso real** — o servidor não informa a fase. Por
 * isso as frases valem para os quatro tipos de consulta (nem todos têm triagem) e nenhuma promete
 * prazo. Os limites seguem o que se mediu no `rs` com `medium`: triagem ~5 s, total
 * 7–9 s.
 *
 * Só `transform` e `opacity` animam (regra `no-layout-property-animation` do react-doctor), e
 * com `prefers-reduced-motion` tudo fica parado — o texto e o contador seguem informando.
 */
export const ETAPAS: { aPartirDe: number; texto: string }[] = [
  { aPartirDe: 0, texto: "Buscando nos documentos" },
  { aPartirDe: 3, texto: "Separando o que se aplica à sua pergunta" },
  { aPartirDe: 8, texto: "Redigindo a resposta" },
  { aPartirDe: 25, texto: "Quase lá — respostas longas levam um pouco mais" },
];

/** A etapa vigente aos `segundos` decorridos: a última cujo limite já passou. */
export const etapaEm = (segundos: number) =>
  [...ETAPAS].reverse().find((e) => segundos >= e.aPartirDe)!.texto;

const pulo = keyframes`
  0%, 80%, 100% { transform: translateY(0); opacity: 0.35; }
  40% { transform: translateY(-6px); opacity: 1; }
`;

const entrada = keyframes`
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: translateY(0); }
`;

const SEM_MOVIMENTO = { "@media (prefers-reduced-motion: reduce)": { animation: "none" } };

export const Pensando = () => {
  const [segundos, setSegundos] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setSegundos((s) => s + 1), 1000);
    return () => clearInterval(id);
  }, []);
  const etapa = etapaEm(segundos);

  return (
    <Flex px={3} py={2} w="100%">
      <Flex
        alignItems="center"
        gap={3}
        bg="bg.canvas"
        color="fg"
        borderRadius="18px 18px 18px 4px"
        padding="12px 16px"
        minW="200px"
        maxW="85%"
        border="1px solid var(--chakra-colors-border)"
      >
        <Flex gap="5px" aria-hidden="true">
          {[0, 1, 2].map((i) => (
            <Box
              key={i}
              w="8px"
              h="8px"
              borderRadius="full"
              bg="accent"
              css={{
                animation: `${pulo} 1.2s ease-in-out ${i * 0.15}s infinite`,
                ...SEM_MOVIMENTO,
              }}
            />
          ))}
        </Flex>
        {/* `status` + polite: o leitor de tela anuncia a troca de etapa, não cada segundo. */}
        <Text role="status" aria-live="polite" fontSize="15px" color="fg.muted">
          {/* A `key` remonta o texto a cada etapa, o que reinicia a animação de entrada. */}
          <Box
            as="span"
            key={etapa}
            display="inline-block"
            css={{ animation: `${entrada} 0.35s ease-out`, ...SEM_MOVIMENTO }}
          >
            {etapa}…
          </Box>
        </Text>
        <Text
          fontSize="12px"
          color="fg.muted"
          ml="auto"
          aria-hidden="true"
          fontVariantNumeric="tabular-nums"
        >
          {segundos} s
        </Text>
      </Flex>
    </Flex>
  );
};
