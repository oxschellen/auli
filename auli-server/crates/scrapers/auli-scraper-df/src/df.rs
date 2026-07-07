//! Coleta dos serviços da SEFAZ-DF a partir da Carta de Serviços (ColdFusion).
//!
//! O portal da Receita/SEEC-DF expõe a Carta em `receita.fazenda.df.gov.br/aplicacoes/CartaServicos/`
//! (app ColdFusion). Achado central: **qualquer** `listaSubCategorias.cfm?...` (independente dos
//! params) embute a **árvore inteira** do catálogo como um objeto JS — subcategorias mapeando para
//! `{'item':[{'url':'…servico.cfm?…','desc':'Título'}, …]}`. Logo **1 fetch** enumera os 472 serviços.
//! Cada `servico.cfm` traz a descrição rica num accordion (`div.panel-body`).
//!
//! Modelagem: `titulo` = `desc` da árvore; `classe` = subcategoria (chave-pai imediata); `publico` =
//! `codTipoPessoa` (Cidadão p/ 6/22, Empresa p/ 7/8); `descricao` = texto dos painéis do accordion;
//! `link` = a URL absoluta do `servico.cfm` (única por serviço).
//!
//! **⚠️ Gotcha WAF (JA3):** o host reseta a conexão do `ureq` (rustls/native-tls) mas responde 200 ao
//! `curl` — allowlist por fingerprint TLS, como o GO. Toda a coleta vai por `kit::http::get_via_curl`
//! (subprocess curl). Requer o binário `curl` no PATH.

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use regex::Regex;
use scraper::{Html, Selector};

const HOST: &str = "https://www.receita.fazenda.df.gov.br";
/// Listagem-âncora: os params não importam (a árvore vem inteira), mas precisam ser válidos.
const LISTA_URL: &str = "https://www.receita.fazenda.df.gov.br/aplicacoes/CartaServicos/listaSubCategorias.cfm?codCategoriaServico=6&codTipoPessoa=6";

const ORGAO: &str = "SEFAZ-DF";
/// Cortesia entre GETs (a listagem + 472 detalhes).
const COURTESY: Duration = Duration::from_millis(300);
/// Guard: piso de serviços (o catálogo tem ~472).
const MIN_SERVICOS: usize = 400;

const PUB_CIDADAO: &str = "Cidadão";
const PUB_EMPRESA: &str = "Empresa";

/// Um serviço da árvore (antes de buscar o corpo).
struct Row {
    link: String,
    titulo: String,
    classe: String,
    publico: &'static str,
}

/// Público a partir do `codTipoPessoa`: 7/8 = pessoa jurídica (Empresa); 6/22/demais = Cidadão.
fn publico_de(tp: &str) -> &'static str {
    match tp {
        "7" | "8" => PUB_EMPRESA,
        _ => PUB_CIDADAO,
    }
}

/// Raspa a Carta e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let mut pending: Vec<(String, String)> = Vec::new();

    // 1) A listagem-âncora -> a árvore inteira -> (link, título, classe, público).
    let lista = load(data_dir, LISTA_URL, use_cache, &mut pending)?;
    let rows = parse_arvore(&lista);
    println!("DF: {} serviços na árvore da Carta", rows.len());

    // 2) Corpo (accordion) de cada serviço.
    let mut items: Vec<ServicoRaw> = Vec::new();
    let mut vistos: HashSet<String> = HashSet::new();
    for r in &rows {
        if !vistos.insert(r.link.clone()) {
            continue;
        }
        let detalhe = load(data_dir, &r.link, use_cache, &mut pending)?;
        items.push(ServicoRaw {
            titulo: r.titulo.clone(),
            descricao: extract_descricao(&detalhe),
            link: r.link.clone(),
            orgao: ORGAO.to_string(),
            ocorrencias: vec![Ocorrencia {
                publico: r.publico.to_string(),
                classe: r.classe.clone(),
            }],
        });
    }

    validar(&items)?;

    for (url, raw) in &pending {
        auli_scraper_kit::cache::write(data_dir, url, raw);
    }

    let classes: HashSet<&str> =
        items.iter().flat_map(|s| s.ocorrencias.iter()).map(|o| o.classe.as_str()).collect();
    println!("DF: {} serviços em {} classe(s)", items.len(), classes.len());
    let publicos_ordem = vec![
        Publico { nome: PUB_CIDADAO.to_string(), slug: "servicos-ao-cidadao".to_string() },
        Publico { nome: PUB_EMPRESA.to_string(), slug: "servicos-a-empresa".to_string() },
    ];
    Ok((items, publicos_ordem))
}

/// GET (HTML) via **curl** (WAF JA3, ver header) com cache. Miss + `--usecache` = erro. Rede ->
/// `pending` + cortesia.
fn load(
    data_dir: &str,
    url: &str,
    use_cache: bool,
    pending: &mut Vec<(String, String)>,
) -> Result<String> {
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, url) {
        return Ok(cached);
    }
    if use_cache {
        bail!("cache vazio para {} (--usecache, sem rede). Rode uma coleta com rede primeiro.", url);
    }
    let body = auli_scraper_kit::http::get_via_curl(
        url,
        &GetOpts { log_prefix: "DF", ..Default::default() },
    )?;
    if body.contains("Error Occurred While Processing Request") {
        bail!("erro ColdFusion em {} (params inválidos / app mudou?)", url);
    }
    pending.push((url.to_string(), body.clone()));
    sleep(COURTESY);
    Ok(body)
}

/// Parseia a árvore JS da listagem. Cada folha `{'url':'…servico.cfm?…','desc':'Título'}` é um serviço;
/// a **classe** é a chave-pai imediata `'Subcategoria': {'item':[ … ]}` (a chave mais próxima ANTES da
/// folha, na ordem do texto — subcategorias abrem logo antes de suas folhas). Público = `codTipoPessoa`.
fn parse_arvore(html: &str) -> Vec<Row> {
    // Posições das chaves de agrupamento (temas e subcategorias); a mais próxima antes da folha é a
    // subcategoria dela.
    let key_re = Regex::new(r"'([^']{2,70}?)'\s*:\s*\{\s*'item'").unwrap();
    let keys: Vec<(usize, String)> = key_re
        .captures_iter(html)
        .map(|c| (c.get(0).unwrap().start(), html_to_text(&c[1])))
        .collect();

    let leaf_re =
        Regex::new(r"\{'url':'([^']*servico\.cfm[^']*codServico=\d+[^']*)',\s*'desc':'([^']*)'\}")
            .unwrap();
    let tp_re = Regex::new(r"codTipoPessoa=(\d+)").unwrap();

    let mut out: Vec<Row> = Vec::new();
    for cap in leaf_re.captures_iter(html) {
        let pos = cap.get(0).unwrap().start();
        let url = &cap[1];
        let titulo = html_to_text(&cap[2]);
        if titulo.len() < 3 {
            continue;
        }
        let tp = tp_re.captures(url).map(|c| c[1].to_string()).unwrap_or_default();
        let classe = keys
            .iter()
            .rev()
            .find(|(p, _)| *p < pos)
            .map(|(_, c)| c.clone())
            .unwrap_or_else(|| "Serviços".to_string());
        out.push(Row {
            link: format!("{}{}", HOST, url),
            titulo,
            classe,
            publico: publico_de(&tp),
        });
    }
    out
}

/// Corpo do serviço: concatena o texto dos painéis do accordion (`div.panel-body`) — Descrição, prazo,
/// requisitos, canais, legislação, arquivos. Remove `<style>`/`<script>` antes de extrair o texto.
fn extract_descricao(html: &str) -> String {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("div.panel-body").unwrap();
    let re_css = Regex::new(r"(?is)<(style|script)\b[^>]*>.*?</(style|script)>").unwrap();
    let partes: Vec<String> = doc
        .select(&sel)
        .map(|e| html_to_text(&re_css.replace_all(&e.inner_html(), " ")))
        .filter(|t| !t.is_empty())
        .collect();
    partes.join("\n")
}

/// HTML curto -> texto: tags viram espaço, entidades decodificadas (html5ever), clean.
fn html_to_text(html: &str) -> String {
    let mut spaced = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                spaced.push(' ');
            }
            _ if !in_tag => spaced.push(c),
            _ => {}
        }
    }
    let decoded: String = Html::parse_fragment(&spaced).root_element().text().collect();
    clean(&decoded)
}

/// Guard (princípio D-RJ5): reprova coleta capada (árvore/markup mudou).
fn validar(items: &[ServicoRaw]) -> Result<()> {
    if items.len() < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). A árvore JS da listagem pode ter mudado; \
             se veio do cache, limpe data/df/raw/cache/ e re-raspe.",
            items.len(),
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fragmento fiel da árvore JS: tema de topo -> subcategoria -> folhas.
    const ARVORE: &str = r#"
      var arvore = {
        'IPTU/TLP': { 'item': [
          'Cadastro de Im&oacute;veis - Consulta': { 'item': [
            {'url':'/aplicacoes/CartaServicos/servico.cfm?codTipoPessoa=6&codServico=437&codSubCategoria=16', 'desc':'Ficha Cadastral de Im&oacute;vel - Consultar'},
            {'url':'/aplicacoes/CartaServicos/servico.cfm?codTipoPessoa=6&codServico=376&codSubCategoria=16', 'desc':'Rela&ccedil;&atilde;o de Im&oacute;vel - Consultar'},
          ]},
        ]},
        'CONTRIBUINTES DE ICMS / ISS': { 'item': [
          'Impugna&ccedil;&atilde;o - PJ': { 'item': [
            {'url':'/aplicacoes/CartaServicos/servico.cfm?codServico=511&codTipoPessoa=7&codSubCategoria=113', 'desc':'Recurso e Impugna&ccedil;&atilde;o'},
          ]},
        ]},
      };
    "#;

    const DETALHE: &str = r#"<html><body>
      <div class="container conteudo_servico">
        <div class="panel-group" id="accordion">
          <div class="panel panel-default">
            <div class="panel-heading"><h3 class="panel-title"><a>Descrição</a></h3></div>
            <div id="collapseOne" class="panel-collapse collapse show"><div class="panel-body">
              <style>.x{transition:all .3s;}</style>
              Solicitar inclus&atilde;o de im&oacute;vel no Cadastro de Im&oacute;veis do DF.
            </div></div>
          </div>
          <div class="panel panel-default">
            <div class="panel-heading"><h3 class="panel-title"><a>Requisitos</a></h3></div>
            <div class="panel-collapse collapse"><div class="panel-body">
              Observar lista de documentos no Atendimento Virtual.
            </div></div>
          </div>
        </div>
      </div>
      <footer>Rua qualquer — CNPJ 00.000.000/0001-00</footer>
    </body></html>"#;

    #[test]
    fn parse_arvore_extrai_link_titulo_classe_publico() {
        let rows = parse_arvore(ARVORE);
        assert_eq!(rows.len(), 3);
        let ficha = rows.iter().find(|r| r.link.contains("codServico=437")).unwrap();
        assert_eq!(ficha.titulo, "Ficha Cadastral de Imóvel - Consultar"); // entidades decodificadas
        assert_eq!(ficha.classe, "Cadastro de Imóveis - Consulta"); // chave-pai imediata
        assert_eq!(ficha.publico, PUB_CIDADAO); // codTipoPessoa=6
        assert!(ficha.link.starts_with("https://www.receita.fazenda.df.gov.br/aplicacoes/"));

        let rec = rows.iter().find(|r| r.link.contains("codServico=511")).unwrap();
        assert_eq!(rec.publico, PUB_EMPRESA); // codTipoPessoa=7
        assert_eq!(rec.classe, "Impugnação - PJ");
    }

    #[test]
    fn extract_descricao_junta_paineis_sem_css_nem_rodape() {
        let d = extract_descricao(DETALHE);
        assert!(d.starts_with("Solicitar inclusão de imóvel"), "veio: {d}");
        assert!(d.contains("Observar lista de documentos"), "deve juntar os 2 painéis: {d}");
        assert!(!d.contains("transition"), "não deve pegar o CSS do <style>: {d}");
        assert!(!d.contains("CNPJ"), "não deve pegar o rodapé");
        assert!(!d.contains('<') && !d.contains("&atilde;"));
    }

    #[test]
    fn publico_de_mapeia_tipopessoa() {
        assert_eq!(publico_de("6"), PUB_CIDADAO);
        assert_eq!(publico_de("22"), PUB_CIDADAO);
        assert_eq!(publico_de("7"), PUB_EMPRESA);
        assert_eq!(publico_de("8"), PUB_EMPRESA);
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
