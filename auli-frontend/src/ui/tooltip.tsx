import { Tooltip as ChakraTooltip, Portal } from "@chakra-ui/react";
import type { ReactNode, Ref, RefObject } from "react";

export interface TooltipProps extends ChakraTooltip.RootProps {
  /** React 19: `ref` é prop comum, e vai para o conteúdo da dica. */
  ref?: Ref<HTMLDivElement>;
  showArrow?: boolean;
  portalled?: boolean;
  portalRef?: RefObject<HTMLElement | null>;
  content: ReactNode;
  contentProps?: ChakraTooltip.ContentProps;
  disabled?: boolean;
}

export function Tooltip(props: TooltipProps) {
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
  } = props;

  if (disabled) return children;

  return (
    <ChakraTooltip.Root {...rest}>
      <ChakraTooltip.Trigger asChild>{children}</ChakraTooltip.Trigger>
      <Portal disabled={!portalled} container={portalRef}>
        <ChakraTooltip.Positioner>
          <ChakraTooltip.Content ref={ref} {...contentProps}>
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
  );
}
