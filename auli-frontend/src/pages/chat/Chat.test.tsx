// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { renderWithProvider } from "../../test/render";
import { EntityProvider } from "../../shared/EntityContext";
import { getEntity } from "../../shared/entities";
import { Chat } from "./Chat";
import { SIDEBAR_VAR, SIDEBAR_WIDTH } from "../../shared/layout";

// A consulta nunca responde: os testes daqui olham o que acontece no ENVIO, não na chegada.
vi.mock("axios", () => ({
  default: { post: vi.fn(() => new Promise(() => {})), isCancel: vi.fn(() => false) },
}));

/** O Chat exige uma entidade: a da aba de estado em que ele estaria. */
const RS = getEntity("rs")!;

const montar = () =>
  renderWithProvider(
    <EntityProvider entity={RS}>
      <Chat />
    </EntityProvider>,
  );

describe("Chat", () => {
  // A entrega 2 em uma linha: o seletor de tipo de consulta era IRMÃO da caixa de mensagem, na
  // barra fixa, e passou a ser filho dela. Sem isto, um refactor que o tirasse de volta para fora
  // passaria por todos os outros testes — nenhum deles olha a composição.
  it("monta o seletor de tipo de consulta DENTRO da caixa de mensagem", () => {
    montar();
    const caixa = screen.getByRole("group", { name: "Caixa de mensagem" });
    const seletor = screen.getByRole("button", { name: /^Tipo de consulta:/ });
    expect(caixa).toContainElement(seletor);
    expect(caixa).toContainElement(screen.getByLabelText("Sua pergunta"));
  });

  it("o seletor vem ANTES do campo dentro da caixa", () => {
    montar();
    const seletor = screen.getByRole("button", { name: /^Tipo de consulta:/ });
    const campo = screen.getByLabelText("Sua pergunta");
    // `DOCUMENT_POSITION_FOLLOWING` = o campo vem depois do seletor na ordem do documento, que é
    // a ordem que o teclado e o leitor de tela seguem.
    expect(seletor.compareDocumentPosition(campo) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  // A troca de tipo em si é do RadioGroup do Chakra, que o jsdom não aciona por clique; ela é
  // conferida no navegador (Chromium). Aqui: abrir, listar, fechar.
  it("o seletor abre a lista dos tipos (radiogroup) e fecha com Esc", () => {
    montar();
    const botao = screen.getByRole("button", { name: /^Tipo de consulta:/ });
    expect(botao).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(botao);
    expect(botao).toHaveAttribute("aria-expanded", "true");
    const grupo = screen.getByRole("radiogroup", { name: "Tipo de consulta" });
    expect(grupo).toHaveTextContent("Serviços + FAQs");
    expect(grupo).toHaveTextContent("Pareceres");
    fireEvent.keyDown(grupo, { key: "Escape" });
    expect(screen.queryByRole("radiogroup")).toBeNull();
  });

  it("antes da primeira pergunta, mostra a apresentação do tipo e os exemplos vão para a caixa", () => {
    localStorage.setItem("auli.questionType", "1");
    montar();
    expect(screen.getByRole("heading", { name: "Como posso ajudar?" })).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "Empresa nova: como apurar e recolher o ICMS?" }),
    );
    expect(screen.getByLabelText("Sua pergunta")).toHaveValue(
      "Empresa nova: como apurar e recolher o ICMS?",
    );
  });
});

/**
 * Regressão que chegou a produção na v0.1.59: a barra de composição é `position: fixed` — o que é
 * o que permite subi-la acima do teclado virtual — e `fixed` se ancora na VIEWPORT, não na área de
 * conteúdo. Com `left: 0` ela cobria a sidebar inteira.
 *
 * O teste olha o CSS que o Chakra emite porque é onde a correção vive: não há atributo nem estado
 * de React para inspecionar, só a regra e a media query.
 */
describe("barra de composição não invade a sidebar", () => {
  const regrasDaBarra = () => {
    const caixa = screen.getByRole("group", { name: "Caixa de mensagem" });
    const classes = caixa.parentElement!.className.split(/\s+/);
    const css = [...document.querySelectorAll("style")].map((s) => s.textContent).join("\n");
    return css.split("\n").filter((l) => classes.some((c) => c && l.includes(c)));
  };

  it("ocupa a largura toda abaixo de md, onde a sidebar é drawer", () => {
    montar();
    const base = regrasDaBarra().find((r) => !r.startsWith("@media"));
    expect(base).toMatch(/position:fixed/);
    expect(base).toMatch(/left:0/);
    expect(base).toMatch(/right:0/); // `right`, não `width:100%` — senão transbordaria à direita
  });

  it("recua a largura da sidebar a partir de md", () => {
    montar();
    const media = regrasDaBarra().find((r) => r.startsWith("@media"));
    expect(media).toBeDefined();
    // Pela variável que o `Home` zera com a sidebar oculta; a largura cheia é o reserva.
    expect(media).toContain(`left:var(${SIDEBAR_VAR}, ${SIDEBAR_WIDTH})`);
  });
});

/**
 * Recolher a caixa no celular. O esconder é CSS responsivo (`base: none`, `md: block`), então o
 * teste lê a regra que o Chakra emite para a classe da caixa — o mesmo método do bloco acima.
 */
describe("recolher a caixa de mensagem (celular)", () => {
  const regrasDe = (el: HTMLElement) => {
    const classes = el.className.split(/\s+/).filter(Boolean);
    const css = [...document.querySelectorAll("style")].map((s) => s.textContent).join("\n");
    return css
      .split("}")
      .filter((r) => classes.some((c) => r.includes(`.${c}`)))
      .join("}\n");
  };
  // Pelo `aria-label`, e não pelo papel: recolhida, a caixa sai da árvore de acessibilidade (o
  // jsdom aplica o `none` da base) e o `getByRole` deixa de achá-la.
  const caixa = () => screen.getByLabelText("Caixa de mensagem");

  it("Recolher esconde a caixa só abaixo de md", () => {
    montar();
    expect(regrasDe(caixa())).not.toMatch(/display:\s*none/);
    fireEvent.click(screen.getByRole("button", { name: "Recolher a caixa de mensagem" }));
    expect(regrasDe(caixa())).toMatch(/display:\s*none/);
    expect(regrasDe(caixa())).toMatch(/display:\s*block/); // o `md: block` segue valendo
  });

  it("a barrinha traz a caixa de volta com o foco no campo", async () => {
    montar();
    fireEvent.click(screen.getByRole("button", { name: "Recolher a caixa de mensagem" }));
    fireEvent.click(screen.getByRole("button", { name: "Mostrar a caixa de mensagem" }));
    expect(regrasDe(caixa())).not.toMatch(/display:\s*none/);
    await waitFor(() => expect(document.activeElement).toBe(screen.getByLabelText("Sua pergunta")));
  });
});

/**
 * O `EstadoInicial` tomou o lugar da saudação (D-UI-10), mas ela seguia como a primeira mensagem
 * do estado: ao enviar a primeira pergunta, o balão "Olá! Como posso ajudar?" surgia acima dela.
 */
describe("a conversa começa vazia", () => {
  it("depois do primeiro envio, não aparece saudação acima da pergunta", async () => {
    montar();
    const pergunta = "Qual o prazo para pagar o ICMS da empresa?";
    fireEvent.change(screen.getByLabelText("Sua pergunta"), { target: { value: pergunta } });
    fireEvent.click(screen.getByLabelText("Enviar pesquisa"));
    // A pergunta aparece no balão e, com o painel "Minhas perguntas", também na lista dele.
    const ocorrencias = await screen.findAllByText(pergunta);
    expect(ocorrencias.filter((el) => !el.closest("aside"))).toHaveLength(1);
    expect(screen.queryByText("Olá! Como posso ajudar?")).toBeNull();
  });
});

/**
 * "Minhas perguntas" (D-HIST-*). Em jsdom o `hideBelow`/`hideFrom` não recorta nada, então o painel
 * fixo e os dois botões renderizam juntos — o que se confere aqui é o comportamento, não o modo.
 */
describe("minhas perguntas", () => {
  const CHAVE = "auli.historico.rs";
  const pergunta = "Qual o prazo para pagar o ICMS da empresa?";
  const enviar = (texto: string) => {
    fireEvent.change(screen.getByLabelText("Sua pergunta"), { target: { value: texto } });
    fireEvent.click(screen.getByLabelText("Enviar pesquisa"));
  };
  const painel = () => within(screen.getByRole("complementary", { name: "Minhas perguntas" }));

  beforeEach(() => {
    localStorage.removeItem(CHAVE);
    localStorage.removeItem("auli.historicoPainel");
    localStorage.setItem("auli.questionType", "1");
  });

  it("a pergunta enviada entra no painel e fica gravada no navegador, por entidade", async () => {
    montar();
    enviar(pergunta);
    expect(await painel().findByText(pergunta)).toBeInTheDocument();
    const gravado = JSON.parse(localStorage.getItem(CHAVE)!);
    expect(gravado.itens).toHaveLength(1);
    expect(gravado.itens[0]).toMatchObject({ pergunta, tipo: "1", favorita: false });
  });

  it("clicar numa pergunta do painel a devolve à caixa, sem enviar", () => {
    localStorage.setItem(
      CHAVE,
      JSON.stringify({ v: 1, itens: [{ id: "a", pergunta, tipo: "1", em: 1, favorita: false }] }),
    );
    montar();
    fireEvent.click(painel().getByText(pergunta));
    expect(screen.getByLabelText("Sua pergunta")).toHaveValue(pergunta);
    // Não enviou: a conversa segue vazia, com a apresentação do tipo.
    expect(screen.getByRole("heading", { name: "Como posso ajudar?" })).toBeInTheDocument();
  });

  it("devolve a pergunta com o tipo de consulta em que ela foi feita", () => {
    localStorage.setItem(
      CHAVE,
      JSON.stringify({ v: 1, itens: [{ id: "a", pergunta, tipo: "2", em: 1, favorita: false }] }),
    );
    montar();
    expect(
      screen.getByRole("button", { name: /^Tipo de consulta: Serviços \+ FAQs/ }),
    ).toBeInTheDocument();
    fireEvent.click(painel().getByText(pergunta));
    expect(
      screen.getByRole("button", { name: /^Tipo de consulta: Pareceres/ }),
    ).toBeInTheDocument();
  });

  it("a estrela move a pergunta para Favoritas e a marca fica gravada", () => {
    localStorage.setItem(
      CHAVE,
      JSON.stringify({ v: 1, itens: [{ id: "a", pergunta, tipo: "1", em: 1, favorita: false }] }),
    );
    montar();
    expect(painel().queryByRole("region", { name: "Favoritas" })).toBeNull();
    fireEvent.click(painel().getByRole("button", { name: "Marcar como favorita" }));
    expect(painel().getByRole("region", { name: "Favoritas" })).toHaveTextContent(pergunta);
    expect(JSON.parse(localStorage.getItem(CHAVE)!).itens[0].favorita).toBe(true);
  });

  it("limpar recentes preserva as favoritas, e remover tira uma pergunta só", () => {
    localStorage.setItem(
      CHAVE,
      JSON.stringify({
        v: 1,
        itens: [
          { id: "a", pergunta, tipo: "1", em: 2, favorita: false },
          {
            id: "b",
            pergunta: "Como emitir a certidão de situação fiscal?",
            tipo: "1",
            em: 1,
            favorita: true,
          },
        ],
      }),
    );
    montar();
    fireEvent.click(painel().getByRole("button", { name: "Limpar recentes" }));
    expect(painel().queryByText(pergunta)).toBeNull();
    expect(painel().getByText("Como emitir a certidão de situação fiscal?")).toBeInTheDocument();
    fireEvent.click(painel().getByRole("button", { name: "Remover esta pergunta" }));
    expect(painel().getByText(/As perguntas que você enviar aparecem aqui/)).toBeInTheDocument();
    expect(JSON.parse(localStorage.getItem(CHAVE)!).itens).toEqual([]);
  });

  it("fechar o painel fica salvo, e o botão o traz de volta", () => {
    montar();
    fireEvent.click(painel().getByRole("button", { name: "Fechar minhas perguntas" }));
    expect(screen.queryByRole("complementary", { name: "Minhas perguntas" })).toBeNull();
    expect(localStorage.getItem("auli.historicoPainel")).toBe("recolhido");
    fireEvent.click(screen.getByRole("button", { name: "Mostrar minhas perguntas" }));
    expect(screen.getByRole("complementary", { name: "Minhas perguntas" })).toBeInTheDocument();
  });

  it("o drawer abre pelo botão, e Esc o fecha devolvendo o foco", () => {
    localStorage.setItem("auli.historicoPainel", "recolhido");
    montar();
    const botao = screen.getByRole("button", { name: "Abrir minhas perguntas" });
    fireEvent.click(botao);
    expect(screen.getByRole("complementary", { name: "Minhas perguntas" })).toBeInTheDocument();
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("complementary", { name: "Minhas perguntas" })).toBeNull();
    expect(document.activeElement).toBe(botao);
  });
});
