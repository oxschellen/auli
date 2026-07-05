//! Coleta dos serviços da SEFAZ-CE a partir da API JSON do "Portal de Serviços" (Sydle ONE).
//!
//! A página é uma SPA pura (D-CE1): não há HTML server-rendered. A listagem vem do método público
//! `getChildren` (POST) sobre o catálogo `servico-geral` — resposta `{ hits, return[] }` paginada
//! por `params.pageIndex`/`pageSize`. Cada item já traz `name` (título) e `description` não-vazia,
//! então NÃO há chamada de detalhe (a listagem é rica).
//!
//! Auth (D-CE1): o portal usa um Bearer token **anônimo e público** embutido no shell HTML
//! (`useCookieAuthentication: false`). O token é efêmero, então o extraímos fresh do shell a cada
//! rodada. O servidor autoriza `getChildren` ao anônimo (o genérico `_get` é bloqueado por ACL).
//!
//! Modelagem (D-CE2/3): identidade = `_id` do documento (único); `link` = URL canônica de detalhe
//! `…/servico-geral+<identifier>+<_id>`. Os itens não têm `_tags` → sem eixo de público (cenário A,
//! padrão RJ): público único "Serviços", classe "Geral".
//!
//! Paginação: `params.sorters` usa o formato do front (`{"sort":{"name._current.keyword":"asc"}}`)
//! por fidelidade — mas a ordenação NÃO muda a completude. Medido empiricamente, o `getChildren`
//! entrega **382 documentos distintos** (cru=382, zero duplicatas) com QUALQUER sorter — name asc,
//! name desc, `_lastUpdateDate`, ou sem nenhum. O `hits` do servidor é **inflado** (≈392): é a
//! contagem do índice, ~10 acima do que a paginação realmente contém (deletados-não-purgados /
//! destaques pinados servidos por outra via). Por isso o gap `hits > coletados` é só um WARNING, não
//! reprova — 382 é o catálogo real; o guard duro é `MIN_SERVICOS`. Guards (D-RJ5): o cache só grava
//! DEPOIS dos guards.
//!
//! `pageSize=10` é o do front. Com `pageSize` maior o servidor entrega MENOS (10→382, 100→292,
//! 500→0) — um defeito de paginação do endpoint. A paginação para na página VAZIA (páginas
//! não-finais podem vir curtas).

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use serde::Deserialize;

const BASE: &str = "https://portalservicos.sefaz.ce.gov.br";
/// Shell público de onde extraímos o Bearer anônimo (efêmero).
const SHELL_URL: &str = "https://portalservicos.sefaz.ce.gov.br/";
/// Endpoint do método público de catálogo (classe API do service-desk + método `getChildren`).
const GETCHILDREN_URL: &str = "https://portalservicos.sefaz.ce.gov.br/api/1/servicedesk-embedded/_classId/5cd5e83d59f8170cd4dc2c43/getChildren";
/// `application` no corpo (≠ do header de tenant abaixo).
const APP_IN_BODY: &str = "sefaz-ceara";
/// Tenant no header `X-Explorer-Account-Token`.
const ORG_HEADER: &str = "sefazce";
/// `_id` do catálogo `servico-geral` (= o ObjectId da URL de listagem).
const CATALOGO_ID: &str = "648af76264778b7336c470a3";

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
/// `pageSize` PEQUENO de propósito: com o sort antigo (inválido), pageSize maior entregava MENOS
/// (10→382, 100→292, 500→0). O 10 é o que o próprio front usa. Não aumentar sem re-medir a
/// completude (invariante `hits == únicos`) com o sorter estável, duas coletas seguidas.
const PAGE_SIZE: u32 = 10;
/// Marcador do sorter na chave de cache: cache gravado com outro sort NÃO é reaproveitado (a
/// ordem/cobertura das páginas depende do sorter).
const SORT_KEY: &str = "name";
/// Teto de páginas (guarda contra loop; ~392 itens ÷ 10 ≈ 40 páginas + a vazia).
const MAX_PAGES: u32 = 80;
/// Cortesia entre páginas.
const COURTESY: Duration = Duration::from_millis(400);

/// Público único do CE (cenário A — os itens não têm faceta de público).
const PUBLICO_NOME: &str = "Serviços";
const PUBLICO_SLUG: &str = "servicos-gerais";
/// Classe única (o catálogo é plano; sem tema por item).
const CLASSE_GERAL: &str = "Geral";
/// Órgão de origem.
const ORGAO: &str = "SEFAZ-CE";

/// Guard D-CE (princípio D-RJ5): mínimo de serviços (folga sobre os ~392 observados, mas aperta o
/// bastante para rejeitar catálogo capado — inclusive a regressão de `pageSize` que devolvia 292).
const MIN_SERVICOS: usize = 350;

/// Uma página da resposta de `getChildren`.
#[derive(Debug, Deserialize)]
struct Page {
    #[serde(default)]
    hits: i64,
    #[serde(rename = "return", default)]
    items: Vec<Item>,
}

/// Um item de serviço do `return[]`. Só os campos que usamos; serde ignora o resto.
#[derive(Debug, Deserialize)]
struct Item {
    #[serde(rename = "_id")]
    id: String,
    #[serde(default)]
    identifier: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
}

/// Raspa o catálogo e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));

    // Páginas na ordem: (url_lógica, json_cru, parseada). O json cru fica para o cache — que só
    // grava DEPOIS dos guards (D-RJ5); a parseada evita re-parse na montagem.
    let mut raw_pages: Vec<(String, String, Page)> = Vec::new();
    let mut token: Option<String> = None; // buscado de forma preguiçosa, uma vez.
    let mut fetched_any = false;
    let mut hits_max: i64 = 0;

    let mut page = 1u32;
    loop {
        // `pageSize` e o sorter na chave: cache gravado com outros parâmetros NÃO é reaproveitado
        // (total entregue e cobertura das páginas dependem de ambos — ver PAGE_SIZE/SORT_KEY).
        let logical =
            format!("{}#ps={}&sort={}&page={}", GETCHILDREN_URL, PAGE_SIZE, SORT_KEY, page);
        let (json, from_cache) = match auli_scraper_kit::cache::read(data_dir, &logical) {
            Some(cached) => {
                println!("Cache hit (página {}): {}", page, GETCHILDREN_URL);
                (cached, true)
            }
            None => {
                if use_cache {
                    // Só páginas NÃO-vazias entram no cache, então — havendo cache — o primeiro
                    // miss é a página vazia terminadora: fim da paginação, não erro. Um cache
                    // genuinamente truncado é pego pelos guards (MIN_SERVICOS / invariante de
                    // hits), cuja mensagem já manda limpar o cache. Miss na página 1 = não há
                    // cache algum: aí sim, erro.
                    if page == 1 {
                        return Err(anyhow!(
                            "cache vazio para o catálogo (modo --usecache, sem rede). Rode uma \
                             coleta com rede primeiro."
                        )
                        .into());
                    }
                    break;
                }
                if token.is_none() {
                    token = Some(fetch_token(&agent)?);
                }
                let body = fetch_page(&agent, token.as_ref().unwrap(), page)?;
                fetched_any = true;
                (body, false)
            }
        };

        let parsed = parse_page(&json)?;
        hits_max = hits_max.max(parsed.hits);
        let n = parsed.items.len();
        // Termina SÓ na página vazia (não em página curta): o front pagina até esvaziar, e páginas
        // não-finais podem vir curtas. Página vazia não entra no cache nem no acumulado.
        if n == 0 {
            break;
        }
        println!("CE: página {} -> {} itens (hits={})", page, n, parsed.hits);
        raw_pages.push((logical, json, parsed));

        page += 1;
        if page > MAX_PAGES {
            eprintln!("⚠️  CE: atingiu MAX_PAGES ({}); parando a paginação.", MAX_PAGES);
            break;
        }
        if !from_cache {
            sleep(COURTESY);
        }
    }

    // Monta os serviços (dedup por `_id`) e valida antes de gravar o cache.
    let items = build_servicos(raw_pages.iter().map(|(_, _, p)| p));
    validar(&items, hits_max)?;

    // Cache só DEPOIS dos guards (D-RJ5): uma resposta capada nunca envenena o cache.
    if fetched_any {
        for (logical, json, _) in &raw_pages {
            auli_scraper_kit::cache::write(data_dir, logical, json);
        }
    }

    println!("CE: {} serviços (hits={}, dedup por _id)", items.len(), hits_max);
    let publicos_ordem =
        vec![Publico { nome: PUBLICO_NOME.to_string(), slug: PUBLICO_SLUG.to_string() }];
    Ok((items, publicos_ordem))
}

/// Extrai o Bearer token anônimo do shell público (`"Authorization":"Bearer …"`). Efêmero.
fn fetch_token(agent: &ureq::Agent) -> Result<String> {
    println!("Fetching token (shell): {}", SHELL_URL);
    let shell = get_string(agent, SHELL_URL)?;
    parse_token(&shell)
}

/// POST `getChildren` de uma página; devolve o corpo JSON cru. Retenta com backoff.
fn fetch_page(agent: &ureq::Agent, token: &str, page: u32) -> Result<String> {
    let body = build_body(page);
    let max_attempts = 3;
    let mut delay = Duration::from_millis(800);
    let mut last = anyhow!("sem tentativa");
    println!("POST getChildren (página {})", page);
    for attempt in 1..=max_attempts {
        let sent = agent
            .post(GETCHILDREN_URL)
            .header("Authorization", token)
            .header("X-Explorer-Account-Token", ORG_HEADER)
            .header("Accept", "application/json")
            .send_json(&body);
        match sent {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(s) if !s.trim().is_empty() => return Ok(s),
                Ok(_) => last = anyhow!("resposta vazia"),
                Err(e) => last = anyhow!(e.to_string()),
            },
            Err(e) => last = anyhow!(e.to_string()),
        }
        if attempt < max_attempts {
            eprintln!("⚠️  CE: página {} tentativa {} falhou ({}); retentando…", page, attempt, last);
            sleep(delay);
            delay *= 2;
        }
    }
    Err(anyhow!("falha ao buscar página {} após {} tentativas: {}", page, max_attempts, last))
}

/// GET simples que devolve o corpo como String (com retentativas).
fn get_string(agent: &ureq::Agent, url: &str) -> Result<String> {
    let max_attempts = 3;
    let mut delay = Duration::from_millis(800);
    let mut last = anyhow!("sem tentativa");
    for attempt in 1..=max_attempts {
        match agent.get(url).call() {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(s) if !s.trim().is_empty() => return Ok(s),
                Ok(_) => last = anyhow!("resposta vazia"),
                Err(e) => last = anyhow!(e.to_string()),
            },
            Err(e) => last = anyhow!(e.to_string()),
        }
        if attempt < max_attempts {
            sleep(delay);
            delay *= 2;
        }
    }
    Err(anyhow!("falha ao buscar {} após {} tentativas: {}", url, max_attempts, last))
}

/// Corpo mínimo do POST `getChildren`. `params.sorters` usa o formato verbatim do front por
/// fidelidade e dá ordem estável entre páginas; **não** afeta a completude (ver header do módulo:
/// 382 distintos com qualquer sorter — ou sem nenhum). Qualquer campo válido serve.
fn build_body(page: u32) -> serde_json::Value {
    serde_json::json!({
        "application": APP_IN_BODY,
        "id": CATALOGO_ID,
        "rootId": CATALOGO_ID,
        "path": "servico-geral",
        "view": "list",
        "params": {
            "filters": {},
            "pageIndex": page,
            "pageSize": PAGE_SIZE,
            "sorters": { "sort": { "name._current.keyword": "asc" } }
        }
    })
}

/// Extrai `Bearer …` do primeiro `"Authorization":"…"` do shell. Sem regex (evita a dependência).
fn parse_token(shell: &str) -> Result<String> {
    const KEY: &str = "\"Authorization\":\"";
    let start = shell.find(KEY).ok_or_else(|| anyhow!("token não encontrado no shell (markup mudou?)"))?
        + KEY.len();
    let rest = &shell[start..];
    let end = rest.find('"').ok_or_else(|| anyhow!("token malformado no shell"))?;
    let tok = rest[..end].trim();
    if !tok.starts_with("Bearer ") || tok.len() < 20 {
        bail!("valor de Authorization inesperado no shell");
    }
    Ok(tok.to_string())
}

/// Parseia uma página `{ hits, return[] }`.
fn parse_page(json: &str) -> Result<Page> {
    serde_json::from_str::<Page>(json).map_err(|e| anyhow!("JSON de getChildren inválido: {}", e))
}

/// Monta os `ServicoRaw` a partir das páginas, deduplicando por `_id` (ordem de descoberta).
fn build_servicos<'a>(pages: impl Iterator<Item = &'a Page>) -> Vec<ServicoRaw> {
    let mut vistos: HashSet<String> = HashSet::new();
    let mut out: Vec<ServicoRaw> = Vec::new();
    for page in pages {
        for it in &page.items {
            if it.id.is_empty() || !vistos.insert(it.id.clone()) {
                continue;
            }
            let titulo = clean(&it.name);
            if titulo.is_empty() {
                continue;
            }
            out.push(ServicoRaw {
                titulo,
                descricao: clean(&it.description),
                link: canonical(&it.identifier, &it.id),
                orgao: ORGAO.to_string(),
                ocorrencias: vec![Ocorrencia {
                    publico: PUBLICO_NOME.to_string(),
                    classe: CLASSE_GERAL.to_string(),
                }],
            });
        }
    }
    out
}

/// URL canônica de detalhe: `…/servico-geral+<identifier>+<_id>`. Sem `identifier`, o portal
/// aceita a forma curta `…/+<_id>` (evita o malformado `servico-geral++<_id>`).
fn canonical(identifier: &str, id: &str) -> String {
    if identifier.is_empty() {
        format!("{}/+{}", BASE, id)
    } else {
        format!("{}/servico-geral+{}+{}", BASE, identifier, id)
    }
}

/// Normaliza texto: tira zero-width/nbsp e comprime espaços (padrão dos demais scrapers).
fn clean(s: &str) -> String {
    s.replace('\u{200b}', "").replace('\u{00a0}', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Guard D-CE (princípio D-RJ5): reprova catálogo capado (abaixo do mínimo). O gap `hits > únicos`
/// NÃO reprova — o `hits` do índice infla ~10 acima do que a paginação entrega (ver header do
/// módulo); é só um WARNING informativo. O guard duro é `MIN_SERVICOS`, que pega a regressão de
/// `pageSize` (que devolvia 292).
fn validar(items: &[ServicoRaw], hits_max: i64) -> Result<()> {
    let unicos = items.len();
    if hits_max > unicos as i64 {
        eprintln!(
            "ℹ️  CE: servidor anuncia {} (hits do índice) mas a paginação entrega {} distinto(s) — \
             gap conhecido (índice infla; 382 é o catálogo real). Seguindo.",
            hits_max, unicos
        );
    }
    if unicos < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). Se veio do cache, limpe \
             data/ce/raw/cache/ e re-raspe.",
            unicos,
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixture derivada de itens reais do `getChildren` (campos que usamos).
    const PAGE_JSON: &str = r#"{
      "hits": 392,
      "return": [
        {"_id":"69459012f6e0ff51fef22a9d","identifier":"homologar-di-declaracao-de-importacao",
         "name":"Homologar DI - Declaração de Importação",
         "description":"rEQUISIÇÃO DE AÇÃO FISCAL DE D I AINDA NÃO HOMOLOGADA NO SITRAM"},
        {"_id":"aaa111","identifier":"impugnar-debito-itcd",
         "name":"  Impugnar Débito  Inscrito ",
         "description":"Impugnação de débitos de ITCD."}
      ]
    }"#;

    fn pages(jsons: &[&str]) -> Vec<Page> {
        jsons.iter().map(|j| parse_page(j).unwrap()).collect()
    }

    #[test]
    fn parse_page_le_hits_e_itens() {
        let p = parse_page(PAGE_JSON).unwrap();
        assert_eq!(p.hits, 392);
        assert_eq!(p.items.len(), 2);
        assert_eq!(p.items[0].identifier, "homologar-di-declaracao-de-importacao");
    }

    #[test]
    fn build_mapeia_campos_e_link_canonico() {
        let ps = pages(&[PAGE_JSON]);
        let items = build_servicos(ps.iter());
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].titulo, "Homologar DI - Declaração de Importação");
        assert_eq!(items[0].descricao, "rEQUISIÇÃO DE AÇÃO FISCAL DE D I AINDA NÃO HOMOLOGADA NO SITRAM");
        assert_eq!(
            items[0].link,
            "https://portalservicos.sefaz.ce.gov.br/servico-geral+homologar-di-declaracao-de-importacao+69459012f6e0ff51fef22a9d"
        );
        assert_eq!(items[0].orgao, "SEFAZ-CE");
        assert_eq!(items[0].ocorrencias.len(), 1);
        assert_eq!(items[0].ocorrencias[0].publico, "Serviços");
        assert_eq!(items[0].ocorrencias[0].classe, "Geral");
        // clean() comprime espaços do segundo item.
        assert_eq!(items[1].titulo, "Impugnar Débito Inscrito");
    }

    #[test]
    fn dedup_por_id_mesmo_id_uma_vez() {
        // O mesmo `_id` em duas páginas vira um único serviço.
        let ps = pages(&[PAGE_JSON, PAGE_JSON]);
        let items = build_servicos(ps.iter());
        assert_eq!(items.len(), 2, "dois _id distintos, mesmo repetidos entre páginas");
    }

    #[test]
    fn identifier_repetido_com_ids_distintos_sao_dois() {
        // A identidade é o `_id` (identifier não é único no catálogo real).
        let json = r#"{"return":[
          {"_id":"id1","identifier":"parcelar-auto-de-infracao","name":"Parcelar Auto A","description":"x"},
          {"_id":"id2","identifier":"parcelar-auto-de-infracao","name":"Parcelar Auto B","description":"y"}
        ]}"#;
        let ps = pages(&[json]);
        let items = build_servicos(ps.iter());
        assert_eq!(items.len(), 2);
        assert_ne!(items[0].link, items[1].link);
    }

    #[test]
    fn identifier_vazio_usa_forma_curta_do_link() {
        let json = r#"{"return":[
          {"_id":"64adca7b48c5b8191406b1d9","identifier":"","name":"SAC","description":"d"}
        ]}"#;
        let ps = pages(&[json]);
        let items = build_servicos(ps.iter());
        assert_eq!(
            items[0].link,
            "https://portalservicos.sefaz.ce.gov.br/+64adca7b48c5b8191406b1d9"
        );
    }

    #[test]
    fn parse_token_extrai_bearer() {
        let shell = r#"...<script>window.SYDLE.config = {"ui-api":{"REQUEST_PARAMS":{"headers":{"Authorization":"Bearer eyJabc.def.ghi_LONG_TOKEN_VALUE"}}}}</script>..."#;
        assert_eq!(parse_token(shell).unwrap(), "Bearer eyJabc.def.ghi_LONG_TOKEN_VALUE");
    }

    #[test]
    fn build_body_envia_sorter_estavel() {
        // Sem sorter válido a paginação do ES é instável (perde itens) — o campo é contrato.
        let body = build_body(3);
        assert_eq!(body["params"]["pageIndex"], 3);
        assert_eq!(
            body["params"]["sorters"]["sort"]["name._current.keyword"], "asc",
            "sorters.sort deve carregar um campo válido e estável"
        );
        assert!(body["params"].get("sort").is_none(), "a chave inválida 'sort' não deve voltar");
    }

    #[test]
    fn validar_aceita_hits_inflado_acima_do_minimo() {
        // `hits` do índice > coletados NÃO reprova (o índice infla ~10; 382 é o real). Desde que
        // acima do mínimo, a coleta segue.
        let dummy = |i: usize| ServicoRaw {
            titulo: format!("S{i}"),
            descricao: String::new(),
            link: format!("l{i}"),
            orgao: ORGAO.into(),
            ocorrencias: vec![],
        };
        let items: Vec<ServicoRaw> = (0..MIN_SERVICOS).map(dummy).collect();
        assert!(
            validar(&items, MIN_SERVICOS as i64 + 50).is_ok(),
            "hits inflado acima do mínimo não deve reprovar"
        );
    }

    #[test]
    fn validar_reprova_catalogo_capado_pelo_minimo() {
        // hits coerente com o coletado, mas abaixo do mínimo: guard de MIN_SERVICOS.
        let poucos = vec![ServicoRaw {
            titulo: "x".into(),
            descricao: String::new(),
            link: "l".into(),
            orgao: ORGAO.into(),
            ocorrencias: vec![],
        }];
        let err = validar(&poucos, 1).unwrap_err().to_string();
        assert!(err.contains("capado"), "esperava guard de mínimo, veio: {err}");
    }
}
