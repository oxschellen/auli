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
//! Como o resto do crate, este módulo é só `serde`: não valida nada. O consumidor (o `process`,
//! etapa C) é quem compara [`SNAPSHOT_SCHEMA_VERSION`] e reclama de versão desconhecida — mesmo
//! precedente da [`crate::Table<P>`].

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
}
