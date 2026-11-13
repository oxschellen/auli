import { Flex } from "@chakra-ui/react";
import { useEffect, useRef, useState } from "react";
import { Messages } from "./Messages";
import { Compositor } from "./Compositor";
import { PainelHistorico } from "./PainelHistorico";
import { SelectQuestionType } from "./SelectQuestionType";
import { EstadoInicial } from "./EstadoInicial";
import { useConversa } from "./hooks/useConversa";
import { useQuestionType } from "./hooks/useQuestionType";
import { useHistorico } from "./hooks/useHistorico";
import { usePainelHistorico } from "./hooks/usePainelHistorico";
import { isPromptValid } from "./lib/prompt";
import type { ItemHistorico } from "./lib/historico";
import { TIPOS } from "./lib/textoDosTipos";
import { useSelectedEntity } from "../../contexto/EntityContext";
import { useAvisarResposta } from "../../contexto/RespostaDaAba";

/**
 * O chat de uma aba de estado. Compõe as partes (I8 do docs/ESTUDO-frontend-organizacao.md) — a
 * conversa (`useConversa`), a barra de composição (`Compositor`) e "Minhas perguntas"
 * (`PainelHistorico`) — e cuida do que é dele: o envio, a rolagem até a resposta e o que volta do
 * histórico para a caixa.
 */
export const Chat = () => {
  const entity = useSelectedEntity();
  const conversa = useConversa(entity.id);
  const [prompt, setPrompt] = useState("");
  // O seletor de fonte é dirigido pelo registry: a entidade decide quais tipos existem.
  const { questionType, updateQuestionType, disponiveis } = useQuestionType(entity);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const historico = useHistorico(entity.id);
  // Dois estados, como na barra lateral esquerda: a partir de `xl` o painel é uma coluna e a
  // escolha fica salva, e vale para todas as abas de estado; abaixo dele é um drawer, que abre e
  // fecha sem deixar rastro.
  const [painelAberto, setPainelAberto] = usePainelHistorico();
  const [drawerAberto, setDrawerAberto] = useState(false);
  const botaoDrawerRef = useRef<HTMLButtonElement>(null);

  // O spinner na aba do estado: é com a aba escondida, a pergunta ainda rodando, que ele conta.
  const avisarResposta = useAvisarResposta();
  useEffect(() => {
    avisarResposta(conversa.esperando);
  }, [avisarResposta, conversa.esperando]);

  // Só no celular: a caixa de mensagem ocupa ~220px da tela, e recolhê-la libera a leitura da
  // resposta. Fica uma barrinha que a traz de volta já com o foco no campo.
  const [recolhido, setRecolhido] = useState(false);
  const expandir = () => {
    setRecolhido(false);
    requestAnimationFrame(() => textareaRef.current?.focus());
  };

  // A altura da barra de composição, que é `fixed` e cobre o fim da lista: o `Compositor` a mede.
  // O valor inicial reproduz os 185px de recuo de antes até a primeira medição.
  const [alturaCompositor, setAlturaCompositor] = useState(169);

  // Resposta que chega rola até o COMEÇO dela, não até o fim (D-UI-2): a resposta longa terminava
  // com a tabela e as fontes na tela, e o Resumo — a parte mais lida — ficava acima, fora da vista
  // (no celular, sempre). A pergunta, a espera e o erro rolam até o fim.
  const { mensagens } = conversa;
  useEffect(() => {
    const ultima = mensagens[mensagens.length - 1];
    requestAnimationFrame(() => {
      if (ultima?.natureza === "resposta") {
        document
          .getElementById(`msg-${ultima.id}`)
          ?.scrollIntoView({ behavior: "smooth", block: "start" });
      } else {
        messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
      }
    });
  }, [mensagens]);

  // Antes da primeira pergunta, a apresentação do tipo selecionado no lugar da saudação (D-UI-10).
  const vazio = !mensagens.some((m) => m.natureza === "pergunta");
  const usarExemplo = (pergunta: string) => {
    setPrompt(pergunta);
    requestAnimationFrame(() => textareaRef.current?.focus());
  };

  // Devolve a pergunta à caixa COM o tipo em que foi feita (D-HIST-6): sem isso, a pergunta de
  // Pareceres iria para Serviços + FAQs se o seletor tivesse mudado. O `updateQuestionType` já
  // ignora o tipo que a entidade não tem. Não envia — quem pergunta ainda pode ajustar.
  const usarDoHistorico = (item: ItemHistorico) => {
    updateQuestionType(item.tipo);
    setDrawerAberto(false);
    setRecolhido(false);
    usarExemplo(item.pergunta);
  };

  const enviar = (texto: string) => {
    const pergunta = texto.trim();
    if (conversa.esperando || !isPromptValid(pergunta)) return;
    if (!conversa.enviar(pergunta, questionType)) return;
    // No ENVIO, não na chegada (D-HIST-3): a pergunta que deu erro ou estourou o tempo é
    // justamente a que a pessoa vai querer reenviar.
    historico.registrar(pergunta, questionType);
    // A caixa esvazia no ENVIO, não na chegada (D-UI-2): a pergunta já está no balão do usuário,
    // e deixá-la no campo com "Pronto para enviar" durante a espera convidava a reenviar. Quem
    // quiser reaproveitá-la usa o botão do próprio balão.
    setPrompt("");
  };

  return (
    <Flex w="100%" flex={1} bg="bg.app">
      <Flex flexDirection="column" flex={1} minW={0} pb={`${alturaCompositor + 16}px`}>
        {vazio ? (
          <EstadoInicial tipo={questionType} onExemplo={usarExemplo} />
        ) : (
          <Messages messages={mensagens} setPrompt={setPrompt} />
        )}
        <div ref={messagesEndRef} />
      </Flex>

      <PainelHistorico
        coluna={painelAberto}
        drawer={drawerAberto}
        onFecharColuna={() => setPainelAberto(false)}
        onFecharDrawer={() => setDrawerAberto(false)}
        devolverFocoA={botaoDrawerRef}
        itens={historico.itens}
        onUsar={usarDoHistorico}
        onFavoritar={historico.alternarFavorita}
        onRemover={historico.remover}
        onLimparRecentes={historico.limparRecentes}
      />

      <Compositor
        textareaRef={textareaRef}
        prompt={prompt}
        onDigitar={(e) => setPrompt(e.target.value)}
        esperando={conversa.esperando}
        onEnviar={enviar}
        placeholder={TIPOS[questionType].placeholder}
        painelAberto={painelAberto}
        onAlternarPainel={() => setPainelAberto(!painelAberto)}
        drawerAberto={drawerAberto}
        onAbrirDrawer={() => setDrawerAberto(true)}
        botaoDrawerRef={botaoDrawerRef}
        recolhido={recolhido}
        onRecolher={() => setRecolhido(true)}
        onExpandir={expandir}
        onAltura={setAlturaCompositor}
      >
        <SelectQuestionType
          questionType={questionType}
          updateQuestionType={updateQuestionType}
          disponiveis={disponiveis}
        />
      </Compositor>
    </Flex>
  );
};
