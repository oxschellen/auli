import { useState, useMemo } from 'react'
import type { KeyboardEvent, MouseEvent } from 'react'
import { Box, Flex, Text, chakra } from '@chakra-ui/react'
import { m, AnimatePresence, useReducedMotion } from 'framer-motion'
import { MdExpandMore, MdExpandLess, MdOpenInNew, MdContentCopy } from 'react-icons/md'
import ReactMarkdown from 'react-markdown'
import { searchNodes, getEffectiveUrl, type FaqNode } from './parseFaqs'
import { compactMarkdownComponents } from '../../shared/markdown'
import { utilsCopyTextToClipboard } from '../chat/utils/utils'

interface HighlightProps {
  text: string
  query: string
}

interface TreeNodeProps {
  node: FaqNode
  ancestors: FaqNode[]
  depth: number
  perguntaMap: Map<string, string>
  pageTypeMap: Map<string, string>
}

interface SearchResultProps {
  node: FaqNode
  ancestors: FaqNode[]
  query: string
}

interface FaqsAccordionProps {
  nodes: FaqNode[]
  searchQuery: string
  answerMap: Map<string, string>
  pageTypeMap: Map<string, string>
}

function Highlight({ text, query }: HighlightProps) {
  if (!query) return text
  const parts = text.split(new RegExp(`(${query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi'))
  let offset = 0
  return parts.map((part) => {
    const key = `${offset}-${part}`
    offset += part.length
    return part.toLowerCase() === query.toLowerCase()
      ? <mark key={key}>{part}</mark>
      : part
  })
}

const MENU_TYPES = new Set(['Menu', 'Geral'])

function TreeNode({ node, ancestors, depth, perguntaMap, pageTypeMap }: TreeNodeProps) {
  const reduceMotion = useReducedMotion()
  const [isOpen, setIsOpen] = useState(false)
  const hasChildren = node.children.length > 0
  const effectiveUrl = getEffectiveUrl(node, ancestors)
  const answer = perguntaMap.get(node.text.trim().toLowerCase())
  const isFaqLeaf = !hasChildren && !!answer
  const nodePageType = node.url ? pageTypeMap.get(node.url.replace(/\/$/, '')) : undefined
  const isMenuOrGeral = hasChildren && !!node.url && !!nodePageType && MENU_TYPES.has(nodePageType)

  const indentPx = 16 + depth * 20

  function toggleOrOpenNode() {
    if (hasChildren || isFaqLeaf) {
      setIsOpen(o => !o)
    } else if (effectiveUrl) {
      window.open(effectiveUrl, '_blank', 'noopener,noreferrer')
    }
  }

  function handleLinkClick(e: MouseEvent<HTMLButtonElement>) {
    e.stopPropagation()
    if (effectiveUrl) window.open(effectiveUrl, '_blank', 'noopener,noreferrer')
  }

  const nextAncestors = useMemo(() => [...ancestors, node], [ancestors, node])

  const expandable = hasChildren || isFaqLeaf
  const interactive = expandable || !!effectiveUrl

  function handleKeyDown(e: KeyboardEvent<HTMLDivElement>) {
    if (interactive && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault()
      toggleOrOpenNode()
    }
  }

  return (
    <Box>
      <Flex
        align="center"
        justify="space-between"
        py={2}
        pr={4}
        bg={isOpen ? 'bg.app' : 'bg.canvas'}
        borderBottom="1px solid var(--chakra-colors-border)"
        cursor={interactive ? 'pointer' : 'default'}
        _hover={{ bg: isOpen ? 'bg.app' : 'bg.subtle' }}
        style={{ transition: 'background 0.15s ease', paddingLeft: `${indentPx}px` }}
        onClick={toggleOrOpenNode}
        onKeyDown={handleKeyDown}
        role="button"
        tabIndex={interactive ? 0 : undefined}
        aria-expanded={expandable ? isOpen : undefined}
      >
        <Flex align="center" gap={2} flex={1} mr={2}>
          <Text
            fontSize="1rem"
            fontWeight={hasChildren ? '600' : '400'}
            color="fg"
            lineHeight="1"
          >
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
              onClick={e => { e.stopPropagation(); utilsCopyTextToClipboard(`${node.text}\n\n${answer}`, 'A resposta foi copiada para a área de transferência') }}
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

      <AnimatePresence initial={false}>
        {isOpen && (
          <m.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={reduceMotion ? { duration: 0 } : { duration: 0.25, ease: 'easeInOut' }}
            style={{ overflow: 'hidden' }}
          >
            {hasChildren && node.children.map(child => (
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
                className="faq-answer"
              >
                <ReactMarkdown components={compactMarkdownComponents}>
                  {answer ?? ''}
                </ReactMarkdown>
              </Box>
            )}
          </m.div>
        )}
      </AnimatePresence>
    </Box>
  )
}

function SearchResult({ node, ancestors, query }: SearchResultProps) {
  const effectiveUrl = getEffectiveUrl(node, ancestors)
  const breadcrumb = ancestors.flatMap(a => (a.text ? [a.text] : [])).join(' › ')

  function openNodeUrl() {
    if (effectiveUrl) {
      window.open(effectiveUrl, '_blank', 'noopener,noreferrer')
    }
  }

  return (
    <Box
      py={3}
      px={4}
      borderBottom="1px solid var(--chakra-colors-border)"
      cursor={effectiveUrl ? 'pointer' : 'default'}
      _hover={{ bg: effectiveUrl ? 'bg.subtle' : undefined }}
      style={{ transition: 'background 0.15s ease' }}
      onClick={openNodeUrl}
      onKeyDown={(e) => {
        if (effectiveUrl && (e.key === 'Enter' || e.key === ' ')) {
          e.preventDefault()
          openNodeUrl()
        }
      }}
      role={effectiveUrl ? 'button' : undefined}
      tabIndex={effectiveUrl ? 0 : undefined}
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
          <Highlight text={node.text} query={query} />
        </Text>
      </Flex>
    </Box>
  )
}

export function FaqsAccordion({ nodes, searchQuery, answerMap, pageTypeMap }: FaqsAccordionProps) {
  const results = useMemo(() => {
    if (!searchQuery.trim()) return []
    return searchNodes(nodes, searchQuery.trim())
  }, [nodes, searchQuery])

  if (searchQuery.trim()) {
    if (results.length === 0) {
      return (
        <Box py={10} textAlign="center">
          <Text color="fg.muted" fontSize="0.95rem">
            Nenhum resultado encontrado para &ldquo;{searchQuery}&rdquo;
          </Text>
        </Box>
      )
    }
    return (
      <Box>
        <Box px={4} py={2} borderBottom="1px solid var(--chakra-colors-border)" bg="bg.subtle">
          <Text fontSize="0.8rem" color="fg.muted" fontWeight="500">
            {results.length} resultado{results.length !== 1 ? 's' : ''} encontrado{results.length !== 1 ? 's' : ''}
          </Text>
        </Box>
        {results.map(({ node, ancestors }) => (
          <SearchResult
            key={`${ancestors.map(a => a.id).join('/')}/${node.id}`}
            node={node}
            ancestors={ancestors}
            query={searchQuery.trim()}
          />
        ))}
      </Box>
    )
  }

  return (
    <Box>
      {nodes.map(node => (
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
  )
}
