// SEFAZ-BA FAQs — o "Perguntas e Respostas" do Plantão Fiscal, que continua no portal antigo
// (`portal.sefaz.ba.gov.br/default/perguntas_respostas/`, linkado pelas páginas de ICMS, IPVA, ITD e Taxas do site novo), e
// três páginas de perguntas do site novo (`www.sefaz.ba.gov.br`, WordPress). Estudo, medições e decisões:
// docs/faqs/ESTUDO-BA-Faqs.md (D-BAFAQ-*).
//
// Plantão Fiscal (D-BAFAQ-1 A): os menus Inspetoria, Legislação e Contencioso, por uma lista curada (Finanças Públicas,
// a TEF vazia, o Simples Nacional fora do ar e a NFC-e 1.1 superada ficam fora). As páginas HTML (windows-1252) são
// diagramadas com tabelas: as tabelas viram blocos e o `kit::linhas` segmenta; a NF-e vai por seção. Os PDFs saem do
// `kit::pdf` (poppler) e a pergunta é a linha que começa pelo número seguinte. Site novo: as "Dúvidas Frequentes" do
// Balcão Virtual (acordeão em 3 abas), o Cadastro Sincronizado e o PDF da taxa de incêndio.

use std::cell::Cell;
use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;
use ureq::Agent;
use ureq::tls::{Certificate, RootCerts, TlsConfig};

use auli_contract::FaqRaw;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::html::{Opcoes, html_para_texto};
use auli_scraper_kit::http::GetOpts;
use auli_scraper_kit::linhas::{linhas, segmentar};
use auli_scraper_kit::pdf::mes_ano;

/// A pasta do Plantão Fiscal no portal antigo: o frameset, o menu e as páginas relativas.
const PASTA: &str = "https://portal.sefaz.ba.gov.br/default/perguntas_respostas/";
const PORTAL: &str = "https://portal.sefaz.ba.gov.br";
const SITE: &str = "https://www.sefaz.ba.gov.br";
/// As páginas do site novo pela API do WordPress (os ids são estáveis) e o PDF da taxa de incêndio.
const BALCAO: &str = "https://www.sefaz.ba.gov.br/wp-json/wp/v2/pages/13458";
const CADASTRO: &str = "https://www.sefaz.ba.gov.br/wp-json/wp/v2/pages/9270";
const TAXA_INCENDIO: &str = "https://www.sefaz.ba.gov.br/docs/inspetoria-eletronica/taxas/perguntas_e_respostas_nova_taxa_de_incendio.pdf";
const CORTESIA: Duration = Duration::from_secs(1);
const LIMITE: u64 = 20 * 1024 * 1024;
/// Piso: ~90 % das perguntas de 07/11/2026.
const PISO: usize = 700;
/// A migração do portal (27/08/2023) deu a quase todas as páginas o mesmo `Last-Modified`: só uma data posterior é
/// edição de verdade e vai para a trilha (D-BAFAQ-2 — hoje, o ITD).
const MIGRACAO: (u32, u32) = (2023, 8);

/// As fontes do Plantão Fiscal (D-BAFAQ-1 A): `(menu, nome, caminho)`, o caminho como o `menu.htm` dá (relativo à
/// pasta, ou da raiz do portal), na ordem do menu.
const PLANTAO: &[(&str, &str, &str)] = &[
    (
        "Inspetoria",
        "AIDF",
        "/especiais/aidf_perguntas_respostas.pdf",
    ),
    (
        "Inspetoria",
        "Baixa de Inscrição",
        "perguntas_inspetoria_baixa_insc.htm",
    ),
    (
        "Inspetoria",
        "Certidões",
        "perguntas_inspetoria_certidoes.htm",
    ),
    (
        "Inspetoria",
        "Compra Confirmada",
        "perguntas_inspetoria_compra_confirmada.html",
    ),
    ("Inspetoria", "DAE", "perguntas_inspetoria_dae.htm"),
    (
        "Inspetoria",
        "DSTDA",
        "/especiais/perguntas_respostas_DSTDA.pdf",
    ),
    ("Inspetoria", "DIC-e", "perguntas_inspetoria_dic.htm"),
    ("Inspetoria", "DPD", "perguntas_inspetoria_dpd.htm"),
    (
        "Inspetoria",
        "DTE",
        "/especiais/DTE_Perguntas_e_Respostas.pdf",
    ),
    ("Inspetoria", "GNRE", "perguntas_inspetoria_gnre.htm"),
    ("Inspetoria", "IEF", "perguntas_inspetoria_ief.htm"),
    ("Inspetoria", "IPVA", "perguntas_inspetoria_ipva.htm"),
    (
        "Inspetoria",
        "ITD",
        "/especiais/perguntas_respostas_itd.html",
    ),
    ("Inspetoria", "NF-e", "perguntas_nfe.htm"),
    (
        "Inspetoria",
        "NFC-e",
        "/especiais/_perguntas_respostas_NFCe.pdf",
    ),
    ("Inspetoria", "SEPD", "perguntas_inspetoria_pd.htm"),
    ("Inspetoria", "SENF", "perguntas_inspetoria_senf.htm"),
    (
        "Inspetoria",
        "SINTEGRA",
        "perguntas_inspetoria_sintegra.htm",
    ),
    ("Inspetoria", "TRGM", "perguntas_inspetoria_trgm.htm"),
    (
        "Legislação",
        "Alíquota",
        "/contribuinte/tributacao/duvidas/aliquota.pdf",
    ),
    (
        "Legislação",
        "Antecipação Parcial",
        "/contribuinte/tributacao/duvidas/antecipacao_parcial.pdf",
    ),
    (
        "Legislação",
        "Base de Cálculo",
        "/contribuinte/tributacao/duvidas/base_calculo.pdf",
    ),
    (
        "Legislação",
        "Cadastro",
        "/contribuinte/tributacao/duvidas/cadastro.pdf",
    ),
    (
        "Legislação",
        "Débito e Crédito Fiscal",
        "/contribuinte/tributacao/duvidas/debito_credito_fiscal.pdf",
    ),
    (
        "Legislação",
        "Diferença de Alíquota",
        "/contribuinte/tributacao/duvidas/diferenca_aliquota.pdf",
    ),
    (
        "Legislação",
        "Documentos Fiscais / Escrituração",
        "/contribuinte/tributacao/duvidas/documentos_fiscais_escrituracao.pdf",
    ),
    (
        "Legislação",
        "Isenção / Benefícios",
        "/contribuinte/tributacao/duvidas/isencao_beneficios.pdf",
    ),
    (
        "Legislação",
        "Não Incidência / Suspensão",
        "/contribuinte/tributacao/duvidas/nao_incidencia_suspensao.pdf",
    ),
    (
        "Legislação",
        "Resolução n°13/12, do Senado Federal",
        "/contribuinte/tributacao/duvidas/resolucao_n13_12_senado_federal.pdf",
    ),
    (
        "Legislação",
        "Substituição Tributária",
        "/contribuinte/tributacao/duvidas/substituicao_tributaria.pdf",
    ),
    (
        "Legislação",
        "Taxas",
        "/contribuinte/tributacao/duvidas/taxas.pdf",
    ),
    (
        "Legislação",
        "Outros Assuntos Tributários",
        "/contribuinte/tributacao/duvidas/outros_assuntos_tributarios.pdf",
    ),
    ("Contencioso", "CONSEF", "perguntas_consef.htm"),
];
/// As opções do menu que ficam fora: a TEF ("Conteúdo em Atualização"), o Simples Nacional (404) e a NFC-e 1.1 (2015,
/// superada pela 1.6 — D-BAFAQ-4). O menu "Finanças Públicas" inteiro não é lido.
const FORA: &[&str] = &[
    "perguntas_inspetoria_tef.htm",
    "perguntas_inspetoria_simples_nacional.htm",
    "/especiais/perguntas_respostas_NFCe.pdf",
];
/// Os `<select>` do `menu.htm` que a lista cobre.
const MENUS: [&str; 3] = ["inspetoria", "legislacao", "contencioso"];

/// Intermediário **Sectigo Public Server Authentication CA OV R36**: o `www.sefaz.ba.gov.br` manda o leaf sem ele (o
/// mesmo caso do `sefaz.ac.gov.br` — o PEM é o do `auli-scraper-ac`). Âncora no rustls.
const SECTIGO_R36_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIGTDCCBDSgAwIBAgIQLBo8dulD3d3/GRsxiQrtcTANBgkqhkiG9w0BAQwFADBf
MQswCQYDVQQGEwJHQjEYMBYGA1UEChMPU2VjdGlnbyBMaW1pdGVkMTYwNAYDVQQD
Ey1TZWN0aWdvIFB1YmxpYyBTZXJ2ZXIgQXV0aGVudGljYXRpb24gUm9vdCBSNDYw
HhcNMjEwMzIyMDAwMDAwWhcNMzYwMzIxMjM1OTU5WjBgMQswCQYDVQQGEwJHQjEY
MBYGA1UEChMPU2VjdGlnbyBMaW1pdGVkMTcwNQYDVQQDEy5TZWN0aWdvIFB1Ymxp
YyBTZXJ2ZXIgQXV0aGVudGljYXRpb24gQ0EgT1YgUjM2MIIBojANBgkqhkiG9w0B
AQEFAAOCAY8AMIIBigKCAYEApkMtJ3R06jo0fceI0M52B7K+TyMeGcv2BQ5AVc3j
lYt76TvHIu/nNe22W/RJXX9rWUD/2GE6GF5x0V4bsY7K3IeJ8E7+KzG/TGboySfD
u+F52jqQBbY62ofhYjMeiAbLI02+FqwHeM8uIrUtcX8b2RCxF358TB0NHVccAXZc
FYgZndZCeXxjuca7pJJ20LLUnXtgXcjAE1vY4WvbReW0W6mkeZyNGdmpTcFs5Y+s
yy6LtE5Zocji9J9NlNnReox2RWVyEXpA1ChZ4gqN+ZpVSIQ0HBorVFbBKyhdZyEX
gZgNSNtBRwxqwIzJePJhYd4ZUhO1vk+/uP3nwDk0p95q/j7naXNCSvESnrHPypaB
WRK066nKfPRPi9m9kIOhMdYfS8giFRTcdgL24Ycilj7ecAK9Trh0VbjwouJ4WH+x
bt47u68ZFCD/ac55I0DNHkCpaPruj6e9Rmr7K46wZDAYXuEAqB7tGG/jd6JAA+H2
O44CV98NRsU213f1kScIZntNAgMBAAGjggGBMIIBfTAfBgNVHSMEGDAWgBRWc1hk
lfmSGrASKgRieaFAFYghSTAdBgNVHQ4EFgQU42Z0u3BojSxdTg6mSo+bNyKcgpIw
DgYDVR0PAQH/BAQDAgGGMBIGA1UdEwEB/wQIMAYBAf8CAQAwHQYDVR0lBBYwFAYI
KwYBBQUHAwEGCCsGAQUFBwMCMBsGA1UdIAQUMBIwBgYEVR0gADAIBgZngQwBAgIw
VAYDVR0fBE0wSzBJoEegRYZDaHR0cDovL2NybC5zZWN0aWdvLmNvbS9TZWN0aWdv
UHVibGljU2VydmVyQXV0aGVudGljYXRpb25Sb290UjQ2LmNybDCBhAYIKwYBBQUH
AQEEeDB2ME8GCCsGAQUFBzAChkNodHRwOi8vY3J0LnNlY3RpZ28uY29tL1NlY3Rp
Z29QdWJsaWNTZXJ2ZXJBdXRoZW50aWNhdGlvblJvb3RSNDYucDdjMCMGCCsGAQUF
BzABhhdodHRwOi8vb2NzcC5zZWN0aWdvLmNvbTANBgkqhkiG9w0BAQwFAAOCAgEA
BZXWDHWC3cubb/e1I1kzi8lPFiK/ZUoH09ufmVOrc5ObYH/XKkWUexSPqRkwKFKr
7r8OuG+p7VNB8rifX6uopqKAgsvZtZsq7iAFw04To6vNcxeBt1Eush3cQ4b8nbQR
MQLChgEAqwhuXp9P48T4QEBSksYav7+aFjNySsLYlPzNqVM3RNwvBdvp6vgDtGwc
xlKQZVuuNVIaoYyls8swhxDeSHKpRdxRauTLZ+pl+wGvy0pnrLEJGSz9mOEmfbod
e/XopR2NGqaHJ6bIjyxPu6UtyQGI26En7UAEozACrHz06Nx2jTAY9E6NeB6XuobE
wLK025ZRmvglcURG1BrV24tGHHTgxCe8M3oGlpUSMTKQ2dkgljZVYt+gKdFtWELZ
MuRdi+X3XsrR8LFz+aLUiDRfQqhmw3RxjIyVKvvu9UPYY1nsvxYmFnUSeM+2q1z/
iPUry+xDY9MC6+IhleKT094VKdFVp7LXH42+wvU+17lRolQ2mK2N/nBLVBwaIhib
QXw4VYKwB86Bc6eS6iqsc94KEgD/U4VsjmgfhK+Xp4NM+VYzTTa3QeV3p8xOM0cw
q1p8oZFA+OBcz3FYWpDIe5j0NWKlw9hXsTyPY/HeZUV59akskSOSRSmDfe8wJDPX
58uB9/7lud0G3x0pxQAcffP0ayKavNwDTw4UfJ34cEw=
-----END CERTIFICATE-----";

/// As tabelas de layout das páginas do portal viram blocos: o `kit::linhas` trata uma tabela como uma linha só.
static TABELA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<(/?)(?:table|tbody|thead|tfoot|tr|td|th)\b[^>]*>").unwrap());
/// As seções expansíveis da NF-e: `expandcontent('01',1);">I. Conceito…</a>` → o título da `div#01.switchcontent`.
static SECAO_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"expandcontent\('(\w+)',1\);">([^<]+)</a>"#).unwrap());
const FECHAR: &str = "[x] fechar";
/// O que sobra nas respostas das páginas do portal: o "topo ˆ" (voltar ao topo) do SINTEGRA e os dois links do rodapé
/// (início e fale conosco), que só têm imagem.
static TOPO_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^topo\s*ˆ?$").unwrap());
static RODAPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^"https://portal\.sefaz\.ba\.gov\.br/default(?:/faleconosco)?\.htm"$"#).unwrap()
});
/// O subtítulo do SINTEGRA ("Registro Tipo 50 - …"), que fica no fim da resposta anterior: vira o grupo das perguntas
/// seguintes ("Quem informa esse registro?" só faz sentido com ele).
static SUBTITULO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Registro Tipo \S+\s*[-–]\s*\S.*$").unwrap());
/// O índice do SINTEGRA (o cabeçalho e um link por registro, sem descrição), que fica no fim da última resposta geral.
static INDICE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:Registro Tipo \w+|Questões Ligadas aos Tipos de Registros Magnéticos:)$")
        .unwrap()
});
/// As classes que o `<style>` da página declara em negrito (`.style1 {font-weight: bold}`): o SINTEGRA marca uma
/// pergunta assim, num `<span>`, em vez de `<strong>`.
static ESTILO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<style[^>]*>(.*?)</style>").unwrap());
/// Só o número em negrito e a pergunta logo depois, sem tag ("<strong>23. </strong>Como proceder…?", no SINTEGRA):
/// o negrito passa a cobrir a pergunta.
static NUMERO_NEGRITO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<(strong|b)>(\s*\d+\.\s*)</(?:strong|b)>([^<]*\?)").unwrap());
static CLASSE_NEGRITO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\.([\w-]+)\s*\{[^}]*font-weight\s*:\s*bold").unwrap());
static OPCAO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<option[^>]*value="([^"]*)""#).unwrap());
static SELECT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<select[^>]*(?:name|id)="(\w+)"[^>]*>(.*?)</select>"#).unwrap()
});

/// PDF: a pergunta começa na linha (recuo de até 3) com o número seguinte — e não com o número de um decreto que a
/// quebra de linha deixou no começo ("13.780/12 e Ajuste SINIEF…": dígito logo depois do separador; "21- 1-Empresa…"
/// é pergunta); a seção é um romano em caixa alta ("I - INFORMAÇÕES INICIAIS"); a linha só com número é o número da
/// página; a resposta pode abrir com "Resposta:"/"RESP.".
static NUM_PDF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^ {0,3}(\d+)\s*[.)\-–](?:\s*[^\d\s]|\s+\d)").unwrap());
static SECAO_PDF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*([IVX]+)\s*[-–.]\s+([^a-záéíóúâêôãõç]{3,80})$").unwrap());
static PAGINA_PDF_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\d+\s*$").unwrap());
static RESPOSTA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*(?:resposta|resp)\s*[.:]+\s*").unwrap());
/// O inciso ("III – …") ou a alínea ("b) …") que abre a linha: item de lista, como os marcadores.
static ITEM_PDF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[IVXL]+\s*[-–]|[a-z]\))\s").unwrap());

static BODY: LazyLock<Selector> = LazyLock::new(|| Selector::parse("body").unwrap());
static SWITCH: LazyLock<Selector> = LazyLock::new(|| Selector::parse("div.switchcontent").unwrap());
static ABAS: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".eael-tabs-nav li").unwrap());
static ACORDEAO: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".eael-adv-accordion").unwrap());
static ITEM: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".eael-accordion-list").unwrap());
static CABECALHO: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".eael-accordion-header").unwrap());
static TITULO_ITEM: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".eael-accordion-tab-title").unwrap());
static CONTEUDO_ITEM: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".eael-accordion-content").unwrap());

/// Um nó da árvore: um grupo (menu, seção, aba) ou uma página/PDF, com as perguntas dele e os filhos.
struct No {
    titulo: String,
    url: String,
    perguntas: Vec<Pergunta>,
    filhos: Vec<No>,
}

struct Pergunta {
    pergunta: String,
    resposta: String,
    url: String,
}

impl No {
    fn grupo(titulo: &str, url: &str, filhos: Vec<No>) -> No {
        No {
            titulo: titulo.to_string(),
            url: url.to_string(),
            perguntas: Vec::new(),
            filhos,
        }
    }

    fn total(&self) -> usize {
        self.perguntas.len() + self.filhos.iter().map(No::total).sum::<usize>()
    }
}

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. Tudo é conferido antes de
/// qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let rede = Rede {
        portal: crate::scrape::build_agent_native_tls(crate::UA, Duration::from_secs(60)),
        site: agente_site(),
        data_dir,
        use_cache,
        chamadas: Cell::new(0),
    };

    // 0) O menu: opção nova que a lista não conhece é aviso.
    let (menu, _) = rede.portal(&format!("{PASTA}menu.htm"))?;
    for v in opcoes_novas(&menu) {
        println!(
            "⚠️  BA FAQs: opção nova no menu do Plantão Fiscal: {v} — revise PLANTAO/FORA (faqs.rs)"
        );
    }

    // 1) O Plantão Fiscal, por menu.
    let mut menus: Vec<No> = Vec::new();
    for (menu, nome, caminho) in PLANTAO {
        let url = absoluto(caminho);
        let no = if url.ends_with(".pdf") {
            let (texto, criado) = rede.pdf(&url)?;
            do_pdf(nome, &url, &texto, criado)
        } else {
            let (html, modificado) = rede.portal(&url)?;
            da_pagina(&titulo_da_pagina(nome, modificado), &url, &html)
        }
        .with_context(|| format!("BA FAQs: {menu} | {nome} ({url})"))?;
        println!("BA FAQs: {menu} | {}: {} perguntas", no.titulo, no.total());
        match menus.iter_mut().find(|m| m.titulo == *menu) {
            Some(m) => m.filhos.push(no),
            None => menus.push(No::grupo(
                menu,
                &format!("{PASTA}perguntas_respostas.htm"),
                vec![no],
            )),
        }
    }
    let plantao = No::grupo(
        "Plantão Fiscal",
        &format!("{PASTA}perguntas_respostas.htm"),
        menus,
    );

    // 2) O site novo.
    let balcao = do_balcao(&rede.site(BALCAO)?).context("BA FAQs: Balcão Virtual")?;
    let cadastro = do_cadastro(&rede.site(CADASTRO)?).context("BA FAQs: Cadastro Sincronizado")?;
    let (texto, criado) = rede.pdf(TAXA_INCENDIO)?;
    let taxa = do_pdf("Taxa de incêndio", TAXA_INCENDIO, &texto, criado)
        .context("BA FAQs: taxa de incêndio")?;
    for no in [&balcao, &cadastro, &taxa] {
        println!("BA FAQs: site | {}: {} perguntas", no.titulo, no.total());
    }
    let site = No::grupo(
        "Site da SEFAZ-BA",
        &format!("{SITE}/"),
        vec![balcao, cadastro, taxa],
    );

    let raiz = No::grupo(
        "Perguntas frequentes",
        &format!("{PASTA}perguntas_respostas.htm"),
        vec![plantao, site],
    );
    let faqs = consolidar(&raiz)?;
    piso(faqs.len())?;
    println!("BA FAQs: {} perguntas", faqs.len());

    // Atômico pelo mesmo motivo das outras entidades: a árvore não tem outra fonte (o snapshot achatado não a
    // reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&montar_arvore(&raiz))?,
    )?;
    println!("Wrote {tree_path} (árvore de FAQ p/ o frontend)");

    auli_contract::snapshot::write_faqs(
        crate::ENTITY,
        data_dir,
        &auli_contract::ScraperInfo::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        faqs,
    )?;
    println!("🎉 Coleta de FAQs gravada no snapshot.");
    Ok(())
}

/// O nome da página com a data do `Last-Modified` quando ela é posterior à migração (D-BAFAQ-2): "ITD (mai/2025)".
fn titulo_da_pagina(nome: &str, modificado: Option<(u32, u32)>) -> String {
    match modificado.filter(|&d| d > MIGRACAO) {
        Some(d) => format!("{nome} ({})", mes_ano(d)),
        None => nome.to_string(),
    }
}

/// O caminho do menu como URL: da raiz do portal ("/especiais/…") ou relativo à pasta.
fn absoluto(caminho: &str) -> String {
    if caminho.starts_with('/') {
        format!("{PORTAL}{caminho}")
    } else {
        format!("{PASTA}{caminho}")
    }
}

/// O site novo: TLS 1.3, mas o leaf vem sem o intermediário — o R36 como âncora no rustls.
fn agente_site() -> Agent {
    let cert = Certificate::from_pem(SECTIGO_R36_PEM.as_bytes())
        .expect("PEM do Sectigo R36 inválido")
        .to_owned();
    Agent::config_builder()
        .user_agent(crate::UA)
        .timeout_global(Some(Duration::from_secs(60)))
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::new_with_certs(&[cert]))
                .build(),
        )
        .build()
        .into()
}

/// As chamadas de rede, cada uma com o cache (D-BAFAQ-5): rede sempre, gravando o cache; com `--usecache`, só o
/// cache, e miss é erro. Cortesia entre as chamadas de rede.
struct Rede<'a> {
    portal: Agent,
    site: Agent,
    data_dir: &'a str,
    use_cache: bool,
    chamadas: Cell<usize>,
}

impl Rede<'_> {
    fn cache(&self, chave: &str) -> Result<String> {
        auli_scraper_kit::cache::read(self.data_dir, "faqs", chave)
            .ok_or_else(|| anyhow!("cache miss para {chave} (modo --usecache, sem rede)"))
    }

    fn guardar(&self, chave: &str, corpo: &str) {
        auli_scraper_kit::cache::write(self.data_dir, "faqs", chave, corpo);
    }

    fn cortesia(&self) {
        if self.chamadas.get() > 0 {
            sleep(CORTESIA);
        }
        self.chamadas.set(self.chamadas.get() + 1);
    }

    fn agente(&self, url: &str) -> &Agent {
        if url.starts_with(SITE) {
            &self.site
        } else {
            &self.portal
        }
    }

    /// Uma página do portal (windows-1252) e o `(ano, mês)` do `Last-Modified`, que o cache guarda ao lado.
    fn portal(&self, url: &str) -> Result<(String, Option<(u32, u32)>)> {
        let chave_data = format!("{url}#modificado");
        if self.use_cache {
            return Ok((self.cache(url)?, data(&self.cache(&chave_data)?)));
        }
        self.cortesia();
        let (corpo, modificado) = bytes(&self.portal, url)?;
        let html = cp1252(&corpo);
        let modificado = modificado.as_deref().and_then(last_modified);
        self.guardar(url, &html);
        self.guardar(&chave_data, &texto_data(modificado));
        Ok((html, modificado))
    }

    /// Um JSON da API do WordPress do site novo.
    fn site(&self, url: &str) -> Result<String> {
        if self.use_cache {
            return self.cache(url);
        }
        self.cortesia();
        let json = auli_scraper_kit::http::get_string(
            &self.site,
            url,
            &GetOpts {
                log_prefix: "BA FAQs",
                accept: Some("application/json"),
                ..Default::default()
            },
        )?;
        self.guardar(url, &json);
        Ok(json)
    }

    /// O texto de um PDF e o `(ano, mês)` da criação. O cache guarda os dois já extraídos: o `--usecache` não precisa
    /// do poppler.
    fn pdf(&self, url: &str) -> Result<(String, Option<(u32, u32)>)> {
        let (chave_texto, chave_data) = (format!("{url}#pdftotext"), format!("{url}#criado"));
        if self.use_cache {
            return Ok((self.cache(&chave_texto)?, data(&self.cache(&chave_data)?)));
        }
        self.cortesia();
        let pdf = auli_scraper_kit::pdf::extrair(&bytes(self.agente(url), url)?.0)
            .with_context(|| format!("PDF {url}"))?;
        self.guardar(&chave_texto, &pdf.texto);
        self.guardar(&chave_data, &texto_data(pdf.criado));
        Ok((pdf.texto, pdf.criado))
    }
}

/// "2020-04" → `(2020, 4)`; "-" (sem data) → `None`.
fn data(s: &str) -> Option<(u32, u32)> {
    let (a, m) = s.trim().split_once('-')?;
    Some((a.parse().ok()?, m.parse().ok()?))
}

/// O inverso de [`data`], para o cache.
fn texto_data(d: Option<(u32, u32)>) -> String {
    d.map(|(a, m)| format!("{a:04}-{m:02}"))
        .unwrap_or_else(|| "-".into())
}

/// O `(ano, mês)` do cabeçalho `Last-Modified` ("Wed, 21 May 2025 13:02:11 GMT").
fn last_modified(v: &str) -> Option<(u32, u32)> {
    const MESES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let partes: Vec<&str> = v.split_whitespace().collect();
    let mes = MESES.iter().position(|m| Some(m) == partes.get(2))? as u32 + 1;
    Some((partes.get(3)?.parse().ok()?, mes))
}

/// GET em bytes, com 3 tentativas, e o `Last-Modified` da resposta.
fn bytes(agent: &Agent, url: &str) -> Result<(Vec<u8>, Option<String>)> {
    println!("Fetching: {url}");
    let mut espera = Duration::from_millis(800);
    let mut ultimo = anyhow!("sem tentativa");
    for tentativa in 1..=3 {
        match agent.get(url).call() {
            Ok(mut r) => match r.body_mut().with_config().limit(LIMITE).read_to_vec() {
                Ok(b) if !b.is_empty() => {
                    let modificado = r
                        .headers()
                        .get("last-modified")
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_string);
                    return Ok((b, modificado));
                }
                Ok(_) => ultimo = anyhow!("resposta vazia"),
                Err(e) => ultimo = anyhow!(e.to_string()),
            },
            Err(e) => ultimo = anyhow!(e.to_string()),
        }
        if tentativa < 3 {
            eprintln!("⚠️  BA FAQs: tentativa {tentativa} falhou em {url} ({ultimo}); retentando…");
            sleep(espera);
            espera *= 2;
        }
    }
    Err(ultimo.context(format!("falha ao buscar {url}")))
}

/// As páginas do portal declaram iso-8859-1 mas trazem os caracteres do windows-1252 ("–" é 0x96, as aspas curvas
/// 0x93/0x94): a faixa 0x80–0x9F vai pela tabela do windows-1252, o resto é latin1. UTF-8 válido passa direto.
fn cp1252(bytes: &[u8]) -> String {
    const C1: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes
            .iter()
            .map(|&b| match b {
                0x80..=0x9f => C1[(b - 0x80) as usize],
                _ => b as char,
            })
            .collect(),
    }
}

/// As opções dos menus cobertos que nem a lista nem a exclusão conhecem.
fn opcoes_novas(menu: &str) -> Vec<String> {
    let conhecidas: HashSet<&str> = PLANTAO
        .iter()
        .map(|(_, _, c)| *c)
        .chain(FORA.iter().copied())
        .collect();
    SELECT_RE
        .captures_iter(menu)
        .filter(|c| MENUS.contains(&&c[1]))
        .flat_map(|c| {
            OPCAO_RE
                .captures_iter(&c[2])
                .map(|o| o[1].to_string())
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.starts_with("branco") && !conhecidas.contains(v.as_str()))
        .collect()
}

/// Uma página HTML do portal: o negrito por classe e o do número sozinho viram negrito da pergunta, as tabelas viram
/// blocos e o `kit::linhas` segmenta; com seções expansíveis (a NF-e), uma
/// por seção, com o título dela.
fn da_pagina(nome: &str, url: &str, html: &str) -> Result<No> {
    let html = negrito_por_classe(html);
    let html = NUMERO_NEGRITO_RE.replace_all(&html, "<${1}>${2}${3}</${1}>");
    let html = TABELA_RE.replace_all(&html, "<${1}div>");
    let doc = Html::parse_document(&html);
    let secoes: Vec<ElementRef> = doc.select(&SWITCH).collect();
    let mut no = No::grupo(nome, url, Vec::new());
    if secoes.is_empty() {
        let body = doc
            .select(&BODY)
            .next()
            .ok_or_else(|| anyhow!("página sem <body>"))?;
        distribuir(&mut no, do_html(body, url)?);
    } else {
        for s in secoes {
            let id = s.value().attr("id").unwrap_or("");
            let titulo = SECAO_LINK_RE
                .captures_iter(&html)
                .find(|c| &c[1] == id && c[2].trim() != FECHAR)
                .map(|c| {
                    html_para_texto(
                        &c[2],
                        Opcoes {
                            base: None,
                            quebras_do_fonte: false,
                        },
                    )
                })
                .ok_or_else(|| anyhow!("a seção {id} não tem título"))?;
            let mut secao = No::grupo(&titulo, url, Vec::new());
            distribuir(&mut secao, do_html(s, url)?);
            no.filhos.push(secao);
        }
    }
    if no.total() == 0 {
        bail!("a página não tem pergunta");
    }
    Ok(no)
}

/// Os `<span>` de classe que a página declara em negrito viram `<strong>`, que é o que o `kit::linhas` lê.
fn negrito_por_classe(html: &str) -> String {
    let mut out = html.to_string();
    let estilos: Vec<String> = ESTILO_RE
        .captures_iter(html)
        .map(|c| c[1].to_string())
        .collect();
    for e in estilos {
        for c in CLASSE_NEGRITO_RE.captures_iter(&e) {
            let span = Regex::new(&format!(
                r#"(?is)<span class="{}">(.*?)</span>"#,
                regex::escape(&c[1])
            ))
            .expect("a classe vem escapada");
            out = span.replace_all(&out, "<strong>$1</strong>").into_owned();
        }
    }
    out
}

/// As perguntas no nó: as sem grupo nele, as de cada grupo (consecutivas) num filho com o título do grupo.
fn distribuir(no: &mut No, blocos: Vec<(Option<String>, Pergunta)>) {
    for (grupo, p) in blocos {
        match grupo {
            None => no.perguntas.push(p),
            Some(g) => match no.filhos.last_mut() {
                Some(f) if f.titulo == g => f.perguntas.push(p),
                _ => {
                    let mut f = No::grupo(&g, &p.url, Vec::new());
                    f.perguntas.push(p);
                    no.filhos.push(f);
                }
            },
        }
    }
}

/// As perguntas de um elemento pelo `kit::linhas`, cada uma com o grupo (o subtítulo do SINTEGRA que a antecede). Das
/// respostas saem o "[x] fechar" das seções, o "topo ˆ", o índice do SINTEGRA e os links do rodapé.
fn do_html(el: ElementRef, url: &str) -> Result<Vec<(Option<String>, Pergunta)>> {
    let mut out = Vec::new();
    let mut grupo: Option<String> = None;
    for b in segmentar(&linhas(el)) {
        let texto = html_para_texto(
            &b.resposta_html,
            Opcoes {
                base: Some(PORTAL),
                quebras_do_fonte: false,
            },
        );
        let mut linhas: Vec<&str> = texto
            .lines()
            .filter(|l| {
                let t = l.trim();
                t != FECHAR
                    && !TOPO_RE.is_match(t)
                    && !RODAPE_RE.is_match(t)
                    && !INDICE_RE.is_match(t)
            })
            .collect();
        let mut proximo = None;
        while let Some(l) = linhas.last() {
            if !SUBTITULO_RE.is_match(l.trim()) {
                break;
            }
            proximo.get_or_insert_with(|| l.trim().to_string());
            linhas.pop();
        }
        let resposta = linhas.join("\n");
        if resposta.is_empty() {
            bail!("a pergunta {:?} ficou sem resposta", b.titulo);
        }
        out.push((
            grupo.clone(),
            Pergunta {
                pergunta: b.titulo,
                resposta,
                url: url.to_string(),
            },
        ));
        if proximo.is_some() {
            grupo = proximo;
        }
    }
    Ok(out)
}

/// Um PDF: a pergunta é a linha que começa pelo número seguinte (um salto é aviso), até o "?", a linha vazia ou o
/// "Resposta:"; a resposta vai até a próxima pergunta. Romano em caixa alta abre seção. O título leva a data (D-BAFAQ-2)
/// e o link, a página.
fn do_pdf(nome: &str, url: &str, texto: &str, criado: Option<(u32, u32)>) -> Result<No> {
    let titulo = match criado {
        Some(d) => format!("{nome} ({})", mes_ano(d)),
        None => nome.to_string(),
    };
    let mut no = No::grupo(&titulo, url, Vec::new());
    // A pergunta aberta: o número, as linhas dela, as da resposta, a página e se já está na resposta.
    struct Aberta<'a> {
        pergunta: Vec<&'a str>,
        resposta: Vec<&'a str>,
        pagina: usize,
        na_resposta: bool,
    }
    let mut aberta: Option<Aberta> = None;
    let mut esperado = 1;
    let fechar = |a: Aberta, no: &mut No| -> Result<()> {
        let pergunta = a
            .pergunta
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let resposta = paragrafos(&a.resposta);
        if resposta.is_empty() {
            bail!("a pergunta {pergunta:?} ficou sem resposta");
        }
        let p = Pergunta {
            pergunta,
            resposta,
            url: format!("{url}#page={}", a.pagina),
        };
        match no.filhos.last_mut() {
            Some(secao) => secao.perguntas.push(p),
            None => no.perguntas.push(p),
        }
        Ok(())
    };
    // As linhas com a página de cada uma, sem os números de página.
    let todas: Vec<(usize, &str)> = texto
        .split('\u{c}')
        .enumerate()
        .flat_map(|(i, p)| p.lines().map(move |l| (i + 1, l)))
        .filter(|(_, l)| !PAGINA_PDF_RE.is_match(l))
        .collect();
    for (k, &(pagina, l)) in todas.iter().enumerate() {
        // Romano em caixa alta abre seção só se a próxima linha com texto for a pergunta esperada — "I - CARGA DE
        // INCÊNDIO…" é item de resposta.
        if let Some(c) = SECAO_PDF_RE.captures(l)
            && todas[k + 1..]
                .iter()
                .find(|(_, l)| !l.trim().is_empty())
                .and_then(|(_, l)| NUM_PDF_RE.captures(l))
                .is_some_and(|c| c[1].parse() == Ok(esperado))
        {
            if let Some(a) = aberta.take() {
                fechar(a, &mut no)?;
            }
            let secao = format!("{} - {}", &c[1], c[2].trim());
            no.filhos.push(No::grupo(&secao, url, Vec::new()));
            continue;
        }
        if let Some(c) = NUM_PDF_RE.captures(l) {
            let n: usize = c[1].parse().unwrap_or(0);
            if n == esperado || (n == esperado + 1 && aberta.is_some()) {
                if n != esperado {
                    println!("BA FAQs: aviso — {nome}: a numeração pula a {esperado}");
                }
                if let Some(a) = aberta.take() {
                    fechar(a, &mut no)?;
                }
                esperado = n + 1;
                let na_resposta = l.trim_end().ends_with('?');
                aberta = Some(Aberta {
                    pergunta: vec![l.trim()],
                    resposta: Vec::new(),
                    pagina,
                    na_resposta,
                });
                continue;
            }
        }
        let Some(a) = aberta.as_mut() else { continue };
        if a.na_resposta {
            a.resposta.push(l);
        } else if l.trim().is_empty() || RESPOSTA_RE.is_match(l) {
            a.na_resposta = true;
            a.resposta.push(l);
        } else {
            a.pergunta.push(l.trim());
            a.na_resposta = l.trim_end().ends_with('?');
        }
    }
    if let Some(a) = aberta.take() {
        fechar(a, &mut no)?;
    }
    if no.total() == 0 {
        bail!("o PDF não tem pergunta");
    }
    Ok(no)
}

/// As linhas da resposta em parágrafos, pelo recuo relativo do `-layout`: a linha mais recuada que a margem da
/// resposta (ou com marcador, inciso ou alínea) abre parágrafo; a que está na margem continua o anterior; linha vazia fecha. O
/// "Resposta:"/"RESP." que abre a resposta e a linha só de pontuação saem.
fn paragrafos(linhas: &[&str]) -> String {
    let recuo = |l: &str| l.len() - l.trim_start().len();
    let margem = linhas
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| recuo(l))
        .min()
        .unwrap_or(0);
    let mut out: Vec<String> = Vec::new();
    let mut aberto = false;
    for l in linhas {
        let t = l.trim();
        if t.is_empty() {
            aberto = false;
            continue;
        }
        let t = if out.is_empty() {
            RESPOSTA_RE.replace(t, "").into_owned()
        } else {
            t.to_string()
        };
        // A linha só de pontuação (um "." solto) sai.
        if !t.chars().any(char::is_alphanumeric) {
            continue;
        }
        if !aberto
            || recuo(l) > margem
            || t.starts_with(['▪', '•', '●', '-', '–'])
            || ITEM_PDF_RE.is_match(&t)
        {
            out.push(t);
        } else if let Some(u) = out.last_mut() {
            u.push(' ');
            u.push_str(&t);
        }
        aberto = true;
    }
    out.iter()
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Deserialize)]
struct PaginaWp {
    link: String,
    content: Rendered,
}

#[derive(Deserialize)]
struct Rendered {
    rendered: String,
}

/// As "Dúvidas Frequentes" do Balcão Virtual: um acordeão por aba (Balcão Virtual, ITD, Plantão Fiscal), as mesmas
/// perguntas em cada um com a resposta do canal. Cada pergunta tem âncora própria.
fn do_balcao(json: &str) -> Result<No> {
    let p: PaginaWp = serde_json::from_str(json).context("JSON da página do Balcão Virtual")?;
    let doc = Html::parse_fragment(&p.content.rendered);
    let abas: Vec<String> = doc.select(&ABAS).map(|a| texto(&a)).collect();
    let acordeoes: Vec<ElementRef> = doc.select(&ACORDEAO).collect();
    if abas.len() != acordeoes.len() || abas.is_empty() {
        bail!(
            "{} abas para {} acordeões — o layout mudou?",
            abas.len(),
            acordeoes.len()
        );
    }
    let mut no = No::grupo(
        "Balcão Virtual",
        &format!("{}#duvidasfrequentes", p.link),
        Vec::new(),
    );
    for (aba, acordeao) in abas.iter().zip(acordeoes) {
        let mut grupo = No::grupo(aba, &no.url, Vec::new());
        for item in acordeao.select(&ITEM) {
            let cab = item
                .select(&CABECALHO)
                .next()
                .ok_or_else(|| anyhow!("item sem cabeçalho na aba {aba}"))?;
            let pergunta = cab
                .select(&TITULO_ITEM)
                .next()
                .map(|t| texto(&t))
                .unwrap_or_default();
            let resposta = item
                .select(&CONTEUDO_ITEM)
                .next()
                .map(|c| {
                    html_para_texto(
                        &c.inner_html(),
                        Opcoes {
                            base: Some(SITE),
                            quebras_do_fonte: false,
                        },
                    )
                })
                .unwrap_or_default();
            if pergunta.is_empty() || resposta.is_empty() {
                bail!("pergunta ou resposta vazia na aba {aba} ({pergunta:?})");
            }
            // A âncora é o conteúdo do item (`aria-controls`): o `id` do cabeçalho se repete entre as abas.
            let url = match cab.value().attr("aria-controls") {
                Some(id) if !id.is_empty() => format!("{}#{id}", p.link),
                _ => p.link.clone(),
            };
            grupo.perguntas.push(Pergunta {
                pergunta,
                resposta,
                url,
            });
        }
        no.filhos.push(grupo);
    }
    if no.total() == 0 {
        bail!("o Balcão Virtual não tem pergunta");
    }
    Ok(no)
}

/// O Cadastro Sincronizado: perguntas em negrito no corpo da página (`kit::linhas`).
fn do_cadastro(json: &str) -> Result<No> {
    let p: PaginaWp =
        serde_json::from_str(json).context("JSON da página do Cadastro Sincronizado")?;
    let doc = Html::parse_fragment(&p.content.rendered);
    let mut no = No::grupo("Cadastro Sincronizado", &p.link, Vec::new());
    for b in segmentar(&linhas(doc.root_element())) {
        let resposta = html_para_texto(
            &b.resposta_html,
            Opcoes {
                base: Some(SITE),
                quebras_do_fonte: false,
            },
        );
        if resposta.is_empty() {
            bail!("a pergunta {:?} ficou sem resposta", b.titulo);
        }
        no.perguntas.push(Pergunta {
            pergunta: b.titulo,
            resposta,
            url: p.link.clone(),
        });
    }
    if no.total() == 0 {
        bail!("o Cadastro Sincronizado não tem pergunta");
    }
    Ok(no)
}

fn texto(el: &ElementRef) -> String {
    auli_scraper_kit::clean(&el.text().collect::<String>())
}

/// A árvore → `FaqRaw`, com a trilha (os títulos do 1º nível até a página/seção). O par (link, pergunta) repetido é
/// erro: o nome do `.md` sai dele.
fn consolidar(raiz: &No) -> Result<Vec<FaqRaw>> {
    fn andar(
        no: &No,
        trilha: &mut Vec<String>,
        vistas: &mut HashSet<(String, String)>,
        out: &mut Vec<FaqRaw>,
    ) -> Result<()> {
        trilha.push(no.titulo.clone());
        for p in &no.perguntas {
            if !vistas.insert((p.url.clone(), p.pergunta.clone())) {
                bail!("{} traz {:?} duas vezes", p.url, p.pergunta);
            }
            out.push(FaqRaw {
                pergunta: p.pergunta.clone(),
                resposta: p.resposta.clone(),
                origin: trilha.join(" | "),
                url: p.url.clone(),
            });
        }
        for f in &no.filhos {
            andar(f, trilha, vistas, out)?;
        }
        trilha.pop();
        Ok(())
    }
    let mut out = Vec::new();
    let mut vistas = HashSet::new();
    for f in &raiz.filhos {
        andar(f, &mut Vec::new(), &mut vistas, &mut out)?;
    }
    Ok(out)
}

/// Coleta capada é `Err`, não snapshot menor.
fn piso(n: usize) -> Result<()> {
    if n < PISO {
        bail!("BA FAQs: {n} perguntas (piso {PISO}) — o portal mudou?");
    }
    Ok(())
}

/// A árvore da aba de FAQs: o nó com perguntas é `Faq`; o só de filhos, `Menu`.
fn montar_arvore(no: &No) -> FaqNode {
    FaqNode {
        title: no.titulo.clone(),
        url: no.url.clone(),
        page_type: if no.perguntas.is_empty() {
            PageType::Menu
        } else {
            PageType::Faq
        },
        origin: no.titulo.clone(),
        children: no.filhos.iter().map(montar_arvore).collect(),
        faq_items: no
            .perguntas
            .iter()
            .map(|p| FaqItem {
                pergunta: p.pergunta.clone(),
                resposta: p.resposta.clone(),
                url: Some(p.url.clone()),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // As fontes reais de 07/11/2026, como o cache as guarda: o menu e a página do SINTEGRA já em UTF-8, o texto do
    // `pdftotext -layout` dos PDFs e o JSON da API do WordPress.
    const MENU: &str = include_str!("../tests/fixtures/ba-faqs-menu.htm");
    const SINTEGRA: &str = include_str!("../tests/fixtures/ba-faqs-sintegra.htm");
    const NAO_INCIDENCIA: &str = include_str!("../tests/fixtures/ba-faqs-nao-incidencia.txt");
    const NFCE: &str = include_str!("../tests/fixtures/ba-faqs-nfce.txt");
    const TAXA: &str = include_str!("../tests/fixtures/ba-faqs-taxa-incendio.txt");
    const CADASTRO_JSON: &str = include_str!("../tests/fixtures/ba-faqs-cadastro.json");

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    fn todas(no: &No) -> Vec<&Pergunta> {
        no.perguntas
            .iter()
            .chain(no.filhos.iter().flat_map(todas))
            .collect()
    }

    fn filho<'a>(no: &'a No, prefixo: &str) -> &'a No {
        no.filhos
            .iter()
            .find(|f| f.titulo.starts_with(prefixo))
            .unwrap_or_else(|| panic!("sem filho {prefixo:?}"))
    }

    #[test]
    fn o_menu_real_nao_tem_opcao_nova() {
        assert!(opcoes_novas(MENU).is_empty());
    }

    #[test]
    fn opcao_nova_dos_menus_cobertos_e_aviso() {
        let menu = r#"<select name="inspetoria"><option value="brancoazul.htm">-</option>
            <option value="perguntas_inspetoria_ipva.htm">IPVA</option><option value="perguntas_novas.htm">Nova</option>
            <option value="perguntas_inspetoria_tef.htm">TEF</option></select>
            <select name="financas"><option value="perguntas_empenho.htm">Empenho</option></select>"#;
        assert_eq!(opcoes_novas(menu), ["perguntas_novas.htm"]);
    }

    #[test]
    fn caminhos_do_menu() {
        assert_eq!(
            absoluto("/especiais/aidf_perguntas_respostas.pdf"),
            "https://portal.sefaz.ba.gov.br/especiais/aidf_perguntas_respostas.pdf"
        );
        assert_eq!(
            absoluto("perguntas_consef.htm"),
            "https://portal.sefaz.ba.gov.br/default/perguntas_respostas/perguntas_consef.htm"
        );
    }

    #[test]
    fn windows_1252_e_utf8() {
        // "–", "ç", aspas curvas: a faixa C1 pelo windows-1252, o resto latin1.
        assert_eq!(cp1252(&[0x96, b' ', 0xe7, 0x93, b'x', 0x94]), "– ç“x”");
        assert_eq!(cp1252("ação".as_bytes()), "ação");
    }

    #[test]
    fn data_do_cache_e_do_last_modified() {
        assert_eq!(data("2020-04"), Some((2020, 4)));
        assert_eq!(data("-"), None);
        assert_eq!(data(&texto_data(Some((2025, 5)))), Some((2025, 5)));
        assert_eq!(texto_data(None), "-");
        assert_eq!(
            last_modified("Wed, 21 May 2025 13:02:11 GMT"),
            Some((2025, 5))
        );
        assert_eq!(
            last_modified("Sun, 27 Aug 2023 10:00:00 GMT"),
            Some((2023, 8))
        );
        assert_eq!(last_modified("lixo"), None);
        // Só a data posterior à migração (08/2023) é edição de verdade.
        assert_eq!(titulo_da_pagina("ITD", Some((2025, 5))), "ITD (mai/2025)");
        assert_eq!(titulo_da_pagina("IPVA", Some((2023, 8))), "IPVA");
        assert_eq!(titulo_da_pagina("DAE", None), "DAE");
    }

    #[test]
    fn o_sintegra_real() {
        let url = format!("{PASTA}perguntas_inspetoria_sintegra.htm");
        let no = da_pagina("SINTEGRA", &url, SINTEGRA).unwrap();
        assert_eq!(no.total(), 72);
        // As 10 gerais na página; as outras nos subtítulos "Registro Tipo NN – …".
        assert_eq!(no.perguntas.len(), 10);
        assert_eq!(no.filhos.len(), 17);
        assert_eq!(
            no.filhos[0].titulo,
            "Registro Tipo 10 - dados do contribuinte informante do arquivo"
        );
        let ps = todas(&no);
        let titulo = |prefixo: &str| ps.iter().any(|p| p.pergunta.starts_with(prefixo));
        // A 64 é negrito por classe (`.style1`); a 23, só o número em negrito; a 49, o número sem ponto.
        assert_eq!(
            filho(&no, "Registro Tipo 76").perguntas[0].pergunta,
            "64. Quem informa esse registro ?"
        );
        assert!(titulo(
            "23. Como proceder quando o destinatário/remetente não tem CNPJ?"
        ));
        assert!(
            filho(&no, "Registro Tipo 61")
                .perguntas
                .iter()
                .any(|p| p.pergunta.starts_with("49 Deve ser gerado"))
        );
        // Os números sozinhos em negrito ("10.") são itens da resposta, não perguntas.
        assert!(
            ps.iter()
                .all(|p| p.pergunta.chars().any(char::is_alphabetic))
        );
        for p in &ps {
            assert_eq!(p.url, url);
            for sobra in [
                "topo",
                "faleconosco",
                "default.htm",
                "Questões Ligadas",
                "Registro Tipo 90 -",
            ] {
                assert!(
                    !p.resposta.contains(sobra),
                    "{sobra:?} na resposta de {:?}",
                    p.pergunta
                );
            }
        }
        assert!(ps[9].resposta.ends_with("Transmissão de Documentos - TED"));
    }

    #[test]
    fn negrito_por_classe_e_numero_sozinho_em_negrito() {
        let html = r#"<html><head><style>.style1 {font-weight: bold} .x {color: red}</style></head><body>
            <p><span class="style1">1. Um?</span></p><p>A.</p><p><strong>2. </strong>Dois?</p><p>B.</p>
            <p><span class="x">3. Três?</span></p></body></html>"#;
        let no = da_pagina("P", "u", html).unwrap();
        let titulos: Vec<&str> = no.perguntas.iter().map(|p| p.pergunta.as_str()).collect();
        assert_eq!(titulos, ["1. Um?", "2. Dois?"]);
        assert_eq!(no.perguntas[1].resposta, "B.\n3. Três?");
    }

    #[test]
    fn nfe_por_secao() {
        // As seções expansíveis da NF-e: o título vem do link (com entidades), o "[x] fechar" sai da resposta.
        let html = r#"<html><body><table><tr><td><a onClick="expandcontent('01',1);">I. Conceito</a></td></tr></table>
            <div id="01" class="switchcontent"><table><tr><td><p><b>1. O que &eacute;?</b></p><p>Um documento.</p>
            <p><b>2. H&aacute; lei? </b><b>(Atualizado em 31/12/08)</b></p><p>Sim.</p>
            <a onClick="expandcontent('01',1);">[x] fechar</a></td></tr></table></div>
            <a onClick="expandcontent('02',1);">II. Obriga&ccedil;&otilde;es acess&oacute;rias</a>
            <div id="02" class="switchcontent"><p><b>1. Quem emite?</b></p><p>O vendedor.</p>
            <a onClick="expandcontent('02',1);">[x] fechar</a></div></body></html>"#;
        let no = da_pagina("NF-e", "u", html).unwrap();
        assert!(no.perguntas.is_empty());
        let secoes: Vec<&str> = no.filhos.iter().map(|f| f.titulo.as_str()).collect();
        assert_eq!(secoes, ["I. Conceito", "II. Obrigações acessórias"]);
        let s1 = &no.filhos[0].perguntas;
        assert_eq!(s1[1].pergunta, "2. Há lei? (Atualizado em 31/12/08)");
        assert_eq!(s1[1].resposta, "Sim.");
        assert_eq!(no.filhos[1].perguntas[0].resposta, "O vendedor.");
    }

    #[test]
    fn secao_sem_titulo_pagina_sem_pergunta_e_pergunta_sem_resposta_sao_erro() {
        // Só o "[x] fechar" aponta para a seção: não é título.
        let sem_titulo = r#"<div id="09" class="switchcontent"><p><b>1. P?</b></p><p>R.</p>
            <a onClick="expandcontent('09',1);">[x] fechar</a></div>"#;
        assert!(erro(da_pagina("X", "u", sem_titulo)).contains("não tem título"));
        let vazia = "<html><body><p>Conteúdo em Atualização</p></body></html>";
        assert!(erro(da_pagina("TEF", "u", vazia)).contains("não tem pergunta"));
        let sem_resposta = "<p><b>1. Um?</b></p><p><b>2. Dois?</b></p><p>R.</p>";
        assert!(erro(da_pagina("X", "u", sem_resposta)).contains("sem resposta"));
    }

    #[test]
    fn nao_incidencia_real() {
        let url = "https://portal.sefaz.ba.gov.br/contribuinte/tributacao/duvidas/nao_incidencia_suspensao.pdf";
        let no = do_pdf(
            "Não Incidência / Suspensão",
            url,
            NAO_INCIDENCIA,
            Some((2020, 4)),
        )
        .unwrap();
        assert_eq!(no.titulo, "Não Incidência / Suspensão (abr/2020)");
        assert!(no.filhos.is_empty());
        assert_eq!(no.perguntas.len(), 13);
        // "13.780/12 e Ajuste SINIEF…" no começo da linha é o fim da resposta da 11, não a pergunta 13.
        assert!(
            no.perguntas[10]
                .resposta
                .ends_with("13.780/12 e Ajuste SINIEF n.º 02/18.")
        );
        assert_eq!(
            no.perguntas[11].pergunta,
            "12. Nas saídas para industrialização há suspensão da incidência do ICMS?"
        );
        assert_eq!(no.perguntas[11].url, format!("{url}#page=3"));
        assert!(no.perguntas[12].pergunta.starts_with("13. Há incidência"));
        assert_eq!(no.perguntas[0].url, format!("{url}#page=1"));
    }

    #[test]
    fn nfce_real() {
        let url = "https://portal.sefaz.ba.gov.br/especiais/_perguntas_respostas_NFCe.pdf";
        let no = do_pdf("NFC-e", url, NFCE, Some((2019, 5))).unwrap();
        assert_eq!(no.titulo, "NFC-e (mai/2019)");
        // Uma seção só; a numeração pula a 6 (aviso) e vai até a 45.
        assert!(no.perguntas.is_empty());
        assert_eq!(no.filhos.len(), 1);
        let s = &no.filhos[0];
        assert_eq!(s.titulo, "I - INFORMAÇÕES INICIAIS");
        assert_eq!(s.perguntas.len(), 44);
        assert!(
            s.perguntas[4]
                .pergunta
                .starts_with("5. A NFC-e pode ser usada")
        );
        assert!(s.perguntas[5].pergunta.starts_with("7. Qual é o modelo"));
        assert_eq!(s.perguntas[0].url, format!("{url}#page=2"));
        assert_eq!(s.perguntas[43].url, format!("{url}#page=12"));
        assert!(s.perguntas[43].pergunta.starts_with("45. "));
    }

    #[test]
    fn taxa_de_incendio_real() {
        let no = do_pdf("Taxa de incêndio", TAXA_INCENDIO, TAXA, Some((2014, 3))).unwrap();
        assert_eq!(no.titulo, "Taxa de incêndio (mar/2014)");
        // "I - CARGA DE INCÊNDIO…" em caixa alta é item da resposta da 7, não seção.
        assert!(no.filhos.is_empty());
        assert_eq!(no.perguntas.len(), 11);
        assert!(
            no.perguntas[0]
                .resposta
                .starts_with("CONFORME ESTABELECIDO")
        );
        let p7: Vec<&str> = no.perguntas[6].resposta.lines().collect();
        assert_eq!(p7.len(), 4);
        assert!(p7[0].ends_with("CONFORME A SEGUINTE ESCALA:"));
        assert!(p7[1].starts_with("I - CARGA DE INCÊNDIO ESPECÍFICA ATÉ 300"));
        // A pergunta em duas linhas é uma só.
        assert_eq!(
            no.perguntas[10].pergunta,
            "11. EXISTE ALGUM DESCONTO A SER UTILIZADO NO CÁLCULO DA TAXA??"
        );
        assert_eq!(no.perguntas[10].url, format!("{TAXA_INCENDIO}#page=4"));
    }

    #[test]
    fn pdf_numeracao_resposta_e_paragrafos() {
        let texto = "Título\n\n1. Primeira\n  sem interrogação\nResposta: Sim.\n   Parágrafo recuado\nna margem.\n.\n\n\
                     5. Fora da sequência?\n2- Segunda?\n  Não.\n\u{c}2\n4. Pula a 3?\n  Pula.\n";
        let no = do_pdf("X", "u.pdf", texto, None).unwrap();
        assert_eq!(no.titulo, "X");
        let ps: Vec<(&str, &str, &str)> = no
            .perguntas
            .iter()
            .map(|p| (p.pergunta.as_str(), p.resposta.as_str(), p.url.as_str()))
            .collect();
        assert_eq!(
            ps,
            [
                (
                    "1. Primeira sem interrogação",
                    "Sim.\nParágrafo recuado na margem.\n5. Fora da sequência?",
                    "u.pdf#page=1"
                ),
                ("2- Segunda?", "Não.", "u.pdf#page=1"),
                ("4. Pula a 3?", "Pula.", "u.pdf#page=2"),
            ]
        );
    }

    #[test]
    fn pdf_pergunta_em_varias_linhas() {
        // Termina no "?" de uma linha de continuação, ou na linha vazia (a DSTDA e a AIDF têm as duas).
        let texto = "1. Pergunta longa\n  que continua?\n  Resposta direta.\n2- Título sem interrogação\n\n  Explicação.\n";
        let no = do_pdf("X", "u", texto, None).unwrap();
        let ps: Vec<(&str, &str)> = no
            .perguntas
            .iter()
            .map(|p| (p.pergunta.as_str(), p.resposta.as_str()))
            .collect();
        assert_eq!(
            ps,
            [
                ("1. Pergunta longa que continua?", "Resposta direta."),
                ("2- Título sem interrogação", "Explicação."),
            ]
        );
    }

    #[test]
    fn pdf_item_recuado_com_o_numero_seguinte_e_da_resposta() {
        // A lista numerada dentro da resposta vem recuada pelo `-layout`; a pergunta fica na margem (recuo até 3).
        let texto = "1. Quais são?\n  Duas:\n      2. a segunda via\n2. E depois?\n  Fim.\n";
        let no = do_pdf("X", "u", texto, None).unwrap();
        assert_eq!(no.perguntas.len(), 2);
        assert_eq!(no.perguntas[0].resposta, "Duas:\n2. a segunda via");
        assert_eq!(no.perguntas[1].pergunta, "2. E depois?");
    }

    #[test]
    fn pdf_sem_pergunta_e_pergunta_sem_resposta_sao_erro() {
        assert!(erro(do_pdf("X", "u", "Só texto.\n", None)).contains("não tem pergunta"));
        assert!(
            erro(do_pdf("X", "u", "1. Uma?\n2. Outra?\n  R.\n", None)).contains("sem resposta")
        );
    }

    #[test]
    fn cadastro_sincronizado_real() {
        let no = do_cadastro(CADASTRO_JSON).unwrap();
        assert_eq!(no.perguntas.len(), 18);
        let link = "https://www.sefaz.ba.gov.br/inspetoria-eletronica/icms/cadastro/sobre-o-cadastro/cadastro-sincronizado/";
        assert_eq!(no.url, link);
        assert!(no.perguntas.iter().all(|p| p.url == link));
        assert!(
            no.perguntas[0]
                .pergunta
                .starts_with("No caso do Estado da Bahia")
        );
        assert!(
            no.perguntas[0]
                .resposta
                .starts_with("Para os municípios de Salvador")
        );
        assert!(
            no.perguntas[17]
                .pergunta
                .starts_with("Em caso de indeferimento pela Juceb")
        );
    }

    /// Uma aba do Balcão Virtual: o nome e os itens `(título, conteúdo, aria-controls)`.
    type Aba<'a> = (&'a str, &'a [(&'a str, &'a str, &'a str)]);

    /// A página do Balcão Virtual na API: as abas e um acordeão por aba.
    fn balcao(abas: &[Aba]) -> String {
        let nav: String = abas
            .iter()
            .map(|(a, _)| format!(r#"<li><span class="eael-tab-title">{a}</span></li>"#))
            .collect();
        let acordeoes: String = abas
            .iter()
            .map(|(_, itens)| {
                let itens: String = itens
                    .iter()
                    .map(|(t, c, id)| {
                        format!(
                            r#"<div class="eael-accordion-list"><div id="pergunta" class="eael-accordion-header" aria-controls="{id}"><span class="eael-accordion-tab-title">{t}</span></div><div id="{id}" class="eael-accordion-content">{c}</div></div>"#
                        )
                    })
                    .collect();
                format!(r#"<div class="eael-adv-accordion">{itens}</div>"#)
            })
            .collect();
        serde_json::json!({
            "link": "https://www.sefaz.ba.gov.br/balcao-virtual/",
            "content": {"rendered": format!(r#"<div class="eael-tabs-nav"><ul>{nav}</ul></div>{acordeoes}"#)}
        })
        .to_string()
    }

    #[test]
    fn balcao_virtual_por_aba() {
        // O `id` do cabeçalho se repete entre as abas; a âncora é o `aria-controls`.
        let json = balcao(&[
            (
                "Balcão Virtual",
                &[("Como agendar?", "<p>Pelo site.</p>", "c-1")],
            ),
            (
                "ITD",
                &[("Como agendar?", "<p>Pelo <b>Balcão</b>.</p>", "c-2")],
            ),
        ]);
        let no = do_balcao(&json).unwrap();
        assert_eq!(no.titulo, "Balcão Virtual");
        assert_eq!(
            no.url,
            "https://www.sefaz.ba.gov.br/balcao-virtual/#duvidasfrequentes"
        );
        let abas: Vec<&str> = no.filhos.iter().map(|f| f.titulo.as_str()).collect();
        assert_eq!(abas, ["Balcão Virtual", "ITD"]);
        let itd = &no.filhos[1].perguntas[0];
        assert_eq!(itd.pergunta, "Como agendar?");
        assert_eq!(itd.resposta, "Pelo Balcão.");
        assert_eq!(itd.url, "https://www.sefaz.ba.gov.br/balcao-virtual/#c-2");
        // A trilha separa as duas "Como agendar?"; a árvore inteira consolida sem repetida.
        let raiz = No::grupo("r", "u", vec![no]);
        assert_eq!(consolidar(&raiz).unwrap().len(), 2);
    }

    #[test]
    fn balcao_com_layout_mudado_ou_resposta_vazia_e_erro() {
        let json =
            serde_json::json!({"link": "l", "content": {"rendered": "<p>nada</p>"}}).to_string();
        assert!(erro(do_balcao(&json)).contains("0 abas"));
        let uma_aba = serde_json::json!({"link": "l", "content": {"rendered":
            r#"<div class="eael-tabs-nav"><li>ITD</li></div><div class="eael-adv-accordion"></div><div class="eael-adv-accordion"></div>"#
        }})
        .to_string();
        assert!(erro(do_balcao(&uma_aba)).contains("1 abas para 2 acordeões"));
        let vazia = balcao(&[("ITD", &[("Como?", "", "c-1")])]);
        assert!(erro(do_balcao(&vazia)).contains("vazia"));
    }

    fn pergunta(p: &str, url: &str) -> Pergunta {
        Pergunta {
            pergunta: p.into(),
            resposta: "R.".into(),
            url: url.into(),
        }
    }

    #[test]
    fn trilha_link_e_repetida() {
        let mut ipva = No::grupo("IPVA", "p", Vec::new());
        ipva.perguntas.push(pergunta("1. Quem paga?", "p"));
        let raiz = No::grupo(
            "Perguntas frequentes",
            "r",
            vec![No::grupo(
                "Plantão Fiscal",
                "r",
                vec![No::grupo("Inspetoria", "r", vec![ipva])],
            )],
        );
        let faqs = consolidar(&raiz).unwrap();
        assert_eq!(faqs[0].origin, "Plantão Fiscal | Inspetoria | IPVA");
        assert_eq!(faqs[0].url, "p");
        let mut dupla = No::grupo("X", "p", Vec::new());
        dupla.perguntas = vec![pergunta("P?", "p"), pergunta("P?", "p")];
        assert!(erro(consolidar(&No::grupo("r", "r", vec![dupla]))).contains("duas vezes"));
    }

    #[test]
    fn piso() {
        assert!(super::piso(PISO).is_ok());
        assert!(erro(super::piso(PISO - 1)).contains("piso"));
    }

    #[test]
    fn arvore_menu_e_faq() {
        let mut pagina = No::grupo("IPVA", "p", Vec::new());
        pagina.perguntas.push(pergunta("P?", "p#1"));
        let t = montar_arvore(&No::grupo("Plantão Fiscal", "r", vec![pagina]));
        assert!(matches!(t.page_type, PageType::Menu));
        assert!(matches!(t.children[0].page_type, PageType::Faq));
        assert_eq!(t.children[0].faq_items[0].url.as_deref(), Some("p#1"));
    }
}
