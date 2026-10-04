import { Box } from "@chakra-ui/react";
import type { Fonte } from "../../types/chat";
import { toAbsoluteHref } from "../../shared/markdown";

/** Como o analista lê o veredito da triagem (D-SF-9). `aplica` não ganha selo: é o caso normal. */
const VEREDITO: Record<NonNullable<Fonte["triagem"]>, string> = {
  aplica: "",
  condicional: "depende de condição",
  descarta: "descartada na triagem",
};

/** A linha do resumo: "Fontes consultadas: 15 (3 descartadas na triagem)". */
export function resumoFontes(fontes: Fonte[]): string {
  const descartadas = fontes.filter((f) => f.triagem === "descarta").length;
  const base = `Fontes consultadas: ${fontes.length}`;
  if (descartadas === 0) return base;
  return `${base} (${descartadas} ${descartadas === 1 ? "descartada" : "descartadas"} na triagem)`;
}

interface FontesProps {
  fontes: Fonte[];
}

/**
 * A lista FIXA dos documentos que a busca trouxe (D-SF-10), abaixo da resposta de Serviços + FAQs.
 *
 * Existe para que o analista veja sempre as fontes, independentemente do que o texto do LLM decidiu
 * citar — o núcleo do Auli é achar a informação, e o texto gerado é complemento. Os rótulos são os
 * mesmos da citação no texto (`[Serviço 2]`), na ordem do contexto. Recolhida por padrão: com até 15
 * itens, aberta empurraria a próxima pergunta para longe no celular.
 */
export const Fontes = ({ fontes }: FontesProps) => (
  <Box as="details" mt={3} fontSize="13px" lineHeight="1.5" color="fg">
    <Box as="summary" cursor="pointer" color="fg.muted" userSelect="none">
      {resumoFontes(fontes)}
    </Box>
    <Box as="ul" listStyleType="none" pl={0} mt={2} mb={0}>
      {fontes.map((f) => {
        const descartada = f.triagem === "descarta";
        const selo = f.triagem ? VEREDITO[f.triagem] : "";
        return (
          <Box as="li" key={f.rotulo} py="2px" opacity={descartada ? 0.6 : 1}>
            <Box as="span" fontWeight="600" mr={2} whiteSpace="nowrap">
              [{f.rotulo}]
            </Box>
            {f.link ? (
              <a
                href={toAbsoluteHref(f.link)}
                target="_blank"
                rel="noopener noreferrer"
                style={{
                  color: "var(--chakra-colors-accent)",
                  textDecoration: descartada ? "line-through" : "underline",
                  textUnderlineOffset: "2px",
                }}
              >
                {f.titulo}
              </a>
            ) : (
              <span>{f.titulo}</span>
            )}
            {selo && (
              <Box as="span" ml={2} color="fg.muted" fontSize="11px">
                · {selo}
              </Box>
            )}
          </Box>
        );
      })}
    </Box>
  </Box>
);
