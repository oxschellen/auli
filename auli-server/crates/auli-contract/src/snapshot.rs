//! `snapshot` — a fronteira **scraper → collections**.
//!
//! O *snapshot de coleta* é o arquivo único e padronizado por entidade
//! (`data/<id>/<id>-snapshot.json`) que cada scraper grava. O `auli-collections process`
//! o lê e deriva todos os artefatos ([`crate::Table<P>`], per-público JSONs, `portal-*.txt`).
//! Assim a coleta (rede) fica separada do processamento (offline).
//!
//! Duas invariantes desenham o tipo (D-S1/D-S2 da TAREFA):
//!
//! - O snapshot é a fronteira scraper→collections; a [`crate::Table<P>`] segue sendo a fronteira
//!   collections→engine. Ambos vivem aqui, no crate magro.
//! - O snapshot carrega dado **bruto porém limpo**: texto já normalizado, mas **sem campos
//!   derivados** — sem `id` sequencial e sem `text_to_embed`. Quem deriva é o `process`.
//!
//! Como o resto do crate, este módulo é só `serde`: não valida nada. O consumidor (o `process`,
//! etapa C) é quem compara [`SNAPSHOT_SCHEMA_VERSION`] e reclama de versão desconhecida — mesmo
//! precedente da [`crate::Table<P>`].

use serde::{Deserialize, Serialize};

/// Versão do schema do snapshot. O produtor grava; o `process` compara contra esta constante e
/// emite erro amigável se não bater. Bump quando o formato mudar de forma incompatível.
///
/// v2 (fase 2): `ServicoRaw` troca `classe`/`publicos` por `ocorrencias` (par público×classe),
/// para representar serviços listados sob mais de uma classe no portal. Sem migração — o snapshot é
/// regenerável do cache.
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 2;

/// O snapshot de coleta de uma entidade. Persistido como JSON em `data/<id>/<id>-snapshot.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Versão do schema (ver [`SNAPSHOT_SCHEMA_VERSION`]).
    pub schema_version: u32,
    /// Id da entidade (ex.: `"rs"`).
    pub entidade: String,
    /// Quem gerou o snapshot.
    pub scraper: ScraperInfo,
    /// As coleções raspadas. Cada uma é opcional (ver [`Colecoes`]).
    pub colecoes: Colecoes,
}

/// Identificação do scraper que gravou o snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperInfo {
    /// Nome do scraper (ex.: `"auli-collections"`).
    pub nome: String,
    /// Versão do scraper (ex.: `CARGO_PKG_VERSION`).
    pub versao: String,
}

/// As coleções de um snapshot. Cada scrape atualiza **só a sua** coleção e preserva a outra
/// (merge, não overwrite do arquivo inteiro), por isso os campos são `Option` e uma coleção
/// ausente serializa como omitida. `Default` habilita o `load_or_default` da etapa B.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Colecoes {
    /// Coleção de faqs, se presente.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faqs: Option<ColetaFaqs>,
    /// Coleção de serviços, se presente.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub servicos: Option<ColetaServicos>,
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
    /// URL do serviço — a chave natural única do snapshot.
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

    fn sample_snapshot() -> Snapshot {
        Snapshot {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            entidade: "rs".into(),
            scraper: ScraperInfo { nome: "auli-collections".into(), versao: "0.1.0".into() },
            colecoes: Colecoes {
                faqs: Some(ColetaFaqs {
                    coletado_em: "2026-07-01T09:14:00-03:00".into(),
                    items: vec![FaqRaw {
                        pergunta: "Como emitir a guia?".into(),
                        resposta: "Acesse o portal.".into(),
                        origin: "Inicial | FAQ".into(),
                        url: "https://exemplo/faq/1".into(),
                    }],
                }),
                servicos: Some(ColetaServicos {
                    coletado_em: "2026-07-01T10:02:00-03:00".into(),
                    publicos_ordem: vec![
                        Publico { nome: "Cidadãos".into(), slug: "rs-servicos-ao-cidadao".into() },
                        Publico { nome: "Empresas".into(), slug: "rs-servicos-a-empresas".into() },
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
                }),
            },
        }
    }

    #[test]
    fn snapshot_roundtrips_through_json() {
        let snap = sample_snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        let back: Snapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(back.schema_version, SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(back.entidade, "rs");
        assert_eq!(back.scraper.nome, "auli-collections");

        let faqs = back.colecoes.faqs.unwrap();
        assert_eq!(faqs.items.len(), 1);
        assert_eq!(faqs.items[0].pergunta, "Como emitir a guia?");

        let servicos = back.colecoes.servicos.unwrap();
        assert_eq!(servicos.publicos_ordem.len(), 2);
        assert_eq!(servicos.items[0].ocorrencias.len(), 2);
        assert_eq!(servicos.items[0].ocorrencias[0].publico, "Empresas");
        assert_eq!(servicos.items[0].ocorrencias[0].classe, "ICMS");
    }

    #[test]
    fn missing_collection_deserializes_as_none() {
        let json = r#"{
            "schema_version": 1,
            "entidade": "rs",
            "scraper": { "nome": "auli-collections", "versao": "0.1.0" },
            "colecoes": {
                "faqs": { "coletado_em": "2026-07-01T09:14:00-03:00", "items": [] }
            }
        }"#;
        let snap: Snapshot = serde_json::from_str(json).unwrap();
        assert!(snap.colecoes.faqs.is_some());
        assert!(snap.colecoes.servicos.is_none());
    }

    #[test]
    fn schema_version_present_and_none_collection_omitted() {
        let snap = Snapshot {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            entidade: "rs".into(),
            scraper: ScraperInfo { nome: "auli-collections".into(), versao: "0.1.0".into() },
            colecoes: Colecoes {
                faqs: Some(ColetaFaqs { coletado_em: "2026-07-01T09:14:00-03:00".into(), items: vec![] }),
                servicos: None,
            },
        };
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"schema_version\":2"));
        assert!(!json.contains("servicos"));
    }
}
