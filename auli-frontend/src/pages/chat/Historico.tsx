import { Box, Flex, Text, chakra } from "@chakra-ui/react";
import { MdClose, MdStar, MdStarBorder } from "react-icons/md";
import { LuPanelRight } from "react-icons/lu";
import { TIPOS } from "./utils/tipos";
import type { ItemHistorico } from "./utils/historico";

interface HistoricoProps {
  itens: ItemHistorico[];
  /** Devolve a pergunta à caixa de mensagem (não envia: quem pergunta ainda pode ajustar). */
  onUsar: (item: ItemHistorico) => void;
  onFavoritar: (id: string) => void;
  onRemover: (id: string) => void;
  onLimparRecentes: () => void;
  onFechar: () => void;
}

const dia = (em: number) =>
  new Date(em).toLocaleDateString("pt-BR", { day: "2-digit", month: "2-digit" });

/** Botão de ícone das linhas e do cabeçalho: 32px, para o dedo acertar no celular. */
const Acao = chakra("button", {
  base: {
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    flex: "none",
    w: "32px",
    h: "32px",
    borderRadius: "8px",
    color: "fg.muted",
    cursor: "pointer",
    _hover: { bg: "bg.overlay", color: "fg" },
  },
});

const Linha = ({
  item,
  onUsar,
  onFavoritar,
  onRemover,
}: { item: ItemHistorico } & Pick<HistoricoProps, "onUsar" | "onFavoritar" | "onRemover">) => (
  <Flex as="li" align="flex-start" gap="2px" borderRadius="8px" _hover={{ bg: "bg.subtle" }}>
    <chakra.button
      type="button"
      onClick={() => onUsar(item)}
      flex="1"
      minW={0}
      px="10px"
      py="6px"
      textAlign="left"
      cursor="pointer"
      borderRadius="8px"
    >
      <Text fontSize="0.85rem" lineHeight="1.35" color="fg" lineClamp={2}>
        {item.pergunta}
      </Text>
      <Text fontSize="0.7rem" color="fg.muted" mt="2px">
        {TIPOS[item.tipo].rotulo} · {dia(item.em)}
      </Text>
    </chakra.button>
    {/* `aria-pressed` é o que diz ao leitor de tela que a estrela é um liga/desliga. */}
    <Acao
      type="button"
      aria-pressed={item.favorita}
      aria-label={item.favorita ? "Tirar das favoritas" : "Marcar como favorita"}
      onClick={() => onFavoritar(item.id)}
      color={item.favorita ? "accent" : "fg.muted"}
    >
      {item.favorita ? <MdStar size={18} /> : <MdStarBorder size={18} />}
    </Acao>
    <Acao type="button" aria-label="Remover esta pergunta" onClick={() => onRemover(item.id)}>
      <MdClose size={16} />
    </Acao>
  </Flex>
);

const Grupo = ({
  titulo,
  itens,
  ...acoes
}: { titulo: string; itens: ItemHistorico[] } & Pick<
  HistoricoProps,
  "onUsar" | "onFavoritar" | "onRemover"
>) =>
  itens.length === 0 ? null : (
    <Box as="section" aria-label={titulo}>
      {/* O mesmo título de grupo da barra lateral esquerda. */}
      <Text
        fontSize="0.68rem"
        color="fg.muted"
        letterSpacing="0.06em"
        textTransform="uppercase"
        px="10px"
        mt="14px"
        mb="4px"
      >
        {titulo}
      </Text>
      <Flex as="ul" direction="column" gap="2px" listStyleType="none">
        {itens.map((item) => (
          <Linha key={item.id} item={item} {...acoes} />
        ))}
      </Flex>
    </Box>
  );

/**
 * O conteúdo do painel "Minhas perguntas": as favoritas em cima, as recentes embaixo.
 *
 * Só a lista e os botões — onde ela aparece (coluna fixa à direita ou drawer) é decisão do
 * `Chat`, que a monta nos dois lugares.
 */
export const Historico = ({ itens, onLimparRecentes, onFechar, ...acoes }: HistoricoProps) => {
  const favoritas = itens.filter((i) => i.favorita);
  const recentes = itens.filter((i) => !i.favorita);

  return (
    <Flex direction="column" minH="100%" p="10px 8px">
      <Flex align="center" justify="space-between" pl="10px">
        <Text fontSize="0.9rem" fontWeight="600" color="fg">
          Minhas perguntas
        </Text>
        <Acao type="button" aria-label="Fechar minhas perguntas" onClick={onFechar}>
          <LuPanelRight size={18} aria-hidden="true" />
        </Acao>
      </Flex>

      {itens.length === 0 && (
        <Text fontSize="0.85rem" lineHeight="1.45" color="fg.muted" px="10px" mt="14px">
          As perguntas que você enviar aparecem aqui. Marque com a estrela as que quiser manter.
        </Text>
      )}
      <Grupo titulo="Favoritas" itens={favoritas} {...acoes} />
      <Grupo titulo="Recentes" itens={recentes} {...acoes} />

      <Box mt="auto" pt="14px" px="10px">
        {recentes.length > 0 && (
          <chakra.button
            type="button"
            onClick={onLimparRecentes}
            fontSize="0.8rem"
            color="fg.muted"
            cursor="pointer"
            _hover={{ color: "fg", textDecoration: "underline" }}
          >
            Limpar recentes
          </chakra.button>
        )}
        <Text fontSize="0.7rem" lineHeight="1.4" color="fg.muted" mt="6px">
          Guardadas só neste navegador.
        </Text>
      </Box>
    </Flex>
  );
};
