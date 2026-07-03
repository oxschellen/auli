import { useState, useMemo, useRef, useDeferredValue } from 'react'
import { Box, Flex } from '@chakra-ui/react'
import useSWR from 'swr'
import { FaqsAccordion } from './FaqsAccordion'
import { buildNodesFromJson, buildAnswerMap, buildPageTypeMap, type RawFaqNode } from './parseFaqs'
import { jsonFetcher, SWR_OPTS, entityPath } from '../../shared/fetchers'
import { SearchInput } from '../../shared/SearchInput'
import { AsyncContent } from '../../shared/AsyncContent'
import { useSelectedEntity } from '../../shared/EntityContext'
import { hasCollection } from '../../shared/entities'
import { CollectionEmpty } from '../../shared/CollectionEmpty'

export function FaqsList() {
  const entity = useSelectedEntity()
  const available = hasCollection(entity, 'faqs')
  const { data, error, isLoading } = useSWR(
    available ? entityPath(entity.id, 'faqs-tree.json') : null,
    jsonFetcher<RawFaqNode>,
    SWR_OPTS,
  )
  const nodes = useMemo(() => (data ? buildNodesFromJson(data) : []), [data])
  const answerMap = useMemo(() => (data ? buildAnswerMap(data) : new Map<string, string>()), [data])
  const pageTypeMap = useMemo(() => (data ? buildPageTypeMap(data) : new Map<string, string>()), [data])
  const [searchQuery, setSearchQuery] = useState('')
  // Keep the input instant, but let React run the full-tree search (over the
  // large FAQ dataset) against a deferred value so typing doesn't jank.
  const deferredQuery = useDeferredValue(searchQuery)
  const inputRef = useRef<HTMLInputElement>(null)

  function clearSearch() {
    setSearchQuery('')
    inputRef.current?.focus()
  }

  if (!available) return <CollectionEmpty entity={entity} label="FAQs" />

  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.app">
      {/* Sticky search bar */}
      <Box
        position="sticky"
        top={0}
        zIndex={10}
        bg="bg.app"
        px={4}
        pt={3}
        pb={2}
        borderBottom="1px solid var(--chakra-colors-border)"
      >
        <SearchInput
          ref={inputRef}
          value={searchQuery}
          onChange={e => setSearchQuery(e.target.value)}
          onClear={clearSearch}
          placeholder="Pesquisar FAQs..."
        />
      </Box>

      {/* Scrollable content */}
      <Box w="100%" px={4} pt={3} pb={6}>
        <AsyncContent
          loading={isLoading}
          error={error}
          loadingText="Aguarde enquanto as FAQs são carregadas…"
          errorDescription="Aconteceu um erro ao carregar as FAQs."
        >
          <FaqsAccordion nodes={nodes} searchQuery={deferredQuery} answerMap={answerMap} pageTypeMap={pageTypeMap} />
        </AsyncContent>
      </Box>
    </Flex>
  )
}
