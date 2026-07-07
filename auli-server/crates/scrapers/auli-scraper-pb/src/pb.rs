//! Coleta dos serviços da SEFAZ-PB a partir da Carta de Serviços (`cartaservico.sefaz.pb.gov.br`).
//!
//! A Carta é um app PHP server-rendered. `servicos.php` lista os 101 serviços num accordion aninhado
//! (categoria → público → subcategoria → serviço), cada um com um link `saibamais.php?id=N`. Cada
//! `saibamais.php?id=N` é uma **ficha rica** com pares `<h3>Rótulo:</h3><h6>Valor</h6>` (O que é,
//! Público-alvo, Forma de prestação, Taxa, Exigências, Etapas, Documentação, Horário, Contato) e o
//! link real do serviço em `redireciona('id','URL')`.
//!
//! Modelagem (molde TO/DF): `titulo` e `descricao` vêm do detalhe; **público** vem do campo
//! "Público-alvo" da ficha (Cidadão/Empresa, per-serviço); `classe` = a subcategoria imediata da
//! listagem (a categoria-pai mais próxima que não é rótulo de público); `link` = `saibamais.php?id=N`
//! (único por serviço). Cada serviço aparece 2× na listagem (árvores por público) → dedup por id.

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use regex::Regex;
use scraper::Html;
use ureq::Agent;

const USER_AGENT: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

const BASE: &str = "https://cartaservico.sefaz.pb.gov.br";
const LISTA_URL: &str = "https://cartaservico.sefaz.pb.gov.br/servicos.php";

const ORGAO: &str = "SEFAZ-PB";
const PUB_CIDADAO: &str = "Cidadão";
const PUB_EMPRESA: &str = "Empresa";
const CLASSE_FALLBACK: &str = "Geral";
/// Cortesia entre GETs (a listagem + 101 fichas).
const COURTESY: Duration = Duration::from_millis(300);
/// Guard: piso de serviços (a Carta tem 101).
const MIN_SERVICOS: usize = 90;

/// Um serviço da listagem (antes de buscar a ficha).
struct Card {
    id: String,
    classe: String,
}

/// Raspa a Carta e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));
    let mut pending: Vec<(String, String)> = Vec::new();

    // 1) A listagem -> (id, classe).
    let lista = load(&agent, data_dir, LISTA_URL, use_cache, &mut pending)?;
    let cards = parse_listagem(&lista);
    println!("PB: {} serviços na listagem", cards.len());

    // 2) A ficha de cada serviço.
    let mut items: Vec<ServicoRaw> = Vec::new();
    for c in &cards {
        let url = format!("{}/saibamais.php?id={}", BASE, c.id);
        let ficha = load(&agent, data_dir, &url, use_cache, &mut pending)?;
        let (titulo, descricao, publicos) = parse_ficha(&ficha);
        if titulo.is_empty() {
            continue;
        }
        let ocorrencias = publicos
            .into_iter()
            .map(|publico| Ocorrencia { publico, classe: c.classe.clone() })
            .collect();
        items.push(ServicoRaw { titulo, descricao, link: url, orgao: ORGAO.to_string(), ocorrencias });
    }

    validar(&items)?;

    for (url, raw) in &pending {
        auli_scraper_kit::cache::write(data_dir, url, raw);
    }

    let ocorrencias: usize = items.iter().map(|s| s.ocorrencias.len()).sum();
    let classes: HashSet<&str> =
        items.iter().flat_map(|s| s.ocorrencias.iter()).map(|o| o.classe.as_str()).collect();
    println!("PB: {} serviços ({} ocorrências) em {} classe(s)", items.len(), ocorrencias, classes.len());
    let publicos_ordem = vec![
        Publico { nome: PUB_CIDADAO.to_string(), slug: "servicos-ao-cidadao".to_string() },
        Publico { nome: PUB_EMPRESA.to_string(), slug: "servicos-a-empresa".to_string() },
    ];
    Ok((items, publicos_ordem))
}

/// GET (HTML) com cache. Miss + `--usecache` = erro. Rede -> `pending` + cortesia.
fn load(
    agent: &Agent,
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
    let body = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts { log_prefix: "PB", ..Default::default() },
    )?;
    pending.push((url.to_string(), body.clone()));
    sleep(COURTESY);
    Ok(body)
}

/// Parseia a listagem: para cada `saibamais.php?id=N` (1ª ocorrência), a `classe` é o botão de
/// accordion mais próximo ANTES do link que **não** é um rótulo de público ("Para Empresa"/"Para o
/// Cidadão"). Cada serviço aparece 2× (árvores por público) → dedup por id.
fn parse_listagem(html: &str) -> Vec<Card> {
    let btn_re = Regex::new(r"(?s)<button class=\x22accordion-button[^>]*>(.*?)</button>").unwrap();
    let botoes: Vec<(usize, String)> = btn_re
        .captures_iter(html)
        .map(|c| (c.get(0).unwrap().start(), html_to_text(&c[1])))
        .filter(|(_, t)| !is_publico_label(t))
        .collect();

    let link_re = Regex::new(r"saibamais\.php\?id=(\d+)").unwrap();
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<Card> = Vec::new();
    for cap in link_re.captures_iter(html) {
        let id = cap[1].to_string();
        if !seen.insert(id.clone()) {
            continue;
        }
        let pos = cap.get(0).unwrap().start();
        let classe = botoes
            .iter()
            .rev()
            .find(|(p, _)| *p < pos)
            .map(|(_, c)| c.clone())
            .filter(|c| !c.is_empty())
            .unwrap_or_else(|| CLASSE_FALLBACK.to_string());
        out.push(Card { id, classe });
    }
    out
}

/// `true` se o rótulo do botão é um agrupamento de público (não serve como classe).
fn is_publico_label(t: &str) -> bool {
    let t = t.trim();
    t.eq_ignore_ascii_case("Para Empresa")
        || t.eq_ignore_ascii_case("Para o Cidadão")
        || t.eq_ignore_ascii_case("Para o Cidadao")
}

/// Parseia a ficha: `(titulo, descricao, publicos)`. Título vem do `title=` do `inputbutton01`;
/// descrição = os pares `<h3>Rótulo:</h3><h6>Valor</h6>` (menos o Público-alvo) + o link real
/// (`redireciona`); públicos = o campo "Público-alvo" (lista separada por vírgula).
fn parse_ficha(html: &str) -> (String, String, Vec<String>) {
    let titulo = Regex::new(r#"inputbutton01"[^>]*title="([^"]*)""#)
        .unwrap()
        .captures(html)
        .map(|c| html_to_text(&c[1]))
        .unwrap_or_default();

    // A URL do onclick pode vir com entidades (às vezes duplo-encodadas: `&amp;amp;`) → decodifica
    // e colapsa `&amp;` residual para `&` (URL não deve conter `&amp;` literal).
    let acessar = Regex::new(r"redireciona\('[^']*',\s*'([^']*)'\)")
        .unwrap()
        .captures(html)
        .map(|c| html_to_text(&c[1]).replace("&amp;", "&"));

    let pair_re = Regex::new(r"(?s)<h3>(.*?)</h3>\s*<h6[^>]*>(.*?)</h6>").unwrap();
    let mut linhas: Vec<String> = Vec::new();
    let mut publicos: Vec<String> = Vec::new();
    for cap in pair_re.captures_iter(html) {
        let rotulo = html_to_text(&cap[1]);
        let valor = html_to_text(&cap[2]);
        if rotulo.is_empty() {
            continue;
        }
        if rotulo.starts_with("Público-alvo") {
            publicos = parse_publicos(&valor);
            continue;
        }
        if valor.is_empty() || valor == "-" {
            continue;
        }
        linhas.push(format!("{} {}", rotulo, valor));
    }
    if let Some(url) = acessar.filter(|u| !u.is_empty()) {
        linhas.push(format!("Acessar o serviço: {}", url));
    }
    if publicos.is_empty() {
        publicos.push(PUB_CIDADAO.to_string());
        publicos.push(PUB_EMPRESA.to_string());
    }
    (titulo, linhas.join("\n"), publicos)
}

/// Normaliza o "Público-alvo" (ex.: "Cidadão, Empresa.") para os rótulos da frota, dedup e na ordem.
fn parse_publicos(valor: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for parte in valor.split(',') {
        let p = parte.trim().trim_end_matches('.').trim();
        let rotulo = if p.eq_ignore_ascii_case("Empresa") {
            PUB_EMPRESA
        } else if p.eq_ignore_ascii_case("Cidadão") || p.eq_ignore_ascii_case("Cidadao") {
            PUB_CIDADAO
        } else {
            continue;
        };
        if !out.iter().any(|x| x == rotulo) {
            out.push(rotulo.to_string());
        }
    }
    out
}

/// HTML -> texto: tags viram espaço, entidades decodificadas (html5ever), clean.
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

/// Guard (princípio D-RJ5): reprova coleta capada (listagem/markup mudou).
fn validar(items: &[ServicoRaw]) -> Result<()> {
    if items.len() < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). A listagem/markup pode ter mudado; se \
             veio do cache, limpe data/pb/raw/cache/ e re-raspe.",
            items.len(),
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTA: &str = r#"
      <div class="accordion" id="accordionServico-abc">
        <button class="accordion-button" type="button">DOCUMENTOS FISCAIS</button>
        <div class="accordion-body">
          <button class="accordion-button" type="button">Para Empresa</button>
          <button class="accordion-button" type="button">NOTA FISCAL ELETRÔNICA</button>
          <ul><li><a href="saibamais.php?id=1">NFe - Consultar</a></li></ul>
          <button class="accordion-button" type="button">Para o Cidadão</button>
          <button class="accordion-button" type="button">NOTA FISCAL ELETRÔNICA</button>
          <ul><li><a href="saibamais.php?id=1">NFe - Consultar</a></li></ul>
        </div>
      </div>
      <button class="accordion-button" type="button">CADASTRO</button>
      <ul><li><a href="saibamais.php?id=50">Emitir Certidão</a></li></ul>
    "#;

    const FICHA: &str = r#"<html><body>
      <div class="inputbutton01" title="NOTA FISCAL ELETR&Ocirc;NICA - NFe - Consultar Credenciamentos.">
        <h3>NOTA FISCAL ELETRÔNICA - NFe - Consultar Credenciamentos.</h3>
      </div>
      <button onclick="redireciona('1', 'https://www.sefaz.pb.gov.br/servirtual/nf-e/consultar')">ACESSAR SERVIÇO</button>
      <h3>O que é o serviço:</h3><h6 class="h6">Permite consultar o credenciamento.</h6>
      <h3>Público-alvo:</h3><h6 class="h6">Cidad&atilde;o, Empresa.</h6>
      <h3>Taxa:</h3><h6 class="h6">Gratuito.</h6>
      <h3>Informações adicionais:</h3><h6 class="h6">-</h6>
    </body></html>"#;

    #[test]
    fn parse_listagem_classe_ignora_rotulo_de_publico_e_dedup() {
        let cards = parse_listagem(LISTA);
        assert_eq!(cards.len(), 2, "id=1 aparece 2× → dedup");
        let c1 = cards.iter().find(|c| c.id == "1").unwrap();
        // classe = subcategoria imediata, NÃO "Para Empresa"
        assert_eq!(c1.classe, "NOTA FISCAL ELETRÔNICA");
        assert_eq!(cards.iter().find(|c| c.id == "50").unwrap().classe, "CADASTRO");
    }

    #[test]
    fn parse_ficha_extrai_titulo_descricao_publicos() {
        let (titulo, descricao, publicos) = parse_ficha(FICHA);
        assert_eq!(titulo, "NOTA FISCAL ELETRÔNICA - NFe - Consultar Credenciamentos.");
        assert!(descricao.contains("O que é o serviço: Permite consultar o credenciamento."));
        assert!(descricao.contains("Taxa: Gratuito."));
        assert!(descricao.contains("Acessar o serviço: https://www.sefaz.pb.gov.br/servirtual/nf-e/consultar"));
        // Público-alvo não entra na descrição (vira ocorrência); "Informações adicionais: -" é descartado
        assert!(!descricao.contains("Público-alvo"));
        assert!(!descricao.contains("Informações adicionais"));
        assert_eq!(publicos, vec!["Cidadão", "Empresa"]);
    }

    #[test]
    fn parse_publicos_normaliza_e_dedup() {
        assert_eq!(parse_publicos("Empresa."), vec!["Empresa"]);
        assert_eq!(parse_publicos("Cidadão, Empresa."), vec!["Cidadão", "Empresa"]);
        assert!(parse_publicos("").is_empty());
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
