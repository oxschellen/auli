//! `auli-contract` — a forma do dado, compartilhada entre o scraper (`auli-collections`) e o
//! engine (`auli-core`/`auli-cli`).
//!
//! A **struct é a única fonte da verdade**. O scraper compila o dado bruto em uma [`Table<P>`],
//! preenchendo o campo `text_to_embed` de cada registro (a "key" a ser vetorizada). O engine
//! apenas consome: lê a [`Table<P>`], embedda `text_to_embed` e armazena [`Embeddable::stored_repr`].
//! O arquivo `portal-*.txt` deixa de ser contrato e passa a ser um *print* legível da struct
//! (unidirecional: só escrito, nunca lido de volta).
//!
//! Este crate é deliberadamente leve (serde + anyhow/time para o I/O do snapshot — D-C1): nada
//! de embedder, HTTP ou domínio de tributação. É o único ponto onde produtor e consumidor
//! concordam — sobre a forma E sobre o caminho/versão/leitura/escrita da fronteira.

pub mod snapshot;
pub use snapshot::*;

use serde::{Deserialize, Serialize};

/// Envelope genérico de uma tabela. Cada *tipo de tabela* é uma instanciação:
/// `Table<Faq>`, `Table<Servico>`, etc. As tabelas são sempre processadas isoladamente, então o
/// envelope nunca precisa segurar tipos diferentes juntos — daí o genérico (em vez de um enum).
///
/// Persistido como JSON em `data/<id>/raw/<id>-<nome>.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table<P> {
    /// Id da entidade (ex.: `"rs"`).
    pub id: String,
    /// Nome da tabela (ex.: `"faqs"`, `"servicos"`).
    pub nome: String,
    /// Os registros desta tabela.
    pub items: Vec<P>,
}

impl<P> Table<P> {
    /// Cria uma tabela a partir da entidade, do nome e dos registros.
    pub fn new(id: impl Into<String>, nome: impl Into<String>, items: Vec<P>) -> Self {
        Self { id: id.into(), nome: nome.into(), items }
    }

    /// Quantos registros a tabela contém.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Verdadeiro se a tabela não tem registros.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// O que o engine precisa de cada registro `P`, sem conhecer seus campos: a key a embeddar e a
/// representação textual a armazenar (servida ao LLM no contexto do RAG).
///
/// `text_to_embed` é um campo **materializado pelo scraper** — aqui só o expomos; o engine não o
/// recalcula. `stored_repr` é derivado dos campos do registro.
pub trait Embeddable {
    /// A key vetorizada (preenchida na origem pelo scraper).
    fn text_to_embed(&self) -> &str;
    /// O payload textual armazenado junto do vetor (entra no prompt do RAG).
    fn stored_repr(&self) -> String;
}

/// Um registro da tabela `faqs`: um par pergunta/resposta achatado (uma entrada por pergunta),
/// com a trilha de navegação (`origin`) e a URL da página de origem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faq {
    /// Texto da pergunta.
    pub pergunta: String,
    /// Texto da resposta.
    pub resposta: String,
    /// Breadcrumb da página (ex.: `"Inicial | Perguntas Frequentes | ..."`). Pode ser vazio.
    #[serde(default)]
    pub origin: String,
    /// URL canônica da página de origem.
    pub url: String,
    /// Key a embeddar — materializada pelo scraper (para faqs: breadcrumb `origin` + a pergunta).
    pub text_to_embed: String,
}

impl Embeddable for Faq {
    fn text_to_embed(&self) -> &str {
        &self.text_to_embed
    }

    /// Reproduz o bloco `## pergunta` / `## resposta` (mesma forma do antigo `portal-faqs.txt`),
    /// para o contexto do RAG continuar coerente.
    fn stored_repr(&self) -> String {
        let mut s = String::from("## pergunta\n");
        if !self.origin.is_empty() {
            s.push_str(&self.origin);
            s.push('\n');
        }
        s.push_str(&self.pergunta);
        s.push_str("\n\n## resposta\n");
        s.push_str(&self.resposta);
        s.push_str(&format!("\nLink: {}", self.url));
        s
    }
}

/// Um registro da tabela `servicos`. Campos do serviço raspado, mais a key materializada.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Servico {
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
    /// Descrição do serviço (corpo da página de detalhe).
    pub descricao: String,
    /// Key a embeddar — preenchida pelo scraper.
    pub text_to_embed: String,
}

impl Embeddable for Servico {
    fn text_to_embed(&self) -> &str {
        &self.text_to_embed
    }

    /// Reproduz o bloco `## pergunta` / `## resposta` no mesmo formato dos demais kinds: breadcrumb
    /// `tipo | classe` + título no `## pergunta`, descrição + link no `## resposta`.
    fn stored_repr(&self) -> String {
        format!(
            "## pergunta\n{} | {}\n{}\n\n## resposta\n{}\nLink: {}",
            self.tipo, self.classe, self.titulo, self.descricao, self.link
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_faq() -> Faq {
        Faq {
            pergunta: "Como emitir nota?".into(),
            resposta: "Acesse o portal.".into(),
            origin: "Inicial | FAQ".into(),
            url: "https://exemplo/faq/1".into(),
            text_to_embed: "Como emitir nota?".into(),
        }
    }

    #[test]
    fn faq_table_roundtrips_through_json() {
        let table = Table::new("rs", "faqs", vec![sample_faq()]);
        let json = serde_json::to_string(&table).unwrap();
        let back: Table<Faq> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "rs");
        assert_eq!(back.nome, "faqs");
        assert_eq!(back.len(), 1);
        assert_eq!(back.items[0].pergunta, "Como emitir nota?");
    }

    #[test]
    fn embeddable_exposes_key_and_renders_block() {
        let faq = sample_faq();
        assert_eq!(faq.text_to_embed(), "Como emitir nota?");
        let block = faq.stored_repr();
        assert!(block.starts_with("## pergunta\nInicial | FAQ\nComo emitir nota?"));
        assert!(block.contains("## resposta\nAcesse o portal."));
        assert!(block.contains("Link: https://exemplo/faq/1"));
    }

    #[test]
    fn servico_block_has_breadcrumb_and_link() {
        let s = Servico {
            id: 1,
            tipo: "Empresas".into(),
            classe: "ICMS".into(),
            orgao: "SEFAZ".into(),
            link: "https://exemplo/svc/1".into(),
            titulo: "Emitir guia".into(),
            descricao: "Passos para emitir a guia.".into(),
            text_to_embed: "Empresas | ICMS Emitir guia".into(),
        };
        let block = s.stored_repr();
        assert!(block.starts_with("## pergunta\nEmpresas | ICMS\nEmitir guia"));
        assert!(block.contains("Link: https://exemplo/svc/1"));
    }
}
