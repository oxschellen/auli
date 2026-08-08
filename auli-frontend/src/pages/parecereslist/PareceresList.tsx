import { JurisprudenciaList } from "./JurisprudenciaList";

/** Consultas tributárias respondidas — "Pareceres" no RS, "Respostas de Consultas" em SP, COPAT em SC. */
export const PareceresList = () => (
  <JurisprudenciaList
    colecao="pareceres"
    titulo="Pareceres"
    singular="parecer"
    plural="pareceres"
  />
);
