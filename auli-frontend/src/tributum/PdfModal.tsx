import { useEffect } from "react";
import { Box, Button, Flex, Link, Text } from "@chakra-ui/react";
import { MdClose, MdOpenInNew } from "react-icons/md";

interface PdfModalProps {
  titulo: string;
  /** URL do PDF hospedado. O chamador só abre isto quando `temPdfHospedado` for verdade. */
  pdfUrl: string;
  onFechar: () => void;
}

const ABRIR_NOVA_ABA = "Abrir em nova aba";

/**
 * Visualizador de PDF em modal, por `<iframe>` nativo (D-TRIB-6) — sem pdf.js nem react-pdf.
 *
 * **Feito à mão (Box `fixed` + overlay), não com o `Dialog` do Chakra**, e a divergência é do
 * repositório e não do gosto: o `LogModal.tsx` do chat já é assim, e o doc dele diz por quê — "para
 * não depender da API de Dialog da versão instalada do Chakra". Duas mecânicas de modal no mesmo
 * app seriam duas superfícies para acertar Esc, foco e overlay. Este é o mesmo esqueleto com outro
 * corpo.
 *
 * **O "Abrir em nova aba" fica SEMPRE visível**, e não é um fallback de erro: é o caminho normal no
 * Android, onde o Chrome recusa renderizar PDF em iframe e mostra um quadro em branco — sem jeito
 * de detectar isso de dentro da página (o iframe carrega "com sucesso"). Por isso a saída não pode
 * depender de detecção: ela está no cabeçalho antes de o usuário precisar dela, e a nota abaixo do
 * quadro diz o que fazer se nada aparecer.
 */
export function PdfModal({ titulo, pdfUrl, onFechar }: PdfModalProps) {
  // Esc fecha, como em qualquer diálogo — e como no LogModal.
  useEffect(() => {
    const aoTeclar = (e: KeyboardEvent) => e.key === "Escape" && onFechar();
    window.addEventListener("keydown", aoTeclar);
    return () => window.removeEventListener("keydown", aoTeclar);
  }, [onFechar]);

  return (
    <Box position="fixed" inset={0} zIndex={100}>
      <Box position="absolute" inset={0} bg="blackAlpha.600" onClick={onFechar} />
      <Flex
        position="absolute"
        top="50%"
        left="50%"
        transform="translate(-50%, -50%)"
        flexDirection="column"
        w={{ base: "94vw", md: "min(900px, 90vw)" }}
        maxH="85vh"
        bg="bg.canvas"
        color="fg"
        border="1px solid var(--chakra-colors-border)"
        borderRadius="12px"
        overflow="hidden"
        role="dialog"
        aria-modal="true"
        aria-label={titulo}
      >
        <Flex
          align="center"
          justify="space-between"
          gap={3}
          px={4}
          py={3}
          borderBottom="1px solid var(--chakra-colors-border)"
        >
          <Text fontWeight="600" fontSize="15px" lineClamp={1}>
            {titulo}
          </Text>
          <Flex gap={1} flexShrink={0} align="center">
            <Link
              href={pdfUrl}
              target="_blank"
              rel="noopener noreferrer"
              color="accent"
              fontSize="0.8rem"
              display="inline-flex"
              alignItems="center"
              gap={1}
              px={2}
            >
              <MdOpenInNew size={14} /> {ABRIR_NOVA_ABA}
            </Link>
            <Button size="xs" variant="ghost" aria-label="Fechar" onClick={onFechar}>
              <MdClose size={16} />
            </Button>
          </Flex>
        </Flex>

        <Box flex={1} minH={0} px={4} py={3}>
          <Box
            as="iframe"
            // @ts-expect-error — `src`/`title` são props do <iframe>, que o polimorfismo do
            // Chakra v3 não propaga para o tipo do Box (o mesmo motivo pelo qual o download
            // da aba Dados usa `Button asChild` com âncora de verdade).
            src={pdfUrl}
            title={titulo}
            w="100%"
            h="70vh"
            border="none"
            bg="bg.subtle"
            borderRadius="6px"
          />
          <Text fontSize="xs" color="fg.muted" mt={2}>
            Se o documento não aparecer neste dispositivo, use “{ABRIR_NOVA_ABA}”. Alguns
            navegadores de celular não exibem PDF embutido.
          </Text>
        </Box>
      </Flex>
    </Box>
  );
}
