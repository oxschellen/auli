import { Box, Flex, Heading, Text, chakra } from "@chakra-ui/react";
import { useState } from "react";
import type { ReactNode } from "react";
import { MdChevronRight, MdExpandMore } from "react-icons/md";
import type { Fonte } from "../../types/chat";
import { toAbsoluteHref } from "../../shared/markdown";
import { agruparFontes } from "./utils/fontes";
import { ancora } from "./utils/citacoes";

/** A linha de contagem do cabeçalho: "8 recuperadas · 3 descartadas na triagem". */
export function resumoFontes(fontes: Fonte[]): string {
  const descartadas = fontes.filter((f) => f.triagem === "descarta").length;
  const base = `${fontes.length} ${fontes.length === 1 ? "recuperada" : "recuperadas"}`;
  if (descartadas === 0) return base;
  return `${base} · ${descartadas} ${descartadas === 1 ? "descartada" : "descartadas"} na triagem`;
}

interface FontesProps {
  fontes: Fonte[];
  /** Rótulos que o texto da resposta cita (`rotulosCitados`). */
  citados: Set<string>;
  /** Prefixo dos ids dos itens — o id da mensagem —, porque o mesmo rótulo existe em toda resposta. */
  idBase: string;
  /** Rótulo realçado agora (o selo clicado no texto). */
  destaque?: string | null;
}

const Item = ({ f, idBase, ativo }: { f: Fonte; idBase: string; ativo: boolean }) => {
  const descartada = f.triagem === "descarta";
  return (
    <Flex
      id={`${idBase}-${ancora(f.rotulo)}`}
      align="center"
      wrap="wrap"
      columnGap={2.5}
      rowGap={0.5}
      px={2}
      py={1}
      mx={-2}
      borderRadius="8px"
      bg={ativo ? "bg.resumo" : "transparent"}
      transition="background-color 0.3s ease"
      color={descartada ? "fg.muted" : "fg"}
    >
      <Text as="span" flex="none" minW="64px" fontSize="12px" fontWeight="700" color={ativo ? "accent.strong" : "inherit"}>
        {f.rotulo}
      </Text>
      {f.link ? (
        <a
          href={toAbsoluteHref(f.link)}
          target="_blank"
          rel="noopener noreferrer"
          style={{ color: descartada ? "inherit" : "var(--chakra-colors-accent)", textDecoration: "underline", textUnderlineOffset: "3px" }}
        >
          {f.titulo}
        </a>
      ) : (
        <span>{f.titulo}</span>
      )}
      {f.triagem === "condicional" && (
        <Text as="span" fontSize="12px" fontWeight="600" px={1.5} lineHeight="20px" borderRadius="6px" bg="condicional.bg" color="condicional.fg">
          depende de condição
        </Text>
      )}
    </Flex>
  );
};

const Grupo = ({ titulo, children }: { titulo: string; children: ReactNode }) => (
  <Flex direction="column" gap={1}>
    <Text fontSize="11px" fontWeight="700" letterSpacing="0.06em" color="fg.muted" textTransform="uppercase">
      {titulo}
    </Text>
    {children}
  </Flex>
);

const Alternar = ({ aberto, onClick, children }: { aberto: boolean; onClick: () => void; children: ReactNode }) => (
  <chakra.button
    type="button"
    aria-expanded={aberto}
    onClick={onClick}
    alignSelf="flex-start"
    display="flex"
    alignItems="center"
    gap={1}
    h="32px"
    px={2}
    ml={-2}
    borderRadius="8px"
    fontSize="13.5px"
    color="fg.muted"
    _hover={{ bg: "bg.overlay" }}
  >
    {aberto ? <MdExpandMore size={16} /> : <MdChevronRight size={16} />}
    {children}
  </chakra.button>
);

/**
 * As fontes da resposta de Serviços + FAQs, em três grupos (D-UI-8; antes era uma lista única
 * recolhida, D-SF-10).
 *
 * As **citadas** ficam sempre à vista: são poucas e são o que sustenta o texto. As **consideradas**
 * — inclusive as `condicional`, a alternativa que o texto às vezes omite — abrem por padrão só
 * quando o texto não cita nada (aí elas são tudo o que há). As **descartadas** ficam recolhidas, mas
 * não somem: o descarte errado tem que ser visível.
 */
export const Fontes = ({ fontes, citados, idBase, destaque = null }: FontesProps) => {
  const g = agruparFontes(fontes, citados);
  const [consideradasAbertas, setConsideradasAbertas] = useState(g.citadas.length === 0);
  const [descartadasAbertas, setDescartadasAbertas] = useState(false);

  return (
    <Flex as="section" aria-label="Fontes consultadas" direction="column" gap={3} mt={1} pt={3} borderTop="1px solid" borderColor="border" fontSize="14.5px" lineHeight="1.45">
      <Flex align="baseline" justify="space-between" gap={3} wrap="wrap">
        <Heading as="h3" fontSize="15px" fontWeight="700" m={0}>
          Fontes
        </Heading>
        <Text fontSize="13px" color="fg.muted">
          {resumoFontes(fontes)}
        </Text>
      </Flex>

      {g.citadas.length > 0 && (
        <Grupo titulo="Citadas na resposta">
          {g.citadas.map((f) => (
            <Item key={f.rotulo} f={f} idBase={idBase} ativo={destaque === f.rotulo} />
          ))}
        </Grupo>
      )}

      {g.consideradas.length > 0 &&
        (consideradasAbertas ? (
          <Grupo titulo="Também consideradas">
            {g.consideradas.map((f) => (
              <Item key={f.rotulo} f={f} idBase={idBase} ativo={destaque === f.rotulo} />
            ))}
          </Grupo>
        ) : (
          <Alternar aberto={false} onClick={() => setConsideradasAbertas(true)}>
            {g.consideradas.length} também {g.consideradas.length === 1 ? "considerada" : "consideradas"}
          </Alternar>
        ))}

      {g.descartadas.length > 0 && (
        <Box>
          <Alternar aberto={descartadasAbertas} onClick={() => setDescartadasAbertas((v) => !v)}>
            {g.descartadas.length} {g.descartadas.length === 1 ? "descartada" : "descartadas"} na triagem
          </Alternar>
          {descartadasAbertas && (
            <Flex direction="column" gap={1} mt={1}>
              {g.descartadas.map((f) => (
                <Item key={f.rotulo} f={f} idBase={idBase} ativo={destaque === f.rotulo} />
              ))}
            </Flex>
          )}
        </Box>
      )}
    </Flex>
  );
};
