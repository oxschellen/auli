import { useState, useMemo } from "react";
import type { MouseEvent } from "react";
import { Box, Flex, Text, chakra } from "@chakra-ui/react";
import { comoBotao, PainelRevelado } from "../../ui/Acordeao";
import { MdExpandMore, MdExpandLess, MdOpenInNew, MdContentCopy } from "react-icons/md";
import ReactMarkdown from "react-markdown";
import { getEffectiveUrl, type FaqNode, type FaqSearchHit } from "./faqs";
import { compactMarkdownComponents, markdownPlugins } from "../../lib/markdown";
import { Highlight } from "../../lib/highlight";
import { parseQuery } from "../../lib/textSearch";
import { SemResultados } from "../../ui/busca";
import { abrirEmNovaAba } from "../../ui/LinkExterno";
import { utilsCopyTextToClipboard } from "../../ui/copiar";

interface TreeNodeProps {
  node: FaqNode;
  ancestors: FaqNode[];
  depth: number;
  perguntaMap: Map<string, string>;
  pageTypeMap: Map<string, string>;
}

interface SearchResultProps {
  node: FaqNode;
  ancestors: FaqNode[];
  /** Termos já normalizados (`parseQuery`) — os MESMOS que filtraram, para marcar o que casou. */
  terms: string[];
}

interface FaqsAccordionProps {
  nodes: FaqNode[];
  searchQuery: string;
  /** Acertos da busca, calculados pelo `FaqsList` — que também os conta na barra fixa. */
  results: FaqSearchHit[];
  answerMap: Map<string, string>;
  pageTypeMap: Map<string, string>;
}

const MENU_TYPES = new Set(["Menu", "Geral"]);

function TreeNode({ node, ancestors, depth, perguntaMap, pageTypeMap }: TreeNodeProps) {
  const [isOpen, setIsOpen] = useState(false);
  const hasChildren = node.children.length > 0;
  const effectiveUrl = getEffectiveUrl(node, ancestors);
  const answer = perguntaMap.get(node.text.trim().toLowerCase());
  const isFaqLeaf = !hasChildren && !!answer;
  const nodePageType = node.url ? pageTypeMap.get(node.url.replace(/\/$/, "")) : undefined;
  const isMenuOrGeral = hasChildren && !!node.url && !!nodePageType && MENU_TYPES.has(nodePageType);

  const indentPx = 16 + depth * 20;

  function toggleOrOpenNode() {
    if (hasChildren || isFaqLeaf) {
      setIsOpen((o) => !o);
    } else if (effectiveUrl) {
      abrirEmNovaAba(effectiveUrl);
    }
  }

  function handleLinkClick(e: MouseEvent<HTMLButtonElement>) {
    e.stopPropagation();
    if (effectiveUrl) abrirEmNovaAba(effectiveUrl);
  }

  const nextAncestors = useMemo(() => [...ancestors, node], [ancestors, node]);

  const expandable = hasChildren || isFaqLeaf;
  const interactive = expandable || !!effectiveUrl;

  return (
    <Box>
      <Flex
        align="center"
        justify="space-between"
        py={2}
        pr={4}
        bg={isOpen ? "bg.app" : "bg.canvas"}
        borderBottom="1px solid var(--chakra-colors-border)"
        cursor={interactive ? "pointer" : "default"}
        _hover={{ bg: isOpen ? "bg.app" : "bg.subtle" }}
        style={{ transition: "background 0.15s ease", paddingLeft: `${indentPx}px` }}
        {...comoBotao(toggleOrOpenNode, interactive)}
        aria-expanded={expandable ? isOpen : undefined}
      >
        <Flex align="center" gap={2} flex={1} mr={2}>
          <Text fontSize="1rem" fontWeight={hasChildren ? "600" : "400"} color="fg" lineHeight="1">
            {node.text}
          </Text>
        </Flex>
        <Flex align="center" gap={1} flexShrink={0}>
          {isFaqLeaf && (
            <chakra.button
              type="button"
              color="fg.muted"
              opacity={0.7}
              display="flex"
              cursor="pointer"
              onClick={(e) => {
                e.stopPropagation();
                utilsCopyTextToClipboard(
                  `${node.text}\n\n${answer}`,
                  "A resposta foi copiada para a área de transferência",
                );
              }}
              aria-label="Copiar resposta"
              title="Copiar resposta"
            >
              <MdContentCopy size={14} />
            </chakra.button>
          )}
          {(isFaqLeaf || isMenuOrGeral) && effectiveUrl && (
            <chakra.button
              type="button"
              color="accent"
              opacity={0.7}
              display="flex"
              cursor="pointer"
              onClick={handleLinkClick}
              aria-label="Abrir no portal"
              title="Abrir no portal"
            >
              <MdOpenInNew size={14} />
            </chakra.button>
          )}
          {!isFaqLeaf && !hasChildren && effectiveUrl && (
            <Box color="accent" opacity={0.6} display="flex">
              <MdOpenInNew size={14} />
            </Box>
          )}
          {(hasChildren || isFaqLeaf) && (
            <Box color="fg.muted" display="flex">
              {isOpen ? <MdExpandLess size={20} /> : <MdExpandMore size={20} />}
            </Box>
          )}
        </Flex>
      </Flex>

      <PainelRevelado aberto={isOpen}>
        {hasChildren &&
          node.children.map((child) => (
            <TreeNode
              key={child.id}
              node={child}
              ancestors={nextAncestors}
              depth={depth + 1}
              perguntaMap={perguntaMap}
              pageTypeMap={pageTypeMap}
            />
          ))}
        {isFaqLeaf && (
          <Box
            style={{ paddingLeft: `${indentPx}px` }}
            pr={4}
            py={3}
            bg="bg.app"
            borderBottom="1px solid var(--chakra-colors-border)"
            fontSize="0.95rem"
            color="fg"
            lineHeight="1.7"
          >
            <ReactMarkdown remarkPlugins={markdownPlugins} components={compactMarkdownComponents}>
              {answer ?? ""}
            </ReactMarkdown>
          </Box>
        )}
      </PainelRevelado>
    </Box>
  );
}

function SearchResult({ node, ancestors, terms }: SearchResultProps) {
  const effectiveUrl = getEffectiveUrl(node, ancestors);
  const breadcrumb = ancestors.flatMap((a) => (a.text ? [a.text] : [])).join(" › ");

  function openNodeUrl() {
    if (effectiveUrl) {
      abrirEmNovaAba(effectiveUrl);
    }
  }

  return (
    <Box
      py={3}
      px={4}
      borderBottom="1px solid var(--chakra-colors-border)"
      cursor={effectiveUrl ? "pointer" : "default"}
      _hover={{ bg: effectiveUrl ? "bg.subtle" : undefined }}
      style={{ transition: "background 0.15s ease" }}
      {...comoBotao(openNodeUrl, !!effectiveUrl)}
    >
      {breadcrumb && (
        <Text fontSize="0.75rem" color="fg.muted" mb={1} lineHeight="1.4">
          {breadcrumb}
        </Text>
      )}
      <Flex align="center" gap={2}>
        {effectiveUrl && (
          <Box color="accent" opacity={0.6} display="flex" flexShrink={0}>
            <MdOpenInNew size={15} />
          </Box>
        )}
        <Text fontSize="1rem" fontWeight="500" color="fg" flex={1} lineHeight="1.5">
          <Highlight text={node.text} terms={terms} />
        </Text>
      </Flex>
    </Box>
  );
}

export function FaqsAccordion({
  nodes,
  searchQuery,
  results,
  answerMap,
  pageTypeMap,
}: FaqsAccordionProps) {
  // Uma vez por query, não por resultado: `parseQuery` devolve array novo a cada chamada.
  const terms = useMemo(() => parseQuery(searchQuery), [searchQuery]);

  if (searchQuery.trim()) {
    if (results.length === 0) {
      return <SemResultados consulta={searchQuery} />;
    }
    return (
      <Box>
        {/* O "N resultados" ficava aqui; subiu para a barra fixa do `FaqsList`, ao lado do total
            da coleção — os dois números pertencem à mesma linha. */}
        {results.map(({ node, ancestors }) => (
          <SearchResult
            key={`${ancestors.map((a) => a.id).join("/")}/${node.id}`}
            node={node}
            ancestors={ancestors}
            terms={terms}
          />
        ))}
      </Box>
    );
  }

  return (
    <Box>
      {nodes.map((node) => (
        <TreeNode
          key={node.id}
          node={node}
          ancestors={[]}
          depth={0}
          perguntaMap={answerMap}
          pageTypeMap={pageTypeMap}
        />
      ))}
    </Box>
  );
}
