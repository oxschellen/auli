import { useEffect, useRef } from "react";
import type { KeyboardEvent } from "react";
import { Box, Flex, Spinner, chakra } from "@chakra-ui/react";
import { MdAdd, MdClose } from "react-icons/md";
import { Tooltip } from "../pages/chat/ui/tooltip";
import { getEntity } from "./entities";
import { idDaAba, idDoPainel } from "./abas";
import type { Abas } from "./abas";

/** Estilo da aba com o mouse sobre ela — só onde há mouse. O `_groupHover` do Chakra não tem o
 *  `@media (hover: hover)` que o `_hover` tem, e no celular o toque liga o `:hover` antes de decidir
 *  o alvo: o × apareceria sob o dedo e o toque no sinal fecharia a aba. */
const comMouseNaAba = (estilo: Record<string, string>) => ({
  "@media (hover: hover)": { ".group:hover &": estilo },
});

interface AbasDeEstadoProps {
  abas: Abas;
  /** As chaves das abas cuja conversa espera resposta: ganham um spinner, e "respondendo" no
   *  nome acessível. */
  respondendo: ReadonlySet<string>;
  /** As chaves das abas que receberam resposta escondidas: ganham um ponto até serem abertas, e
   *  "resposta nova" no nome acessível. */
  novas: ReadonlySet<string>;
  onAtivar: (chave: string) => void;
  onFechar: (chave: string) => void;
  onNova: () => void;
}

/**
 * A faixa de abas de estado, no cabeçalho ao lado do logo: `[RS SEFAZ-RS ×] [SP SEFAZ-SP ×] [+]`.
 *
 * Padrão WAI-ARIA de tabs com ativação automática: as setas trocam de aba (dando a volta), Delete
 * fecha a aba com o foco. O × fica DENTRO do botão da aba, como `span` escondido do leitor de tela:
 * um botão dentro de outro é HTML inválido, e um botão ao lado poria no `tablist` um filho que não
 * é `tab`. Pelo teclado, fecha-se com Delete, anunciado em `aria-keyshortcuts`. O "+" fica fora do
 * `tablist` pelo mesmo motivo.
 *
 * Abaixo de `sm` a aba mostra só a sigla: o nome não cabe ao lado do logo. Com muitas abas, a faixa
 * rola na horizontal e a ativa é trazida para a vista.
 *
 * O spinner (esperando resposta) e o ponto (resposta nova) ocupam o lugar do ×, como o ponto do
 * arquivo não salvo no VS Code: a aba não muda de largura, e as da direita não andam. Com o mouse
 * sobre a aba, o × volta. Sem mouse (celular), o toque no sinal troca de aba em vez de fechá-la.
 */
export const AbasDeEstado = ({
  abas,
  respondendo,
  novas,
  onAtivar,
  onFechar,
  onNova,
}: AbasDeEstadoProps) => {
  const refs = useRef<Record<string, HTMLButtonElement | null>>({});
  // Fechar pelo teclado tira de cena a aba que tinha o foco, e a nova ativa só existe depois do
  // render: o foco vai para ela no efeito.
  const focarAtiva = useRef(false);

  useEffect(() => {
    const ativa = refs.current[abas.ativa];
    ativa?.scrollIntoView({ block: "nearest", inline: "nearest" });
    if (focarAtiva.current) {
      focarAtiva.current = false;
      ativa?.focus();
    }
  }, [abas]);

  const aoTeclar = (chave: string) => (e: KeyboardEvent<HTMLButtonElement>) => {
    if (e.key === "Delete") {
      e.preventDefault();
      focarAtiva.current = true;
      onFechar(chave);
      return;
    }
    const delta = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (!delta) return;
    e.preventDefault();
    const n = abas.lista.length;
    const i = abas.lista.findIndex((a) => a.chave === chave);
    const proxima = abas.lista[(i + delta + n) % n];
    onAtivar(proxima.chave);
    refs.current[proxima.chave]?.focus();
  };

  // A única aba, ainda no mapa: fechá-la só daria outra igual, então ela não tem ×.
  const soOMapa = abas.lista.length === 1 && abas.lista[0].entityId === null;

  return (
    <Flex alignItems="center" gap={1} minW={0}>
      <Flex
        role="tablist"
        aria-label="Estados abertos"
        alignItems="center"
        gap={1}
        minW={0}
        overflowX="auto"
        css={{ scrollbarWidth: "none" }}
      >
        {abas.lista.map((aba) => {
          const entity = getEntity(aba.entityId);
          const ativa = aba.chave === abas.ativa;
          const sinal = respondendo.has(aba.chave)
            ? "respondendo"
            : novas.has(aba.chave) && !ativa
              ? "resposta nova"
              : null;
          const nome = entity?.name ?? "Novo estado";
          return (
            <chakra.button
              key={aba.chave}
              ref={(el: HTMLButtonElement | null) => {
                refs.current[aba.chave] = el;
              }}
              type="button"
              role="tab"
              // Rótulo explícito: o do conteúdo juntaria a sigla ao nome ("RSSEFAZ-RS") e, abaixo
              // de `sm`, com o nome escondido, seria só a sigla.
              aria-label={sinal ? `${nome}, ${sinal}` : nome}
              id={idDaAba(aba.chave)}
              aria-selected={ativa}
              aria-controls={idDoPainel(aba.chave)}
              aria-keyshortcuts="Delete"
              // Roving tabindex: só a ativa na ordem de Tab; as demais pelas setas.
              tabIndex={ativa ? 0 : -1}
              onClick={() => onAtivar(aba.chave)}
              onKeyDown={aoTeclar(aba.chave)}
              className="group"
              display="flex"
              alignItems="center"
              gap={1.5}
              h="32px"
              pl={entity ? 1.5 : 2.5}
              pr={soOMapa ? 2.5 : 1}
              flexShrink={0}
              borderRadius="8px"
              border="1px solid"
              borderColor={ativa ? "border" : "transparent"}
              bg={ativa ? "bg.subtle" : "transparent"}
              color={ativa ? "fg" : "fg.muted"}
              fontWeight="500"
              _hover={ativa ? undefined : { bg: "bg.overlay", color: "fg" }}
            >
              {entity && (
                <Box
                  as="span"
                  fontWeight="700"
                  fontSize="11px"
                  bg={ativa ? "accent" : "bg.muted"}
                  color={ativa ? "accent.fg" : "fg.muted"}
                  borderRadius="5px"
                  px={1.5}
                  lineHeight="20px"
                >
                  {entity.uf}
                </Box>
              )}
              <Box
                as="span"
                fontSize="14px"
                whiteSpace="nowrap"
                display={{ base: entity ? "none" : "inline", sm: "inline" }}
              >
                {nome}
              </Box>
              {!soOMapa && (
                <chakra.span
                  aria-hidden="true"
                  title="Fechar (Delete)"
                  onClick={(e) => {
                    e.stopPropagation();
                    onFechar(aba.chave);
                  }}
                  display="flex"
                  alignItems="center"
                  justifyContent="center"
                  boxSize="18px"
                  borderRadius="4px"
                  color="fg.muted"
                  _hover={{ bg: "bg.muted", color: "fg" }}
                  // Com o sinal no lugar do ×, só fecha com o × à vista (mouse sobre a aba).
                  pointerEvents={sinal ? "none" : undefined}
                  css={sinal ? comMouseNaAba({ pointerEvents: "auto" }) : undefined}
                >
                  {sinal && (
                    <Box display="flex" css={comMouseNaAba({ display: "none" })}>
                      {sinal === "respondendo" ? (
                        <Spinner size="xs" color="fg.muted" />
                      ) : (
                        <Box boxSize="8px" borderRadius="full" bg="accent" />
                      )}
                    </Box>
                  )}
                  <Box display={sinal ? "none" : "flex"} css={comMouseNaAba({ display: "flex" })}>
                    <MdClose size={14} />
                  </Box>
                </chakra.span>
              )}
            </chakra.button>
          );
        })}
      </Flex>
      <Tooltip content="Abrir outro estado">
        <chakra.button
          type="button"
          aria-label="Abrir outro estado"
          onClick={onNova}
          display="flex"
          alignItems="center"
          p="6px"
          flexShrink={0}
          borderRadius="8px"
          color="fg.muted"
          _hover={{ bg: "bg.overlay", color: "fg" }}
        >
          <MdAdd size={18} aria-hidden="true" />
        </chakra.button>
      </Tooltip>
    </Flex>
  );
};
