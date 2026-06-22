'use client'

import { ThemeProvider, useTheme } from 'next-themes'
import { Box } from '@chakra-ui/react'
import { MdDarkMode, MdLightMode } from 'react-icons/md'

export function ColorModeProvider(props) {
  return (
    <ThemeProvider attribute='class' defaultTheme='system' enableSystem disableTransitionOnChange {...props} />
  )
}

// Used only by ColorModeButton below; not exported.
function useColorMode() {
  const { resolvedTheme, setTheme } = useTheme()
  const toggleColorMode = () => setTheme(resolvedTheme === 'dark' ? 'light' : 'dark')
  return { colorMode: resolvedTheme, setColorMode: setTheme, toggleColorMode }
}

export function ColorModeButton(props) {
  const { colorMode, toggleColorMode } = useColorMode()
  // CSR-only app, so no SSR hydration to guard against. `colorMode` is
  // undefined until next-themes resolves on the client; default the icon to
  // the "switch to dark" affordance until then.
  return (
    <Box
      as='button'
      type='button'
      aria-label={colorMode === 'dark' ? 'Ativar modo claro' : 'Ativar modo escuro'}
      onClick={toggleColorMode}
      display='flex'
      alignItems='center'
      justifyContent='center'
      boxSize='36px'
      borderRadius='full'
      color='fg.inverted'
      cursor='pointer'
      transition='background 0.15s ease'
      _hover={{ bg: 'border.inverted' }}
      {...props}
    >
      {colorMode === 'dark' ? <MdLightMode size={18} /> : <MdDarkMode size={18} />}
    </Box>
  )
}
