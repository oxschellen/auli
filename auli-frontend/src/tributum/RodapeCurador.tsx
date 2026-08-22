import { Box, Link, Text } from "@chakra-ui/react";
import type { Curador } from "./types";

/** `mailto:` para e-mail, link para URL, texto puro para o resto (um @perfil, um ramal). */
function comoContato(contato: string) {
  const c = contato.trim();
  if (/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(c)) return `mailto:${c}`;
  if (/^https?:\/\//i.test(c)) return c;
  return null;
}

/**
 * O rodapé que o critério editorial promete: *"Os Curadores e seus contatos estão listados no
 * rodapé de cada estante."*
 *
 * **Não renderiza nada quando a estante não declara curador.** Isso é deliberado e não é
 * degradação silenciosa: um nome de curador é uma pessoa assumindo escolha e responsabilidade
 * públicas, e não se inventa nem se herda de outra estante. Enquanto o catálogo não nomear alguém,
 * a promessa do texto fica sem cumprir — e a ausência do rodapé é o sinal disso para quem mantém.
 */
export function RodapeCurador({ curador }: { curador?: Curador }) {
  if (!curador?.nome?.trim()) return null;
  const href = curador.contato ? comoContato(curador.contato) : null;

  return (
    <Box
      as="footer"
      mt={6}
      pt={3}
      borderTop="1px solid var(--chakra-colors-border)"
      fontSize="0.8rem"
      color="fg.muted"
    >
      <Text>
        Curadoria desta estante: <Text as="span" color="fg">{curador.nome}</Text>
        {curador.contato && (
          <>
            {" · "}
            {href ? (
              <Link
                href={href}
                color="accent"
                target={href.startsWith("mailto:") ? undefined : "_blank"}
                rel={href.startsWith("mailto:") ? undefined : "noopener noreferrer"}
              >
                {curador.contato}
              </Link>
            ) : (
              curador.contato
            )}
          </>
        )}
      </Text>
      <Text mt={1}>
        A escolha dos itens é dela, e a responsabilidade também — o Tributum não fala por
        instituição nenhuma.
      </Text>
    </Box>
  );
}
