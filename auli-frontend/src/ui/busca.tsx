import { useDeferredValue, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Box, Text } from "@chakra-ui/react";
import { parseQuery } from "../lib/textSearch";
import { SearchInput } from "./SearchInput";
import { Z } from "./layout";

/**
 * As peças da busca das listas (I3 do docs/ESTUDO-frontend-organizacao.md): o estado, a barra fixa
 * e a mensagem de vazio. São peças, não um componente único — cada lista continua dona do seu
 * filtro e do seu corpo, o que respeita a decisão da `LegislacaoList` de não fundir listas que só
 * compartilham a caixa de busca.
 */

/** O estado de uma caixa de busca: o texto na hora, o texto adiado e os termos já normalizados. */
export function useBusca() {
  const [texto, setTexto] = useState("");
  // O campo responde na hora; filtrar o acervo (que pode ter dezenas de milhares de itens) usa o
  // valor adiado, para a digitação não engasgar.
  const adiado = useDeferredValue(texto);
  // Uma vez por busca, não por item: `parseQuery` devolve array novo a cada chamada.
  const termos = useMemo(() => parseQuery(adiado), [adiado]);
  return {
    texto,
    setTexto,
    adiado,
    termos,
    // Do texto, e não dos termos: uma letra isolada sai do `parseQuery`, mas ainda é busca ativa.
    buscando: adiado.trim().length > 0,
  };
}
export type Busca = ReturnType<typeof useBusca>;

/** A barra fixa no topo da lista. `antes` recebe o que vem acima do campo (as abas de público dos
 *  Serviços); `children` é a linha do contador, e sem ela a linha não aparece. A barra é dona do
 *  campo: limpar pelo botão devolve o foco a ele. */
export function BarraDeBusca({
  busca,
  placeholder,
  antes,
  children,
}: {
  busca: Busca;
  placeholder: string;
  antes?: ReactNode;
  children?: ReactNode;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  return (
    <Box
      position="sticky"
      top={0}
      zIndex={Z.barraFixa}
      bg="bg.app"
      px={4}
      pt={3}
      pb={2}
      borderBottom="1px solid"
      borderColor="border"
    >
      {antes}
      <SearchInput
        ref={inputRef}
        value={busca.texto}
        onChange={(e) => busca.setTexto(e.target.value)}
        onClear={() => {
          busca.setTexto("");
          inputRef.current?.focus();
        }}
        placeholder={placeholder}
      />
      {children && (
        <Text pt={2} fontSize="0.8rem" color="fg.muted" fontWeight="500">
          {children}
        </Text>
      )}
    </Box>
  );
}

/** A busca não achou nada. */
export const SemResultados = ({ consulta }: { consulta: string }) => (
  <Box py={10} textAlign="center">
    <Text color="fg.muted" fontSize="0.95rem">
      Nenhum resultado encontrado para &ldquo;{consulta}&rdquo;
    </Text>
  </Box>
);
