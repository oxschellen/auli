import { useEffectEvent, useLayoutEffect, useRef } from "react";
import type { ReactNode, RefObject } from "react";
import { Box } from "@chakra-ui/react";
import type { BoxProps } from "@chakra-ui/react";
import { Z } from "./layout";

interface SobreposicaoProps extends BoxProps {
  onFechar: () => void;
  /** Quem recebe o foco quando a camada fecha: o botão que abriu. Sem ele, volta a quem tinha o
   *  foco ao abrir — o que falha no Safari, onde clicar num botão não o foca. */
  devolverFocoA?: RefObject<HTMLElement | null>;
  /** O véu sobre o app. */
  fundo?: string;
  children: ReactNode;
}

/** O que o Tab alcança: os focáveis por natureza, menos os tirados da ordem (`tabindex="-1"`,
 *  como as abas não ativas de um tablist com tabindex móvel). */
const FOCAVEIS =
  'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), iframe, [tabindex]';
const focaveis = (el: HTMLElement) =>
  [...el.querySelectorAll<HTMLElement>(FOCAVEIS)].filter((f) => f.tabIndex >= 0);

/** Foca e diz se deu certo: elemento escondido por CSS não recebe foco, e aí a camada não pode
 *  segurar o Tab (é o drawer de seções aberto no celular e a janela alargada até o desktop). */
const irPara = (alvo: HTMLElement | undefined) => {
  alvo?.focus();
  return !!alvo && document.activeElement === alvo;
};

/**
 * Uma camada sobre o app (I6 do docs/ESTUDO-frontend-organizacao.md): o véu que fecha ao clicar, o
 * Esc que fecha, e o painel por cima — o esqueleto dos dois drawers (seções e "Minhas perguntas")
 * e dos dois modais (log da pergunta e leitor de PDF).
 *
 * **Prende o foco** enquanto está aberta: ao abrir, o foco vai para o primeiro focável do painel;
 * o Tab e o Shift+Tab dão a volta nas pontas; o foco que escapa (o Tab que sai do iframe do PDF,
 * por exemplo) volta para dentro. Ao fechar, por qualquer caminho, o foco volta a quem abriu.
 *
 * Feita à mão, e não com o `Drawer`/`Dialog` do Chakra, para não depender da API da versão
 * instalada. Os ouvintes são do DOCUMENTO: é por ele que passa toda tecla e todo foco.
 */
export function Sobreposicao({
  onFechar,
  devolverFocoA,
  fundo = "blackAlpha.500",
  children,
  ...resto
}: SobreposicaoProps) {
  const camada = useRef<HTMLDivElement>(null);
  // O `onFechar` costuma ser recriado a cada render; como evento, ele não reinstala os ouvintes —
  // e reinstalá-los puxaria o foco de volta ao primeiro focável a cada render.
  const fechar = useEffectEvent(() => onFechar());

  // `useLayoutEffect`: a volta do foco acontece no commit, antes de qualquer `requestAnimationFrame`
  // — quem fecha a camada e foca outra coisa no quadro seguinte (o histórico que devolve a pergunta
  // à caixa) é que fica com o foco.
  useLayoutEffect(() => {
    const el = camada.current;
    if (!el) return;
    const anterior = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    // Quem recebe o foco na volta, guardado agora: o botão que abriu fica montado o tempo todo.
    const devolver = devolverFocoA?.current ?? anterior;
    irPara(focaveis(el)[0]);

    const aoTeclar = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        fechar();
        return;
      }
      if (e.key !== "Tab") return;
      const lista = focaveis(el);
      const primeiro = lista[0];
      const ultimo = lista[lista.length - 1];
      const ativo = document.activeElement;
      const alvo = !el.contains(ativo)
        ? primeiro
        : e.shiftKey && ativo === primeiro
          ? ultimo
          : !e.shiftKey && ativo === ultimo
            ? primeiro
            : undefined;
      if (alvo && irPara(alvo)) e.preventDefault();
    };
    const aoFocar = (e: FocusEvent) => {
      if (!el.contains(e.target as Node)) irPara(focaveis(el)[0]);
    };
    document.addEventListener("keydown", aoTeclar);
    document.addEventListener("focusin", aoFocar);
    return () => {
      document.removeEventListener("keydown", aoTeclar);
      document.removeEventListener("focusin", aoFocar);
      devolver?.focus();
    };
  }, [devolverFocoA]);

  return (
    <Box ref={camada} position="fixed" inset={0} zIndex={Z.sobreposicao} {...resto}>
      <Box position="absolute" inset={0} bg={fundo} onClick={onFechar} />
      {children}
    </Box>
  );
}
