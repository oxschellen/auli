// SEFAZ-GO FAQs — as "Perguntas Frequentes" do site Orientação Tributária da Economia-GO
// (`orientacaotributaria.sefaz.go.gov.br`), um SharePoint instalado no órgão, à parte da API do Portal
// Expresso dos serviços (`go.rs`). Estudo, medições e decisões: docs/faqs/ESTUDO-GO-Faqs.md (D-GOFAQ-*).
//
// A lista "Perguntas Frequentes" sai inteira da REST `_api` anônima, num GET. A categoria de cada pergunta é
// um termo de taxonomia, e a REST só dá o nome do termo — "Informações Gerais" nomeia uns 20 termos de temas
// diferentes. O caminho ("Decreto nº 4.852/97 - RCTE:Informações Gerais") só aparece no formulário público do
// item (`DispForm.aspx`; o campo é configurado para exibir o caminho): a coleta lê um formulário por termo.
// A lista oculta da taxonomia e a página de FAQ do site exigem login.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;
use serde::Deserialize;
use ureq::Agent;

use auli_contract::FaqRaw;
use auli_scraper_kit::clean;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::http::GetOpts;

/// O host que serve o site. O `orientacaotributaria.economia.go.gov.br`, que a página declara canônico, responde
/// 404 a tudo (02/11/2026).
const BASE: &str = "https://orientacaotributaria.sefaz.go.gov.br";
/// O nome do site. Abre a trilha de cada pergunta, com o ano da alteração mais recente da lista (D-GOFAQ-3).
const FONTE: &str = "Orientação Tributária";
/// O campo oculto que acompanha a categoria (o `Categoria_0` do SharePoint): `Nome|TermGuid`.
const NOME_DO_TERMO: &str = "m611689e352b49adae244c5880ecdf12";

/// A identidade das FAQs da frota (D-GOFAQ-5); o site não tem `robots.txt` (404).
const UA: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";
const COURTESY: Duration = Duration::from_millis(1000);

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. As guardas
/// (D-GOFAQ-4) rodam antes de qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = auli_scraper_kit::build_agent(UA, Some(Duration::from_secs(120)));
    let url = format!(
        "{BASE}/_api/web/lists/getbytitle('Perguntas%20Frequentes')/items?$top=5000\
         &$select=Id,Pergunta0,Resposta,CategoriaId,{NOME_DO_TERMO},Modified"
    );
    let itens = itens(&buscar(
        data_dir,
        &agent,
        &url,
        Some("application/json;odata=verbose"),
        use_cache,
    )?)?;
    let termos = um_item_por_termo(&itens);
    println!(
        "GO FAQs: {} itens em {} termos — lendo um formulário por termo",
        itens.len(),
        termos.len()
    );

    let mut caminhos = BTreeMap::new();
    for (termo, id) in termos {
        let pagina = buscar(data_dir, &agent, &link_do_item(id), None, use_cache)?;
        let caminho =
            caminho_da_categoria(&pagina).with_context(|| format!("formulário do item {id}"))?;
        caminhos.insert(termo.to_string(), caminho);
    }

    let perguntas = montar(itens, &caminhos)?;
    let fonte = rotulo_da_fonte(&perguntas)?;
    let faqs = consolidar(&fonte, &perguntas);

    // Atômico pelo mesmo motivo das outras entidades: a árvore não tem outra fonte (o snapshot achatado
    // não a reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&montar_arvore(&fonte, &perguntas))?,
    )?;
    println!("Wrote {tree_path} (árvore de FAQ p/ o frontend)");

    auli_contract::snapshot::write_faqs(crate::ENTITY, data_dir, &crate::scraper_info(), faqs)?;
    println!("🎉 Coleta de FAQs gravada no snapshot.");
    Ok(())
}

/// Uma página (D-GOFAQ-5): rede sempre, gravando o cache; com `--usecache`, só o cache, e miss é erro.
fn buscar(
    data_dir: &str,
    agent: &Agent,
    url: &str,
    accept: Option<&str>,
    use_cache: bool,
) -> Result<String> {
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", url)
            .ok_or_else(|| anyhow!("cache miss para {url} (modo --usecache, sem rede)"));
    }
    sleep(COURTESY);
    let corpo = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "GO FAQs",
            accept,
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", url, &corpo);
    Ok(corpo)
}

#[derive(Deserialize)]
struct Envelope {
    d: Resultados,
}

#[derive(Deserialize)]
struct Resultados {
    results: Vec<Item>,
    #[serde(rename = "__next")]
    next: Option<String>,
}

#[derive(Deserialize)]
struct Item {
    #[serde(rename = "Id")]
    id: u32,
    /// A pergunta, em HTML. O `Title` da lista vem vazio.
    #[serde(rename = "Pergunta0")]
    pergunta: Option<String>,
    #[serde(rename = "Resposta")]
    resposta: Option<String>,
    /// O `TermGuid` da categoria.
    #[serde(rename = "CategoriaId")]
    termo: Option<String>,
    /// [`NOME_DO_TERMO`]: `Nome|TermGuid`.
    #[serde(rename = "m611689e352b49adae244c5880ecdf12")]
    nome_do_termo: Option<String>,
    #[serde(rename = "Modified")]
    modificado: String,
}

/// A lista inteira. `$top=5000` a traz numa página (1.298 itens em 02/11/2026): se vier `__next`, ela passou
/// do teto e a coleta ficaria pela metade.
fn itens(json: &str) -> Result<Vec<Item>> {
    let e: Envelope = serde_json::from_str(json)
        .context("resposta da _api fora do formato {\"d\": {\"results\": [...]}}")?;
    if e.d.next.is_some() {
        bail!("a _api paginou a lista (__next) — passou de 5.000 itens?");
    }
    if e.d.results.is_empty() {
        bail!("a lista de perguntas veio vazia — a _api mudou?");
    }
    Ok(e.d.results)
}

/// Termo → o menor `Id` que o usa: o formulário dele dá o caminho do termo.
fn um_item_por_termo(itens: &[Item]) -> BTreeMap<&str, u32> {
    let mut termos = BTreeMap::new();
    for item in itens {
        if let Some(t) = item.termo.as_deref() {
            let id = termos.entry(t).or_insert(item.id);
            *id = (*id).min(item.id);
        }
    }
    termos
}

static CATEGORIA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""Categoria":("(?:[^"\\]|\\.)*")"#).unwrap());

/// O caminho do termo no JSON que o formulário embute (`WPQ1FormCtx`): `"Categoria":"Pai:Termo"`, com os
/// escapes `\uXXXX` do JSON. Há exatamente um por página; zero ou dois é página que mudou.
fn caminho_da_categoria(pagina: &str) -> Result<String> {
    let mut achados = CATEGORIA_RE.captures_iter(pagina);
    let (Some(c), None) = (achados.next(), achados.next()) else {
        bail!("o formulário não traz exatamente um \"Categoria\" — a página mudou?");
    };
    Ok(serde_json::from_str(&c[1])?)
}

struct Pergunta {
    id: u32,
    pergunta: String,
    resposta: String,
    /// Os níveis da categoria, do tema ao termo.
    caminho: Vec<String>,
    modificado: String,
}

/// As perguntas, na ordem do `Id`, com as guardas (D-GOFAQ-4).
///
/// O caminho é o do formulário, que mostra o nome ATUAL do termo; o campo oculto de cada item guarda o nome de
/// quando o item foi salvo. Um termo renomeado ("Informações Fiscais" → "Informações Gerais") deixa itens com o
/// nome antigo (3 em 02/11/2026) — eles seguem o formulário, com aviso. O que pega formulário mal lido é a
/// conferência por termo: o caminho tem de terminar no nome que algum item dá ao termo.
fn montar(itens: Vec<Item>, caminhos: &BTreeMap<String, String>) -> Result<Vec<Pergunta>> {
    let mut ids = BTreeSet::new();
    let mut conferidos = BTreeSet::new();
    let mut nome_antigo = 0;
    let mut perguntas = Vec::new();
    for item in itens {
        let id = item.id;
        if !ids.insert(id) {
            bail!("o item {id} veio duas vezes — a _api mudou?");
        }
        let pergunta = uma_linha(item.pergunta.as_deref().unwrap_or(""));
        if pergunta.is_empty() {
            bail!("o item {id} não tem pergunta");
        }
        let termo = item
            .termo
            .as_deref()
            .ok_or_else(|| anyhow!("o item {id} não tem categoria"))?;
        let nome = item
            .nome_do_termo
            .as_deref()
            .and_then(|n| n.split('|').next())
            .filter(|n| !n.is_empty())
            .ok_or_else(|| anyhow!("o item {id} não traz o nome do termo {termo}"))?;
        let categoria = caminhos
            .get(termo)
            .ok_or_else(|| anyhow!("sem o caminho do termo {termo} (item {id})"))?;
        let caminho: Vec<String> = categoria.split(':').map(clean).collect();
        if caminho.iter().any(String::is_empty) {
            bail!("a categoria {categoria:?} do formulário tem nível vazio (item {id})");
        }
        if caminho.last().map(String::as_str) == Some(clean(nome).as_str()) {
            conferidos.insert(termo.to_string());
        } else {
            nome_antigo += 1;
        }
        perguntas.push(Pergunta {
            id,
            pergunta,
            resposta: html_para_texto(item.resposta.as_deref().unwrap_or("")),
            caminho,
            modificado: item.modificado,
        });
    }
    if let Some((termo, categoria)) = caminhos.iter().find(|(t, _)| !conferidos.contains(*t)) {
        bail!(
            "o formulário dá a categoria {categoria:?}, que não termina no nome que algum item dá ao termo \
             {termo} — formulário mal lido?"
        );
    }
    if nome_antigo > 0 {
        eprintln!(
            "⚠️  GO FAQs: {nome_antigo} item(ns) guardam um nome antigo do termo — vale o do formulário"
        );
    }
    perguntas.sort_by_key(|p| p.id);
    Ok(perguntas)
}

/// `Orientação Tributária (até 2022)`: o ano da alteração mais recente da lista (D-GOFAQ-3, decisão do
/// mantenedor). O site avisa que a FAQ está "atualizada até 31/12/2019", mas há itens alterados até 2022: o
/// rótulo marca a idade do acervo inteiro na trilha, que chega ao chat, sem data por pergunta.
fn rotulo_da_fonte(perguntas: &[Pergunta]) -> Result<String> {
    let ano = perguntas
        .iter()
        .filter_map(|p| p.modificado.get(..4))
        .filter(|a| a.bytes().all(|b| b.is_ascii_digit()))
        .max()
        .ok_or_else(|| anyhow!("nenhum item com data de alteração — a _api mudou?"))?;
    Ok(format!("{FONTE} (até {ano})"))
}

fn consolidar(fonte: &str, perguntas: &[Pergunta]) -> Vec<FaqRaw> {
    let sem_resposta = perguntas.iter().filter(|p| p.resposta.is_empty()).count();
    if sem_resposta > 0 {
        eprintln!("⚠️  GO FAQs: {sem_resposta} pergunta(s) com resposta vazia depois da conversão");
    }
    println!("GO FAQs: {} perguntas", perguntas.len());
    perguntas
        .iter()
        .map(|p| FaqRaw {
            pergunta: p.pergunta.clone(),
            resposta: p.resposta.clone(),
            origin: trilha(fonte, &p.caminho),
            url: link_do_item(p.id),
        })
        .collect()
}

fn trilha(fonte: &str, caminho: &[String]) -> String {
    format!("{fonte} | {}", caminho.join(" | "))
}

/// O formulário público do item (D-GOFAQ-2): um link por pergunta — 49 textos de pergunta se repetem na
/// lista, e o nome do `.md` sai de link + pergunta.
fn link_do_item(id: u32) -> String {
    format!("{BASE}/Lists/PerguntasFrequentes/DispForm.aspx?ID={id}")
}

#[derive(Default)]
struct Ramo<'a> {
    filhos: BTreeMap<&'a str, Ramo<'a>>,
    perguntas: Vec<&'a Pergunta>,
}

/// A árvore da aba de FAQs: raiz → o site (com o ano na marca) → os níveis da categoria. A interface começa
/// pelos filhos da raiz. Os nós apontam para a página inicial do site — a página de FAQ exige login — e cada
/// pergunta tem o seu formulário.
fn montar_arvore(fonte: &str, perguntas: &[Pergunta]) -> FaqNode {
    let mut site = Ramo::default();
    for p in perguntas {
        let mut ramo = &mut site;
        for nivel in &p.caminho {
            ramo = ramo.filhos.entry(nivel.as_str()).or_default();
        }
        ramo.perguntas.push(p);
    }
    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: pagina_inicial(),
        page_type: PageType::Menu,
        origin: String::new(),
        children: vec![no(fonte, fonte, &site)],
        faq_items: Vec::new(),
    }
}

fn pagina_inicial() -> String {
    format!("{BASE}/Paginas/default.aspx")
}

fn no(titulo: &str, trilha: &str, ramo: &Ramo) -> FaqNode {
    FaqNode {
        title: titulo.to_string(),
        url: pagina_inicial(),
        page_type: if ramo.filhos.is_empty() {
            PageType::Faq
        } else {
            PageType::Menu
        },
        origin: if ramo.perguntas.is_empty() {
            String::new()
        } else {
            trilha.to_string()
        },
        children: ramo
            .filhos
            .iter()
            .map(|(t, r)| no(t, &format!("{trilha} | {t}"), r))
            .collect(),
        faq_items: ramo
            .perguntas
            .iter()
            .map(|p| FaqItem {
                pergunta: p.pergunta.clone(),
                resposta: p.resposta.clone(),
                url: Some(link_do_item(p.id)),
            })
            .collect(),
    }
}

/// HTML → texto pelo conversor da frota (`auli_scraper_kit::html`, D-KITHTML-1): links da raiz com o host do
/// `BASE`.
fn html_para_texto(s: &str) -> String {
    auli_scraper_kit::html::html_para_texto(
        s,
        auli_scraper_kit::html::Opcoes {
            base: Some(BASE),
            quebras_do_fonte: false,
        },
    )
}

/// Uma pergunta numa linha só.
fn uma_linha(s: &str) -> String {
    clean(&html_para_texto(s).replace('\n', " "))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Recortes da coleta de 02/11/2026: 10 itens da lista (a pergunta repetida em dois termos, 122 e 123; o
    // link relativo para um parecer, 108; o link ftp, 60; a tabela, 718; os símbolos matemáticos, 1092; a
    // resposta que é só imagem, 1198; os dois "Informações Gerais" de temas diferentes, 66 e 758) e o JSON
    // da categoria que o formulário do item 66 embute (a página reduzida ao `<script>` que o contém). Os itens
    // estão na ordem inversa do `Id`, para a ordenação ter o que provar.
    const ITENS: &str = include_str!("../tests/fixtures/go-faqs-itens.json");
    const FORMULARIO_66: &str = include_str!("../tests/fixtures/go-faqs-dispform-66.html");

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    /// O caminho de cada termo do recorte: os de 66 e 758 lidos dos formulários reais; os outros com um tema
    /// inventado na frente do nome real.
    fn caminhos(itens: &[Item]) -> BTreeMap<String, String> {
        let mut c: BTreeMap<String, String> = itens
            .iter()
            .map(|i| {
                let nome = i
                    .nome_do_termo
                    .as_deref()
                    .unwrap()
                    .split('|')
                    .next()
                    .unwrap();
                (i.termo.clone().unwrap(), format!("Tema {}:{nome}", i.id))
            })
            .collect();
        c.insert(
            "53b0c621-8068-4243-a82e-f1bc5504ec5e".into(),
            caminho_da_categoria(FORMULARIO_66).unwrap(),
        );
        c.insert(
            "e0d26000-8072-4538-8dde-c822bc928574".into(),
            "Substituição Tributária pelas Operações Posteriores:Informações Gerais".into(),
        );
        c
    }

    fn perguntas() -> Vec<Pergunta> {
        let itens = itens(ITENS).unwrap();
        let c = caminhos(&itens);
        montar(itens, &c).unwrap()
    }

    fn pergunta(id: u32) -> Pergunta {
        perguntas().into_iter().find(|p| p.id == id).unwrap()
    }

    #[test]
    fn le_a_lista_e_um_item_por_termo() {
        let itens = itens(ITENS).unwrap();
        assert_eq!(itens.len(), 10);
        let termos = um_item_por_termo(&itens);
        assert_eq!(termos.len(), 9);
        assert_eq!(termos["e0d26000-8072-4538-8dde-c822bc928574"], 758);
    }

    #[test]
    fn perguntas_saem_na_ordem_do_id() {
        let ids: Vec<u32> = perguntas().iter().map(|p| p.id).collect();
        assert_eq!(ids, [4, 60, 66, 108, 122, 123, 718, 758, 1092, 1198]);
    }

    #[test]
    fn lista_paginada_vazia_ou_fora_do_formato_e_erro() {
        let e = erro(itens(
            r#"{"d":{"results":[],"__next":"https://x/_api/...?$skiptoken=1"}}"#,
        ));
        assert!(e.contains("paginou"), "{e}");
        let e = erro(itens(r#"{"d":{"results":[]}}"#));
        assert!(e.contains("vazia"), "{e}");
        let e = erro(itens(r#"{"error":{"message":"Acesso negado"}}"#));
        assert!(e.contains("fora do formato"), "{e}");
    }

    #[test]
    fn caminho_vem_do_json_do_formulario() {
        assert_eq!(
            caminho_da_categoria(FORMULARIO_66).unwrap(),
            "Decreto nº 4.852/97 - RCTE:Informações Gerais"
        );
        let duas = format!("{FORMULARIO_66}{FORMULARIO_66}");
        for pagina in ["<html>login</html>", duas.as_str()] {
            let e = erro(caminho_da_categoria(pagina));
            assert!(e.contains("exatamente um"), "{e}");
        }
    }

    #[test]
    fn os_dois_informacoes_gerais_ficam_em_temas_diferentes() {
        assert_eq!(
            pergunta(66).caminho,
            ["Decreto nº 4.852/97 - RCTE", "Informações Gerais"]
        );
        assert_eq!(
            pergunta(758).caminho,
            [
                "Substituição Tributária pelas Operações Posteriores",
                "Informações Gerais"
            ]
        );
    }

    #[test]
    fn caminho_que_nao_termina_no_termo_e_erro() {
        let itens = itens(ITENS).unwrap();
        let mut c = caminhos(&itens);
        c.insert(
            "53b0c621-8068-4243-a82e-f1bc5504ec5e".into(),
            "Decreto nº 4.852/97 - RCTE:Conceitos Gerais".into(),
        );
        let e = erro(montar(itens, &c));
        assert!(
            e.contains("\"Decreto nº 4.852/97 - RCTE:Conceitos Gerais\", que não termina no nome"),
            "{e}"
        );
        assert!(e.contains("53b0c621-8068-4243-a82e-f1bc5504ec5e"), "{e}");
    }

    #[test]
    fn item_com_o_nome_antigo_do_termo_segue_o_formulario() {
        // 758 e 1092 são do mesmo termo; um termo renomeado deixa itens com o nome de antes no campo oculto.
        let mut v = itens(ITENS).unwrap();
        let c = caminhos(&v);
        v.iter_mut().find(|i| i.id == 1092).unwrap().nome_do_termo =
            Some("Informações Fiscais|e0d26000-8072-4538-8dde-c822bc928574".into());
        let ps = montar(v, &c).unwrap();
        let p1092 = ps.iter().find(|p| p.id == 1092).unwrap();
        assert_eq!(
            p1092.caminho,
            [
                "Substituição Tributária pelas Operações Posteriores",
                "Informações Gerais"
            ]
        );

        // Se nenhum item do termo tem o nome do formulário, o formulário foi mal lido.
        let mut v = itens(ITENS).unwrap();
        for i in v.iter_mut().filter(|i| i.id == 758 || i.id == 1092) {
            i.nome_do_termo =
                Some("Informações Fiscais|e0d26000-8072-4538-8dde-c822bc928574".into());
        }
        let e = erro(montar(v, &c));
        assert!(e.contains("formulário mal lido"), "{e}");
    }

    #[test]
    fn categoria_com_nivel_vazio_e_erro() {
        let itens = itens(ITENS).unwrap();
        let mut c = caminhos(&itens);
        c.insert(
            "53b0c621-8068-4243-a82e-f1bc5504ec5e".into(),
            "Decreto nº 4.852/97 - RCTE::Informações Gerais".into(),
        );
        let e = erro(montar(itens, &c));
        assert!(e.contains("nível vazio (item 66)"), "{e}");
    }

    #[test]
    fn item_repetido_sem_pergunta_ou_sem_categoria_e_erro() {
        let base = || itens(ITENS).unwrap();
        let c = caminhos(&base());

        let mut v = base();
        v[1].id = v[0].id;
        assert!(erro(montar(v, &c)).contains("veio duas vezes"));

        let mut v = base();
        v[2].pergunta = Some("<div class=\"ExternalClassX\">&#160;</div>".into());
        assert!(erro(montar(v, &c)).contains("não tem pergunta"));

        let mut v = base();
        v[3].termo = None;
        assert!(erro(montar(v, &c)).contains("não tem categoria"));
    }

    #[test]
    fn resposta_absolutiza_links_e_decodifica_os_dois_pontos() {
        let r = pergunta(108).resposta;
        assert!(
            r.contains("P_0069_2018 \"https://orientacaotributaria.sefaz.go.gov.br/Pareceres/PARECER_069_2018_SEI.doc\""),
            "{r}"
        );
        assert!(
            r.starts_with("Sim, ambos os produtos: quirera de arroz"),
            "{r}"
        );
        let r = pergunta(60).resposta;
        assert!(
            r.contains("Decreto nº 9.108 \"ftp://ftp.sefaz.go.gov.br/sefazgo/legislacao/Decretos/D_09108.htm\""),
            "{r}"
        );
    }

    #[test]
    fn resposta_so_com_imagem_vira_o_link_da_imagem() {
        assert_eq!(
            pergunta(1198).resposta,
            "\"https://orientacaotributaria.sefaz.go.gov.br/PublishingImages/Lists/PerguntasFrequentes/AgrupadoPorCategoria/demonstracao.png\""
        );
    }

    #[test]
    fn resposta_sai_sem_tag_nem_entidade() {
        for p in perguntas() {
            for t in [&p.pergunta, &p.resposta] {
                assert!(!t.contains('<') && !t.contains("&#"), "{}: {t}", p.id);
            }
        }
        let tabela = pergunta(718).resposta;
        assert!(
            tabela.contains("TRANSPORTE DE CARGA FRIGORIFICADA TRANSPORTE DE CARGA FRIGORIFICADA"),
            "{tabela}"
        );
        assert!(
            pergunta(1092)
                .resposta
                .contains("\n2. o preço final a consumidor")
        );
    }

    #[test]
    fn pergunta_repetida_tem_um_link_por_item() {
        let ps = perguntas();
        let faqs = consolidar("F", &ps);
        let repetida: Vec<&FaqRaw> = faqs
            .iter()
            .filter(|f| {
                f.pergunta
                    .starts_with("É devido diferencial de alíquotas quando um contribuinte goiano")
            })
            .collect();
        assert_eq!(repetida.len(), 2);
        assert_eq!(repetida[0].pergunta, repetida[1].pergunta);
        assert_eq!(
            repetida[0].url,
            "https://orientacaotributaria.sefaz.go.gov.br/Lists/PerguntasFrequentes/DispForm.aspx?ID=122"
        );
        assert_ne!(repetida[0].url, repetida[1].url);
    }

    #[test]
    fn a_trilha_abre_com_o_site_e_o_ano_da_ultima_alteracao() {
        let ps = perguntas();
        let fonte = rotulo_da_fonte(&ps).unwrap();
        assert_eq!(fonte, "Orientação Tributária (até 2021)");
        let faqs = consolidar(&fonte, &ps);
        let f66 = faqs.iter().find(|f| f.url.ends_with("ID=66")).unwrap();
        assert_eq!(
            f66.origin,
            "Orientação Tributária (até 2021) | Decreto nº 4.852/97 - RCTE | Informações Gerais"
        );
        assert_eq!(
            faqs.iter()
                .map(|f| f.url.clone())
                .collect::<BTreeSet<_>>()
                .len(),
            10
        );
    }

    #[test]
    fn arvore_segue_o_caminho_das_categorias() {
        let ps = perguntas();
        let raiz = montar_arvore("Orientação Tributária (até 2021)", &ps);
        assert_eq!((raiz.page_type, raiz.children.len()), (PageType::Menu, 1));
        let site = &raiz.children[0];
        assert_eq!(site.title, "Orientação Tributária (até 2021)");
        assert_eq!(site.page_type, PageType::Menu);
        let rcte = site
            .children
            .iter()
            .find(|n| n.title == "Decreto nº 4.852/97 - RCTE")
            .unwrap();
        let ig = &rcte.children[0];
        assert_eq!(
            (ig.title.as_str(), ig.page_type),
            ("Informações Gerais", PageType::Faq)
        );
        assert_eq!(
            ig.origin,
            "Orientação Tributária (até 2021) | Decreto nº 4.852/97 - RCTE | Informações Gerais"
        );
        assert_eq!(
            ig.faq_items[0].url.as_deref(),
            Some(
                "https://orientacaotributaria.sefaz.go.gov.br/Lists/PerguntasFrequentes/DispForm.aspx?ID=66"
            )
        );
        let st = site
            .children
            .iter()
            .find(|n| n.title == "Substituição Tributária pelas Operações Posteriores")
            .unwrap();
        assert_eq!(st.children[0].faq_items.len(), 2);
    }
}
