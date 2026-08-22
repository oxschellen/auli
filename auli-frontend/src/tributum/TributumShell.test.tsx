// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { SWRConfig } from "swr";
import { renderWithProvider } from "../test/render";
import { ArtigosLista } from "./ArtigosLista";

/**
 * Contrato do esqueleto compartilhado das quatro listas, exercitado pela `ArtigosLista` — que é a
 * mais completa (badge, metadados, link) e por isso a que cobre mais do esqueleto por render.
 *
 * O `axios` é dublado porque é ele que o `jsonFetcher` da casa usa; interceptar `fetch` não pegaria
 * nada.
 *
 * Cada teste monta com um `SWRConfig` de cache PRÓPRIO (`provider: () => new Map()`). O cache do
 * SWR é global ao módulo e a chave é a mesma nos cinco casos — sem o provider isolado, o teste de
 * erro recebia o catálogo já resolvido pelo teste anterior e passava sem exercitar nada. Medido:
 * dois dos cinco falhavam por isso.
 */
vi.mock("axios", () => ({
  default: { get: vi.fn() },
}));
const axios = (await import("axios")).default as unknown as {
  get: ReturnType<typeof vi.fn>;
};

const CATALOGO = {
  artigos: [
    {
      id: "publicado",
      titulo: "Gasto tributário no ICMS",
      autores: "A. Silva",
      instituicao: "IPEA",
      ano: 2025,
      resumo: "Renúncia fiscal por unidade federada.",
      link: "https://exemplo.org/a",
      placeholder: false,
    },
    {
      id: "sem-campo",
      titulo: "Reforma e atendimento",
      autores: "SEFAZ-RS",
      instituicao: "SEFAZ-RS",
      ano: 2026,
      resumo: "Transição IBS/CBS no balcão.",
      link: "https://exemplo.org/b",
      // `placeholder` AUSENTE de propósito — é o fail-safe da D-TRIB-7.
    },
  ],
  instituicoes: [],
  dados: [],
  analises: [],
};

beforeEach(() => {
  axios.get.mockReset();
});
afterEach(() => {
  vi.clearAllMocks();
});

/** Monta a lista com cache do SWR exclusivo deste teste (ver o cabeçalho). */
const montar = () =>
  renderWithProvider(
    <SWRConfig value={{ provider: () => new Map(), dedupingInterval: 0 }}>
      <ArtigosLista />
    </SWRConfig>,
  );

/** Espera o catálogo resolver: até lá o `AsyncContent` mostra o spinner. */
const aguardarCarga = () =>
  waitFor(() => screen.getByText("Gasto tributário no ICMS"));

/**
 * Catálogo do teste da D-TRIB-8/10: os quatro estados que decidem o botão "Ler PDF". Só o
 * primeiro autoriza — os outros três são cada uma das condições faltando sozinha.
 */
const CATALOGO_PDF = {
  artigos: [
    {
      ...CATALOGO.artigos[0],
      id: "nosso",
      titulo: "Nosso estudo",
      pdf: "/tributum/a.pdf",
      hospedado: true,
      autorizacao: "Autoria própria da curadoria (2026-08-21).",
    },
    {
      ...CATALOGO.artigos[0],
      id: "sem-procedencia",
      titulo: "Sem procedência",
      pdf: "/tributum/c.pdf",
      hospedado: true,
    },
    {
      ...CATALOGO.artigos[0],
      id: "terceiro",
      titulo: "Estudo de terceiro",
      pdf: "/tributum/b.pdf",
      hospedado: false,
    },
    {
      ...CATALOGO.artigos[0],
      id: "sem-arquivo",
      titulo: "Sem arquivo",
      hospedado: true,
    },
  ],
  instituicoes: [],
  dados: [],
  analises: [],
};

describe("TributumShell", () => {
  it("lista os itens e conta no cabeçalho", async () => {
    axios.get.mockResolvedValue({ data: CATALOGO });
    montar();
    await aguardarCarga();

    expect(screen.getByText("Reforma e atendimento")).toBeInTheDocument();
    expect(screen.getByText("2 artigos")).toBeInTheDocument();
  });

  /**
   * D-TRIB-7 chegando à tela: um selo, não dois nem zero. O item com `placeholder: false` não pode
   * ganhar selo, e o SEM O CAMPO precisa ganhar — é o caso que o default ingênuo erraria.
   */
  it("marca com 'exemplo' só quem não se declarou publicado", async () => {
    axios.get.mockResolvedValue({ data: CATALOGO });
    montar();
    await aguardarCarga();

    expect(screen.getAllByText("exemplo")).toHaveLength(1);
  });

  /** A busca é a util da casa: sem acento, multi-termo, casando em campos diferentes do item. */
  it("filtra por termo, ignorando acento e cruzando campos", async () => {
    axios.get.mockResolvedValue({ data: CATALOGO });
    montar();
    await aguardarCarga();

    fireEvent.change(screen.getByPlaceholderText(/Pesquisar em artigos/i), {
      target: { value: "renuncia federada" },
    });

    await waitFor(() =>
      expect(screen.getByText("1 artigo de 2")).toBeInTheDocument(),
    );
    expect(screen.queryByText("Reforma e atendimento")).not.toBeInTheDocument();
  });

  /**
   * O catálogo é mantido à mão, e a falha realista é uma seção faltando — não o arquivo inteiro
   * quebrado. Uma seção ausente não pode derrubar a aba: ela mostra o vazio e segue de pé.
   */
  it("degrada para o vazio quando a seção não vem no JSON", async () => {
    axios.get.mockResolvedValue({ data: { instituicoes: [] } });
    montar();

    await waitFor(() =>
      expect(
        screen.getByText(/Nada em artigos e estudos por enquanto/i),
      ).toBeInTheDocument(),
    );
  });

  /**
   * Nunca sumir em silêncio (guarda da TAREFA): catálogo que não carrega vira mensagem, e a
   * mensagem é a do `AsyncContent`, a mesma das outras abas.
   */
  it("mostra erro quando o catálogo não carrega", async () => {
    axios.get.mockRejectedValue(new Error("HTTP 404"));
    montar();

    await waitFor(() =>
      expect(
        screen.getByText(/Erro ao carregar o Tributum/i),
      ).toBeInTheDocument(),
    );
  });
});

describe("D-TRIB-8 na UI: o botão 'Ler PDF'", () => {
  /**
   * A guarda de direitos. Os DOIS campos são necessários, e o caso perigoso é o do meio: um `pdf`
   * copiado por engano para um item de terceiro não pode abrir o visualizador do nosso domínio.
   *
   * Verificação por mutação: trocar `temPdfHospedado(a)` por `a.pdf` na `ArtigosLista` faz este
   * teste achar DOIS botões e morrer.
   */
  it("aparece só com `hospedado: true` E `pdf` — nunca com um dos dois", async () => {
    axios.get.mockResolvedValue({ data: CATALOGO_PDF });
    montar();
    await waitFor(() => screen.getByText("Nosso estudo"));

    expect(
      screen.getByLabelText('Ler o PDF de "Nosso estudo"'),
    ).toBeInTheDocument();
    expect(
      screen.queryByLabelText('Ler o PDF de "Estudo de terceiro"'),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByLabelText('Ler o PDF de "Sem arquivo"'),
    ).not.toBeInTheDocument();
    // D-TRIB-10: hospedado e com arquivo, mas sem a procedência registrada — degrada para link.
    expect(
      screen.queryByLabelText('Ler o PDF de "Sem procedência"'),
    ).not.toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: /Ler o PDF/ })).toHaveLength(
      1,
    );
  });

  /**
   * O visualizador em si (D-TRIB-6): iframe nativo, e o "Abrir em nova aba" SEMPRE presente — é o
   * caminho do Android, onde o Chrome não renderiza PDF embutido e o iframe fica branco sem que a
   * página tenha como saber. Ele não é fallback de erro; é saída de primeira classe.
   */
  it("abre o modal com iframe e com a saída para nova aba", async () => {
    axios.get.mockResolvedValue({ data: CATALOGO_PDF });
    const { container } = montar();
    await waitFor(() => screen.getByText("Nosso estudo"));

    fireEvent.click(screen.getByLabelText('Ler o PDF de "Nosso estudo"'));

    const dialogo = screen.getByRole("dialog");
    expect(dialogo).toHaveAttribute("aria-label", "Nosso estudo");
    const iframe = container.querySelector("iframe");
    expect(iframe).toHaveAttribute("src", "/tributum/a.pdf");
    // Por ROLE, não por texto: a frase "Abrir em nova aba" aparece duas vezes no modal — no link
    // e, entre aspas, na nota de rodapé que ensina a usá-lo. `getByText` não distingue os dois.
    expect(screen.getByRole("link", { name: /Abrir em nova aba/ })).toHaveAttribute(
      "href",
      "/tributum/a.pdf",
    );

    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
