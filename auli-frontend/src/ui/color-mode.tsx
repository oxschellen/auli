import { ThemeProvider, useTheme, type ThemeProviderProps } from "next-themes";
import { chakra, type HTMLChakraProps } from "@chakra-ui/react";
import { MdDarkMode, MdLightMode } from "react-icons/md";

export type ColorModeProviderProps = ThemeProviderProps;

export function ColorModeProvider(props: ColorModeProviderProps) {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      disableTransitionOnChange
      {...props}
    />
  );
}

// Used only by ColorModeButton below; not exported.
function useColorMode() {
  const { resolvedTheme, setTheme } = useTheme();
  const toggleColorMode = () => setTheme(resolvedTheme === "dark" ? "light" : "dark");
  return { colorMode: resolvedTheme, setColorMode: setTheme, toggleColorMode };
}

export function ColorModeButton(props: HTMLChakraProps<"button">) {
  const { colorMode, toggleColorMode } = useColorMode();
  // CSR-only app, so no SSR hydration to guard against. `colorMode` is
  // undefined until next-themes resolves on the client; default the icon to
  // the "switch to dark" affordance until then.
  return (
    <chakra.button
      type="button"
      aria-label={colorMode === "dark" ? "Ativar modo claro" : "Ativar modo escuro"}
      onClick={toggleColorMode}
      display="flex"
      alignItems="center"
      justifyContent="center"
      boxSize="36px"
      borderRadius="full"
      color="fg.inverted"
      cursor="pointer"
      transition="background 0.15s ease"
      _hover={{ bg: "border.inverted" }}
      {...props}
    >
      {colorMode === "dark" ? <MdLightMode size={18} /> : <MdDarkMode size={18} />}
    </chakra.button>
  );
}
