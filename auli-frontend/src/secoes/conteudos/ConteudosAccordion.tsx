import { useState, useMemo } from "react";
import { Box, Flex, Text, Badge } from "@chakra-ui/react";
import { comoBotao, PainelRevelado } from "../../ui/Acordeao";
import { Highlight } from "../../lib/highlight";
import { parseQuery } from "../../lib/textSearch";
import { SemResultados } from "../../ui/busca";
import { abrirEmNovaAba } from "../../ui/LinkExterno";
import {
  MdExpandMore,
  MdExpandLess,
  MdOpenInNew,
  MdPictureAsPdf,
  MdDescription,
  MdDownload,
  MdTableChart,
  MdPlayCircle,
  MdMenuBook,
  MdArticle,
  MdInfo,
  MdArchive,
} from "react-icons/md";
import { searchCategories, type ConteudoCategory, type ConteudoItem } from "./conteudos";

function typeIcon(type?: string) {
  const t = (type || "").toLowerCase();
  if (t.includes("pdf")) return <MdPictureAsPdf size={14} />;
  if (t.includes("xls")) return <MdTableChart size={14} />;
  if (t.includes("doc")) return <MdDescription size={14} />;
  if (t === "exe") return <MdDownload size={14} />;
  if (t.includes("zip") || t.includes(".png")) return <MdArchive size={14} />;
  if (t.includes("video")) return <MdPlayCircle size={14} />;
  if (t === "manual") return <MdMenuBook size={14} />;
  if (t === "pub") return <MdArticle size={14} />;
  if (t === "uif") return <MdInfo size={14} />;
  if (t.includes("link") || t.includes("icone") || t === "icone") return <MdOpenInNew size={14} />;
  return null;
}

function ItemRow({ item, terms }: { item: ConteudoItem; terms: string[] }) {
  const icon = typeIcon(item.type);
  const hasUrl = !!item.url;

  return (
    <Box borderBottom="1px solid var(--chakra-colors-border)">
      <Flex
        align="center"
        justify="space-between"
        py={1}
        px={4}
        pl={8}
        cursor={hasUrl ? "pointer" : "default"}
        _hover={{ bg: hasUrl ? "bg.subtle" : undefined }}
        style={{ transition: "background 0.12s ease" }}
        {...comoBotao(() => item.url && abrirEmNovaAba(item.url), hasUrl)}
      >
        <Flex align="center" gap={2} flex={1}>
          {icon && (
            <Box color="fg.muted" flexShrink={0} display="flex">
              {icon}
            </Box>
          )}
          <Text fontSize="1rem" color="fg" lineHeight="1.5">
            {item.title ? (
              <Highlight text={item.title} terms={terms} />
            ) : (
              <em style={{ color: "var(--chakra-colors-fg-muted)" }}>sem título</em>
            )}
          </Text>
        </Flex>
        {hasUrl && (
          <Box color="accent" opacity={0.6} display="flex" flexShrink={0} ml={2}>
            <MdOpenInNew size={14} />
          </Box>
        )}
      </Flex>
    </Box>
  );
}

function CategoryPanel({ category, terms }: { category: ConteudoCategory; terms: string[] }) {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <Box borderBottom="1px solid var(--chakra-colors-border)">
      <Flex
        align="center"
        justify="space-between"
        py={2}
        px={4}
        bg={isOpen ? "bg.app" : "bg.canvas"}
        cursor="pointer"
        _hover={{ bg: isOpen ? "bg.app" : "bg.subtle" }}
        style={{ transition: "background 0.15s ease" }}
        {...comoBotao(() => setIsOpen((o) => !o))}
        aria-expanded={isOpen}
      >
        <Flex align="center" gap={2}>
          <Text fontSize="1rem" fontWeight="600" color="fg" lineHeight="1">
            {category.name}
          </Text>
          <Badge
            fontSize="0.6rem"
            borderRadius="full"
            px="5px"
            py="1px"
            bg="border"
            color="fg"
            lineHeight="1.4"
          >
            {category.items.length}
          </Badge>
        </Flex>
        <Box color="fg.muted" display="flex">
          {isOpen ? <MdExpandLess size={20} /> : <MdExpandMore size={20} />}
        </Box>
      </Flex>

      <PainelRevelado aberto={isOpen}>
        {/* O separador da key é um NUL escapado: é o único caractere que não pode
                aparecer no título nem na URL. Escapado, e não literal — o byte cru fazia
                `grep` e `git diff` tratarem este arquivo como binário. */}
        {category.items.map((item) => (
          <ItemRow key={`${item.title}\u0000${item.url}`} item={item} terms={terms} />
        ))}
      </PainelRevelado>
    </Box>
  );
}

export function ConteudosAccordion({
  categories,
  searchQuery,
}: {
  categories: ConteudoCategory[];
  searchQuery: string;
}) {
  const filtered = useMemo(
    () => searchCategories(categories, searchQuery),
    [categories, searchQuery],
  );

  // Uma vez por query, não por item: `parseQuery` devolve array novo a cada chamada. Só o
  // `item.title` é marcado — `category.name` NÃO entra na busca (conteudos.ts), e marcá-lo
  // afirmaria um casamento que não houve.
  const terms = useMemo(() => parseQuery(searchQuery), [searchQuery]);

  if (searchQuery.trim() && filtered.length === 0) {
    return <SemResultados consulta={searchQuery} />;
  }

  return (
    <Box>
      {filtered.map((cat) => (
        <CategoryPanel key={cat.name} category={cat} terms={terms} />
      ))}
    </Box>
  );
}
