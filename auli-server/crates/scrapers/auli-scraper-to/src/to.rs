//! Coleta dos serviços da SEFAZ-TO a partir da "Carta de Serviços" (servicos.to.gov.br, ASP.NET).
//!
//! O portal é **ASP.NET WebForms / IIS** — HTML server-rendered, sem JSON API (molde HTML-scraping,
//! como BA/RJ). SEFAZ = órgão **`cod_empresa=37`**. Duas etapas:
//! - **Listagem** (1 GET): `listar_servico.aspx?cod_empresa=37` → os 45 serviços (cada um com um link
//!   `servico_detalhado.aspx?cod_assunto_documento_tipo={id}`). A identidade é o `cod`.
//! - **Detalhe** (1 GET por serviço): `servico_detalhado.aspx?cod={id}` — o conteúdo rico (padrão
//!   gov.br "Carta de Serviços") está em spans com id ASP.NET estável (`ctl00_…_lbl*`), mais robusto
//!   que parsear os accordions aninhados. O `scraper` (html5ever) decodifica as entidades.
//!
//! Modelagem (Cenário B, molde MT): `titulo` = `lblTxtServico`; `descricao` = Conceituação + Como
//! solicitar + Documentos + Custos + Prazo (seções não-vazias); **público** = `lblTipoRelacionamento`
//! (vocabulário fixo concatenado — Cidadão/Empresa/Órgão Público); **classe** = `lblTxtServicoGrupo`;
//! `ocorrencias` = público × classe; `link` = a própria página de detalhe.
//!
//! D-PA-ROBOTS (TO = 3º caso): UA institucional AuliBot, cortesia entre GETs, nunca autenticar.

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use scraper::{Html, Selector};

/// UA institucional do projeto (mitigação D-PA-ROBOTS): nunca UA de browser falso.
const USER_AGENT: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

/// Listagem da Carta de Serviços da SEFAZ (órgão 37).
const LISTA_URL: &str = "https://servicos.to.gov.br/listar_servico.aspx?cod_empresa=37";
/// Base da página de detalhe (também o link canônico).
const DETALHE_BASE: &str =
    "https://servicos.to.gov.br/servico_detalhado.aspx?cod_assunto_documento_tipo=";

/// Vocabulário de público (o campo `lblTipoRelacionamento` concatena valores por espaço; os compostos
/// vêm primeiro para casar "Órgão Público" como UM valor, não "Órgão"+"Público").
const PUBLICOS_VOCAB: [&str; 5] =
    ["Órgão Público", "Servidor Público", "Servidor", "Cidadão", "Empresa"];
/// Fallback quando falta público/classe (defensivo — não observado).
const GERAL: &str = "Geral";
/// Órgão de origem.
const ORGAO: &str = "SEFAZ-TO";
/// Cortesia entre GETs (gentileza com o IIS legado; D-PA-ROBOTS).
const COURTESY: Duration = Duration::from_millis(500);
/// Piso estático (a listagem anuncia ~45; guard contra coleta capada).
const MIN_SERVICOS: usize = 40;

/// Um serviço já parseado do detalhe.
struct Detail {
    nome: String,
    conceituacao: String,
    requisito: String,
    documentacao: String,
    taxa: String,
    prazo: String,
    relacionamento: String,
    grupo: String,
}

/// Raspa a Carta de Serviços da SEFAZ-TO e devolve `(items, publicos_ordem)` para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));

    // Respostas de rede que só entram no cache DEPOIS dos guards (D-RJ5).
    let mut pending: Vec<(String, String)> = Vec::new();

    // 1) Listagem -> ids (cod_assunto_documento_tipo).
    let lista_html = load(&agent, data_dir, LISTA_URL, use_cache, &mut pending)?;
    let ids = parse_ids(&lista_html);
    println!("TO: {} serviços na listagem (órgão 37)", ids.len());
    if ids.is_empty() {
        return Err(format!("listagem sem serviços — markup mudou? ({})", LISTA_URL).into());
    }

    // 2) Detalhe rico de cada serviço.
    let mut publicos_ordem: Vec<String> = Vec::new();
    let mut items: Vec<ServicoRaw> = Vec::new();
    let mut vistos: HashSet<String> = HashSet::new();
    for id in &ids {
        if !vistos.insert(id.clone()) {
            continue;
        }
        let url = format!("{}{}", DETALHE_BASE, id);
        let html = load(&agent, data_dir, &url, use_cache, &mut pending)?;
        let det = parse_detail(&html);
        if let Some(s) = build_servico(id, &det, &mut publicos_ordem) {
            items.push(s);
        }
    }

    validar(&items, ids.len())?;

    // Cache só DEPOIS dos guards.
    for (url, raw) in &pending {
        auli_scraper_kit::cache::write(data_dir, "servicos", url, raw);
    }

    let ocorr: usize = items.iter().map(|s| s.ocorrencias.len()).sum();
    println!("TO: {} serviços ({} ocorrências) em {} público(s)", items.len(), ocorr, publicos_ordem.len());
    let publicos = publicos_ordem
        .into_iter()
        .map(|nome| Publico { slug: slug_publico(&nome), nome })
        .collect();
    Ok((items, publicos))
}

/// GET com cache. Miss + `--usecache` = erro (nunca fallback). Rede -> `pending` + cortesia.
fn load(
    agent: &ureq::Agent,
    data_dir: &str,
    url: &str,
    use_cache: bool,
    pending: &mut Vec<(String, String)>,
) -> Result<String> {
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, "servicos", url) {
        return Ok(cached);
    }
    if use_cache {
        bail!("cache vazio para {} (--usecache, sem rede). Rode uma coleta com rede primeiro.", url);
    }
    let body = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts { log_prefix: "TO", ..Default::default() },
    )?;
    if !body.contains("cod_assunto_documento_tipo") && !body.contains("ContentPlaceHolder1") {
        bail!("HTML inesperado de {} (markup mudou / erro?)", url);
    }
    pending.push((url.to_string(), body.clone()));
    sleep(COURTESY);
    Ok(body)
}

/// Extrai os `cod_assunto_documento_tipo` (ordem de aparição, distintos) dos links da listagem.
fn parse_ids(html: &str) -> Vec<String> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("a[href]").unwrap();
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for a in doc.select(&sel) {
        if let Some(href) = a.value().attr("href")
            && let Some(id) = extract_cod(href)
            && seen.insert(id.clone())
        {
            out.push(id);
        }
    }
    out
}

/// Extrai os dígitos após `cod_assunto_documento_tipo=` de um href.
fn extract_cod(href: &str) -> Option<String> {
    const K: &str = "cod_assunto_documento_tipo=";
    let i = href.find(K)? + K.len();
    let digits: String = href[i..].chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() { None } else { Some(digits) }
}

/// Texto (decodificado + limpo) de um span ASP.NET por id `ctl00_ContentPlaceHolder1_<lbl>`.
fn field(doc: &Html, lbl: &str) -> String {
    let sel = Selector::parse(&format!("#ctl00_ContentPlaceHolder1_{}", lbl)).unwrap();
    doc.select(&sel)
        .next()
        .map(|e| clean(&e.text().collect::<String>()))
        .unwrap_or_default()
}

/// Parseia a página de detalhe nos campos que usamos (spans `lbl*`).
fn parse_detail(html: &str) -> Detail {
    let doc = Html::parse_document(html);
    Detail {
        nome: field(&doc, "lblTxtServico"),
        conceituacao: field(&doc, "lblTxtConceituacao"),
        requisito: field(&doc, "lblTxtRequisitoAcesso"),
        documentacao: field(&doc, "lblTxtDocumentacao"),
        taxa: field(&doc, "lblTituloTaxa"),
        prazo: field(&doc, "lblTituloPrazoExecutar"),
        relacionamento: field(&doc, "lblTipoRelacionamento"),
        grupo: field(&doc, "lblTxtServicoGrupo"),
    }
}

/// Monta um `ServicoRaw` do detalhe; registra os públicos na ordem de descoberta. `None` se sem título.
fn build_servico(id: &str, det: &Detail, publicos_ordem: &mut Vec<String>) -> Option<ServicoRaw> {
    let titulo = det.nome.clone();
    if titulo.is_empty() {
        return None;
    }
    let classe = if det.grupo.is_empty() { GERAL.to_string() } else { det.grupo.clone() };

    let mut publicos = parse_publicos(&det.relacionamento);
    if publicos.is_empty() {
        // Nenhum termo do vocabulário casou. Se o campo trouxe algo (um público NOVO fora do
        // vocabulário), preserva o texto cru (avisa, para o vocabulário ser atualizado) em vez de
        // descartá-lo em "Geral". Vazio de verdade -> "Geral".
        let raw = det.relacionamento.trim();
        if raw.is_empty() {
            publicos.push(GERAL.to_string());
        } else {
            eprintln!(
                "⚠️  TO: público fora do vocabulário (serviço {}): {:?} — usando cru. Atualize PUBLICOS_VOCAB.",
                id, raw
            );
            publicos.push(raw.to_string());
        }
    }
    let mut ocorrencias = Vec::with_capacity(publicos.len());
    for p in &publicos {
        if !publicos_ordem.contains(p) {
            publicos_ordem.push(p.clone());
        }
        ocorrencias.push(Ocorrencia { publico: p.clone(), classe: classe.clone() });
    }

    Some(ServicoRaw {
        titulo,
        descricao: montar_descricao(det),
        link: format!("{}{}", DETALHE_BASE, id),
        orgao: ORGAO.to_string(),
        ocorrencias,
    })
}

/// Descrição rica: Conceituação (o que é) + Como solicitar + Documentos + Custos + Prazo (não-vazias).
fn montar_descricao(det: &Detail) -> String {
    [&det.conceituacao, &det.requisito, &det.documentacao, &det.taxa, &det.prazo]
        .into_iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Público(s) a partir do `lblTipoRelacionamento` (vocabulário fixo, compostos primeiro). Devolve na
/// ordem canônica do vocabulário.
fn parse_publicos(raw: &str) -> Vec<String> {
    let mut s = raw.to_string();
    let mut out = Vec::new();
    for p in PUBLICOS_VOCAB {
        if s.contains(p) {
            out.push(p.to_string());
            s = s.replace(p, " ");
        }
    }
    out
}

/// Slug do arquivo per-público (ex.: `Órgão Público` -> `servicos-orgao-publico`).
fn slug_publico(nome: &str) -> String {
    format!("servicos-{}", slugify(nome))
}

/// ASCII-fold pt-BR + kebab.
fn slugify(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    for c in s.chars() {
        let m = match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
            'é' | 'ê' | 'è' | 'ë' | 'É' | 'Ê' | 'È' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' => 'i',
            'ó' | 'ô' | 'õ' | 'ò' | 'ö' | 'Ó' | 'Ô' | 'Õ' | 'Ò' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            c if c.is_ascii_alphanumeric() => c.to_ascii_lowercase(),
            _ => '-',
        };
        buf.push(m);
    }
    buf.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-")
}

/// Guard (D-RJ5): reprova coleta capada (abaixo do mínimo). Avisa se coletou menos que a listagem.
fn validar(items: &[ServicoRaw], n_listagem: usize) -> Result<()> {
    if items.len() < n_listagem {
        eprintln!(
            "ℹ️  TO: listagem tinha {} serviço(s), montados {} (alguns sem título/detalhe?).",
            n_listagem,
            items.len()
        );
    }
    if items.len() < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). Se veio do cache, limpe data/to/raw/cache/ \
             e re-raspe.",
            items.len(),
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTA: &str = r#"<html><body>
      <div><span class="card-servico-nome">Serviço A</span>
        <a href="servico_detalhado.aspx?cod_assunto_documento_tipo=8017">Ver detalhes</a></div>
      <div><span class="card-servico-nome">Serviço B</span>
        <a href="servico_detalhado.aspx?cod_assunto_documento_tipo=7805&amp;x=1">Ver detalhes</a></div>
      <a href="servico_detalhado.aspx?cod_assunto_documento_tipo=8017">dup</a>
    </body></html>"#;

    const DET: &str = r#"<html><body>
      <span id="ctl00_ContentPlaceHolder1_lblTxtServico">Acessar o Boletim  Informativo</span>
      <span id="ctl00_ContentPlaceHolder1_lblTxtConceituacao"><p>Permite voc&ecirc; acessar a lista.</p></span>
      <span id="ctl00_ContentPlaceHolder1_lblTxtRequisitoAcesso">Voc&ecirc; solicita pela internet.</span>
      <span id="ctl00_ContentPlaceHolder1_lblTxtDocumentacao">Inscri&ccedil;&atilde;o Estadual.</span>
      <span id="ctl00_ContentPlaceHolder1_lblTituloTaxa">Custos: R$ 30,00.</span>
      <span id="ctl00_ContentPlaceHolder1_lblTituloPrazoExecutar"></span>
      <span id="ctl00_ContentPlaceHolder1_lblTipoRelacionamento">Cidad&atilde;o Empresa</span>
      <span id="ctl00_ContentPlaceHolder1_lblTxtServicoGrupo">Finan&ccedil;as, Impostos e Gest&atilde;o P&uacute;blica</span>
    </body></html>"#;

    #[test]
    fn parse_ids_distintos_em_ordem() {
        let ids = parse_ids(LISTA);
        assert_eq!(ids, vec!["8017", "7805"]); // dup ignorada, ordem preservada
    }

    #[test]
    fn field_decodifica_entidades_e_limpa() {
        let doc = Html::parse_document(DET);
        assert_eq!(field(&doc, "lblTxtConceituacao"), "Permite você acessar a lista.");
        assert_eq!(field(&doc, "lblTxtServico"), "Acessar o Boletim Informativo"); // espaços comprimidos
    }

    #[test]
    fn parse_publicos_vocabulario() {
        assert_eq!(parse_publicos("Cidadão Empresa"), vec!["Cidadão", "Empresa"]);
        assert_eq!(parse_publicos("Órgão Público"), vec!["Órgão Público"]); // composto = 1 valor
        assert_eq!(parse_publicos("Empresa"), vec!["Empresa"]);
        assert!(parse_publicos("").is_empty());
    }

    #[test]
    fn build_monta_ocorrencias_classe_link() {
        let det = parse_detail(DET);
        let mut ord = Vec::new();
        let s = build_servico("8017", &det, &mut ord).unwrap();
        assert_eq!(s.titulo, "Acessar o Boletim Informativo");
        assert_eq!(s.link, "https://servicos.to.gov.br/servico_detalhado.aspx?cod_assunto_documento_tipo=8017");
        assert_eq!(s.orgao, "SEFAZ-TO");
        // Cidadão + Empresa -> 2 ocorrências, mesma classe (grupo).
        assert_eq!(s.ocorrencias.len(), 2);
        assert_eq!(s.ocorrencias[0].publico, "Cidadão");
        assert_eq!(s.ocorrencias[0].classe, "Finanças, Impostos e Gestão Pública");
        assert_eq!(ord, vec!["Cidadão", "Empresa"]);
        // descricao rica junta as seções não-vazias (prazo vazio fica de fora).
        assert!(s.descricao.starts_with("Permite você acessar a lista."));
        assert!(s.descricao.contains("Você solicita pela internet."));
        assert!(s.descricao.contains("Custos: R$ 30,00."));
        assert!(!s.descricao.contains("\n\n\n"));
    }

    #[test]
    fn detalhe_parseia_relacionamento_composto() {
        let d = r#"<span id="ctl00_ContentPlaceHolder1_lblTxtServico">X</span>
          <span id="ctl00_ContentPlaceHolder1_lblTipoRelacionamento">Órgão Público</span>
          <span id="ctl00_ContentPlaceHolder1_lblTxtServicoGrupo">G</span>"#;
        let det = parse_detail(d);
        let mut ord = Vec::new();
        let s = build_servico("1", &det, &mut ord).unwrap();
        assert_eq!(s.ocorrencias.len(), 1);
        assert_eq!(s.ocorrencias[0].publico, "Órgão Público");
    }

    #[test]
    fn publico_fora_do_vocabulario_usa_cru_e_vazio_vira_geral() {
        let base = |rel: &str| Detail {
            nome: "X".into(),
            conceituacao: "c".into(),
            requisito: String::new(),
            documentacao: String::new(),
            taxa: String::new(),
            prazo: String::new(),
            relacionamento: rel.into(),
            grupo: "G".into(),
        };
        // valor fora do vocabulário -> preserva o cru (não vira "Geral").
        let mut ord = Vec::new();
        let s = build_servico("1", &base("Produtor Rural"), &mut ord).unwrap();
        assert_eq!(s.ocorrencias.len(), 1);
        assert_eq!(s.ocorrencias[0].publico, "Produtor Rural");
        // campo vazio -> "Geral".
        let mut ord2 = Vec::new();
        let s2 = build_servico("2", &base(""), &mut ord2).unwrap();
        assert_eq!(s2.ocorrencias[0].publico, "Geral");
    }

    #[test]
    fn slug_publico_ascii_fold() {
        assert_eq!(slug_publico("Órgão Público"), "servicos-orgao-publico");
        assert_eq!(slug_publico("Cidadão"), "servicos-cidadao");
    }

    #[test]
    fn validar_reprova_capado() {
        let poucos = vec![ServicoRaw {
            titulo: "x".into(),
            descricao: String::new(),
            link: "l".into(),
            orgao: ORGAO.into(),
            ocorrencias: vec![],
        }];
        assert!(validar(&poucos, 45).unwrap_err().to_string().contains("capado"));
    }
}
