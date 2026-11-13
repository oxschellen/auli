import { Link } from "@chakra-ui/react";
import { MdOpenInNew } from "react-icons/md";

/**
 * Link para fora: cor `accent`, ícone antes do rótulo, `target="_blank"` com
 * `rel="noopener noreferrer"`. É o "Abrir no portal" de Pareceres e Legislação e o "Abrir na
 * fonte" das cinco listas do Tributum.
 *
 * Existe como componente porque o `rel` é fácil de esquecer numa cópia — sem ele, a página aberta
 * ganha `window.opener` e pode navegar a nossa.
 *
 * O `aria-label` é obrigatório: vários links "Abrir na fonte" na mesma tela são indistinguíveis
 * para quem navega por lista de links, e o rótulo visível sozinho não diz de qual item ele é.
 */
/** Abre `url` numa aba nova sem lhe dar `window.opener` — o equivalente, em código, do `rel` acima.
 *  Para as linhas inteiras que agem como link (resultado de busca das FAQs, item dos Conteúdos). */
export function abrirEmNovaAba(url: string) {
  window.open(url, "_blank", "noopener,noreferrer");
}

export function LinkExterno({
  href,
  rotulo,
  ariaLabel,
}: {
  href: string;
  rotulo: string;
  ariaLabel: string;
}) {
  return (
    <Link
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      color="accent"
      fontSize="0.85rem"
      display="inline-flex"
      alignItems="center"
      gap={1}
      aria-label={ariaLabel}
    >
      <MdOpenInNew size={14} /> {rotulo}
    </Link>
  );
}
