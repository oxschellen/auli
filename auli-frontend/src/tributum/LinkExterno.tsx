import { Link } from "@chakra-ui/react";
import { MdOpenInNew } from "react-icons/md";

/**
 * Link para fora, no mesmo idioma visual do "Abrir no portal" da aba Legislação: cor `accent`,
 * ícone antes do rótulo, `target="_blank"` com `rel="noopener noreferrer"`.
 *
 * Existe como componente porque as quatro listas do Tributum o repetem e porque o `rel` é fácil de
 * esquecer numa cópia — sem ele, a página aberta ganha `window.opener` e pode navegar a nossa.
 *
 * O `aria-label` é obrigatório: quatro links "Abrir na fonte" na mesma tela são indistinguíveis
 * para quem navega por lista de links, e o rótulo visível sozinho não diz de qual item ele é.
 */
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
