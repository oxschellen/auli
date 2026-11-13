import type { KeyboardEvent, ReactNode } from "react";
import { AnimatePresence, m, useReducedMotion } from "framer-motion";

/**
 * As duas peças dos acordeões (I4 do docs/ESTUDO-frontend-organizacao.md): o painel que se revela e
 * as props de um elemento que não é `<button>` mas age como um.
 */

/**
 * O painel de um acordeão: revela os filhos animando a altura de 0 a `auto`, uma vez, e sem
 * movimento com `prefers-reduced-motion`.
 *
 * `contain: layout paint` prende o custo da animação na fronteira do painel: animar `height` até
 * `auto` refaz o layout a cada quadro, e sem a contenção esse recálculo subiria a árvore (o painel
 * de Serviços passa de cem linhas). A animação de altura FICA, e é deliberado: revelação intrínseca
 * é o caso que a regra `no-layout-property-animation` descreve como "a opção menos distorcida" e
 * manda conter em vez de reescrever (ver `doctor.config.json`). `scale` distorceria o texto, e
 * `grid-template-rows` só enganaria o detector, que não olha essa chave. Este é o ÚNICO lugar do
 * app com essa animação.
 */
export function PainelRevelado({ aberto, children }: { aberto: boolean; children: ReactNode }) {
  const semMovimento = useReducedMotion();
  return (
    <AnimatePresence initial={false}>
      {aberto && (
        <m.div
          key="painel"
          initial={{ height: 0, opacity: 0 }}
          animate={{ height: "auto", opacity: 1 }}
          exit={{ height: 0, opacity: 0 }}
          transition={semMovimento ? { duration: 0 } : { duration: 0.22, ease: "easeInOut" }}
          style={{ overflow: "hidden", contain: "layout paint" }}
        >
          {children}
        </m.div>
      )}
    </AnimatePresence>
  );
}

/**
 * Props de um elemento que não é `<button>` mas age como um: papel de botão, foco pelo Tab, clique,
 * Enter e Espaço. Com `ativo` falso, nada disso — o elemento é só texto.
 */
export function comoBotao(acao: () => void, ativo = true) {
  if (!ativo) return {};
  return {
    role: "button" as const,
    tabIndex: 0,
    onClick: acao,
    onKeyDown: (e: KeyboardEvent) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        acao();
      }
    },
  };
}
