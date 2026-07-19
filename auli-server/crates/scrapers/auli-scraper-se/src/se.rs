//! Coleta dos serviços da SEFAZ-SE a partir da Carta de Serviços (`servicos_cidadao.aspx`).
//!
//! O portal é SharePoint 2013, mas a Carta é uma **única página HTML** (`servicos_cidadao.aspx`,
//! ~890 KB) com **91 serviços** num Bootstrap accordion. Cada serviço é um painel: o **título** no
//! heading (`<a href="#{id}">▾ Título</a>`, o `id` é a âncora/identidade) e o **corpo rico** no
//! `panel-body` (campos `<p><strong>Rótulo:</strong> valor</p>`: Descrição, Legislação, Área
//! responsável, Requisitos, Onde solicitar, Forma de prestação, Canais…).
//!
//! Modelagem (molde AC/PB, 1 GET, sem detalhe por serviço): público único "Serviços"; `classe` = o
//! tema do accordion que contém o painel (`accordion_<tema>`; standalone antes do 1º tema →
//! "Serviços Gerais"); `link` = `servicos_cidadao.aspx#{id}`.
//!
//! **Gotcha de rede:** o `ureq` falha com `unexpected end of file` ao baixar a página (~890 KB) — o
//! servidor SharePoint encerra a conexão de um jeito que o rustls rejeita mas o `curl` tolera. Coleta
//! via `kit::http::get_via_curl` (subprocess curl; requer curl no PATH), como o GO/DF.

use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use regex::Regex;
use scraper::Html;

const CARTA_URL: &str = "https://www.sefaz.se.gov.br/SitePages/servicos_cidadao.aspx";
const ORGAO: &str = "SEFAZ-SE";
const PUBLICO_NOME: &str = "Serviços";
const PUBLICO_SLUG: &str = "servicos-gerais";
const CLASSE_GERAL: &str = "Serviços Gerais";
/// Cap do corpo (1 painel do ITCMD tem ~22 KB de formulário; o resto tem mediana ~900).
const MAX_DESC: usize = 2500;
/// Guard: piso de serviços (a Carta tem 91).
const MIN_SERVICOS: usize = 70;

/// Raspa a Carta e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let (html, da_rede) = load(data_dir, use_cache)?;
    let items = parse(&html);
    validar(&items)?;
    if da_rede {
        // Cache só depois de o parse+guard passarem (D-RJ5).
        auli_scraper_kit::cache::write(data_dir, "servicos", CARTA_URL, &html);
    }

    let classes: HashSet<&str> =
        items.iter().flat_map(|s| s.ocorrencias.iter()).map(|o| o.classe.as_str()).collect();
    println!("SE: {} serviços em {} classe(s)", items.len(), classes.len());
    let publicos_ordem =
        vec![Publico { nome: PUBLICO_NOME.to_string(), slug: PUBLICO_SLUG.to_string() }];
    Ok((items, publicos_ordem))
}

/// GET (HTML) com cache. Devolve `(corpo, veio_da_rede)`. Miss + `--usecache` = erro. O caller grava
/// o cache só depois do guard (D-RJ5).
fn load(data_dir: &str, use_cache: bool) -> Result<(String, bool), Box<dyn std::error::Error>> {
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, "servicos", CARTA_URL) {
        return Ok((cached, false));
    }
    if use_cache {
        return Err(format!(
            "cache vazio para {} (--usecache, sem rede). Rode uma coleta com rede primeiro.",
            CARTA_URL
        )
        .into());
    }
    let body = auli_scraper_kit::http::get_via_curl(
        CARTA_URL,
        &GetOpts { log_prefix: "SE", ..Default::default() },
    )?;
    if !body.contains("panel-collapse") {
        return Err(format!("HTML inesperado de {} (accordion sumiu / markup mudou?)", CARTA_URL).into());
    }
    Ok((body, true))
}

/// Parseia o accordion: um `ServicoRaw` por painel (`panel-collapse` + `panel-body`).
fn parse(html: &str) -> Vec<ServicoRaw> {
    let heads = headings(html);
    let temas = temas(html);

    let panel_re = Regex::new(
        r#"(?s)panel-collapse collapse"[^>]*id="([a-z0-9_]+)"[^>]*>\s*<div class="panel-body"[^>]*>(.*?)</div>\s*</div>"#,
    )
    .unwrap();

    let mut items: Vec<ServicoRaw> = Vec::new();
    let mut vistos: HashSet<String> = HashSet::new();
    for cap in panel_re.captures_iter(html) {
        let pos = cap.get(0).unwrap().start();
        let id = &cap[1];
        let titulo = heads.get(id).cloned().unwrap_or_default();
        if titulo.is_empty() {
            continue;
        }
        let link = format!("{}#{}", CARTA_URL, id);
        if !vistos.insert(link.clone()) {
            continue;
        }
        // Corta o corpo no próximo `panel-heading` (evita vazar para o painel seguinte quando o
        // corpo tem <div> aninhado) e limita o tamanho.
        let mut body = &cap[2];
        if let Some(i) = body.find("panel-heading") {
            body = &body[..i];
        }
        let descricao = cap_len(html_to_text(body), MAX_DESC);

        let classe = classe_para(pos, &temas);
        items.push(ServicoRaw {
            titulo,
            descricao,
            link,
            orgao: ORGAO.to_string(),
            ocorrencias: vec![Ocorrencia { publico: PUBLICO_NOME.to_string(), classe }],
        });
    }
    items
}

/// Mapa `id -> título` a partir dos headings `<a href="#{id}">▾ Título</a>` (1ª ocorrência).
fn headings(html: &str) -> HashMap<String, String> {
    let re = Regex::new(r##"(?s)<a[^>]*href="#([a-z0-9_]+)"[^>]*>(.*?)</a>"##).unwrap();
    let mut out: HashMap<String, String> = HashMap::new();
    for cap in re.captures_iter(html) {
        let id = cap[1].to_string();
        if out.contains_key(&id) {
            continue;
        }
        // remove o marcador "▾" do início do título.
        let titulo = html_to_text(&cap[2]);
        let titulo = titulo.trim_start_matches('▾').trim().to_string();
        if titulo.len() > 2 {
            out.insert(id, titulo);
        }
    }
    out
}

/// Posições dos temas (`id="accordion_<tema>"`), na ordem do documento.
fn temas(html: &str) -> Vec<(usize, String)> {
    Regex::new(r#"id="accordion_(\w+)""#)
        .unwrap()
        .captures_iter(html)
        .map(|c| (c.get(0).unwrap().start(), c[1].to_string()))
        .collect()
}

/// Classe do painel na posição `pos` = tema aberto mais recente antes dele; "Serviços Gerais" se
/// nenhum (serviços standalone antes do 1º tema) ou tema desconhecido.
fn classe_para(pos: usize, temas: &[(usize, String)]) -> String {
    temas
        .iter()
        .rev()
        .find(|(p, _)| *p < pos)
        .map(|(_, key)| nome_do_tema(key))
        .filter(|c| !c.is_empty())
        .unwrap_or(CLASSE_GERAL)
        .to_string()
}

/// Nome legível do tema a partir do sufixo do `accordion_<tema>`. Desconhecido -> "" (vira "Gerais").
fn nome_do_tema(key: &str) -> &'static str {
    match key {
        "dfe" => "Documentos Fiscais Eletrônicos",
        "icms" => "ICMS",
        "itcmd" => "ITCMD",
        "ipva" => "IPVA",
        "simples_nacional" => "Simples Nacional",
        "contencioso" => "Contencioso",
        "cadastro_contribuinte" => "Cadastro de Contribuinte",
        _ => "",
    }
}

/// Limita o texto a `max` caracteres, cortando na última palavra e sinalizando com " …".
fn cap_len(s: String, max: usize) -> String {
    if s.chars().count() <= max {
        return s;
    }
    let truncado: String = s.chars().take(max).collect();
    let corte = truncado.rsplit_once(' ').map(|(a, _)| a).unwrap_or(&truncado);
    format!("{} …", corte.trim_end())
}

/// HTML -> texto: strip de tags (2×, para o caso de tags entity-encodadas), entidades via html5ever, clean.
fn html_to_text(html: &str) -> String {
    let once = strip_tags(html);
    let decoded: String = Html::parse_fragment(&once).root_element().text().collect();
    clean(&strip_tags(&decoded))
}

/// Remove `<…>` (tag -> espaço).
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Guard (princípio D-RJ5): reprova coleta capada (accordion/markup mudou).
fn validar(items: &[ServicoRaw]) -> Result<()> {
    if items.len() < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). O accordion da Carta pode ter mudado; se \
             veio do cache, limpe data/se/raw/cache/ e re-raspe.",
            items.len(),
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Estrutura fiel: temas são só `<div class="panel-group" id="accordion_<tema>">` envolvendo os
    // painéis-folha (NÃO há painel container). 'plantao_fiscal' vem antes de qualquer tema.
    const CARTA: &str = r##"
      <div class="panel-group" id="accordion">
        <div class="panel panel-default">
          <div class="panel-heading"><h4><a data-toggle="collapse" href="#plantao_fiscal"><span>▾</span><strong>Plantão Fiscal</strong></a></h4></div>
          <div class="panel-collapse collapse" id="plantao_fiscal"><div class="panel-body" style="x">
            <p><strong>Descri&ccedil;&atilde;o do servi&ccedil;o:</strong> Atendimento fiscal.</p>
            <p><strong>Onde solicitar:</strong> Presencial.</p>
          </div></div>
        </div>
        <div class="panel-group" id="accordion_itcmd">
          <div class="panel panel-default">
            <div class="panel-heading"><h4><a href="#entrada_decl_itcmd"><span>▾</span><strong>Entrada de Declara&ccedil;&atilde;o de ITCMD</strong></a></h4></div>
            <div class="panel-collapse collapse" id="entrada_decl_itcmd"><div class="panel-body">
              <p><strong>&Aacute;rea respons&aacute;vel:</strong> GAITCMD.</p>
            </div></div>
          </div>
        </div>
      </div>
    "##;

    #[test]
    fn parse_extrai_servicos_titulo_classe_link() {
        let items = parse(CARTA);
        assert_eq!(items.len(), 2);
        let plantao = items.iter().find(|s| s.link.ends_with("#plantao_fiscal")).unwrap();
        assert_eq!(plantao.titulo, "Plantão Fiscal");
        assert_eq!(plantao.ocorrencias[0].classe, "Serviços Gerais"); // antes de qualquer tema
        assert!(plantao.descricao.contains("Descrição do serviço: Atendimento fiscal."));
        assert!(plantao.descricao.contains("Onde solicitar: Presencial."));

        let itcmd_svc = items.iter().find(|s| s.link.ends_with("#entrada_decl_itcmd")).unwrap();
        assert_eq!(itcmd_svc.titulo, "Entrada de Declaração de ITCMD");
        assert_eq!(itcmd_svc.ocorrencias[0].classe, "ITCMD"); // sob accordion_itcmd
        assert!(itcmd_svc.descricao.contains("Área responsável: GAITCMD."));
        assert!(!itcmd_svc.descricao.contains('<') && !itcmd_svc.descricao.contains("&aacute;"));
    }

    #[test]
    fn cap_len_corta_na_palavra() {
        let s = "um dois tres quatro cinco".to_string();
        let c = cap_len(s, 10);
        assert!(c.ends_with('…'));
        assert!(!c.contains("quatro"));
    }

    #[test]
    fn nome_do_tema_mapeia_conhecidos_e_desconhecidos() {
        assert_eq!(nome_do_tema("ipva"), "IPVA");
        assert_eq!(nome_do_tema("dfe"), "Documentos Fiscais Eletrônicos");
        assert_eq!(nome_do_tema("xpto"), "");
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
        assert!(validar(&poucos).unwrap_err().to_string().contains("capado"));
    }
}
