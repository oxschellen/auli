import type { ComponentProps, RefObject } from "react";
import { Box } from "@chakra-ui/react";
import { Historico } from "./Historico";
import { HEADER_HEIGHT, HISTORICO_WIDTH, Z } from "../../ui/layout";
import { Sobreposicao } from "../../ui/Sobreposicao";

/**
 * "Minhas perguntas" nas duas formas (I8 do docs/ESTUDO-frontend-organizacao.md): a coluna à
 * direita do chat a partir de `xl`, cuja escolha fica salva, e o drawer abaixo dele, que abre e
 * fecha sem deixar rastro. As duas mostram o mesmo `Historico`.
 */
export function PainelHistorico({
  coluna,
  drawer,
  onFecharColuna,
  onFecharDrawer,
  devolverFocoA,
  ...historico
}: Omit<ComponentProps<typeof Historico>, "onFechar"> & {
  /** A coluna está aberta (só aparece a partir de `xl`). */
  coluna: boolean;
  /** O drawer está aberto (só aparece abaixo de `xl`). */
  drawer: boolean;
  onFecharColuna: () => void;
  onFecharDrawer: () => void;
  /** O botão que abriu o drawer, que recebe o foco quando o Esc o fecha. */
  devolverFocoA: RefObject<HTMLElement | null>;
}) {
  return (
    <>
      {/* Coluna (>= xl): DENTRO do fluxo — o chat encolhe em vez de ser coberto, e a barra de
          rolagem da área de conteúdo segue à vista, à direita dele. `sticky` porque o Chat mora
          dentro do único container de scroll (ver `Home`): sem isso o painel subiria junto com as
          mensagens. A altura é a da área que rola, que nesta largura é a tela menos o cabeçalho.
          O zIndex fica ACIMA da barra de composição (ver `Z`). */}
      {coluna && (
        <Box
          as="aside"
          aria-label="Minhas perguntas"
          hideBelow="xl"
          position="sticky"
          top={0}
          alignSelf="flex-start"
          flexShrink={0}
          w={HISTORICO_WIDTH}
          h={`calc(100dvh - ${HEADER_HEIGHT})`}
          overflowY="auto"
          bg="bg.app"
          borderLeft="1px solid var(--chakra-colors-border)"
          zIndex={Z.painelHistorico}
        >
          <Historico {...historico} onFechar={onFecharColuna} />
        </Box>
      )}

      {/* Drawer (< xl): o véu + o MESMO painel, vindo da direita. Esc fecha e devolve o foco ao
          botão que o abriu, como no drawer de seções do `Home`. */}
      {drawer && (
        <Sobreposicao hideFrom="xl" onFechar={onFecharDrawer} devolverFocoA={devolverFocoA}>
          <Box
            as="aside"
            aria-label="Minhas perguntas"
            position="absolute"
            top={0}
            bottom={0}
            right={0}
            w="min(320px, 88vw)"
            overflowY="auto"
            bg="bg.app"
            borderLeft="1px solid var(--chakra-colors-border)"
          >
            <Historico {...historico} onFechar={onFecharDrawer} />
          </Box>
        </Sobreposicao>
      )}
    </>
  );
}
