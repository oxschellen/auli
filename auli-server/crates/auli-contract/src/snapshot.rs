//! `snapshot` — a fronteira **scraper → collections**.
//!
//! O *snapshot de coleta* é gravado por cada scraper **um arquivo por coleção**:
//! `data/<id>/<id>-servicos-snapshot.json` e (só o RS) `data/<id>/<id>-faqs-snapshot.json`. O
//! `auli-collections process` os lê e deriva todos os artefatos ([`crate::Table<P>`], per-público
//! JSONs, `portal-*.txt`). Assim a coleta (rede) fica separada do processamento (offline).
//!
//! Duas invariantes desenham o tipo (D-S1/D-S2 da TAREFA):
//!
//! - O snapshot é a fronteira scraper→collections; a [`crate::Table<P>`] segue sendo a fronteira
//!   collections→engine. Ambos vivem aqui, no crate magro.
//! - O snapshot carrega dado **bruto porém limpo**: texto já normalizado, mas **sem campos
//!   derivados** — sem `id` sequencial e sem `text_to_embed`. Quem deriva é o `process`.
//!
//! Um arquivo por coleção (v3): cada scrape grava só o seu arquivo, sem merge nem read-modify-write
//! do arquivo do vizinho — nomes honestos, sem corrida entre scrapes, raio de dano isolado.
//!
//! Desde a D-C1, este módulo carrega também o **I/O** da fronteira ([`load`]/[`write_faqs`]/
//! [`write_servicos`]): forma, versão ([`SNAPSHOT_SCHEMA_VERSION`]), convenção de caminho
//! (`<id>-<kind>-snapshot.json`) e leitura/escrita moram juntos — produtor (scrapers) e consumidor
//! (`process`) usam exatamente as mesmas funções, então nada disso pode derivar. O [`load`] valida
//! `schema_version`/`entidade` pelo header **antes** da desserialização tipada (mensagem amigável
//! em vez de erro cru do serde); o write carimba `coletado_em` (RFC 3339, UTC).

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Versão do schema do snapshot. O produtor grava; o `process` compara contra esta constante e
/// emite erro amigável se não bater. Bump quando o formato mudar de forma incompatível.
///
/// v2 (fase 2): `ServicoRaw` troca `classe`/`publicos` por `ocorrencias` (par público×classe).
/// v3: **um arquivo por coleção** ([`CollectionSnapshot`]) no lugar do `Snapshot` com `colecoes`
/// mesclado. Sem migração automática — o snapshot é regenerável do cache.
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 3;

/// O snapshot de **uma** coleção de uma entidade. Persistido como JSON em
/// `data/<id>/<id>-<kind>-snapshot.json` (kind ∈ {`servicos`, `faqs`)}. `C` é a coleta concreta
/// ([`ColetaServicos`] ou [`ColetaFaqs`]); o `kind` é o nome do arquivo, não um campo — o tipo já
/// discrimina a coleção.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSnapshot<C> {
    /// Versão do schema (ver [`SNAPSHOT_SCHEMA_VERSION`]).
    pub schema_version: u32,
    /// Id da entidade (ex.: `"rs"`).
    pub entidade: String,
    /// Quem gerou o snapshot.
    pub scraper: ScraperInfo,
    /// A coleta desta coleção.
    pub coleta: C,
}

/// Identificação do scraper que gravou o snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperInfo {
    /// Nome do scraper (ex.: `"auli-collections"`).
    pub nome: String,
    /// Versão do scraper (ex.: `CARGO_PKG_VERSION`).
    pub versao: String,
}

/// A coleta de faqs: quando foi raspada e os registros brutos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColetaFaqs {
    /// Instante da coleta, RFC 3339 (ex.: `"2026-07-01T09:14:00-03:00"`).
    pub coletado_em: String,
    /// Os registros brutos de faq.
    pub items: Vec<FaqRaw>,
}

/// A coleta de serviços: quando foi raspada, a ordem dos públicos e os registros brutos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColetaServicos {
    /// Instante da coleta, RFC 3339.
    pub coletado_em: String,
    /// Ordem de exibição dos públicos (gera o `servicos-index.json` e desempata o público
    /// primário de cada serviço).
    pub publicos_ordem: Vec<Publico>,
    /// Os registros brutos de serviço.
    pub items: Vec<ServicoRaw>,
}

/// Um público-alvo dos serviços (uma aba do portal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Publico {
    /// Nome legível (ex.: `"Cidadãos"`).
    pub nome: String,
    /// Slug do arquivo per-público, sem prefixo de entidade (ex.: `"servicos-ao-cidadao"`) — a pasta
    /// `data/<id>/` (e `public/<id>/`) já provê o escopo por entidade.
    pub slug: String,
}

/// Um registro bruto de faq no snapshot — o par pergunta/resposta já achatado, **sem** a key
/// `text_to_embed` (o `process` a materializa a partir de `origin` + `pergunta`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaqRaw {
    /// Texto da pergunta.
    pub pergunta: String,
    /// Texto da resposta.
    pub resposta: String,
    /// Breadcrumb da página (ex.: `"Inicial | Perguntas Frequentes | ..."`). Pode ser vazio.
    #[serde(default)]
    pub origin: String,
    /// URL canônica da página de origem.
    pub url: String,
}

/// Um registro bruto de serviço no snapshot. Um serviço = **um** registro (chave `link`), com todas
/// as suas `ocorrencias` no portal (par público×classe, na ordem de descoberta). O dedup por link e
/// o fan-out per-público são do `process` (D-S3/D-F2.3). **Sem** `id`, `tipo`, `classe` primária ou
/// `text_to_embed` — todos derivados no `process`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicoRaw {
    /// Título legível.
    pub titulo: String,
    /// Corpo limpo da descrição (sem o header `tipo/classe/titulo`).
    pub descricao: String,
    /// URL do serviço — a chave natural do snapshot, única para os scrapers que passam por
    /// `aggregate_servicos` (dedup por link). Exceção deliberada: o SP monta os `ServicoRaw` direto
    /// e vários serviços compartilham a URL de login, então lá o link não é único.
    pub link: String,
    /// Órgão de origem.
    pub orgao: String,
    /// Onde o serviço aparece no portal, na ordem de descoberta. Um serviço listado sob duas classes
    /// (mesma ou públicos diferentes) tem uma [`Ocorrencia`] por listagem.
    pub ocorrencias: Vec<Ocorrencia>,
}

/// Uma listagem do serviço no portal: sob qual `publico` e sob qual `classe`. Preserva o caso
/// multi-classe que o schema v1 perdia (D-F2.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ocorrencia {
    /// Nome do público onde o serviço foi listado (refere-se a [`Publico::nome`]).
    pub publico: String,
    /// Classe/grupo sob a qual foi listado nesse público.
    pub classe: String,
}

/// Um serviço raspado de **um** público — a entrada do `aggregate_servicos` do kit dos scrapers
/// e também o shape dos **JSONs per-público** que o `process` grava (e o frontend consome): por
/// isso é contrato, não infra de scraper. Na **entrada** do scrape, `descricao` carrega o header
/// `tipo/classe/titulo` que o `descricao_body` (kit) remove ao materializar o corpo limpo do
/// snapshot; nos JSONs per-público gravados pelo `process`, `descricao` já é esse **corpo limpo**.
///
/// Não confundir com os irmãos: [`crate::Servico`] é o registro RAG/[`crate::Embeddable`] (com
/// `text_to_embed`); [`ServicoRaw`] é o item do snapshot (um por link, com `ocorrencias`).
#[derive(Serialize, Deserialize, Debug)]
pub struct ServicoPerPublico {
    /// Id sequencial por arquivo (começa em 1). Não é globalmente único — use `link` para isso.
    pub id: usize,
    /// Público/categoria (ex.: `"Cidadãos"`, `"Empresas"`).
    pub tipo: String,
    /// Classe/grupo do serviço (do título do card).
    pub classe: String,
    /// Órgão de origem.
    pub orgao: String,
    /// URL do serviço.
    pub link: String,
    /// Título legível.
    pub titulo: String,
    /// Descrição do serviço (na entrada do scrape, com o header `tipo/classe/titulo`; nos JSONs
    /// per-público derivados, o corpo limpo).
    pub descricao: String,
}

// ---------------------------------------------------------------------------
// I/O da fronteira (D-C1) — usado pelos scrapers (write) e pelo `process` (load).
// ---------------------------------------------------------------------------

/// Caminho do snapshot de uma coleção: `../data/<id>/<id>-<kind>-snapshot.json`, irmão de `raw/`. O
/// `data_dir` aponta para `.../<id>/raw`, então subimos um nível.
fn snapshot_path(id: &str, data_dir: &str, kind: &str) -> PathBuf {
    let base = Path::new(data_dir).parent().unwrap_or_else(|| Path::new(data_dir));
    base.join(format!("{}-{}-snapshot.json", id, kind))
}

/// Instante atual em RFC 3339 (UTC). Metadado de auditoria; não entra em nenhum artefato derivado.
fn now_rfc3339() -> String {
    use time::OffsetDateTime;
    use time::format_description::well_known::Rfc3339;
    OffsetDateTime::now_utc().format(&Rfc3339).unwrap_or_default()
}

/// Grava a coleta de faqs no arquivo `<id>-faqs-snapshot.json`. `scraper` identifica quem gravou.
pub fn write_faqs(id: &str, data_dir: &str, scraper: &ScraperInfo, items: Vec<FaqRaw>) -> Result<()> {
    let coleta = ColetaFaqs { coletado_em: now_rfc3339(), items };
    save(id, data_dir, "faqs", scraper, coleta)
}

/// Grava a coleta de serviços no arquivo `<id>-servicos-snapshot.json`.
pub fn write_servicos(
    id: &str,
    data_dir: &str,
    scraper: &ScraperInfo,
    publicos_ordem: Vec<Publico>,
    items: Vec<ServicoRaw>,
) -> Result<()> {
    let coleta = ColetaServicos { coletado_em: now_rfc3339(), publicos_ordem, items };
    save(id, data_dir, "servicos", scraper, coleta)
}

/// Serializa o snapshot de uma coleção no seu arquivo próprio — **sem merge**, sobrescrevendo apenas
/// o arquivo daquela coleção.
fn save<C: Serialize>(
    id: &str,
    data_dir: &str,
    kind: &str,
    scraper: &ScraperInfo,
    coleta: C,
) -> Result<()> {
    let snapshot = CollectionSnapshot {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        entidade: id.to_string(),
        scraper: scraper.clone(),
        coleta,
    };
    let path = snapshot_path(id, data_dir, kind);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&snapshot)?)?;
    println!("Wrote {} (snapshot)", path.display());
    Ok(())
}

/// Prefixo mínimo lido **antes** da desserialização tipada. Um snapshot de schema incompatível (ex.:
/// o v2 mesclado com `colecoes`) falharia com um erro cru do serde ao desserializar o
/// `CollectionSnapshot` inteiro; lendo só o header primeiro conseguimos dar a mensagem amigável.
#[derive(serde::Deserialize)]
struct SnapshotHeader {
    schema_version: u32,
    entidade: String,
}

/// Valida `schema_version` (contra [`SNAPSHOT_SCHEMA_VERSION`]) e `entidade` a partir do header,
/// antes da desserialização tipada.
fn check_header(bytes: &[u8], id: &str) -> Result<()> {
    let header: SnapshotHeader = serde_json::from_slice(bytes)
        .map_err(|e| anyhow::anyhow!("snapshot ilegível (nem o header desserializa): {e}"))?;
    if header.schema_version != SNAPSHOT_SCHEMA_VERSION {
        anyhow::bail!(
            "snapshot na versão de schema v{} (esperado v{}). Re-raspe a entidade — o snapshot é \
             regenerável do cache, não há migração.",
            header.schema_version,
            SNAPSHOT_SCHEMA_VERSION
        );
    }
    if header.entidade != id {
        anyhow::bail!("entidade do snapshot ('{}') não bate com a pedida ('{}').", header.entidade, id);
    }
    Ok(())
}

/// Carrega o snapshot de uma coleção (`<id>-<kind>-snapshot.json`), ou `None` se o arquivo ainda não
/// existe. `C` é a coleta concreta esperada para o `kind` (o chamador escolhe: `ColetaServicos` para
/// `"servicos"`, `ColetaFaqs` para `"faqs"`). Valida `schema_version`/`entidade` antes de desserializar.
pub fn load<C: DeserializeOwned>(
    id: &str,
    data_dir: &str,
    kind: &str,
) -> Result<Option<CollectionSnapshot<C>>> {
    let path = snapshot_path(id, data_dir, kind);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path)?;
    check_header(&bytes, id)?;
    Ok(Some(serde_json::from_slice(&bytes)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_servicos() -> CollectionSnapshot<ColetaServicos> {
        CollectionSnapshot {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            entidade: "rs".into(),
            scraper: ScraperInfo { nome: "auli-collections".into(), versao: "0.1.0".into() },
            coleta: ColetaServicos {
                coletado_em: "2026-07-01T10:02:00-03:00".into(),
                publicos_ordem: vec![
                    Publico { nome: "Cidadãos".into(), slug: "servicos-ao-cidadao".into() },
                    Publico { nome: "Empresas".into(), slug: "servicos-a-empresas".into() },
                ],
                items: vec![ServicoRaw {
                    titulo: "Emitir guia".into(),
                    descricao: "Corpo limpo.".into(),
                    link: "https://exemplo/svc/1".into(),
                    orgao: "SEFAZ".into(),
                    ocorrencias: vec![
                        Ocorrencia { publico: "Empresas".into(), classe: "ICMS".into() },
                        Ocorrencia { publico: "Cidadãos".into(), classe: "ICMS".into() },
                    ],
                }],
            },
        }
    }

    #[test]
    fn servicos_snapshot_roundtrips_through_json() {
        let snap = sample_servicos();
        let json = serde_json::to_string(&snap).unwrap();
        let back: CollectionSnapshot<ColetaServicos> = serde_json::from_str(&json).unwrap();

        assert_eq!(back.schema_version, SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(back.entidade, "rs");
        assert_eq!(back.scraper.nome, "auli-collections");
        assert_eq!(back.coleta.publicos_ordem.len(), 2);
        assert_eq!(back.coleta.items[0].ocorrencias.len(), 2);
        assert_eq!(back.coleta.items[0].ocorrencias[0].publico, "Empresas");
        assert_eq!(back.coleta.items[0].ocorrencias[0].classe, "ICMS");
    }

    #[test]
    fn faqs_snapshot_roundtrips_through_json() {
        let snap = CollectionSnapshot {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            entidade: "rs".into(),
            scraper: ScraperInfo { nome: "auli-collections".into(), versao: "0.1.0".into() },
            coleta: ColetaFaqs {
                coletado_em: "2026-07-01T09:14:00-03:00".into(),
                items: vec![FaqRaw {
                    pergunta: "Como emitir a guia?".into(),
                    resposta: "Acesse o portal.".into(),
                    origin: "Inicial | FAQ".into(),
                    url: "https://exemplo/faq/1".into(),
                }],
            },
        };
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"schema_version\":3"));
        let back: CollectionSnapshot<ColetaFaqs> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.coleta.items.len(), 1);
        assert_eq!(back.coleta.items[0].pergunta, "Como emitir a guia?");
    }

    // ---- I/O (movidos do kit dos scrapers na D-C1) ----

    fn tmp_data_dir(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("auli_snap_test_{}_{}", std::process::id(), tag));
        p.push("raw");
        p
    }

    fn scraper() -> ScraperInfo {
        ScraperInfo { nome: "test".into(), versao: "0".into() }
    }

    fn faq(pergunta: &str) -> FaqRaw {
        FaqRaw { pergunta: pergunta.into(), resposta: "r".into(), origin: String::new(), url: "u".into() }
    }

    fn svc(link: &str) -> ServicoRaw {
        ServicoRaw {
            titulo: "t".into(),
            descricao: "d".into(),
            link: link.into(),
            orgao: "o".into(),
            ocorrencias: vec![Ocorrencia { publico: "Cidadãos".into(), classe: "c".into() }],
        }
    }

    #[test]
    fn each_collection_writes_its_own_file_and_leaves_the_other_untouched() {
        let data_dir = tmp_data_dir("split");
        let dd = data_dir.to_str().unwrap();
        let sc = scraper();

        write_faqs("rs", dd, &sc, vec![faq("q1")]).unwrap();
        write_servicos(
            "rs",
            dd,
            &sc,
            vec![Publico { nome: "Cidadãos".into(), slug: "servicos-ao-cidadao".into() }],
            vec![svc("l1")],
        )
        .unwrap();

        let faqs_path = snapshot_path("rs", dd, "faqs");
        let servicos_path = snapshot_path("rs", dd, "servicos");
        assert!(faqs_path.exists(), "faqs snapshot file must exist");
        assert!(servicos_path.exists(), "servicos snapshot file must exist");

        // Re-writing servicos must NOT touch the faqs file (the invariant the old merge protected).
        let faqs_before = std::fs::read(&faqs_path).unwrap();
        write_servicos(
            "rs",
            dd,
            &sc,
            vec![Publico { nome: "Cidadãos".into(), slug: "servicos-ao-cidadao".into() }],
            vec![svc("l2")],
        )
        .unwrap();
        assert_eq!(
            std::fs::read(&faqs_path).unwrap(),
            faqs_before,
            "a servicos write must not touch the faqs snapshot file"
        );

        // Each loads back as its own typed coleta.
        let faqs: CollectionSnapshot<ColetaFaqs> = load("rs", dd, "faqs").unwrap().unwrap();
        assert_eq!(faqs.coleta.items[0].pergunta, "q1");
        let servicos: CollectionSnapshot<ColetaServicos> = load("rs", dd, "servicos").unwrap().unwrap();
        assert_eq!(servicos.coleta.items[0].link, "l2");

        let _ = std::fs::remove_dir_all(data_dir.parent().unwrap());
    }

    #[test]
    fn load_missing_file_is_none() {
        let data_dir = tmp_data_dir("missing");
        let dd = data_dir.to_str().unwrap();
        let got: Option<CollectionSnapshot<ColetaServicos>> = load("rs", dd, "servicos").unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn load_rejects_old_schema_with_friendly_message() {
        let data_dir = tmp_data_dir("oldschema");
        let dd = data_dir.to_str().unwrap();
        let path = snapshot_path("rs", dd, "servicos");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // An old v2 (merged `colecoes`) file at the new path: the header check must fire on the
        // version before the typed body — with the "re-raspe" message, not a raw serde error.
        std::fs::write(&path, r#"{"schema_version":2,"entidade":"rs","colecoes":{}}"#).unwrap();

        let err = load::<ColetaServicos>("rs", dd, "servicos").unwrap_err().to_string();
        assert!(err.contains("Re-raspe"), "esperava mensagem amigável, veio: {err}");

        let _ = std::fs::remove_dir_all(data_dir.parent().unwrap());
    }

    // ---- ServicoPerPublico: a forma serializada é contrato (frontend consome) ----

    #[test]
    fn servico_per_publico_serializes_with_stable_field_names() {
        let s = ServicoPerPublico {
            id: 1,
            tipo: "Cidadãos".into(),
            classe: "ICMS".into(),
            orgao: "SEFAZ".into(),
            link: "https://exemplo/svc/1".into(),
            titulo: "Emitir guia".into(),
            descricao: "corpo".into(),
        };
        let json = serde_json::to_string(&s).unwrap();
        // O rename do tipo (kit::Servico -> ServicoPerPublico) NÃO pode mudar o JSON: os arquivos
        // per-público existentes têm que continuar byte a byte (o golden do collections é o juiz).
        for campo in ["\"id\":1", "\"tipo\":", "\"classe\":", "\"orgao\":", "\"link\":", "\"titulo\":", "\"descricao\":"] {
            assert!(json.contains(campo), "campo esperado ausente no JSON: {campo} em {json}");
        }
    }
}
