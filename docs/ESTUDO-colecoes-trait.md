# ESTUDO-colecoes-trait — Exemplo concreto do trait `Colecao` (fase 3, longo prazo)

> **IMPLANTADO em 08/08/2026** — por outro desenho, e vale registrar a diferença.
>
> Este estudo propunha um **trait `Colecao`** com três structs de domínio implementando-o. O que
> foi mergeado (`TAREFA-DOCUMENTO`, PRs #132 e #133) chegou ao mesmo lugar por um caminho mais
> simples: **uma struct só** (`Documento`) mais um enum (`Kind`), com `Servico` e `Faq` recuando
> para as bordas e se projetando nela (`para_documento`). Não há trait de coleção — o único trait
> que sobrou é o `Embeddable`, que é a fronteira do engine e hoje tem uma implementação só.
>
> Por que o desenho mais simples bastou: o estudo assumia que "os DADOS continuam em três structs
> distintas (a doutrina dos irmãos fica de pé)". Entre a escrita dele e a implementação, a
> unificação do vocabulário das árvores (`TAREFA-FORMATO-MD`, #128) tirou o chão dessa premissa —
> com um `.md` de formato único, as três structs passaram a descrever a MESMA forma com nomes
> diferentes. Aí o polimorfismo virou custo sem contrapartida.
>
> O que o estudo previu e se confirmou: o custo marginal de uma coleção nova. O TARF entrou como
> uma variante do `Kind` e algumas linhas de fiação; súmulas, RFB ou leis entram do mesmo jeito.
>
> O exemplo abaixo fica como registro do caminho considerado.

**Status:** ~~estudo exploratório, sem decisão de implementação~~ — **superado pela implementação**
(ver a nota acima). Companheiro do `ESTUDO-TARF-formato.md`. O gatilho previsto era a chegada da
**quarta** coleção; ela chegou (TARF) e antecipou a decisão.

**A ideia em uma frase:** os DADOS continuam em três structs de domínio distintas (a doutrina
dos irmãos fica de pé); o que unifica é o COMPORTAMENTO — o contrato que o encanamento
(update, packs, downloads, MCP, chat) consome sem saber qual coleção está processando.

O exemplo abaixo é autocontido (compila sozinho com `anyhow`); no repo real, cada `impl`
delegaria aos irmãos existentes (`mddoc*`, `compose_*`), que permanecem intocados com suas
travas de teste.

```rust
use anyhow::Result;

// ─── As structs de domínio: três shapes, de propósito ─────────────────────────
// (no repo: auli-contract; aqui reproduzidas para o exemplo ser autocontido)

/// Formato-pareceres: pareceres E acórdãos TARF (fase 2 do estudo TARF).
pub struct Jurisprudencia {
    pub numero: String,
    pub assunto: String, // ementa, no TARF
    pub link: String,
    pub sinopse: String,
    pub corpo: String,
}

pub struct Servico {
    pub titulo: String,
    pub tipo: String,
    pub classe: String,
    pub orgao: String,
    pub link: String,
    pub descricao: String,
}

pub struct Faq {
    pub origin: String, // assunto/trilha; pode ser vazio
    pub pergunta: String,
    pub resposta: String,
    pub url: String,
}

// ─── O contrato ───────────────────────────────────────────────────────────────

pub enum Rebuild {
    /// Árvore só cresce; "existe ⇒ pula" (jurisprudência — sinopse pode ter custado LLM).
    AppendOnly,
    /// Árvore regenerada do zero a cada re-scrape (serviços, FAQs).
    DeleteRebuild,
}

/// Uma coleção do acervo: o que o encanamento precisa saber, e nada mais.
pub trait Colecao {
    type Doc;
    fn kind(&self) -> &'static str;   // rotas, packs, manifesto
    fn dir(&self) -> &'static str;    // subdiretório em data/<uf>/docs/
    fn rotulo(&self) -> &'static str; // frontend/manuais
    fn politica(&self) -> Rebuild;
    /// No repo: delega ao mddoc irmão (mddoc / mddoc_servico / mddoc_faq).
    fn parse(&self, md: &str) -> Result<Self::Doc>;
    /// No repo: delega ao compose irmão — as fórmulas congeladas NÃO mudam.
    fn text_to_embed(&self, d: &Self::Doc) -> String;
}

// ─── As quatro coleções (a Jurisprudencia serve a DUAS) ───────────────────────

pub struct Pareceres;
pub struct Tarf;
pub struct Servicos;
pub struct Faqs;

impl Colecao for Pareceres {
    type Doc = Jurisprudencia;
    fn kind(&self) -> &'static str { "pareceres" }
    fn dir(&self) -> &'static str { "pareceres" }
    fn rotulo(&self) -> &'static str { "Pareceres" }
    fn politica(&self) -> Rebuild { Rebuild::AppendOnly }
    fn parse(&self, _md: &str) -> Result<Jurisprudencia> {
        unimplemented!("repo: mddoc::parse_doc(md) → (header, sinopse, corpo)")
    }
    // Fórmula real dos pareceres: numero + assunto + resumo, uma por linha, pulando vazios.
    fn text_to_embed(&self, d: &Jurisprudencia) -> String {
        [d.numero.as_str(), d.assunto.as_str(), d.sinopse.as_str()]
            .into_iter().filter(|s| !s.is_empty())
            .collect::<Vec<_>>().join("\n")
    }
}

impl Colecao for Tarf {
    type Doc = Jurisprudencia; // MESMA struct, outro kind/dir — o ganho da fase 2
    fn kind(&self) -> &'static str { "tarf" }
    fn dir(&self) -> &'static str { "tarf" }
    fn rotulo(&self) -> &'static str { "Acórdãos TARF" }
    fn politica(&self) -> Rebuild { Rebuild::AppendOnly }
    fn parse(&self, md: &str) -> Result<Jurisprudencia> { Pareceres.parse(md) } // mesmo parser
    fn text_to_embed(&self, d: &Jurisprudencia) -> String { Pareceres.text_to_embed(d) }
}

impl Colecao for Servicos {
    type Doc = Servico;
    fn kind(&self) -> &'static str { "servicos" }
    fn dir(&self) -> &'static str { "servicos" }
    fn rotulo(&self) -> &'static str { "Serviços" }
    fn politica(&self) -> Rebuild { Rebuild::DeleteRebuild }
    fn parse(&self, _md: &str) -> Result<Servico> {
        unimplemented!("repo: mddoc_servico::parse(md)")
    }
    // Fórmula real: breadcrumb `tipo | classe` + título + snippet da descrição (300 chars).
    fn text_to_embed(&self, d: &Servico) -> String {
        let snippet: String = d.descricao.chars().take(300).collect();
        format!("{} | {}\n{}\n{}", d.tipo, d.classe, d.titulo, snippet)
    }
}

impl Colecao for Faqs {
    type Doc = Faq;
    fn kind(&self) -> &'static str { "faqs" }
    fn dir(&self) -> &'static str { "faqs" }
    fn rotulo(&self) -> &'static str { "Perguntas Frequentes" }
    fn politica(&self) -> Rebuild { Rebuild::DeleteRebuild }
    fn parse(&self, _md: &str) -> Result<Faq> {
        unimplemented!("repo: mddoc_faq::parse(md)")
    }
    // Template real da TAREFA-FAQ-PR: {assunto}\nP: {pergunta}\nR: {resposta},
    // linha omitida se o campo é vazio.
    fn text_to_embed(&self, d: &Faq) -> String {
        let mut out = String::new();
        if !d.origin.is_empty() { out.push_str(&d.origin); out.push('\n'); }
        out.push_str(&format!("P: {}\nR: {}", d.pergunta, d.resposta));
        out
    }
}

// ─── O encanamento genérico: UM laço, N coleções ──────────────────────────────
// O núcleo é genérico (dispatch estático, monomorfizado):

fn preparar<C: Colecao>(col: &C, mds: &[&str]) -> Result<Vec<String>> {
    println!("📄 {} ({}): {} documento(s) de docs/{}/",
        col.rotulo(), col.kind(), mds.len(), col.dir());
    mds.iter()
        .map(|md| Ok(col.text_to_embed(&col.parse(md)?)))
        .collect()
}

// PONTO FINO (a pegadinha do associated type): `dyn Colecao` não é object-safe por
// causa do `type Doc` — não dá para pôr as quatro num `Vec<Box<dyn Colecao>>`.
// Solução: uma casca dyn fina que ESCONDE o Doc, com blanket impl. É o mesmo
// padrão do auli-retrieval: motor genérico + faces finas.

pub trait ColecaoDyn {
    fn kind(&self) -> &'static str;
    fn preparar(&self, mds: &[&str]) -> Result<Vec<String>>;
}
impl<C: Colecao> ColecaoDyn for C {
    fn kind(&self) -> &'static str { Colecao::kind(self) }
    fn preparar(&self, mds: &[&str]) -> Result<Vec<String>> { preparar(self, mds) }
}

/// O registry: adicionar a 5ª coleção (súmulas, RFB, leis) = uma linha aqui
/// + uma struct + um impl. O compilador aponta o resto.
pub fn colecoes() -> Vec<Box<dyn ColecaoDyn>> {
    vec![Box::new(Pareceres), Box::new(Tarf), Box::new(Servicos), Box::new(Faqs)]
}

// ─── Exemplo com dados reais do projeto ───────────────────────────────────────

fn main() {
    let acordao = Jurisprudencia {
        numero: "ACÓRDÃO 510/24".into(),
        assunto: "ICMS. INDUSTRIALIZAÇÃO POR ENCOMENDA. ESTABELECIMENTO SIMULADO. \
                  DIFERIMENTO INAPLICÁVEL. OPERAÇÃO INTERESTADUAL TRIBUTADA. SIMULAÇÃO. \
                  RECURSO VOLUNTÁRIO. DESPROVIMENTO.".into(),
        link: "https://tarf.sefaz.rs.gov.br/documento/ee87f4bc-12b9-4666-02cb-08dee4034157".into(),
        sinopse: "Para fins da legislação tributária, estabelecimento é o local onde o \
                  sujeito passivo exerce suas atividades (artigo 5º, § 3 da Lei nº 8.820/89). \
                  No caso concreto, o estabelecimento gaúcho era comprovadamente inexistente, \
                  simulado em conluio com a contribuinte.".into(),
        corpo: "…texto integral (relatório + voto)…".into(),
    };

    let servico = Servico {
        titulo: "Solicitar isenção de IPVA para veículo de pessoa com deficiência".into(),
        tipo: "Serviço".into(),
        classe: "IPVA".into(),
        orgao: "Receita Estadual".into(),
        link: "https://atendimento.receita.rs.gov.br/isencao-ipva-pcd".into(),
        descricao: "Permite ao proprietário requerer a isenção do IPVA prevista para \
                    veículos adaptados…".into(),
    };

    let faq = Faq {
        origin: "IPVA".into(),
        pergunta: "Quem tem direito à isenção de IPVA por deficiência?".into(),
        resposta: "Pessoas com deficiência física, visual, mental severa ou profunda, \
                   ou autistas, conforme os requisitos da legislação estadual.".into(),
        url: "https://atendimento.receita.rs.gov.br/perguntas-frequentes/ipva#isencao".into(),
    };

    // O encanamento não sabe (nem precisa saber) qual coleção é qual:
    println!("[tarf]\n{}\n", Tarf.text_to_embed(&acordao));
    println!("[servicos]\n{}\n", Servicos.text_to_embed(&servico));
    println!("[faqs]\n{}\n", Faqs.text_to_embed(&faq));

    // E o laço do update vira:
    for col in colecoes() {
        println!("→ processaria docs/{}/ com política {}", col.kind(),
            "da coleção"); // no repo: ⏭️ se o diretório não existir na entidade
    }
}
```

Saída dos três `text_to_embed` (os packs que o BGE-M3 vetorizaria):

```text
[tarf]
ACÓRDÃO 510/24
ICMS. INDUSTRIALIZAÇÃO POR ENCOMENDA. ESTABELECIMENTO SIMULADO. DIFERIMENTO INAPLICÁVEL. OPERAÇÃO INTERESTADUAL TRIBUTADA. SIMULAÇÃO. RECURSO VOLUNTÁRIO. DESPROVIMENTO.
Para fins da legislação tributária, estabelecimento é o local onde o sujeito passivo exerce suas atividades (artigo 5º, § 3 da Lei nº 8.820/89). No caso concreto, o estabelecimento gaúcho era comprovadamente inexistente, simulado em conluio com a contribuinte.

[servicos]
Serviço | IPVA
Solicitar isenção de IPVA para veículo de pessoa com deficiência
Permite ao proprietário requerer a isenção do IPVA prevista para veículos adaptados…

[faqs]
IPVA
P: Quem tem direito à isenção de IPVA por deficiência?
R: Pessoas com deficiência física, visual, mental severa ou profunda, ou autistas, conforme os requisitos da legislação estadual.
```

## O que o exemplo demonstra

1. **Três shapes, um contrato.** Nenhuma super struct de dados: `Jurisprudencia`, `Servico` e
   `Faq` seguem distintas e fortes (campos obrigatórios, sem `Option` de união). O trait
   unifica só o que o encanamento consome.
2. **TARF de graça.** `Tarf` e `Pareceres` compartilham `type Doc = Jurisprudencia` e delegam
   parse/compose um ao outro — é o enum `TipoJurisprudencia` da fase 2 reexpresso como duas
   implementações do trait. A fase 2, portanto, não é jogada fora: é absorvida.
3. **A pegadinha real (e sua solução).** `dyn Colecao` não é object-safe por causa do
   `type Doc`; a casca `ColecaoDyn` com blanket impl esconde o Doc e permite o registry
   heterogêneo. Núcleo genérico + face fina — o mesmo padrão do `auli-retrieval`.
4. **Extensão = 3 peças.** Quinta coleção (súmulas, RFB, leis): struct de domínio (ou reuso
   de uma existente), `impl Colecao`, uma linha no registry. As fórmulas congeladas e os
   parsers irmãos não são tocados — o trait os cataloga, não os substitui.

---

## Adendo — a ampulheta: a struct geral existe, no DESTINO (argumento do Carlos)

A análise acima descartou a struct geral olhando para a ORIGEM (os shapes de domínio
divergem). O argumento correto olha para o DESTINO: no fim do funil, TODA coleção se reduz a
"um texto embeddado + um conteúdo recuperado". Nesse plano a struct geral é legítima — e a
`Consulta` do repo já carrega metade dela (`text_to_embed` + `doc_path` materializados):

```rust
/// A cintura do funil: o registro uniforme que o índice conhece.
/// Nasce da PROJEÇÃO de um doc de domínio; a infra abaixo dele é 100% genérica
/// (embed, pack, vector-store, /v1/retrieve, MCP, contexto RAG, citação do link).
pub struct Registro {
    pub chave: String,          // identidade estável (slug do numero, (url,pergunta), ...)
    pub kind: &'static str,     // coleção de origem
    pub titulo: String,         // rótulo humano na citação
    pub link: String,           // fonte oficial — o princípio do Auli
    pub text_to_embed: String,  // o que o BGE-M3 vetoriza
    pub doc_path: String,       // onde o conteúdo integral vive (o .md da árvore)
}
```

### Um exemplo por tabela — a mesma struct, quatro projeções

```rust
// ── PARECERES (rs) — chave = slug do numero; embed = numero+assunto+sinopse ──
let parecer = Registro {
    chave: "parecer-24-084".into(),
    kind: "pareceres",
    titulo: "Parecer 24/084".into(),
    link: "https://atendimento.receita.rs.gov.br/pareceres/24-084".into(),
    text_to_embed: "Parecer 24/084\n\
        ICMS. SUBSTITUIÇÃO TRIBUTÁRIA. RESSARCIMENTO. OPERAÇÃO INTERESTADUAL.\n\
        Contribuinte substituído que promove saída interestadual de mercadoria \
        recebida com ST tem direito ao ressarcimento do imposto retido, mediante \
        os procedimentos do Livro III do RICMS.".into(),
    doc_path: "data/rs/docs/pareceres/parecer-24-084.md".into(),
};

// ── TARF (rs) — MESMA projeção dos pareceres (Doc = Jurisprudencia), outro kind/dir ──
let acordao = Registro {
    chave: "acordao-510-24".into(),
    kind: "tarf",
    titulo: "ACÓRDÃO 510/24".into(),
    link: "https://tarf.sefaz.rs.gov.br/documento/ee87f4bc-12b9-4666-02cb-08dee4034157".into(),
    text_to_embed: "ACÓRDÃO 510/24\n\
        ICMS. INDUSTRIALIZAÇÃO POR ENCOMENDA. ESTABELECIMENTO SIMULADO. DIFERIMENTO \
        INAPLICÁVEL. OPERAÇÃO INTERESTADUAL TRIBUTADA. SIMULAÇÃO. RECURSO VOLUNTÁRIO. \
        DESPROVIMENTO.\n\
        Para fins da legislação tributária, estabelecimento é o local onde o sujeito \
        passivo exerce suas atividades (artigo 5º, § 3 da Lei nº 8.820/89). No caso \
        concreto, o estabelecimento gaúcho era comprovadamente inexistente, simulado \
        em conluio com a contribuinte.".into(),
    doc_path: "data/rs/docs/tarf/acordao-510-24.md".into(),
};

// ── SERVIÇOS (rs) — chave = slug(titulo)+hash8(link); embed = breadcrumb+título+snippet ──
let servico = Registro {
    chave: "solicitar-isencao-de-ipva-para-veiculo-de-pessoa-com-defici-3fa9c21b".into(),
    kind: "servicos",
    titulo: "Solicitar isenção de IPVA para veículo de pessoa com deficiência".into(),
    link: "https://atendimento.receita.rs.gov.br/isencao-ipva-pcd".into(),
    text_to_embed: "Serviço | IPVA\n\
        Solicitar isenção de IPVA para veículo de pessoa com deficiência\n\
        Permite ao proprietário requerer a isenção do IPVA prevista para veículos \
        adaptados ou conduzidos por pessoa com deficiência, mediante laudo médico \
        e documentação do veículo…".into(), // snippet: 300 chars da descrição
    doc_path: "data/rs/docs/servicos/solicitar-isencao-de-ipva-para-veiculo-de-pessoa-com-defici-3fa9c21b.md".into(),
};

// ── FAQS (rs) — chave = slug(pergunta)~60+hash8(url+pergunta); embed = template P:/R: ──
let faq = Registro {
    chave: "quem-tem-direito-a-isencao-de-ipva-por-deficiencia-b07d41ce".into(),
    kind: "faqs",
    titulo: "Quem tem direito à isenção de IPVA por deficiência?".into(),
    link: "https://atendimento.receita.rs.gov.br/perguntas-frequentes/ipva#isencao".into(),
    text_to_embed: "IPVA\n\
        P: Quem tem direito à isenção de IPVA por deficiência?\n\
        R: Pessoas com deficiência física, visual, mental severa ou profunda, ou \
        autistas, conforme os requisitos da legislação estadual.".into(),
    doc_path: "data/rs/docs/faqs/quem-tem-direito-a-isencao-de-ipva-por-deficiencia-b07d41ce.md".into(),
};
```

### Leitura dos exemplos

- **Cada campo tem uma convenção POR COLEÇÃO, mas o shape é um só.** `chave` reusa as
  identidades já decididas (slug do numero; slug do título + hash8 do link; slug da pergunta
  truncado + hash8 de url+pergunta). `titulo` é o rótulo de citação natural de cada tabela
  (numero, título do serviço, a própria pergunta). `text_to_embed` é exatamente a saída das
  fórmulas congeladas — nada re-embedda.
- **`doc_path` uniformiza a doutrina "pack leve".** Depois das migrações de julho, as três
  coleções têm árvore .md; o conteúdo integral é sempre lazy via `doc_path`, para todas —
  a regra dos pareceres promovida a regra geral.
- **Pareceres e TARF produzem Registros IDÊNTICOS em forma** (mesma projeção, kinds
  distintos) — a fase 2 aparece aqui como "duas linhas da mesma projeção".
- **Onde o Registro NÃO entra:** frontend (as tabs continuam consumindo os JSONs de domínio —
  a tab de Serviços precisa de tipo/classe/orgao em colunas) e as bordas de coleta. A
  ampulheta: domínio forte nas bordas, `Registro` na cintura, infra genérica abaixo.
- **O trait `Colecao` encolhe e ganha foco:** `parse` + `fn projetar(&self, d: &Self::Doc)
  -> Registro` + metadados (kind, política de rebuild, política de retrieval — os 10+20 de
  serviços/FAQs vs. score bands dos pareceres viram config por coleção, não shape).
- **Custo real:** unificar o formato de pack é migração (re-empacotar tudo) — barata, porque
  os vetores não mudam.

---

## Adendo 2 — A struct genérica ÚNICA que acomoda todas as tabelas

Se a pergunta é "uma struct só, capaz de carregar qualquer linha de qualquer tabela sem perder
o domínio", a forma idiomática em Rust não é a união de `Option`s — é **núcleo comum + soma
tipada** (enum de domínio). Uma struct, sem campo opcional, com o compilador garantindo que
cada variante carrega exatamente os campos da sua tabela:

```rust
use serde::{Deserialize, Serialize};

/// O documento universal do acervo: UMA struct para qualquer tabela.
/// O núcleo é o `Registro` (o que a infra genérica consome); `dominio` carrega
/// o resto dos campos, tipado por variante — nada de Option, nada se perde.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Documento {
    // ── núcleo: o Registro, achatado ─────────────────────────────
    pub chave: String,
    pub kind: Kind,
    pub titulo: String,
    pub link: String,
    pub text_to_embed: String,
    pub doc_path: String,
    // ── o domínio específico da tabela ───────────────────────────
    pub dominio: Dominio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind { Pareceres, Tarf, Servicos, Faqs }

/// Os campos que NÃO são comuns, sem perda e sem Option:
/// cada variante é o shape exato da sua tabela (menos o que já subiu ao núcleo).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")] // JSON: {"tipo":"jurisprudencia", ...}
pub enum Dominio {
    /// Pareceres E acórdãos TARF (mesma variante — quem distingue é o `kind`).
    Jurisprudencia {
        numero: String,   // "ACÓRDÃO 510/24" (== titulo aqui; redundância aceita p/ round-trip)
        assunto: String,  // ementa
        sinopse: String,
    },
    Servico {
        tipo: String,
        classe: String,
        orgao: String,
    },
    Faq {
        origin: String,   // pode ser vazio
        pergunta: String, // == titulo; redundância aceita p/ round-trip
        resposta: String,
    },
}

impl Documento {
    /// O acesso genérico que a infra usa — sem match, sem saber a tabela.
    pub fn registro(&self) -> (&str, &str, &str) {
        (&self.chave, &self.text_to_embed, &self.doc_path)
    }
    /// O acesso de domínio, quando alguém PRECISA saber — e aí o match é
    /// exaustivo: tabela nova ⇒ o compilador aponta cada ponto a tratar.
    pub fn resumo_curto(&self) -> &str {
        match &self.dominio {
            Dominio::Jurisprudencia { sinopse, .. } => sinopse,
            Dominio::Servico { classe, .. } => classe,
            Dominio::Faq { resposta, .. } => resposta,
        }
    }
}
```

### As quatro tabelas na MESMA struct

```rust
let docs: Vec<Documento> = vec![
    Documento {
        chave: "parecer-24-084".into(),
        kind: Kind::Pareceres,
        titulo: "Parecer 24/084".into(),
        link: "https://atendimento.receita.rs.gov.br/pareceres/24-084".into(),
        text_to_embed: "Parecer 24/084\nICMS. SUBSTITUIÇÃO TRIBUTÁRIA. RESSARCIMENTO…\n\
                        Contribuinte substituído que promove saída interestadual…".into(),
        doc_path: "data/rs/docs/pareceres/parecer-24-084.md".into(),
        dominio: Dominio::Jurisprudencia {
            numero: "Parecer 24/084".into(),
            assunto: "ICMS. SUBSTITUIÇÃO TRIBUTÁRIA. RESSARCIMENTO…".into(),
            sinopse: "Contribuinte substituído que promove saída interestadual…".into(),
        },
    },
    Documento {
        chave: "acordao-510-24".into(),
        kind: Kind::Tarf, // mesma variante Jurisprudencia, kind distinto
        titulo: "ACÓRDÃO 510/24".into(),
        link: "https://tarf.sefaz.rs.gov.br/documento/ee87f4bc-12b9-4666-02cb-08dee4034157".into(),
        text_to_embed: "ACÓRDÃO 510/24\nICMS. INDUSTRIALIZAÇÃO POR ENCOMENDA…\n\
                        Para fins da legislação tributária, estabelecimento é o local…".into(),
        doc_path: "data/rs/docs/tarf/acordao-510-24.md".into(),
        dominio: Dominio::Jurisprudencia {
            numero: "ACÓRDÃO 510/24".into(),
            assunto: "ICMS. INDUSTRIALIZAÇÃO POR ENCOMENDA. ESTABELECIMENTO SIMULADO…".into(),
            sinopse: "Para fins da legislação tributária, estabelecimento é o local…".into(),
        },
    },
    Documento {
        chave: "solicitar-isencao-de-ipva-para-veiculo-de-pessoa-com-defici-3fa9c21b".into(),
        kind: Kind::Servicos,
        titulo: "Solicitar isenção de IPVA para veículo de pessoa com deficiência".into(),
        link: "https://atendimento.receita.rs.gov.br/isencao-ipva-pcd".into(),
        text_to_embed: "Serviço | IPVA\nSolicitar isenção de IPVA…\nPermite ao proprietário…".into(),
        doc_path: "data/rs/docs/servicos/solicitar-isencao-…-3fa9c21b.md".into(),
        dominio: Dominio::Servico {
            tipo: "Serviço".into(),
            classe: "IPVA".into(),
            orgao: "Receita Estadual".into(),
        },
    },
    Documento {
        chave: "quem-tem-direito-a-isencao-de-ipva-por-deficiencia-b07d41ce".into(),
        kind: Kind::Faqs,
        titulo: "Quem tem direito à isenção de IPVA por deficiência?".into(),
        link: "https://atendimento.receita.rs.gov.br/perguntas-frequentes/ipva#isencao".into(),
        text_to_embed: "IPVA\nP: Quem tem direito à isenção de IPVA por deficiência?\n\
                        R: Pessoas com deficiência física, visual, mental severa…".into(),
        doc_path: "data/rs/docs/faqs/quem-tem-direito-…-b07d41ce.md".into(),
        dominio: Dominio::Faq {
            origin: "IPVA".into(),
            pergunta: "Quem tem direito à isenção de IPVA por deficiência?".into(),
            resposta: "Pessoas com deficiência física, visual, mental severa…".into(),
        },
    },
];

// O laço genérico funciona sobre o Vec heterogêneo, sem match:
for d in &docs {
    let (chave, embed, path) = d.registro();
    println!("[{:?}] {} → {} ({} chars de embed)", d.kind, chave, path, embed.len());
}
```

### Trade-offs desta forma (honestidade do estudo)

- **Ganha:** um único `Vec<Documento>`, um único formato de pack/manifesto/JSON (serde com
  `tag = "tipo"` serializa limpo), acesso genérico sem match pelo núcleo, acesso de domínio
  COM match exaustivo — tabela nova quebra a compilação em cada ponto que precisa tratá-la,
  em vez de passar batida como um `Option` `None`.
- **Custa:** mundo fechado — adicionar tabela = tocar o enum (ok para um acervo controlado
  num repo só; era exatamente o espírito do `TipoJurisprudencia`); o `match` se espalha pelos
  pontos que precisam do domínio (o preço da exaustividade); leve redundância núcleo×variante
  (`titulo`/`numero`/`pergunta`) aceita para round-trip fiel com as árvores .md.
- **Relação com os adendos anteriores:** `Documento` = `Registro` (núcleo) + `Dominio`
  (a soma). O trait `Colecao` não morre — vira o produtor: `projetar(&Doc) -> Documento`.
  E o frontend continua fora: as tabs consomem os JSONs de domínio, não o `Documento`.
- **Variante genérica alternativa** (se um dia quiser homogeneidade estática):
  `struct Doc<D> { nucleo: Registro, dominio: D }` com `type DocDin = Doc<Dominio>` — a mesma
  ideia parametrizada, útil se algum caminho de código quiser trabalhar só com
  `Doc<Jurisprudencia>` sem match.

---

## Adendo 3 — Unificação FUNCIONAL: campos nomeados pelo papel, marcadores unificados

Argumento do Carlos (aceito): os nomes que divergem entre tabelas (numero/titulo/pergunta;
assunto; sinopse/resposta; tipo|classe/origin) cumprem A MESMA função nos dois funis do chat
(embed + bloco RAG) — e os marcadores do bloco somos NÓS que geramos, dos dois lados (o código
que monta o contexto E os prompts que instruem a LLM a lê-lo). Logo, tudo é unificável.

### 3.1 Os campos, renomeados pelo papel funcional

Verificação empírica no código (rag.rs, lib.rs — stored_repr/render_consulta_block): o chat
nunca consome campos de domínio diretamente; consome duas projeções (text_to_embed e o bloco
`## pergunta`/`## resposta`) mais o link. A tabela de papéis:

| papel (nome unificado) | jurisprudência | serviço | faq |
|---|---|---|---|
| `titulo` — identificação humana | numero | titulo | pergunta |
| `trilha` — classificação/breadcrumb | *(vazio)* | tipo \| classe | origin |
| `ementa` — síntese junto ao título | assunto | *(vazio)* | *(vazio)* |
| `resumo` — síntese embedável (e fallback de degradação) | sinopse | snippet descrição | resposta |
| conteúdo integral — o `## resposta` | corpo via doc_path | descrição | resposta |

`orgao` fica FORA da struct geral: não entra em funil nenhum do chat (verificado) — vive só
no JSON de domínio do frontend.

```rust
pub struct Documento {
    pub chave: String,
    pub kind: Kind,
    pub link: String,
    pub doc_path: String,  // conteúdo integral lazy, para TODAS as tabelas
    pub titulo: String,
    pub trilha: String,    // "" = sem classificação
    pub ementa: String,    // "" = sem síntese-de-título
    pub resumo: String,
}
```

### 3.2 As duas projeções viram UMA fórmula cada — com paridade byte a byte

```rust
// UM compose (a geometria: trilha ANTES do título, ementa DEPOIS — os slots
// vazios se anulam e cada tabela reproduz seu formato congelado):
fn text_to_embed(d: &Documento) -> String {
    [&d.trilha, &d.titulo, &d.ementa, &d.resumo]
        .iter().filter(|s| !s.is_empty()).map(|s| s.as_str())
        .collect::<Vec<_>>().join("\n")
}
// jurisprudência: numero\nassunto\nsinopse  ✓ byte-idêntico à fórmula atual
// serviço: tipo|classe\ntitulo\nsnippet     ✓ byte-idêntico

// UM template de bloco:
fn bloco(d: &Documento, conteudo: &str) -> String {
    let mut p = String::from("## pergunta\n");
    if !d.trilha.is_empty() { p.push_str(&d.trilha); p.push('\n'); }
    p.push_str(&d.titulo);
    if !d.ementa.is_empty() { p.push('\n'); p.push_str(&d.ementa); }
    format!("{p}\n\n## resposta\n{conteudo}\nLink: {}", d.link)
}
// reproduz render_consulta_block, stored_repr do Servico e da Faq — inclusive
// a omissão do origin vazio.                 ✓ byte-idêntico nos três
```

Exceção única: o embed de FAQ pós-TAREFA-FAQ-PR tem prefixos `P:`/`R:` que a fórmula
unificada não produz. Quem decide é o gate A/B de retrieval (P5) já instituído — testar a
fórmula unificada NA MESMA rodada do gate, para não migrar os embeddings de FAQ duas vezes
(a migração com re-embed já está planejada; carona nela).

### 3.3 Os marcadores — inventário e unificação

O template interno JÁ é unificado (`## pergunta` / `## resposta` / `Link:` nos três
stored_repr). Divergem só os invólucros externos (rag.rs:157–175):

| hoje | coleção |
|---|---|
| `\n## servico\n{i}\n` | serviços |
| `\n// Resultado: {i}\n` | faqs |
| `\n## PARECER\n{i}\n` | pareceres |
| `\n## PARECER RELACIONADO\n{i}\n` | expansão por grafo |

E os prompts referenciam esse vocabulário (linha ~14 dos 27 estaduais: "Cada serviço do
texto inicia com o marcador: ## pergunta"; os 4 `*-pareceres.txt`: "Cada consulta inicia com
o marcador: ## PARECER").

Proposta — UM invólucro, com o rótulo da coleção como DADO (o `rotulo()` do trait):

```text
\n## documento {i}: {rotulo}\n{bloco}\n
```

Ex.: `## documento 1: Serviço`, `## documento 2: FAQ`, `## documento 3: Parecer`,
`## documento 4: Acórdão TARF`, `## documento 5: Parecer relacionado`. A LLM continua
sabendo a natureza da fonte (importa para a citação), mas o TEMPLATE é um só. A instrução
dos prompts vira uma frase única compartilhada: "Cada documento inicia com o marcador
`## documento`; dentro dele, o título vem após `## pergunta` e o conteúdo após
`## resposta`."

### 3.4 Disciplina de migração (nada disso re-embedda, exceto FAQ)

1. **Marcadores e bloco**: mudança de SERVING apenas — nenhum vetor muda. As travas "pina o
   formato" são atualizadas deliberadamente no mesmo commit (elas existem para impedir
   mudança ACIDENTAL, não decidida); a paridade chat×MCP é automática (mesma função
   `montar_*`).
2. **Prompts**: atualizar a frase do marcador nos 31 arquivos; testar primeiro no `rs.txt`
   com perguntas reais (o padrão já usado na mudança Resumo/Detalhes) antes de replicar.
3. **Campos/fórmula de embed**: jurisprudência e serviços saem byte-idênticos (prova em
   3.2) — sem re-embed; FAQ decide no gate P5, pegando carona na migração já planejada.
4. **Sequência com as fases**: esta unificação é a FORMA FINAL da fase 3 — não bloqueia a
   fase 1 (scraper TARF, em andamento) nem a fase 2 (`Jurisprudencia`), que continuam como
   degraus; o gatilho segue sendo a quarta coleção (ou o momento do gate P5, o que vier
   primeiro para a parte de FAQ).
