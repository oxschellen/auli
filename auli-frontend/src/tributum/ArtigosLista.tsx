import { useState } from "react";
import { Button, Flex, Text } from "@chakra-ui/react";
import { MdPictureAsPdf } from "react-icons/md";
import { Highlight } from "../shared/highlight";
import { TributumShell } from "./TributumShell";
import { PlaceholderBadge } from "./PlaceholderBadge";
import { LinkExterno } from "./LinkExterno";
import { PdfModal } from "./PdfModal";
import { temPdfHospedado, type Artigo } from "./types";

/**
 * Artigos e estudos da curadoria. Terceiros entram como **link + resenha própria** (D-TRIB-8) — o
 * `resumo` é texto nosso sobre o trabalho alheio, nunca o abstract copiado.
 *
 * O estado do visualizador mora AQUI, e não dentro do cartão: o `children` do shell é chamado por
 * item, então um `useState` lá dentro seria um por cartão. Com o item aberto guardado na lista, o
 * modal é um só e o `temPdfHospedado` estreita o tipo — é ele que garante que `pdf` existe quando
 * o modal recebe a URL, sem `!` nem asserção.
 */
export function ArtigosLista() {
  const [aberto, setAberto] = useState<Artigo | null>(null);

  return (
    <>
      <TributumShell<Artigo>
        titulo="Artigos e estudos"
        secao="artigos"
        substantivo={["artigo", "artigos"]}
        campos={(a) => [a.titulo, a.autores, a.instituicao, a.ano, a.resumo]}
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
              <Highlight text={`${a.autores} · ${a.instituicao} · ${a.ano}`} terms={terms} />
            </Text>
            <Text fontSize="0.9rem" color="fg" lineHeight="1.6" mt={2}>
              <Highlight text={a.resumo} terms={terms} />
            </Text>
            <Flex mt={3} gap={4} align="center" wrap="wrap">
              {/* D-TRIB-8: só o que é NOSSO abre aqui dentro. Item de terceiro, mesmo com `pdf`
                  preenchido por engano no catálogo, não ganha este botão. */}
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
              <LinkExterno
                href={a.link}
                rotulo={a.hospedado ? "Fonte oficial" : "Abrir na fonte"}
                ariaLabel={`Abrir "${a.titulo}" na fonte`}
              />
            </Flex>
          </>
        )}
      </TributumShell>

      {aberto && temPdfHospedado(aberto) && (
        <PdfModal titulo={aberto.titulo} pdfUrl={aberto.pdf} onFechar={() => setAberto(null)} />
      )}
    </>
  );
}
