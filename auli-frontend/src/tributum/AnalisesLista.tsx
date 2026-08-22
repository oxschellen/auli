import { useState } from "react";
import { Button, Flex, Text } from "@chakra-ui/react";
import { MdPictureAsPdf } from "react-icons/md";
import { Highlight } from "../shared/highlight";
import { TributumShell } from "./TributumShell";
import { PlaceholderBadge } from "./PlaceholderBadge";
import { LinkExterno } from "./LinkExterno";
import { PdfModal } from "./PdfModal";
import { temPdfHospedado, type Analise } from "./types";

/**
 * Análises e visualizações. Mesma mecânica dos artigos — ver lá o porquê de o estado do modal
 * morar na lista. O `link` é opcional aqui: uma análise pode existir só como PDF nosso, e nesse
 * caso o cartão leva só ao visualizador.
 */
export function AnalisesLista() {
  const [aberto, setAberto] = useState<Analise | null>(null);

  return (
    <>
      <TributumShell<Analise>
        titulo="Análises"
        secao="analises"
        substantivo={["análise", "análises"]}
        campos={(a) => [a.titulo, a.autores, a.resumo]}
      >
        {(a, terms) => (
          <>
            <Flex justify="space-between" align="flex-start" gap={3}>
              <Text fontWeight="600" fontSize="0.95rem" color="fg">
                <Highlight text={a.titulo} terms={terms} />
              </Text>
              <PlaceholderBadge item={a} />
            </Flex>
            <Text fontSize="0.8rem" color="fg.muted" mt={1}>
              <Highlight text={a.autores} terms={terms} />
            </Text>
            <Text fontSize="0.9rem" color="fg" lineHeight="1.6" mt={2}>
              <Highlight text={a.resumo} terms={terms} />
            </Text>
            {(temPdfHospedado(a) || a.link) && (
              <Flex mt={3} gap={4} align="center" wrap="wrap">
                {temPdfHospedado(a) && (
                  <Button
                    size="xs"
                    variant="outline"
                    onClick={() => setAberto(a)}
                    aria-label={`Ler o PDF de "${a.titulo}"`}
                  >
                    <MdPictureAsPdf size={14} /> Ler PDF
                  </Button>
                )}
                {a.link && (
                  <LinkExterno
                    href={a.link}
                    rotulo="Abrir na fonte"
                    ariaLabel={`Abrir "${a.titulo}" na fonte`}
                  />
                )}
              </Flex>
            )}
          </>
        )}
      </TributumShell>

      {aberto && temPdfHospedado(aberto) && (
        <PdfModal titulo={aberto.titulo} pdfUrl={aberto.pdf} onFechar={() => setAberto(null)} />
      )}
    </>
  );
}
