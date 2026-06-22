'use client'

import { ChakraProvider } from '@chakra-ui/react'
import { system } from '../../../theme/system'
import { ColorModeProvider } from './color-mode'

export function Provider(props) {
  return (
    <ChakraProvider value={system}>
      <ColorModeProvider {...props} />
    </ChakraProvider>
  )
}
