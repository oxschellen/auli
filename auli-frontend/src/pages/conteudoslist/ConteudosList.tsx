import { useState, useRef, useDeferredValue } from 'react'
import { Box, Flex } from '@chakra-ui/react'
import useSWR from 'swr'
import { ConteudosAccordion } from './ConteudosAccordion'
import { jsonFetcher, SWR_OPTS, entityPath } from '../../shared/fetchers'
import { SearchInput } from '../../shared/SearchInput'
import { AsyncContent } from '../../shared/AsyncContent'
import { useSelectedEntity } from '../../shared/EntityContext'
import { hasCollection } from '../../shared/entities'
import { CollectionEmpty } from '../../shared/CollectionEmpty'
import type { ConteudoTree } from './parseConteudos'

export function ConteudosList() {
  const entity = useSelectedEntity()
  const available = hasCollection(entity, 'conteudos')
  const { data, error, isLoading } = useSWR(
    available ? entityPath(entity.id, 'conteudo_site_tree.json') : null,
    jsonFetcher<ConteudoTree>,
    SWR_OPTS,
  )
  const categories = data?.categories || []
  const [searchQuery, setSearchQuery] = useState('')
  // Input stays instant; filtering runs against the deferred value.
  const deferredQuery = useDeferredValue(searchQuery)
  const inputRef = useRef<HTMLInputElement>(null)

  function clearSearch() {
    setSearchQuery('')
    inputRef.current?.focus()
  }

  if (!available) return <CollectionEmpty entity={entity} label="Conteúdos" />

  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.app">
      <Box w="100%" px={4} pt={3} pb={6}>
        <Box mb={3}>
          <SearchInput
            ref={inputRef}
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            onClear={clearSearch}
            placeholder="Pesquisar conteúdos..."
          />
        </Box>

        <AsyncContent
          loading={isLoading}
          error={error}
          loadingText="Aguarde enquanto os conteúdos são carregados…"
          errorDescription="Aconteceu um erro ao carregar os conteúdos."
        >
          <ConteudosAccordion categories={categories} searchQuery={deferredQuery} />
        </AsyncContent>
      </Box>
    </Flex>
  )
}
