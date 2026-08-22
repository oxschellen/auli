import { Box, Button, Flex, Text } from "@chakra-ui/react";
import { MdFileDownload } from "react-icons/md";
import { Highlight } from "../shared/highlight";
import { TributumShell } from "./TributumShell";
import { PlaceholderBadge } from "./PlaceholderBadge";
import { LinkExterno } from "./LinkExterno";
import type { Dataset } from "./types";

/**
 * Material de dados. `arquivo` (hospedado por nós) e `link` (fonte externa) são alternativos —
 * D-TRIB-8 permite hospedar dataset público citando a fonte, ao contrário de PDF de terceiro.
 *
 * O `arquivo` ganha `download` no `<a>` de propósito: é um `.csv`, e sem o atributo o navegador
 * abre a planilha como texto na aba em vez de salvá-la.
 */
export function DadosLista() {
  return (
    <TributumShell<Dataset>
      titulo="Dados"
      secao="dados"
      substantivo={["conjunto", "conjuntos"]}
      campos={(d) => [d.titulo, d.fonte, d.resumo]}
    >
      {(d, terms) => (
        <>
          <Flex justify="space-between" align="flex-start" gap={3}>
            <Text fontWeight="600" fontSize="0.95rem" color="fg">
              <Highlight text={d.titulo} terms={terms} />
            </Text>
            <PlaceholderBadge item={d} />
          </Flex>
          <Text fontSize="0.8rem" color="fg.muted" mt={1}>
            <Highlight
              text={`${d.fonte} · ${d.periodo} · ${d.formato.toUpperCase()} · atualizado em ${d.atualizado_em}`}
              terms={terms}
            />
          </Text>
          <Text fontSize="0.9rem" color="fg" lineHeight="1.6" mt={2}>
            <Highlight text={d.resumo} terms={terms} />
          </Text>
          <Box mt={3}>
            {d.arquivo ? (
              /* `asChild` → âncora de verdade, que é o que faz o atributo `download` valer.
                 Mesmo padrão do botão de `.zip` da aba Downloads. */
              <Button asChild size="xs" variant="outline">
                <a
                  href={d.arquivo}
                  download
                  aria-label={`Baixar ${d.titulo} em ${d.formato.toUpperCase()}`}
                >
                  <MdFileDownload size={14} /> Baixar ({d.formato.toUpperCase()}
                  )
                </a>
              </Button>
            ) : (
              d.link && (
                <LinkExterno
                  href={d.link}
                  rotulo="Abrir na fonte"
                  ariaLabel={`Abrir "${d.titulo}" na fonte`}
                />
              )
            )}
          </Box>
        </>
      )}
    </TributumShell>
  );
}
