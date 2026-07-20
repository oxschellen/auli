import { Box, Flex, Link, Stack, Text } from '@chakra-ui/react'
import { AnimatePresence, m, useReducedMotion } from 'framer-motion'
import { MdExpandLess, MdExpandMore } from 'react-icons/md'
import type { Servico } from './utils'
import { Highlight } from '../../shared/highlight'

interface AccordionItemProps {
  classe: string
  items: Servico[]
  isOpen: boolean
  onToggle: () => void
  isFirst: boolean
  isLast: boolean
  /** Termos já normalizados (`parseQuery`) — os MESMOS que filtraram, para marcar o que casou. */
  terms: string[]
}

export function AccordionItem({ classe, items, isOpen, onToggle, isFirst, isLast, terms }: AccordionItemProps) {
  const reduceMotion = useReducedMotion()
  return (
    <Box>
      <Flex
        align="center"
        justify="space-between"
        py={2}
        px={4}
        bg={isOpen ? 'bg.app' : 'bg.canvas'}
        borderBottom="1px solid var(--chakra-colors-border)"
        cursor="pointer"
        _hover={{ bg: isOpen ? 'bg.app' : 'bg.subtle' }}
        style={{ transition: 'background 0.15s ease' }}
        onClick={onToggle}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault()
            onToggle()
          }
        }}
        tabIndex={0}
        role="button"
        aria-expanded={isOpen}
        borderTopRadius={isFirst ? '6px' : undefined}
        borderBottomRadius={isLast && !isOpen ? '6px' : undefined}
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

      <AnimatePresence initial={false}>
        {isOpen && (
          <m.div
            key="content"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={reduceMotion ? { duration: 0 } : { duration: 0.25, ease: 'easeInOut' }}
            style={{ overflow: 'hidden' }}
          >
            <Box
              bg="bg.canvas"
              p={4}
              borderBottom="1px solid var(--chakra-colors-border)"
              borderBottomRadius={isLast ? '6px' : undefined}
            >
              <Stack gap={2}>
                {items.map((s) => (
                  <Link
                    key={s.id}
                    href={s.link}
                    target="_blank"
                    rel="noopener noreferrer"
                    color="accent"
                    fontSize="1rem"
                    lineHeight="1.75"
                    display="block"
                  >
                    <Highlight text={s.titulo} terms={terms} />
                  </Link>
                ))}
              </Stack>
            </Box>
          </m.div>
        )}
      </AnimatePresence>
    </Box>
  )
}