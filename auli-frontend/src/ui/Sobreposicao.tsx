import { useEffect } from "react";
import type { ReactNode, RefObject } from "react";
import { Box } from "@chakra-ui/react";
import type { BoxProps } from "@chakra-ui/react";
import { Z } from "./layout";

interface SobreposicaoProps extends BoxProps {
  onFechar: () => void;
  /** Quem recebe o foco quando o Esc fecha: o botão que abriu. */
  devolverFocoA?: RefObject<HTMLElement | null>;
  /** O véu sobre o app. */
  fundo?: string;
  children: ReactNode;
}

/**
 * Uma camada sobre o app (I6 do docs/ESTUDO-frontend-organizacao.md): o véu que fecha ao clicar, o
 * Esc que fecha, e o painel por cima — o esqueleto dos dois drawers (seções e "Minhas perguntas")
 * e dos dois modais (log da pergunta e leitor de PDF).
 *
 * Feita à mão, e não com o `Drawer`/`Dialog` do Chakra, para não depender da API da versão
 * instalada. O ouvinte do Esc é do DOCUMENTO, não do painel: ao abrir um drawer, o foco segue no
 * botão que o abriu, que fica FORA dele, e um `onKeyDown` no painel não veria a tecla.
 *
 * Ainda não prende o foco dentro do painel. Com um componente só, isso se resolve uma vez: uma
 * armadilha de foco aqui dentro, ou o miolo trocado pelo `Dialog` do Chakra.
 */
export function Sobreposicao({
  onFechar,
  devolverFocoA,
  fundo = "blackAlpha.500",
  children,
  ...resto
}: SobreposicaoProps) {
  useEffect(() => {
    const aoTeclar = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      onFechar();
      devolverFocoA?.current?.focus();
    };
    document.addEventListener("keydown", aoTeclar);
    return () => document.removeEventListener("keydown", aoTeclar);
  }, [onFechar, devolverFocoA]);

  return (
    <Box position="fixed" inset={0} zIndex={Z.sobreposicao} {...resto}>
      <Box position="absolute" inset={0} bg={fundo} onClick={onFechar} />
      {children}
    </Box>
  );
}
