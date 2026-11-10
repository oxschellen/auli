import { Tooltip as ChakraTooltip, Portal } from '@chakra-ui/react'

export function Tooltip(props) {
  const {
    ref,
    showArrow,
    children,
    disabled,
    portalled = true,
    content,
    contentProps,
    portalRef,
    ...rest
  } = props

  if (disabled) return children

  return (
    <ChakraTooltip.Root {...rest}>
      <ChakraTooltip.Trigger asChild>{children}</ChakraTooltip.Trigger>
      <Portal disabled={!portalled} container={portalRef}>
        <ChakraTooltip.Positioner>
          {/* O padding da receita do Chakra vive em `@layer recipes` e perde para o `* { padding: 0 }`
              SEM camada do index.css — a dica saía colada no texto. Repetido como style prop, que
              o Chakra emite sem camada e ganha por especificidade. */}
          <ChakraTooltip.Content ref={ref} px="2.5" py="1" {...contentProps}>
            {showArrow && (
              <ChakraTooltip.Arrow>
                <ChakraTooltip.ArrowTip />
              </ChakraTooltip.Arrow>
            )}
            {content}
          </ChakraTooltip.Content>
        </ChakraTooltip.Positioner>
      </Portal>
    </ChakraTooltip.Root>
  )
}
