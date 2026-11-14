import { useState } from "react";
import { Box, Flex, Link, Stack, Text } from "@chakra-ui/react";
import { comoBotao, PainelRevelado } from "../../ui/Acordeao";
import { MdExpandLess, MdExpandMore, MdOpenInNew } from "react-icons/md";
import { temDescricao, type Servico } from "./servicos";
import { Highlight } from "../../lib/highlight";
import { editLinks } from "../../lib/linkify";

interface AccordionItemProps {
  classe: string;
  items: Servico[];
  isOpen: boolean;
  onToggle: () => void;
  isFirst: boolean;
  isLast: boolean;
  /** Termos já normalizados (`parseQuery`) — os MESMOS que filtraram, para marcar o que casou. */
  terms: string[];
}

/**
 * Um serviço da classe. Com descrição, a linha abre e fecha a descrição ali mesmo, e o portal fica
 * no ícone ao lado; sem descrição, a linha continua sendo o link para o portal, como sempre foi
 * (docs/Estudo-servicos-descricao.md, D-SVC-2 a D-SVC-5).
 *
 * O ícone é IRMÃO do botão, não filho: um `<a>` dentro de `role="button"` teria o Enter capturado
 * pelo `onKeyDown` do `comoBotao` (o acordeão abriria em vez de o link seguir), e os filhos de um
 * botão são apresentacionais para o leitor de tela. Lado a lado, nenhum `stopPropagation` é
 * preciso. E é `<a>` de verdade, não `window.open`, para Ctrl+clique e "copiar endereço" valerem.
 *
 * A descrição é texto puro (`pre-wrap` + `editLinks`), não markdown: o acervo não tem markdown, e
 * o `ReactMarkdown` juntaria os parágrafos separados por quebra simples, viraria lista as linhas
 * começadas por `* ` ou `1. ` e apagaria trechos como `<Manual de Orientação>`.
 */
function ServicoItem({ s, terms }: { s: Servico; terms: string[] }) {
  const [aberto, setAberto] = useState(false);

  if (!temDescricao(s)) {
    return (
      <Link
        href={s.link}
        target="_blank"
        rel="noopener noreferrer"
        color="accent"
        fontSize="1rem"
        fontWeight="500"
        lineHeight="1.75"
        display="block"
      >
        <Highlight text={s.titulo} terms={terms} />
      </Link>
    );
  }

  return (
    <Box bg={aberto ? "bg.app" : undefined} borderRadius="md" mx={-2} px={2}>
      <Flex align="center" gap={1}>
        <Flex
          {...comoBotao(() => setAberto((a) => !a))}
          aria-expanded={aberto}
          flex={1}
          minW={0}
          align="center"
          gap={2}
          cursor="pointer"
        >
          <Text fontSize="1rem" fontWeight="500" lineHeight="1.75" color="fg" flex={1}>
            <Highlight text={s.titulo} terms={terms} />
          </Text>
          <Box color="fg.muted" display="flex" aria-hidden>
            {aberto ? <MdExpandLess size={20} /> : <MdExpandMore size={20} />}
          </Box>
        </Flex>
        <Link
          href={s.link}
          target="_blank"
          rel="noopener noreferrer"
          aria-label={`Abrir ${s.titulo} no portal oficial`}
          title="Abrir no portal"
          color="accent"
          display="flex"
          flexShrink={0}
          p={2}
        >
          <MdOpenInNew size={14} />
        </Link>
      </Flex>
      <PainelRevelado aberto={aberto}>
        {/* Um recuo de 20 px sob o título, o mesmo passo do serviço sob a classe. */}
        <Box
          pl="20px"
          pb={3}
          pt={1}
          fontSize="0.95rem"
          lineHeight="1.7"
          color="fg"
          whiteSpace="pre-wrap"
          overflowWrap="anywhere"
        >
          {editLinks(s.descricao ?? "")}
        </Box>
      </PainelRevelado>
    </Box>
  );
}

export function AccordionItem({
  classe,
  items,
  isOpen,
  onToggle,
  isFirst,
  isLast,
  terms,
}: AccordionItemProps) {
  return (
    <Box>
      <Flex
        align="center"
        justify="space-between"
        py={2}
        px={4}
        bg={isOpen ? "bg.app" : "bg.canvas"}
        borderBottom="1px solid var(--chakra-colors-border)"
        cursor="pointer"
        _hover={{ bg: isOpen ? "bg.app" : "bg.subtle" }}
        style={{ transition: "background 0.15s ease" }}
        {...comoBotao(onToggle)}
        aria-expanded={isOpen}
        borderTopRadius={isFirst ? "6px" : undefined}
        borderBottomRadius={isLast && !isOpen ? "6px" : undefined}
      >
        <Text fontSize="1rem" fontWeight="600" color="fg" flex={1} mr={4} lineHeight="1">
          <Highlight text={classe} terms={terms} />
        </Text>
        <Flex align="center" gap={1} flexShrink={0}>
          <Box color="fg.muted" display="flex">
            {isOpen ? <MdExpandLess size={20} /> : <MdExpandMore size={20} />}
          </Box>
        </Flex>
      </Flex>

      <PainelRevelado aberto={isOpen}>
        <Box
          bg="bg.canvas"
          p={4}
          borderBottom="1px solid var(--chakra-colors-border)"
          borderBottomRadius={isLast ? "6px" : undefined}
        >
          {/* O recuo de um nível da árvore das FAQs (20 px): o serviço fica sob a classe, não
              alinhado com ela. Peso 500 nos títulos, entre o 600 da classe e o 400 do texto. */}
          <Stack gap={2} pl="20px">
            {/* A `key` é o par que identifica o serviço, não o `id`: o `id` recomeça em cada
                arquivo de público, e com o "aberto" guardado na linha a mesma `key` numa classe
                de mesmo nome em outra aba passaria o estado de um serviço a outro (§5.4). */}
            {items.map((s) => (
              <ServicoItem key={`${s.titulo}\u0000${s.link}`} s={s} terms={terms} />
            ))}
          </Stack>
        </Box>
      </PainelRevelado>
    </Box>
  );
}
