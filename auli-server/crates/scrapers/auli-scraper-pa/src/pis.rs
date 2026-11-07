//! O catálogo de serviços do site novo da SEFA-PA: o tipo `pis` do WordPress que o site (`www.sefa.pa.gov.br`,
//! Angular) consome. Estudo, medições e decisões: docs/ESTUDO-PA-Servicos-pis.md (D-PASERV-*).
//!
//! - `GET /wp-json/wp/v2/pis?per_page=100&page=N&_envelope` — 154 páginas em 2. O `_envelope` traz os cabeçalhos
//!   no corpo, e com eles o `X-WP-Total` da guarda vale também com o cache.
//! - `GET /wp-json/wp/v2/pis-categories?per_page=100&_envelope` — os 19 temas.
//!
//! Uma página é a ficha de um serviço: `content` e até 8 blocos ACF (`tituloN`/`descricaoN`, em geral "Como
//! solicitar", "Documentos necessários", "Manual/Tutorial", "Base legal") e o `service_link`. Ficam de fora as 15
//! páginas-índice de tema, e as cópias de um serviço (um post por tema) viram um serviço só (D-PASERV-3).

use std::collections::HashMap;
use std::sync::LazyLock;

use anyhow::{Result, bail};
use auli_contract::{Ocorrencia, ServicoRaw};
use auli_scraper_kit::html::{Opcoes, endereco, html_para_texto};
use regex::Regex;
use serde::Deserialize;
use serde::de::DeserializeOwned;

const WP: &str = "https://site-sefa-wordpress.sefa.pa.gov.br/wp-json/wp/v2";
/// O site novo: base do link do serviço e dos caminhos da raiz (`/multiple-contents/…`) no texto.
pub(crate) const SITE: &str = "https://www.sefa.pa.gov.br";
const POR_PAGINA: usize = 100;
/// "Home" é o destaque da página inicial, não tema.
const HOME: &str = "Home";
const CLASSE_SEM_TEMA: &str = "Geral";
/// As páginas-índice de tema, como o próprio site as lista no bundle (`main.js`, a lista que decide se a página abre
/// como tema ou como serviço). "CADIN" e "Consulta Protocolo" têm o nome de um tema, mas o site os abre como serviço.
const INDICES: [&str; 15] = [
    "Cadastro de Contribuintes",
    "Consulta",
    "Débitos e Parcelamentos",
    "Documentos Fiscais",
    "Benefício Imunidades e isenções",
    "IPVA",
    "ICMS",
    "ITCD",
    "Portal de Serviços",
    "Trânsito de Mercadorias",
    "Comunicação Oficial",
    "Declarações",
    "Comércio Exterior",
    "Solicitação",
    "AINF - Auto de Infração",
];
/// Guarda: piso de serviços distintos (127 em 07/11/2026).
const MIN_SERVICOS: usize = 110;

/// Tag de bloco: o campo é HTML, e a quebra do fonte é só formatação. Sem nenhuma, o campo é texto puro, e a quebra
/// separa parágrafos.
static BLOCO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<(p|br|li|ul|ol|div|table|h[1-6])\b").unwrap());

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    headers: Cabecalhos,
    body: T,
}

#[derive(Debug, Deserialize)]
struct Cabecalhos {
    #[serde(rename = "X-WP-Total")]
    total: usize,
    #[serde(rename = "X-WP-TotalPages")]
    paginas: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Pagina {
    id: u64,
    modified: String,
    title: Rendered,
    content: Rendered,
    #[serde(rename = "pis-categories", default)]
    categorias: Vec<u64>,
    /// Objeto com os campos ACF (o WP manda `[]` quando não há nenhum).
    #[serde(default)]
    acf: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct Rendered {
    rendered: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Categoria {
    id: u64,
    name: String,
}

pub(crate) fn url_categorias() -> String {
    format!("{WP}/pis-categories?per_page={POR_PAGINA}&_envelope")
}

pub(crate) fn url_pagina(n: usize) -> String {
    format!("{WP}/pis?per_page={POR_PAGINA}&page={n}&_envelope")
}

/// Lê um envelope do WP: o corpo e os cabeçalhos (`X-WP-Total`, `X-WP-TotalPages`).
fn envelope<T: DeserializeOwned>(json: &str, url: &str) -> Result<(Vec<T>, Cabecalhos)> {
    let e: Envelope<Vec<T>> = serde_json::from_str(json)
        .map_err(|err| anyhow::anyhow!("JSON de {url} inválido: {err}"))?;
    Ok((e.body, e.headers))
}

/// Os temas, com a guarda do total.
pub(crate) fn categorias(json: &str) -> Result<Vec<Categoria>> {
    let url = url_categorias();
    let (cats, h) = envelope::<Categoria>(json, &url)?;
    if cats.len() != h.total {
        bail!(
            "{url}: {} temas lidos, X-WP-Total = {} (mais de {POR_PAGINA}?)",
            cats.len(),
            h.total
        );
    }
    Ok(cats)
}

/// As páginas do catálogo: `baixar(n)` devolve o JSON da página `n`. Guarda: a soma das páginas é o `X-WP-Total`.
pub(crate) fn paginas(mut baixar: impl FnMut(usize) -> Result<String>) -> Result<Vec<Pagina>> {
    let mut todas: Vec<Pagina> = Vec::new();
    let mut n = 1;
    loop {
        let (corpo, h) = envelope::<Pagina>(&baixar(n)?, &url_pagina(n))?;
        todas.extend(corpo);
        if n >= h.paginas {
            if todas.len() != h.total {
                bail!(
                    "pis: {} páginas lidas, X-WP-Total = {}",
                    todas.len(),
                    h.total
                );
            }
            return Ok(todas);
        }
        n += 1;
    }
}

/// Os serviços do catálogo, um por título (D-PASERV-3), com o público único (D-PASERV-2).
pub(crate) fn servicos(
    paginas: &[Pagina],
    categorias: &[Categoria],
    publico: &str,
) -> Result<Vec<ServicoRaw>> {
    let titulo = |p: &Pagina| html_para_texto(&p.title.rendered, opcoes(false));
    for i in INDICES {
        if !paginas.iter().any(|p| titulo(p) == i) {
            bail!(
                "a página-índice \"{i}\" não está no catálogo — o site mudou a lista? (pis.rs, INDICES)"
            );
        }
    }
    let temas: HashMap<u64, &str> = categorias.iter().map(|c| (c.id, c.name.as_str())).collect();

    // As cópias de um serviço, na ordem do catálogo.
    let mut grupos: Vec<(String, Vec<&Pagina>)> = Vec::new();
    for p in paginas {
        let t = titulo(p);
        if INDICES.contains(&t.as_str()) {
            continue;
        }
        let k = chave(&t);
        match grupos.iter_mut().find(|(g, _)| *g == k) {
            Some((_, ps)) => ps.push(p),
            None => grupos.push((k, vec![p])),
        }
    }

    let mut out = Vec::with_capacity(grupos.len());
    for (_, mut ps) in grupos {
        // O texto é o da cópia alterada por último; os temas, os de todas.
        ps.sort_by(|a, b| (&b.modified, b.id).cmp(&(&a.modified, a.id)));
        let p = ps[0];
        let mut classes: Vec<String> = Vec::new();
        for q in &ps {
            for c in &q.categorias {
                let Some(nome) = temas.get(c) else {
                    bail!("pis {}: tema {c} fora de pis-categories", q.id);
                };
                let nome = html_para_texto(nome, opcoes(false));
                if nome != HOME && !classes.contains(&nome) {
                    classes.push(nome);
                }
            }
        }
        if classes.is_empty() {
            classes.push(CLASSE_SEM_TEMA.to_string());
        }
        let descricao = descricao(p);
        if descricao.is_empty() {
            bail!("pis {} (\"{}\"): descrição vazia", p.id, titulo(p));
        }
        out.push(ServicoRaw {
            titulo: titulo(p),
            descricao,
            link: link(p.id),
            orgao: crate::pa::ORGAO.to_string(),
            ocorrencias: classes
                .into_iter()
                .map(|classe| Ocorrencia {
                    publico: publico.to_string(),
                    classe,
                })
                .collect(),
        });
    }
    Ok(out)
}

/// Guarda: o piso de serviços distintos.
pub(crate) fn validar(servicos: &[ServicoRaw]) -> Result<()> {
    if servicos.len() < MIN_SERVICOS {
        bail!(
            "pis: só {} serviços (piso {MIN_SERVICOS}) — catálogo capado?",
            servicos.len()
        );
    }
    Ok(())
}

/// A identidade de um serviço entre as cópias: o título em minúsculas, só letras e números ("DIEF - Comércio
/// Exterior" = "DIEF Comércio Exterior").
pub(crate) fn chave(titulo: &str) -> String {
    titulo
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// O link público: `/internal/services/<id>`, cada dígito do id trocado por uma letra (`a` = 0 … `j` = 9), como o
/// site codifica (856 → `ifg`).
fn link(id: u64) -> String {
    let codigo: String = id
        .to_string()
        .bytes()
        .map(|d| (d - b'0' + b'a') as char)
        .collect();
    format!("{SITE}/internal/services/{codigo}")
}

fn opcoes(quebras_do_fonte: bool) -> Opcoes<'static> {
    Opcoes {
        base: Some(SITE),
        quebras_do_fonte,
    }
}

/// HTML → texto; a quebra do fonte só conta no campo sem tag de bloco.
fn texto(html: &str) -> String {
    html_para_texto(html, opcoes(!BLOCO_RE.is_match(html)))
}

fn acf<'a>(p: &'a Pagina, campo: &str) -> &'a str {
    p.acf.get(campo).and_then(|v| v.as_str()).unwrap_or("")
}

/// `content`, cada bloco preenchido ("Título:" e o texto) e "Acessar o serviço:" com o `service_link`.
fn descricao(p: &Pagina) -> String {
    let mut partes: Vec<String> = Vec::new();
    let c = texto(&p.content.rendered);
    if !c.is_empty() {
        partes.push(c);
    }
    for n in 1..=8 {
        let corpo = texto(acf(p, &format!("descricao{n}")));
        if corpo.is_empty() {
            continue;
        }
        let t = html_para_texto(acf(p, &format!("titulo{n}")), opcoes(false));
        partes.push(match t.chars().last() {
            None => corpo,
            Some('?' | ':' | '.' | '!') => format!("{t}\n{corpo}"),
            Some(_) => format!("{t}:\n{corpo}"),
        });
    }
    if let Some(l) = endereco(acf(p, "service_link").trim(), Some(SITE)) {
        partes.push(format!("Acessar o serviço: {l}"));
    }
    partes.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // As 28 páginas reais de 07/11/2026: as 15 páginas-índice (só id, título e temas) e 13 serviços — as cópias
    // da DIEF Comércio Exterior (o título difere na pontuação), da Consulta Pública Dívida Ativa (uma sem tema) e da
    // Isenção de IPVA PcD; "CADIN" e "Consulta Protocolo" (nome de tema, mas serviço); "ALMA web" (Home); "Renúncia
    // Fiscal" (link do serviço relativo); "Vista de Processo" (blocos em texto puro); "Contribuinte Cadastro" e
    // "Novo Manter Usuário" (sem tema).
    const PIS: &str = include_str!("../tests/fixtures/pa-pis.json");
    const CATS: &str = include_str!("../tests/fixtures/pa-pis-categorias.json");

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    fn fixture() -> Vec<Pagina> {
        paginas(|_| Ok(PIS.to_string())).unwrap()
    }

    fn catalogo(ps: &[Pagina]) -> Result<Vec<ServicoRaw>> {
        servicos(ps, &categorias(CATS).unwrap(), "Serviços")
    }

    fn servico<'a>(ss: &'a [ServicoRaw], titulo: &str) -> &'a ServicoRaw {
        ss.iter()
            .find(|s| s.titulo == titulo)
            .unwrap_or_else(|| panic!("sem \"{titulo}\""))
    }

    fn classes(s: &ServicoRaw) -> Vec<&str> {
        s.ocorrencias.iter().map(|o| o.classe.as_str()).collect()
    }

    #[test]
    fn indices_saem_e_as_copias_viram_um_servico() {
        let ss = catalogo(&fixture()).unwrap();
        // 28 páginas − 15 índices = 13 serviços, 3 deles com cópia.
        assert_eq!(ss.len(), 10);
        assert!(ss.iter().all(|s| !INDICES.contains(&s.titulo.as_str())));
        servico(&ss, "CADIN");
        servico(&ss, "Consulta Protocolo");
        assert!(
            ss.iter()
                .all(|s| s.ocorrencias.iter().all(|o| o.publico == "Serviços"))
        );
    }

    #[test]
    fn copia_fica_com_o_texto_da_alterada_por_ultimo_e_os_temas_das_duas() {
        let ss = catalogo(&fixture()).unwrap();
        // 886 (ICMS, 27/06 17:08) e 8616 (Comércio Exterior, 27/06 05:55).
        let s = servico(&ss, "DIEF Comércio Exterior");
        assert_eq!(s.link, format!("{SITE}/internal/services/iig"));
        assert_eq!(classes(s), ["ICMS", "Comércio Exterior"]);
        // 892 (Benefícios, 12/08) e 912 (IPVA, 03/07).
        let s = servico(&ss, "Isenção de IPVA - Pessoa com Deficiência");
        assert_eq!(s.link, format!("{SITE}/internal/services/ijc"));
        assert_eq!(classes(s), ["Benefício Imunidades e Isenções", "IPVA"]);
    }

    #[test]
    fn home_sai_e_sem_tema_vira_geral() {
        let ss = catalogo(&fixture()).unwrap();
        assert_eq!(
            classes(servico(&ss, "ALMA web")),
            ["Trânsito de Mercadorias"]
        );
        assert_eq!(classes(servico(&ss, "Contribuinte Cadastro")), ["Geral"]);
        // A cópia sem tema não acrescenta "Geral".
        assert_eq!(
            classes(servico(&ss, "Consulta Pública Divida Ativa")),
            ["Consulta Protocolo"]
        );
    }

    #[test]
    fn descricao_tem_o_conteudo_os_blocos_e_o_link_do_servico() {
        let ss = catalogo(&fixture()).unwrap();
        let d = &servico(&ss, "CADIN").descricao;
        assert!(d.starts_with("Instituído pela Lei 8.873/2019"), "{d}");
        // O título do bloco que já é pergunta fica sem os dois-pontos.
        assert!(d.contains("\n\nO que é CADIN?\nCadastro único"), "{d}");
        assert!(d.contains("\n\nObjetivos:\nPermitir"), "{d}");
        assert!(
            d.ends_with("\n\nAcessar o serviço: https://app.sefa.pa.gov.br/cadin/bem-vindo"),
            "{d}"
        );
        // O caminho da raiz ganha o host do site.
        let d = &servico(&ss, "Renúncia Fiscal").descricao;
        assert!(
            d.ends_with(
                "Acessar o serviço: https://www.sefa.pa.gov.br/multiple-contents/beneficios-fiscais"
            ),
            "{d}"
        );
        // Bloco em texto puro: as linhas do fonte ficam.
        let d = &servico(&ss, "Vista de Processo").descricao;
        assert!(
            d.contains("Documentos necessários:\nRequerimento;\nProcuração e/ou substabelecimento"),
            "{d}"
        );
    }

    #[test]
    fn quebra_do_fonte_so_conta_sem_tag_de_bloco() {
        assert_eq!(
            texto("Requerimento;\r\nProcuração"),
            "Requerimento;\nProcuração"
        );
        assert_eq!(
            texto("<p>nome, endereço, telefone,\r\n  endereço eletrônico</p>"),
            "nome, endereço, telefone, endereço eletrônico"
        );
    }

    #[test]
    fn link_troca_cada_digito_por_letra() {
        assert_eq!(link(856), format!("{SITE}/internal/services/ifg"));
        assert_eq!(link(10516), format!("{SITE}/internal/services/bafbg"));
    }

    #[test]
    fn chave_ignora_caixa_e_pontuacao() {
        assert_eq!(
            chave("DIEF - Comércio Exterior"),
            chave("dief comércio  exterior")
        );
    }

    #[test]
    fn indice_que_sumiu_e_erro() {
        let ps: Vec<Pagina> = fixture()
            .into_iter()
            .filter(|p| p.title.rendered != "Consulta")
            .collect();
        let e = erro(catalogo(&ps));
        assert!(e.contains("\"Consulta\""), "{e}");
    }

    #[test]
    fn descricao_vazia_e_erro() {
        let mut ps = fixture();
        let p = ps.iter_mut().find(|p| p.id == 840).unwrap();
        p.content.rendered.clear();
        p.acf = serde_json::Value::Array(vec![]);
        assert!(erro(catalogo(&ps)).contains("descrição vazia"));
    }

    #[test]
    fn tema_desconhecido_e_erro() {
        let mut ps = fixture();
        ps.iter_mut()
            .find(|p| p.id == 840)
            .unwrap()
            .categorias
            .push(9999);
        assert!(erro(catalogo(&ps)).contains("tema 9999"));
    }

    fn envelope_de(ids: &[u64], total: usize, paginas: usize) -> String {
        let body: Vec<String> = ids
            .iter()
            .map(|i| {
                format!(
                    r#"{{"id":{i},"modified":"x","title":{{"rendered":"t{i}"}},"content":{{"rendered":""}}}}"#
                )
            })
            .collect();
        format!(
            r#"{{"body":[{}],"status":200,"headers":{{"X-WP-Total":{total},"X-WP-TotalPages":{paginas}}}}}"#,
            body.join(",")
        )
    }

    #[test]
    fn paginas_le_ate_a_ultima_e_confere_o_total() {
        let ps = paginas(|n| match n {
            1 => Ok(envelope_de(&[1, 2], 3, 2)),
            2 => Ok(envelope_de(&[3], 3, 2)),
            _ => panic!("página {n}"),
        })
        .unwrap();
        assert_eq!(ps.iter().map(|p| p.id).collect::<Vec<_>>(), [1, 2, 3]);
        let e = erro(paginas(|n| Ok(envelope_de(&[n as u64], 4, 2))));
        assert!(e.contains("X-WP-Total = 4"), "{e}");
    }

    #[test]
    fn categorias_confere_o_total() {
        let e = erro(categorias(
            &CATS.replace("\"X-WP-Total\": 19", "\"X-WP-Total\": 20"),
        ));
        assert!(e.contains("X-WP-Total = 20"), "{e}");
    }

    #[test]
    fn validar_reprova_capado() {
        let ss = catalogo(&fixture()).unwrap();
        assert!(erro(validar(&ss)).contains("piso 110"));
    }
}
