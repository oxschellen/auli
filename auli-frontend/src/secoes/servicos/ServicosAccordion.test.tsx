// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { fireEvent, renderWithProvider, screen } from "../../test/render";
import { AccordionItem } from "./ServicosAccordion";
import type { Servico } from "./servicos";
import { utilsCopyTextToClipboard } from "../../ui/copiar";
import { parseQuery } from "../../lib/textSearch";

vi.mock("../../ui/copiar", () => ({ utilsCopyTextToClipboard: vi.fn() }));

const comDescricao: Servico = {
  id: 1,
  classe: "Atendimento",
  titulo: "Comunicação por WhatsApp",
  link: "http://portal/whatsapp",
  descricao: "O WhatsApp da Receita é usado para avisos.\nExemplos: a) prazos;\nb) atrasos.",
};

const semDescricao: Servico = {
  id: 2,
  classe: "Atendimento",
  titulo: "Agendamento presencial",
  link: "http://portal/agenda",
  descricao: "",
};

function renderClasse(items: Servico[], busca = "") {
  return renderWithProvider(
    <AccordionItem
      classe="Atendimento"
      items={items}
      isOpen
      onToggle={() => {}}
      isFirst
      isLast
      terms={parseQuery(busca)}
    />,
  );
}

/** Todos os `<mark>` do documento, na ordem, pelo texto. */
const marcados = (): string[] =>
  Array.from(document.querySelectorAll("mark")).map((m) => m.textContent ?? "");

/** O jsdom não navega: o clique num `<a target="_blank">` só polui o console. */
const naoNavegar = (e: Event) => e.preventDefault();
beforeEach(() => document.addEventListener("click", naoNavegar));
afterEach(() => {
  document.removeEventListener("click", naoNavegar);
  vi.mocked(utilsCopyTextToClipboard).mockClear();
});

describe("ServicosAccordion — a descrição do serviço", () => {
  it("serviço com descrição: a linha abre e fecha o texto ali mesmo", () => {
    renderClasse([comDescricao]);
    const linha = screen.getByRole("button", { name: "Comunicação por WhatsApp" });
    expect(linha).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText(/usado para avisos/)).toBeNull();

    fireEvent.click(linha);
    expect(linha).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText(/usado para avisos/)).toBeInTheDocument();

    fireEvent.keyDown(linha, { key: "Enter" });
    expect(linha).toHaveAttribute("aria-expanded", "false");
  });

  it("serviço sem descrição continua sendo o link para o portal, sem acordeão", () => {
    renderClasse([semDescricao]);
    const link = screen.getByRole("link", { name: "Agendamento presencial" });
    expect(link).toHaveAttribute("href", "http://portal/agenda");
    expect(link).not.toHaveAttribute("aria-expanded");
    expect(screen.queryByRole("button", { name: "Agendamento presencial" })).toBeNull();
  });

  it("o portal do serviço com descrição é um `<a>` de verdade, numa aba nova sem opener", () => {
    renderClasse([comDescricao]);
    const icone = screen.getByRole("link", {
      name: "Abrir Comunicação por WhatsApp no portal oficial",
    });
    expect(icone).toHaveAttribute("href", "http://portal/whatsapp");
    expect(icone).toHaveAttribute("target", "_blank");
    expect(icone).toHaveAttribute("rel", "noopener noreferrer");
  });

  it("Enter e clique no ícone do portal não mexem no acordeão", () => {
    renderClasse([comDescricao]);
    // A linha pelo `aria-expanded`, não pelo nome: se o ícone fosse parar DENTRO do botão, o
    // rótulo dele entraria no nome do botão, e a busca por nome falharia antes da asserção.
    const linha = screen.getByRole("button", { expanded: false });
    const icone = screen.getByRole("link", {
      name: "Abrir Comunicação por WhatsApp no portal oficial",
    });
    fireEvent.keyDown(icone, { key: "Enter" });
    fireEvent.keyDown(icone, { key: " " });
    fireEvent.click(icone);
    expect(linha).toHaveAttribute("aria-expanded", "false");
  });

  it("o painel começa pelo endereço do serviço, clicável, e depois vem a descrição", () => {
    renderClasse([comDescricao]);
    fireEvent.click(screen.getByRole("button", { name: "Comunicação por WhatsApp" }));
    const link = screen.getByRole("link", { name: "http://portal/whatsapp" });
    expect(link).toHaveAttribute("href", "http://portal/whatsapp");
    expect(link).toHaveAttribute("target", "_blank");
    expect(link).toHaveAttribute("rel", "noopener noreferrer");
    expect(link.parentElement).toHaveTextContent(/^http:\/\/portal\/whatsapp\s*O WhatsApp/);
  });

  it("a descrição sai como texto puro: nada some, e o endereço entre aspas vira link", () => {
    // O caso do SC: `<Manual de Orientação>` e a URL entre aspas. Em markdown o `<…>` sumiria
    // como HTML, e as linhas com `* ` e `1. ` virariam listas.
    const descricao =
      'Preencher conforme  <Manual de Orientação> "https://www.sef.sc.gov.br/servicos/48".\n' +
      "* Sócio sem IE deverá se cadastrar.\n1. Gerar o DAE.";
    renderClasse([{ ...comDescricao, descricao }]);
    fireEvent.click(screen.getByRole("button", { name: "Comunicação por WhatsApp" }));
    expect(document.body.textContent).toContain(descricao);
    expect(
      screen.getByRole("link", { name: "https://www.sef.sc.gov.br/servicos/48" }),
    ).toHaveAttribute("href", "https://www.sef.sc.gov.br/servicos/48");
    expect(document.querySelector("ul, ol, li")).toBeNull();
  });

  it("o ícone de copiar leva nome, link e descrição, sem mexer no acordeão", () => {
    renderClasse([comDescricao]);
    const linha = screen.getByRole("button", { expanded: false });
    const copiar = screen.getByRole("button", {
      name: "Copiar a descrição de Comunicação por WhatsApp",
    });
    // Uma asserção por evento: com o ícone aninhado no botão, o Enter abriria e o clique fecharia,
    // e uma asserção só no fim veria "false" de novo.
    fireEvent.keyDown(copiar, { key: "Enter" });
    expect(linha).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(copiar);
    expect(linha).toHaveAttribute("aria-expanded", "false");
    expect(utilsCopyTextToClipboard).toHaveBeenCalledTimes(1);
    expect(utilsCopyTextToClipboard).toHaveBeenCalledWith(
      `Comunicação por WhatsApp\nhttp://portal/whatsapp\n\n${comDescricao.descricao}`,
      "A descrição foi copiada para a área de transferência",
    );
  });

  it("serviço sem descrição não tem o que copiar", () => {
    renderClasse([semDescricao]);
    expect(screen.queryByRole("button", { name: /^Copiar/ })).toBeNull();
  });
});

describe("ServicosAccordion — a descrição na busca", () => {
  it("quando só a descrição casa, a linha mostra o trecho marcado, com o painel fechado", () => {
    renderClasse([comDescricao], "avisos");
    const linha = screen.getByRole("button", { expanded: false });
    expect(linha).toHaveTextContent("O WhatsApp da Receita é usado para avisos.");
    expect(marcados()).toEqual(["avisos"]);
  });

  it("quando o título já casa, não há trecho", () => {
    renderClasse([comDescricao], "whatsapp");
    expect(screen.queryByText(/usado para avisos/)).toBeNull();
    expect(marcados()).toEqual(["WhatsApp"]);
  });

  it("no grupo que entrou pela classe, não há trecho", () => {
    // "atendimento" está na classe e também na descrição: quem explica o resultado é a classe.
    const descricao = "Atendimento por mensagem, sem fila.";
    renderClasse([{ ...comDescricao, descricao }], "atendimento");
    expect(screen.queryByText(/sem fila/)).toBeNull();
  });

  it("o painel aberto marca o termo na descrição, no texto e no endereço", () => {
    const descricao = 'Avisos pelo WhatsApp: veja "https://portal/avisos".';
    renderClasse([{ ...comDescricao, descricao }], "avisos");
    fireEvent.click(screen.getByRole("button", { expanded: false }));
    // Trecho da linha (2 marcas: o texto e o endereço) e painel (as mesmas 2).
    expect(marcados()).toEqual(["Avisos", "avisos", "Avisos", "avisos"]);
    expect(screen.getByRole("link", { name: "https://portal/avisos" })).toHaveAttribute(
      "href",
      "https://portal/avisos",
    );
  });
});
