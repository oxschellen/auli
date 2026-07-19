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

pub mod mddoc;
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

/// Proveniência da sinopse gerada pelo passo `auli-sinopse` (fase posterior).
/// `None` = registro sem sinopse gerada: pendente (resumo vazio) ou sumário autorado legado
/// (resumo preenchido na origem, caso RS antigo).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SinopseInfo {
    /// Modelo que gerou a sinopse (ex.: `"llama-3.3-70b-versatile"`).
    pub modelo: String,
    /// Versão do prompt do auli-sinopse (const `SINOPSE_PROMPT_VERSION` no gerador).
    pub prompt_versao: u32,
    /// Instante da geração, ISO-8601 (ex.: `"2026-07-18T14:00:00Z"`).
    pub gerada_em: String,
}

/// Um registro da tabela `pareceres`: uma consulta tributária respondida (parecer/resposta de
/// consulta — termo geral entre estados: "Pareceres" no RS, "Respostas de Consultas" em SP, COPAT
/// em SC). Os campos vêm do conteúdo autorado (o sumário dá `numero`/`assunto`/`resumo`; `corpo` é
/// o texto integral), mais a key materializada.
///
/// (Antes: `Parecer`.) O **kind de domínio permanece `"pareceres"`** por compatibilidade — rota
/// `/v1/{kind}/list`, sufixo da coleção vetorial, nomes de pack e labels não mudam; o nome da
/// struct não aparece na serialização (serde grava só os campos).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Consulta {
    /// Identificador do parecer (ex.: `"PARECER Nº 25148"`).
    pub numero: String,
    /// Assunto/ementa (uma linha).
    pub assunto: String,
    /// Resumo do parecer (descrição resumida + palavras-chave do sumário autorado). Pode ser vazio.
    #[serde(default)]
    pub resumo: String,
    /// Corpo integral do parecer.
    pub corpo: String,
    /// URL do parecer na legislação.
    pub link: String,
    /// Key a embeddar — preenchida na origem (para pareceres: `assunto` + `resumo`).
    pub text_to_embed: String,
    /// Proveniência da sinopse (ver [`SinopseInfo`]). Ausente no JSON quando `None` —
    /// snapshots antigos continuam válidos sem migração.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sinopse_info: Option<SinopseInfo>,
}

impl Embeddable for Consulta {
    fn text_to_embed(&self) -> &str {
        &self.text_to_embed
    }

    /// Reproduz o bloco `## pergunta` / `## resposta` no mesmo formato dos demais kinds: `numero` +
    /// `assunto` no `## pergunta`, corpo integral + link no `## resposta`.
    fn stored_repr(&self) -> String {
        format!(
            "## pergunta\n{}\n{}\n\n## resposta\n{}\nLink: {}",
            self.numero, self.assunto, self.corpo, self.link
        )
    }
}

/// Payload de pack de pareceres (G3): tudo MENOS o corpo, que vive na árvore `docs/` e é lido na
/// query. `doc_path` é relativo ao diretório da entidade (ex.: `"docs/pareceres/<slug>.md"`).
///
/// Ainda NÃO é o que `Consulta::stored_repr` grava — o pack segue gordo até a fiação da G3 (pack e
/// servidor mudam em lockstep, senão o serving renderiza lixo).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsultaPackPayload {
    pub numero: String,
    pub assunto: String,
    pub resumo: String,
    pub link: String,
    pub doc_path: String,
}

/// Renderiza o bloco de contexto de uma consulta a partir do payload leve + corpo lido da árvore.
///
/// MESMO formato do `stored_repr` gordo, byte a byte — é esse o invariante da G3 (o contexto RAG
/// montado não pode mudar). Ponto único: servidor e testes passam por aqui.
pub fn render_consulta_block(p: &ConsultaPackPayload, corpo: &str) -> String {
    format!("## pergunta\n{}\n{}\n\n## resposta\n{}\nLink: {}", p.numero, p.assunto, corpo, p.link)
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

    #[test]
    fn parecer_exposes_key_and_renders_block() {
        let p = Consulta {
            numero: "PARECER Nº 25148".into(),
            assunto: "ICMS – crédito fiscal na cesta básica".into(),
            resumo: "Análise sobre apropriação de crédito.".into(),
            corpo: "É o parecer.".into(),
            link: "https://exemplo/parecer/25148".into(),
            text_to_embed: "ICMS – crédito fiscal na cesta básica\nAnálise sobre apropriação de crédito.".into(),
            sinopse_info: None,
        };
        assert_eq!(p.text_to_embed(), "ICMS – crédito fiscal na cesta básica\nAnálise sobre apropriação de crédito.");
        let block = p.stored_repr();
        assert!(block.starts_with("## pergunta\nPARECER Nº 25148\nICMS – crédito fiscal na cesta básica"));
        assert!(block.contains("## resposta\nÉ o parecer."));
        assert!(block.contains("Link: https://exemplo/parecer/25148"));

        // The serialized shape is contract; round-trips through JSON.
        let table = Table::new("rs", "pareceres", vec![p]);
        let json = serde_json::to_string(&table).unwrap();
        let back: Table<Consulta> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.items[0].numero, "PARECER Nº 25148");
    }

    fn sample_consulta() -> Consulta {
        Consulta {
            numero: "PARECER Nº 25148".into(),
            assunto: "ICMS – crédito fiscal na cesta básica".into(),
            resumo: "Análise sobre apropriação de crédito.".into(),
            corpo: "É o parecer.".into(),
            link: "https://exemplo/parecer/25148".into(),
            text_to_embed: "ICMS – crédito fiscal na cesta básica".into(),
            sinopse_info: None,
        }
    }

    /// Golden da G3: o `stored_repr` gordo de HOJE, reimplementado aqui de propósito. Se o formato
    /// mudar de um lado só, este teste quebra — é ele que garante que trocar o pack gordo pelo
    /// payload leve + leitura tardia não mexe um byte no contexto RAG.
    fn stored_repr_gordo_golden(c: &Consulta) -> String {
        format!(
            "## pergunta\n{}\n{}\n\n## resposta\n{}\nLink: {}",
            c.numero, c.assunto, c.corpo, c.link
        )
    }

    fn payload_de(c: &Consulta, doc_path: &str) -> ConsultaPackPayload {
        ConsultaPackPayload {
            numero: c.numero.clone(),
            assunto: c.assunto.clone(),
            resumo: c.resumo.clone(),
            link: c.link.clone(),
            doc_path: doc_path.into(),
        }
    }

    #[test]
    fn render_do_payload_leve_equivale_ao_stored_repr_gordo() {
        let c = sample_consulta();
        let p = payload_de(&c, "docs/pareceres/parecer-no-25148.md");
        assert_eq!(render_consulta_block(&p, &c.corpo), stored_repr_gordo_golden(&c));
        // E ao que o pack grava hoje, de fato (não só ao golden).
        assert_eq!(render_consulta_block(&p, &c.corpo), c.stored_repr());
    }

    #[test]
    fn render_equivale_com_corpo_multilinha_e_ancoras_no_meio() {
        // Corpo com as próprias âncoras no meio: é só concatenação, nada a escapar.
        let mut c = sample_consulta();
        c.corpo = "Preâmbulo.\n\n## resposta\nRecursivo.\nLink: falso\n## corpo\nfim".into();
        let p = payload_de(&c, "docs/pareceres/x.md");
        assert_eq!(render_consulta_block(&p, &c.corpo), stored_repr_gordo_golden(&c));
    }

    #[test]
    fn render_com_corpo_vazio_nao_perde_o_link() {
        let c = sample_consulta();
        let p = payload_de(&c, "docs/pareceres/x.md");
        let bloco = render_consulta_block(&p, "");
        assert!(bloco.ends_with(&format!("\nLink: {}", c.link)), "bloco: {bloco:?}");
    }

    #[test]
    fn payload_faz_round_trip_por_json() {
        let p = payload_de(&sample_consulta(), "docs/pareceres/parecer-no-25148.md");
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<ConsultaPackPayload>(&json).unwrap(), p);
    }

    #[test]
    fn payload_leve_nao_carrega_o_corpo() {
        // O ganho da fase: o corpo não pode vazar para o pack por nenhum campo.
        let mut c = sample_consulta();
        c.corpo = "MARCADOR-DE-CORPO-UNICO".into();
        let json = serde_json::to_string(&payload_de(&c, "docs/pareceres/x.md")).unwrap();
        assert!(!json.contains("MARCADOR-DE-CORPO-UNICO"), "corpo vazou para o payload: {json}");
    }

    #[test]
    fn consulta_with_sinopse_roundtrips_through_json() {
        let mut c = sample_consulta();
        c.sinopse_info = Some(SinopseInfo {
            modelo: "llama-3.3-70b-versatile".into(),
            prompt_versao: 1,
            gerada_em: "2026-07-18T14:00:00Z".into(),
        });
        let json = serde_json::to_string(&c).unwrap();
        let back: Consulta = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn consulta_json_sem_sinopse_desserializa_como_none() {
        // Forma dos snapshots atuais — sem o campo `sinopse_info`. Guarda-corpo da migração zero.
        let json = r#"{
            "numero": "PARECER Nº 25148",
            "assunto": "ICMS – crédito fiscal na cesta básica",
            "resumo": "Análise sobre apropriação de crédito.",
            "corpo": "É o parecer.",
            "link": "https://exemplo/parecer/25148",
            "text_to_embed": "ICMS – crédito fiscal na cesta básica"
        }"#;
        let c: Consulta = serde_json::from_str(json).unwrap();
        assert_eq!(c.sinopse_info, None);
    }

    #[test]
    fn consulta_com_sinopse_none_omite_o_campo_no_json() {
        // `skip_serializing_if` mantém o registro legado byte-idêntico ao snapshot antigo.
        let json = serde_json::to_string(&sample_consulta()).unwrap();
        assert!(!json.contains("sinopse_info"), "JSON não deve conter sinopse_info quando None: {json}");
    }
}
