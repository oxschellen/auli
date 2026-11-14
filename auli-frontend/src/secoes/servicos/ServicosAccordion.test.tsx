// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { fireEvent, renderWithProvider, screen } from "../../test/render";
import { AccordionItem } from "./ServicosAccordion";
import type { Servico } from "./servicos";

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

function renderClasse(items: Servico[]) {
  return renderWithProvider(
    <AccordionItem
      classe="Atendimento"
      items={items}
      isOpen
      onToggle={() => {}}
      isFirst
      isLast
      terms={[]}
    />,
  );
}

/** O jsdom não navega: o clique num `<a target="_blank">` só polui o console. */
const naoNavegar = (e: Event) => e.preventDefault();
beforeEach(() => document.addEventListener("click", naoNavegar));
afterEach(() => document.removeEventListener("click", naoNavegar));

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
});
