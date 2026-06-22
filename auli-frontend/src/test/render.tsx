import type { ReactElement, ReactNode } from "react";
import { render } from "@testing-library/react";
import { Provider } from "../pages/chat/ui/provider";

/**
 * Render a component inside the app's Chakra + color-mode Provider, which the
 * UI components need for tokens and theming to resolve. Use in jsdom-env tests.
 */
export function renderWithProvider(ui: ReactElement) {
  return render(ui, {
    wrapper: ({ children }: { children: ReactNode }) => <Provider>{children}</Provider>,
  });
}

export * from "@testing-library/react";
