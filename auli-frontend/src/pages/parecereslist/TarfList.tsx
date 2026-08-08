import { JurisprudenciaList } from "./JurisprudenciaList";

/**
 * Acórdãos do TARF — o Tribunal Administrativo de Recursos Fiscais do RS, segunda instância do
 * contencioso tributário estadual.
 *
 * Mesmo formato dos pareceres, com outro papel: o `numero` é o do acórdão, o `assunto` é a ementa e
 * o `resumo` é a fundamentação que o próprio acórdão traz no cabeçalho — não uma sinopse gerada.
 */
export const TarfList = () => (
  <JurisprudenciaList
    colecao="tarf"
    titulo="Acórdãos TARF"
    singular="acórdão"
    plural="acórdãos"
  />
);
