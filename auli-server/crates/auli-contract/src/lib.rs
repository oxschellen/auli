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

pub mod kind;
pub mod mddoc;
pub mod snapshot;
pub use kind::Kind;
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
        Self {
            id: id.into(),
            nome: nome.into(),
            items,
        }
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
    /// Key a embeddar. A fórmula é ÚNICA e vive em [`compose_faq_text_to_embed`] — não repetir
    /// aqui (foi assim que esta linha ficou descrevendo a fórmula anterior à TAREFA-FAQ-PR). O
    /// produtor (`derive_faqs`) grava, e o `auli update` rematerializa ao ler `docs/faqs/*.md`.
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

/// Proveniência do `## resumo` gerado pelo passo `auli-sinopse`.
/// `None` = registro sem resumo gerado: pendente (resumo vazio) ou sumário autorado legado
/// (resumo preenchido na origem, caso RS antigo).
///
/// (Antes: `SinopseInfo`.) O nome do TIPO acompanhou o vocabulário unificado das árvores na D-FMT-4;
/// o CAMPO ficou `sinopse_info` até a D-B1, sob a justificativa de ser serializado nos snapshots —
/// justificativa que não se sustentava (`snapshot.rs` guarda `ColetaFaqs`/`ColetaServicos`, e o
/// [`Documento`] não aparece em nenhum deles). Hoje os dois se chamam `resumo_info`, igual ao slot
/// do [`mddoc::DocHeader`] de onde o valor vem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResumoInfo {
    /// Modelo que gerou a sinopse (ex.: `"llama-3.3-70b-versatile"`).
    pub modelo: String,
    /// Versão do prompt do auli-sinopse (const `SINOPSE_PROMPT_VERSION` no gerador).
    pub prompt_versao: u32,
    /// Instante da geração, ISO-8601 (ex.: `"2026-07-18T14:00:00Z"`).
    pub gerada_em: String,
}

/// **Um documento do acervo**, no vocabulário unificado das árvores (D-B1).
///
/// Hoje só a jurisprudência chega aqui — pareceres (o termo geral entre estados: "Pareceres" no RS,
/// "Respostas de Consultas" em SP, COPAT em SC) e acórdãos do TARF. Serviços e FAQs ainda têm as
/// próprias structs de borda ([`Servico`], [`Faq`]) e passarão a se projetar nesta.
///
/// (Antes: `Parecer`, depois `Consulta`.) O **kind de domínio permanece `"pareceres"`** por
/// compatibilidade — rota `/v1/{kind}/list`, sufixo da coleção vetorial, nomes de pack e labels não
/// mudam; o nome da struct não aparece na serialização (serde grava só os campos).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Documento {
    /// De QUAL coleção este documento veio — hoje [`Kind::Pareceres`] ou [`Kind::Tarf`]. É o que
    /// decide o `docs/<kind>/` do `doc_path` gravado no pack; sem ele, um acórdão apontaria para a
    /// árvore dos pareceres e o corpo sairia indisponível na query.
    ///
    /// Ausente no JSON legado ⇒ `Pareceres`, que era a única coleção quando esses registros foram
    /// escritos (esta struct não é persistida hoje — a fonte é a árvore `.md` desde a G5b).
    #[serde(default = "kind_pareceres")]
    pub kind: Kind,
    /// **Trilha** — o caminho até o documento no portal de origem (`"Cidadãos | IPVA"` num serviço,
    /// `"Inicial | Tema | Subtema"` numa FAQ). **Vazia na jurisprudência**, que não tem navegação:
    /// um parecer é achado pelo número, não por trilha. Mesmo papel do `trilha` do frontmatter.
    ///
    /// Vazia é estado normal, não pendência — e é por isso que o [`bloco`] a omite em vez de deixar
    /// uma linha em branco.
    #[serde(default)]
    pub trilha: String,
    /// **Título** — o identificador legível do documento (ex.: `"PARECER Nº 25148"`,
    /// `"ACÓRDÃO 510/24"`). Mesmo papel do `titulo` do frontmatter. (Antes: `numero`.)
    pub titulo: String,
    /// **Ementa** — o assunto em uma linha. Mesmo papel do `ementa` do frontmatter.
    /// (Antes: `assunto`.)
    pub ementa: String,
    /// **Resumo** — a síntese do documento: a sinopse LLM nos pareceres, a fundamentação do
    /// cabeçalho no TARF. Pode ser vazio (pendente).
    #[serde(default)]
    pub resumo: String,
    /// Corpo integral, lido de `## corpo`.
    pub corpo: String,
    /// URL da fonte oficial.
    pub link: String,
    /// Key a embeddar — recomposta pelo ponto único ([`compose_text_to_embed`]).
    pub text_to_embed: String,
    /// Proveniência do resumo (ver [`ResumoInfo`]). Ausente no JSON quando `None`.
    ///
    /// (Antes: `sinopse_info`.) O nome antigo era mantido sob a justificativa de que o campo era
    /// serializado nos snapshots — **o que não é verdade**: `snapshot.rs` guarda `ColetaFaqs` e
    /// `ColetaServicos`, e esta struct não aparece em nenhum deles. Sem esse impedimento, o campo
    /// acompanha o vocabulário e passa a ter o mesmo nome do [`mddoc::DocHeader::resumo_info`] de
    /// onde é copiado.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resumo_info: Option<ResumoInfo>,
}

/// O `kind` dos registros escritos antes de a jurisprudência ter mais de uma coleção.
fn kind_pareceres() -> Kind {
    Kind::Pareceres
}

impl Embeddable for Documento {
    fn text_to_embed(&self) -> &str {
        &self.text_to_embed
    }

    /// G3: o pack de jurisprudência guarda o **payload leve** (JSON, sem o corpo) — o corpo vive na
    /// árvore `docs/` e é lido na query. `doc_path` usa o MESMO `mddoc::slug` da materialização;
    /// nunca recomputar com outra regra. O servidor remonta o bloco `## pergunta`/`## resposta` de
    /// sempre via `render_consulta_block(payload, corpo)`. Os demais kinds seguem com o bloco
    /// pré-renderizado.
    fn stored_repr(&self) -> String {
        let payload = ConsultaPackPayload {
            numero: self.titulo.clone(),
            assunto: self.ementa.clone(),
            resumo: self.resumo.clone(),
            link: self.link.clone(),
            doc_path: format!(
                "docs/{}/{}.md",
                self.kind.as_str(),
                mddoc::slug(&self.titulo)
            ),
        };
        serde_json::to_string(&payload)
            .expect("ConsultaPackPayload serializa sem falha (campos String)")
    }
}

/// Payload de pack de pareceres (G3): tudo MENOS o corpo, que vive na árvore `docs/` e é lido na
/// query. `doc_path` é relativo ao diretório da entidade (ex.: `"docs/pareceres/<slug>.md"`).
///
/// É exatamente o que `Consulta::stored_repr` grava — a fiação da G3 está feita. O servidor lê o
/// corpo da árvore por `doc_path` e remonta o bloco `## pergunta`/`## resposta` via
/// [`render_consulta_block`]. Pack e servidor mudam em lockstep: alterar os campos daqui sem
/// re-gerar os packs faz o serving renderizar lixo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsultaPackPayload {
    pub numero: String,
    pub assunto: String,
    pub resumo: String,
    pub link: String,
    pub doc_path: String,
}

/// Único ponto que compõe o `text_to_embed` de consultas (decisão §2.5 do plano). Indexa o
/// **título** (`numero`), a **ementa** (`assunto`) e a **sinopse** (`resumo`), nesta ordem, uma por
/// linha — pulando os vazios. O `numero` entrou para que buscas que citam a consulta (ex.:
/// "CONSULTA COPAT nº 0037/26") a alcancem. Mudança de fórmula: re-vetorizar os packs de pareceres.
///
/// Vive no contrato (e não no `auli-collections`) porque os DOIS lados precisam dela: o produtor
/// (sinopse/derive) ao gravar, e o `auli update` ao hidratar o `resumo` da árvore (G4).
pub fn compose_text_to_embed(numero: &str, assunto: &str, resumo: &str) -> String {
    compose_unificado("", numero, assunto, resumo)
}

/// **O compose único** (D-FMT-6): os quatro papéis do vocabulário unificado, um por linha, na
/// geometria `trilha → titulo → ementa → resumo`, pulando os vazios.
///
/// A ordem não é estética: é o que faz cada coleção reproduzir a própria fórmula congelada quando
/// os slots que não usa ficam vazios e se anulam. Jurisprudência preenche titulo/ementa/resumo e
/// deixa a trilha vazia; serviços preenchem trilha/titulo/resumo e deixam a ementa vazia; faqs
/// preenchem trilha/titulo/resumo. Os adaptadores por coleção ([`compose_text_to_embed`],
/// [`compose_servico_text_to_embed`], [`compose_faq_text_to_embed`]) só decidem o que vai em cada
/// slot — a fórmula é esta, e mudá-la re-vetoriza TUDO.
pub fn compose_unificado(trilha: &str, titulo: &str, ementa: &str, resumo: &str) -> String {
    [trilha, titulo, ementa, resumo]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Key de embedding de um serviço: breadcrumb `tipo | classe` + título + snippet
/// da descrição (300 chars). Vive no contrato porque os DOIS lados precisam dela:
/// o produtor (`servicos::process`) ao materializar, e o `auli update` ao
/// rematerializar na leitura da árvore `docs/servicos/*.md`.
///
/// Adaptador do compose único: `trilha = "tipo | classe"` (a breadcrumb pré-composta, que é também
/// o que vai para o frontmatter), `titulo`, ementa vazia e `resumo = snippet`.
pub fn compose_servico_text_to_embed(
    tipo: &str,
    classe: &str,
    titulo: &str,
    descricao: &str,
) -> String {
    // O `.trim()` final reproduz o da fórmula congelada: com `tipo` e `classe` vazios a trilha vira
    // `" | "`, e a fórmula antiga aparava esse espaço à esquerda.
    compose_unificado(
        &trilha_servico(tipo, classe),
        titulo,
        "",
        &snippet(descricao),
    )
    .trim()
    .to_string()
}

/// A trilha do serviço: breadcrumb `tipo | classe`. Ponto único — o mesmo texto vai para o
/// `text_to_embed` e para o frontmatter, e é o que o bloco RAG exibe.
pub fn trilha_servico(tipo: &str, classe: &str) -> String {
    format!("{tipo} | {classe}")
}

/// Inverso de [`trilha_servico`]: devolve `(tipo, classe)` de uma trilha de serviço.
///
/// Quebra no PRIMEIRO ` | ` porque é o `tipo` que é o campo simples (o nome de um público:
/// "Cidadãos", "Empresas"), enquanto a classe é texto do portal. Sem separador, tudo é `tipo` e a
/// classe fica vazia. Existe porque a árvore guarda a trilha já composta (é o que os dois funis
/// consomem) e um consumidor — o grafo de serviços — ainda precisa do público isolado.
pub fn partes_trilha_servico(trilha: &str) -> (&str, &str) {
    match trilha.split_once(" | ") {
        Some((tipo, classe)) => (tipo, classe),
        None => (trilha, ""),
    }
}

/// Snippet da descrição que entra na key: 300 chars contados em CHARS (há acentos), aparado.
fn snippet(descricao: &str) -> String {
    descricao
        .chars()
        .take(300)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Key de embedding de uma FAQ (D-FAQPR-1): assunto (= breadcrumb `origin`) +
/// `P:` pergunta + `R:` resposta, um por linha. **Linha de campo vazio é OMITIDA**
/// — nunca linha em branco, placeholder ou `R:` pendurado. Os campos passam por
/// [`mddoc::colapsa_linha`], então o texto final tem no máximo 3 linhas e é
/// determinístico byte a byte (pré-condição de pack reproduzível).
///
/// A resposta entra porque ela carrega vocabulário que a pergunta não tem; o
/// assunto no cabeçalho desambigua perguntas parecidas entre temas ("qual o
/// prazo?" existe em vários). Na fórmula anterior a key era só breadcrumb +
/// pergunta (`QuestionKey`) — cega para a resposta.
///
/// Vive no contrato porque os DOIS lados precisam dela: o produtor
/// (`derive_faqs`) e o `auli update` ao rematerializar da árvore `docs/faqs/*.md`.
///
/// Adaptador do compose único, com uma torção deliberada: os prefixos `P:`/`R:` entram no **valor**
/// dos slots (`titulo = "P: {pergunta}"`, `resumo = "R: {resposta}"`), não na fórmula. É o que
/// permite unificar sem re-vetorizar — a fórmula não sabe de prefixo nenhum, e quando o gate P5
/// decidir removê-los, some este mapeamento, não o compose. (A `## resumo` da FAQ é ausente na
/// árvore: o `R:` é montado aqui a partir do `## corpo`.)
pub fn compose_faq_text_to_embed(origin: &str, pergunta: &str, resposta: &str) -> String {
    let trilha = mddoc::colapsa_linha(origin);
    let pergunta = mddoc::colapsa_linha(pergunta);
    let resposta = mddoc::colapsa_linha(resposta);
    let titulo = format!("P: {pergunta}");
    let resumo = if resposta.is_empty() {
        String::new()
    } else {
        format!("R: {resposta}")
    };
    compose_unificado(&trilha, &titulo, "", &resumo)
}

/// **O bloco único** de contexto RAG (D-B3): a mesma forma para as quatro coleções.
///
/// ```text
/// ## pergunta
/// {trilha}      ← omitida quando vazia
/// {titulo}
/// {ementa}      ← omitida quando vazia
///
/// ## resposta
/// {conteudo}
/// Link: {link}
/// ```
///
/// O `conteudo` vem de FORA em vez de sair do `Documento` porque é ele que o serving lê tarde, da
/// árvore, só para os k documentos escolhidos — e porque nas bordas ele tem nomes diferentes
/// (`corpo` na jurisprudência, `descricao` no serviço, `resposta` na FAQ). O bloco não precisa saber
/// disso.
///
/// **A seção `## pergunta` tem a MESMA geometria do [`compose_unificado`]** — os slots na ordem
/// `trilha → titulo → ementa`, um por linha, pulando os vazios. Não é coincidência: o que a busca
/// indexa e o que o LLM lê descrevem o mesmo documento, e mantê-los na mesma ordem é o que impede
/// que um mude sem o outro. A diferença é o `resumo`, que entra na key e não no bloco (no bloco o
/// que aparece é o conteúdo integral).
///
/// Substitui quatro renderizações que eram a mesma coisa escrita quatro vezes: os `stored_repr` de
/// [`Faq`] e [`Servico`] e o [`render_consulta_block`] da jurisprudência. A equivalência é verificada
/// byte a byte nos testes, contra reimplementações literais dos formatos antigos.
pub fn bloco(d: &Documento, conteudo: &str) -> String {
    let pergunta = [d.trilha.as_str(), d.titulo.as_str(), d.ementa.as_str()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "## pergunta\n{pergunta}\n\n## resposta\n{conteudo}\nLink: {}",
        d.link
    )
}

/// Renderiza o bloco de contexto de uma consulta a partir do payload leve + corpo lido da árvore.
///
/// MESMO formato do `stored_repr` gordo de ANTES da G3 (que não existe mais: hoje ele grava o
/// payload leve), byte a byte — é esse o invariante da G3, o contexto RAG montado não pode mudar.
/// Ponto único: servidor e testes passam por aqui.
pub fn render_consulta_block(p: &ConsultaPackPayload, corpo: &str) -> String {
    format!(
        "## pergunta\n{}\n{}\n\n## resposta\n{}\nLink: {}",
        p.numero, p.assunto, corpo, p.link
    )
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

    /// Reimplementação LITERAL dos três formatos de BLOCO como estavam antes da D-B3 — os dois
    /// `stored_repr` (Faq, Servico) e o `render_consulta_block`. Copiados à mão, sem chamar nada do
    /// código de produção: são a referência independente contra a qual o [`bloco`] único é medido.
    ///
    /// Se um destes divergir do `bloco`, o CONTEXTO QUE VAI AO LLM mudou — o que muda a resposta do
    /// modelo para perguntas que já funcionavam. É a trava mais cara de perder desta etapa.
    mod blocos_antigos {
        pub fn faq(origin: &str, pergunta: &str, resposta: &str, url: &str) -> String {
            let mut s = String::from("## pergunta\n");
            if !origin.is_empty() {
                s.push_str(origin);
                s.push('\n');
            }
            s.push_str(pergunta);
            s.push_str("\n\n## resposta\n");
            s.push_str(resposta);
            s.push_str(&format!("\nLink: {url}"));
            s
        }

        pub fn servico(
            tipo: &str,
            classe: &str,
            titulo: &str,
            descricao: &str,
            link: &str,
        ) -> String {
            format!(
                "## pergunta\n{} | {}\n{}\n\n## resposta\n{}\nLink: {}",
                tipo, classe, titulo, descricao, link
            )
        }

        pub fn jurisprudencia(numero: &str, assunto: &str, corpo: &str, link: &str) -> String {
            format!("## pergunta\n{numero}\n{assunto}\n\n## resposta\n{corpo}\nLink: {link}")
        }
    }

    /// Reimplementação LITERAL das três fórmulas de KEY como estavam antes da unificação (D-FMT-6).
    /// Não chamam nada do código de produção de propósito: são a referência contra a qual o
    /// compose único é medido. Se um dia divergirem, é o compose que mudou — e mudar o compose
    /// re-vetoriza todos os packs.
    mod formulas_antigas {
        pub fn jurisprudencia(numero: &str, assunto: &str, resumo: &str) -> String {
            [numero, assunto, resumo]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        }

        pub fn servico(tipo: &str, classe: &str, titulo: &str, descricao: &str) -> String {
            let snippet: String = descricao.chars().take(300).collect();
            format!("{} | {}\n{}\n{}", tipo, classe, titulo, snippet.trim())
                .trim()
                .to_string()
        }

        pub fn faq(origin: &str, pergunta: &str, resposta: &str) -> String {
            let assunto = crate::mddoc::colapsa_linha(origin);
            let pergunta = crate::mddoc::colapsa_linha(pergunta);
            let resposta = crate::mddoc::colapsa_linha(resposta);
            let mut out = String::new();
            if !assunto.is_empty() {
                out.push_str(&assunto);
                out.push('\n');
            }
            out.push_str("P: ");
            out.push_str(&pergunta);
            if !resposta.is_empty() {
                out.push_str("\nR: ");
                out.push_str(&resposta);
            }
            out
        }
    }

    // ---- D-B3: o bloco único reproduz os três formatos byte a byte ----

    /// Monta um `Documento` só com os slots que o [`bloco`] lê. Os campos que ele ignora
    /// (`corpo`, `text_to_embed`) vão vazios de propósito: se um dia o bloco começar a olhá-los,
    /// estes testes quebram, que é o que se quer.
    fn doc_de(trilha: &str, titulo: &str, ementa: &str, link: &str) -> Documento {
        Documento {
            kind: Kind::Pareceres,
            trilha: trilha.into(),
            titulo: titulo.into(),
            ementa: ementa.into(),
            resumo: String::new(),
            corpo: String::new(),
            link: link.into(),
            text_to_embed: String::new(),
            resumo_info: None,
        }
    }

    #[test]
    fn bloco_reproduz_o_formato_de_faq_byte_a_byte() {
        // Projeção da FAQ: trilha = origin, titulo = pergunta, ementa vazia, conteudo = resposta.
        let casos = [
            ("Inicial | IPVA", "Como emitir a guia?", "Acesse o portal."),
            ("", "Como emitir a guia?", "Acesse o portal."), // sem breadcrumb: a linha SOME
            ("Inicial | IPVA", "Como emitir a guia?", ""),   // resposta vazia
            ("Inicial | IPVA", "Pergunta", "Linha 1\nLinha 2"), // resposta multilinha
        ];
        for (origin, pergunta, resposta) in casos {
            let d = doc_de(origin, pergunta, "", "https://x/faq/1");
            assert_eq!(
                bloco(&d, resposta),
                blocos_antigos::faq(origin, pergunta, resposta, "https://x/faq/1"),
                "divergiu em {origin:?}/{pergunta:?}"
            );
        }
    }

    #[test]
    fn bloco_reproduz_o_formato_de_servico_byte_a_byte() {
        // Projeção do serviço: trilha = `tipo | classe`, titulo, ementa vazia, conteudo = descricao.
        let casos = [
            ("Empresas", "ICMS", "Emitir guia", "Passos para emitir."),
            ("Cidadãos", "", "Classe vazia", "Descrição."),
            // Tipo E classe vazios: a trilha vira `" | "`, que NÃO é string vazia — a linha
            // continua saindo, igual ao formato antigo. É o caso que separa "slot vazio" de
            // "slot com conteúdo degenerado", e o bloco não pode confundir os dois.
            ("", "", "Sem breadcrumb", "Descrição."),
            ("Empresas", "ICMS", "Sem descrição", ""),
        ];
        for (tipo, classe, titulo, descricao) in casos {
            let d = doc_de(&trilha_servico(tipo, classe), titulo, "", "https://x/svc/1");
            assert_eq!(
                bloco(&d, descricao),
                blocos_antigos::servico(tipo, classe, titulo, descricao, "https://x/svc/1"),
                "divergiu em {tipo:?}/{classe:?}/{titulo:?}"
            );
        }
    }

    #[test]
    fn bloco_reproduz_o_formato_de_jurisprudencia_byte_a_byte() {
        // Projeção da jurisprudência: trilha vazia, titulo = numero, ementa = assunto,
        // conteudo = corpo.
        let casos = [
            ("PARECER Nº 25148", "ICMS – crédito fiscal", "É o parecer."),
            ("ACÓRDÃO 510/24", "ICMS. PROCESSUAL.", "Corpo do acórdão."),
            ("PARECER Nº 1", "Assunto", ""), // corpo vazio: o Link não pode se perder
            // Corpo com as próprias âncoras no meio: é só concatenação, nada a escapar.
            (
                "PARECER Nº 2",
                "Assunto",
                "Preâmbulo.\n\n## resposta\nRecursivo.\nLink: falso",
            ),
        ];
        for (numero, assunto, corpo) in casos {
            let d = doc_de("", numero, assunto, "https://x/p/1");
            assert_eq!(
                bloco(&d, corpo),
                blocos_antigos::jurisprudencia(numero, assunto, corpo, "https://x/p/1"),
                "divergiu em {numero:?}/{assunto:?}"
            );
        }
    }

    #[test]
    fn bloco_de_jurisprudencia_sem_ementa_e_a_unica_divergencia_conhecida() {
        // **Honestidade da unificação.** O formato antigo emitia o `assunto` sempre, então com ele
        // vazio sobrava uma LINHA EM BRANCO entre o número e o `## resposta`. O bloco único pula
        // slots vazios, e essa linha some.
        //
        // O estado É alcançável — mas por um documento só. Varredura das árvores de todas as
        // entidades em 08/08/2026: `ementa` vazia em 1 de 19.780 pareceres
        // (`data/pr/docs/pareceres/consulta-no-32-2007.md`) e em 0 de 3.800 acórdãos. Nas FAQs e
        // nos serviços a ementa é vazia SEMPRE, e ali os dois formatos concordam (nenhum dos dois
        // emitia a linha).
        //
        // Ou seja: o contexto RAG muda em UM caractere, em UM documento de todo o acervo, e só
        // quando ele é recuperado. O vetor não muda (a key vem do `compose`, não do bloco), então
        // não há re-vetorização. Fica registrado como decisão, não descoberto depois.
        let d = doc_de("", "CONSULTA Nº 32/2007", "", "https://x/p/32");
        let novo = bloco(&d, "É o parecer.");
        let antigo = blocos_antigos::jurisprudencia(
            "CONSULTA Nº 32/2007",
            "",
            "É o parecer.",
            "https://x/p/32",
        );
        assert_eq!(
            novo,
            "## pergunta\nCONSULTA Nº 32/2007\n\n## resposta\nÉ o parecer.\nLink: https://x/p/32"
        );
        assert_eq!(
            antigo,
            "## pergunta\nCONSULTA Nº 32/2007\n\n\n## resposta\nÉ o parecer.\nLink: https://x/p/32"
        );
        assert_eq!(antigo.len(), novo.len() + 1, "a diferença é UM \\n");
    }

    #[test]
    fn bloco_equivale_ao_render_consulta_block_do_payload_leve() {
        // A outra ponta: o `bloco` tem de reproduzir o que o serving monta HOJE a partir do payload
        // do pack. Enquanto as duas funções coexistirem (o `render_consulta_block` só sai na B5),
        // é este teste que garante que não divergiram.
        let c = sample_documento();
        let p = payload_de(&c, "docs/pareceres/parecer-no-25148.md");
        let d = doc_de("", &c.titulo, &c.ementa, &c.link);
        assert_eq!(bloco(&d, &c.corpo), render_consulta_block(&p, &c.corpo));
    }

    #[test]
    fn a_secao_pergunta_do_bloco_tem_a_mesma_geometria_do_compose() {
        // O invariante que mantém key e bloco descrevendo o mesmo documento: os slots
        // `trilha → titulo → ementa` saem na mesma ordem, pulando os mesmos vazios. O `resumo`
        // é a única diferença — entra na key, não no bloco.
        let casos = [
            ("Cidadãos | IPVA", "Guia", ""),
            ("", "PARECER Nº 1", "ICMS"),
            ("", "Só o título", ""),
        ];
        for (trilha, titulo, ementa) in casos {
            let d = doc_de(trilha, titulo, ementa, "http://x");
            let secao = bloco(&d, "C")
                .strip_prefix("## pergunta\n")
                .unwrap()
                .split("\n\n## resposta\n")
                .next()
                .unwrap()
                .to_string();
            assert_eq!(
                secao,
                compose_unificado(trilha, titulo, ementa, ""),
                "divergiu em {trilha:?}/{titulo:?}/{ementa:?}"
            );
        }
    }

    #[test]
    fn compose_unificado_reproduz_a_formula_de_jurisprudencia_byte_a_byte() {
        let casos = [
            ("PARECER Nº 25148", "ICMS. Crédito.", "Resumo do parecer."),
            ("PARECER Nº 1", "", "só resumo"),
            ("PARECER Nº 2", "só assunto", ""),
            ("PARECER Nº 3", "", ""),
        ];
        for (numero, assunto, resumo) in casos {
            assert_eq!(
                compose_text_to_embed(numero, assunto, resumo),
                formulas_antigas::jurisprudencia(numero, assunto, resumo),
                "divergiu em {numero:?}/{assunto:?}/{resumo:?}"
            );
        }
    }

    #[test]
    fn compose_unificado_reproduz_a_formula_de_servico_byte_a_byte() {
        let descricao_longa = "á".repeat(400);
        let casos = [
            ("Empresas", "ICMS", "Emitir guia", "Passos para emitir."),
            ("Cidadãos", "IPVA", "Guia", descricao_longa.as_str()),
            ("Empresas", "ICMS", "Sem descrição", ""),
            ("Empresas", "ICMS", "Descrição só de espaço", "   "),
            ("", "", "Sem breadcrumb", "Descrição."), // trilha vira " | " e é aparada
            ("Cidadãos", "", "Classe vazia", "Descrição."),
        ];
        for (tipo, classe, titulo, descricao) in casos {
            assert_eq!(
                compose_servico_text_to_embed(tipo, classe, titulo, descricao),
                formulas_antigas::servico(tipo, classe, titulo, descricao),
                "divergiu em {tipo:?}/{classe:?}/{titulo:?}"
            );
        }
    }

    #[test]
    fn compose_unificado_reproduz_a_formula_de_faq_byte_a_byte() {
        let casos = [
            ("Inicial | IPVA", "Como emitir a guia?", "Acesse o portal."),
            ("", "Como emitir a guia?", "Acesse o portal."), // sem breadcrumb
            ("Inicial | IPVA", "Como emitir a guia?", ""),   // sem resposta
            ("Inicial | IPVA", "", "Acesse o portal."),      // pergunta vazia ⇒ `P: ` pelado
            (
                "  Inicial  |  IPVA ",
                " Como  emitir? ",
                " Acesse\no portal. ",
            ), // colapso
        ];
        for (origin, pergunta, resposta) in casos {
            assert_eq!(
                compose_faq_text_to_embed(origin, pergunta, resposta),
                formulas_antigas::faq(origin, pergunta, resposta),
                "divergiu em {origin:?}/{pergunta:?}/{resposta:?}"
            );
        }
    }

    #[test]
    fn compose_de_servico_com_titulo_vazio_e_a_unica_divergencia_conhecida() {
        // Honestidade da unificação: a fórmula antiga deixava uma LINHA EM BRANCO no lugar do
        // título vazio; o compose único pula slots vazios. Só diverge aqui, e este estado não
        // existe na árvore — o título é a identidade do serviço (é dele que sai o nome do
        // arquivo). O teste existe para que a divergência seja uma decisão registrada, não uma
        // descoberta futura.
        let novo = compose_servico_text_to_embed("Empresas", "ICMS", "", "Descrição.");
        let antigo = formulas_antigas::servico("Empresas", "ICMS", "", "Descrição.");
        assert_eq!(novo, "Empresas | ICMS\nDescrição.");
        assert_eq!(antigo, "Empresas | ICMS\n\nDescrição.");
    }

    #[test]
    fn compose_servico_congela_a_formula_e_corta_em_300_chars() {
        // Literal esperado, não recomputado: mudar a fórmula exige re-vetorizar os
        // packs de serviços, então a mudança tem que doer aqui primeiro.
        assert_eq!(
            compose_servico_text_to_embed("Empresas", "ICMS", "Emitir guia", "Passos para emitir."),
            "Empresas | ICMS\nEmitir guia\nPassos para emitir."
        );
        // Snippet: 300 chars da descrição, contados em CHARS (não bytes — há acentos).
        let descricao = "á".repeat(400);
        let key = compose_servico_text_to_embed("Cidadãos", "IPVA", "Guia", &descricao);
        assert_eq!(key, format!("Cidadãos | IPVA\nGuia\n{}", "á".repeat(300)));
    }

    #[test]
    fn compose_faq_congela_o_template() {
        // Literais esperados, não recomputados: mudar o template exige re-vetorizar
        // os packs de faqs, então a mudança tem que doer aqui primeiro.
        // (a) item completo: assunto, P: e R:, nesta ordem.
        assert_eq!(
            compose_faq_text_to_embed("Inicial | IPVA", "Como emitir a guia?", "Acesse o portal."),
            "Inicial | IPVA\nP: Como emitir a guia?\nR: Acesse o portal."
        );
        // (b) sem assunto: a linha some — nada de linha em branco no topo.
        assert_eq!(
            compose_faq_text_to_embed("", "Como emitir a guia?", "Acesse o portal."),
            "P: Como emitir a guia?\nR: Acesse o portal."
        );
        // (c) sem resposta (corpo vazio é estado legal da árvore): nada de `R:` pendurado.
        assert_eq!(
            compose_faq_text_to_embed("Inicial | IPVA", "Como emitir a guia?", ""),
            "Inicial | IPVA\nP: Como emitir a guia?"
        );
        // (d) campos sujos: quebras e espaços múltiplos colapsam — o texto final tem 3 linhas.
        let key = compose_faq_text_to_embed(
            " Inicial |  IPVA ",
            "Como   emitir\na guia?",
            "Acesse o portal.\n\nDepois clique  em Emitir.\n",
        );
        assert_eq!(
            key,
            "Inicial | IPVA\nP: Como emitir a guia?\nR: Acesse o portal. Depois clique em Emitir."
        );
        assert_eq!(key.lines().count(), 3);
    }

    #[test]
    fn parecer_exposes_key_and_renders_block() {
        let p = Documento {
            kind: Kind::Pareceres,
            trilha: String::new(),
            titulo: "PARECER Nº 25148".into(),
            ementa: "ICMS – crédito fiscal na cesta básica".into(),
            resumo: "Análise sobre apropriação de crédito.".into(),
            corpo: "É o parecer.".into(),
            link: "https://exemplo/parecer/25148".into(),
            text_to_embed:
                "ICMS – crédito fiscal na cesta básica\nAnálise sobre apropriação de crédito."
                    .into(),
            resumo_info: None,
        };
        assert_eq!(
            p.text_to_embed(),
            "ICMS – crédito fiscal na cesta básica\nAnálise sobre apropriação de crédito."
        );
        // G3: `stored_repr` agora é o payload LEVE (JSON, sem corpo), com `doc_path` derivado do slug.
        let payload: ConsultaPackPayload = serde_json::from_str(&p.stored_repr()).unwrap();
        assert_eq!(payload.numero, "PARECER Nº 25148");
        assert_eq!(payload.doc_path, "docs/pareceres/parecer-no-25148.md");
        assert!(
            !p.stored_repr().contains("É o parecer."),
            "o corpo NÃO pode ir para o pack (G3)"
        );
        // Remontado com o corpo, reproduz o bloco de sempre.
        let block = render_consulta_block(&payload, &p.corpo);
        assert!(
            block.starts_with(
                "## pergunta\nPARECER Nº 25148\nICMS – crédito fiscal na cesta básica"
            )
        );
        assert!(block.contains("## resposta\nÉ o parecer."));
        assert!(block.contains("Link: https://exemplo/parecer/25148"));

        // The serialized shape is contract; round-trips through JSON.
        let table = Table::new("rs", "pareceres", vec![p]);
        let json = serde_json::to_string(&table).unwrap();
        let back: Table<Documento> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.items[0].titulo, "PARECER Nº 25148");
    }

    fn sample_documento() -> Documento {
        Documento {
            kind: Kind::Pareceres,
            trilha: String::new(),
            titulo: "PARECER Nº 25148".into(),
            ementa: "ICMS – crédito fiscal na cesta básica".into(),
            resumo: "Análise sobre apropriação de crédito.".into(),
            corpo: "É o parecer.".into(),
            link: "https://exemplo/parecer/25148".into(),
            text_to_embed: "ICMS – crédito fiscal na cesta básica".into(),
            resumo_info: None,
        }
    }

    /// Golden da G3: o `stored_repr` gordo de HOJE, reimplementado aqui de propósito. Se o formato
    /// mudar de um lado só, este teste quebra — é ele que garante que trocar o pack gordo pelo
    /// payload leve + leitura tardia não mexe um byte no contexto RAG.
    fn stored_repr_gordo_golden(c: &Documento) -> String {
        format!(
            "## pergunta\n{}\n{}\n\n## resposta\n{}\nLink: {}",
            c.titulo, c.ementa, c.corpo, c.link
        )
    }

    fn payload_de(c: &Documento, doc_path: &str) -> ConsultaPackPayload {
        ConsultaPackPayload {
            numero: c.titulo.clone(),
            assunto: c.ementa.clone(),
            resumo: c.resumo.clone(),
            link: c.link.clone(),
            doc_path: doc_path.into(),
        }
    }

    #[test]
    fn render_do_payload_leve_equivale_ao_stored_repr_gordo() {
        let c = sample_documento();
        let p = payload_de(&c, "docs/pareceres/parecer-no-25148.md");
        assert_eq!(
            render_consulta_block(&p, &c.corpo),
            stored_repr_gordo_golden(&c)
        );
        // Invariante ponta-a-ponta da G3: o que o pack GRAVA (stored_repr → payload JSON), desserializado
        // e remontado com o corpo, reproduz o bloco gordo byte a byte. É o coração da fase.
        let do_pack: ConsultaPackPayload = serde_json::from_str(&c.stored_repr()).unwrap();
        assert_eq!(
            render_consulta_block(&do_pack, &c.corpo),
            stored_repr_gordo_golden(&c)
        );
    }

    #[test]
    fn render_equivale_com_corpo_multilinha_e_ancoras_no_meio() {
        // Corpo com as próprias âncoras no meio: é só concatenação, nada a escapar.
        let mut c = sample_documento();
        c.corpo = "Preâmbulo.\n\n## resposta\nRecursivo.\nLink: falso\n## corpo\nfim".into();
        let p = payload_de(&c, "docs/pareceres/x.md");
        assert_eq!(
            render_consulta_block(&p, &c.corpo),
            stored_repr_gordo_golden(&c)
        );
    }

    #[test]
    fn render_com_corpo_vazio_nao_perde_o_link() {
        let c = sample_documento();
        let p = payload_de(&c, "docs/pareceres/x.md");
        let bloco = render_consulta_block(&p, "");
        assert!(
            bloco.ends_with(&format!("\nLink: {}", c.link)),
            "bloco: {bloco:?}"
        );
    }

    #[test]
    fn payload_faz_round_trip_por_json() {
        let p = payload_de(&sample_documento(), "docs/pareceres/parecer-no-25148.md");
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(
            serde_json::from_str::<ConsultaPackPayload>(&json).unwrap(),
            p
        );
    }

    #[test]
    fn payload_leve_nao_carrega_o_corpo() {
        // O ganho da fase: o corpo não pode vazar para o pack por nenhum campo.
        let mut c = sample_documento();
        c.corpo = "MARCADOR-DE-CORPO-UNICO".into();
        let json = serde_json::to_string(&payload_de(&c, "docs/pareceres/x.md")).unwrap();
        assert!(
            !json.contains("MARCADOR-DE-CORPO-UNICO"),
            "corpo vazou para o payload: {json}"
        );
    }

    #[test]
    fn documento_com_resumo_info_faz_round_trip_por_json() {
        let mut c = sample_documento();
        c.resumo_info = Some(ResumoInfo {
            modelo: "llama-3.3-70b-versatile".into(),
            prompt_versao: 1,
            gerada_em: "2026-07-18T14:00:00Z".into(),
        });
        let json = serde_json::to_string(&c).unwrap();
        let back: Documento = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn documento_json_sem_resumo_info_e_sem_kind_desserializa_com_os_defaults() {
        // Os dois campos opcionais, testados juntos: `resumo_info` ausente ⇒ `None`, `kind` ausente
        // ⇒ `Pareceres` (a única coleção que existia quando registros assim eram escritos).
        //
        // O literal usa o vocabulário NOVO (`titulo`/`ementa`) porque não há compatibilidade a
        // preservar: esta struct não é serializada em lugar nenhum. O teste anterior dizia guardar
        // "a forma dos snapshots atuais", mas `snapshot.rs` só guarda `ColetaFaqs`/`ColetaServicos`
        // — a premissa era falsa, e é ela que liberou o rename dos campos na D-B1.
        let json = r#"{
            "titulo": "PARECER Nº 25148",
            "ementa": "ICMS – crédito fiscal na cesta básica",
            "resumo": "Análise sobre apropriação de crédito.",
            "corpo": "É o parecer.",
            "link": "https://exemplo/parecer/25148",
            "text_to_embed": "ICMS – crédito fiscal na cesta básica"
        }"#;
        let c: Documento = serde_json::from_str(json).unwrap();
        assert_eq!(c.resumo_info, None);
        assert_eq!(c.kind, Kind::Pareceres);
    }

    #[test]
    fn documento_com_resumo_info_none_omite_o_campo_no_json() {
        // `skip_serializing_if` mantém o registro legado byte-idêntico ao snapshot antigo.
        let json = serde_json::to_string(&sample_documento()).unwrap();
        assert!(
            !json.contains("resumo_info"),
            "JSON não deve conter resumo_info quando None: {json}"
        );
    }
}
