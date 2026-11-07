// SEFAZ-PB FAQs — as páginas de informação do portal institucional (`www.sefaz.pb.gov.br`, Joomla) que trazem
// perguntas e respostas. Estudo, medições e decisões: docs/ESTUDO-PB-Faqs.md (D-PBFAQ-*).
//
// Não há FAQ central: uma lista curada de 9 páginas (D-PBFAQ-1), cada uma com a FAQ na página inteira, numa aba
// (`nn_tabs`) ou em todas as abas. Cada página marca a pergunta de um jeito (`<h4>`, `<strong>`, `<b>`, `<span>` em
// negrito; "1.", "1 -", "P1.", sem número), muitas vezes no mesmo `<p>` da resposta, separada por `<br>`. Uma regra só
// segmenta as nove (D-PBFAQ-2): a seção vira linhas (bloco e `<br>` quebram), e pergunta é a linha toda em destaque que
// termina em "?" — só as numeradas, quando a maioria é numerada.

use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use scraper::{ElementRef, Html, Selector};
use ureq::Agent;

use auli_contract::FaqRaw;
use auli_scraper_kit::clean;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::html::{Opcoes, html_para_texto};
use auli_scraper_kit::http::GetOpts;
use auli_scraper_kit::linhas::{
    Bruta, CLOAK_TEXTO, linhas, numeracao_irregular, segmentar, sem_cloak, sem_prefixo_r,
};

const HOST: &str = "https://www.sefaz.pb.gov.br";

/// Uma página da lista curada.
struct Fonte {
    caminho: &'static str,
    /// A aba que é a FAQ (pelo título dela). `None`: a página inteira — e, se ela tiver abas, cada aba é um assunto.
    aba: Option<&'static str>,
    /// O nome da página na trilha.
    nome: &'static str,
}

/// As FAQs da v1 (D-PBFAQ-1, opção C): as 10 páginas do estudo menos a DeSTDA, que a PB deixou de aceitar desde a
/// competência de janeiro de 2026 (Decreto 47.732/2025: EFD/OIE no lugar da DeSTDA e da GIA-ST).
const FONTES: &[Fonte] = &[
    Fonte {
        caminho: "/info/cadastro/8-perguntas-frequentes",
        aba: None,
        nome: "Cadastro de Contribuintes",
    },
    Fonte {
        caminho: "/info/declaracoes/sped-fiscal",
        aba: None,
        nome: "SPED - EFD-ICMS/IPI",
    },
    Fonte {
        caminho: "/info/produtos-substituicao-tributaria",
        aba: Some("Perguntas e Respostas"),
        nome: "Substituição Tributária",
    },
    Fonte {
        caminho: "/info/documentos-fiscais/mdf-e",
        aba: None,
        nome: "MDF-e",
    },
    Fonte {
        caminho: "/info/notas-fiscais/nfa-e",
        aba: None,
        nome: "NFA-e",
    },
    Fonte {
        caminho: "/info/2016-03-21-17-23-19/simples-nacional",
        aba: Some("Perguntas Frequentes"),
        nome: "Simples Nacional",
    },
    Fonte {
        caminho: "/info/declaracoes/gim",
        aba: Some("Perguntas e Respostas"),
        nome: "GIM",
    },
    Fonte {
        caminho: "/info/documentos-fiscais/aidf",
        aba: Some("Dúvidas Frequentes"),
        nome: "AIDF",
    },
    Fonte {
        caminho: "/info/cobranca-automatica",
        aba: None,
        nome: "Cobrança Automática",
    },
];

/// Cortesia entre as páginas (D-PBFAQ-8).
const CORTESIA: Duration = Duration::from_secs(1);
/// Piso do acervo: ~90 % das 200 perguntas de 07/11/2026.
const PISO: usize = 180;

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. Tudo é coletado e conferido
/// antes de qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = auli_scraper_kit::build_agent(crate::UA, Some(Duration::from_secs(60)));
    let mut paginas = Vec::new();
    for (i, f) in FONTES.iter().enumerate() {
        if i > 0 && !use_cache {
            sleep(CORTESIA);
        }
        let url = format!("{HOST}{}", f.caminho);
        let html = buscar(&agent, data_dir, &url, use_cache)?;
        let pagina =
            extrair(&html, f, &url).with_context(|| format!("PB FAQs: {} ({url})", f.nome))?;
        println!("PB FAQs: {}: {} perguntas", f.nome, pagina.perguntas.len());
        paginas.push(pagina);
    }
    let faqs = consolidar(&paginas)?;
    piso(faqs.len())?;

    // Atômico pelo mesmo motivo das outras entidades: a árvore não tem outra fonte (o snapshot achatado não a
    // reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&montar_arvore(&paginas))?,
    )?;
    println!("Wrote {tree_path} (árvore de FAQ p/ o frontend)");

    auli_contract::snapshot::write_faqs(
        crate::ENTITY,
        data_dir,
        &auli_contract::ScraperInfo::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        faqs,
    )?;
    println!("🎉 Coleta de FAQs gravada no snapshot.");
    Ok(())
}

/// Rede sempre, gravando o cache; com `--usecache`, só o cache, e miss é erro (D-PBFAQ-8).
fn buscar(agent: &Agent, data_dir: &str, url: &str, use_cache: bool) -> Result<String> {
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", url)
            .ok_or_else(|| anyhow!("cache miss para {url} (modo --usecache, sem rede)"));
    }
    let html = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "PB FAQs",
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", url, &html);
    Ok(html)
}

struct Pagina {
    nome: &'static str,
    url: String,
    perguntas: Vec<Pergunta>,
}

struct Pergunta {
    titulo: String,
    resposta: String,
    /// A aba, quando ela é o assunto (Cadastro, Cobrança Automática); `None` nas outras.
    assunto: Option<String>,
    /// O subgrupo da seção (Substituição Tributária); vazio quando não há.
    grupo: String,
    /// A página, com `#<id da aba>` quando a FAQ está numa aba (D-PBFAQ-5).
    link: String,
}

/// Uma seção da página: a aba (título e id) ou o corpo inteiro.
struct Secao<'a> {
    titulo: Option<String>,
    id: Option<String>,
    el: ElementRef<'a>,
}

static CORPO: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.item-page div.content-text").unwrap());
static ABA: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".tab-pane").unwrap());
static TITULO_ABA: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".nn_tabs-title").unwrap());

/// A página → as perguntas, na ordem da página.
fn extrair(html: &str, f: &Fonte, url: &str) -> Result<Pagina> {
    let html = sem_cloak(html);
    let doc = Html::parse_document(&html);
    let corpo = doc.select(&CORPO).next().ok_or_else(|| {
        anyhow!("a página não tem o corpo (div.item-page div.content-text) — o layout mudou?")
    })?;
    let abas: Vec<Secao> = corpo
        .select(&ABA)
        .map(|el| Secao {
            titulo: el
                .select(&TITULO_ABA)
                .next()
                .map(|t| clean(&t.text().collect::<String>())),
            id: el.value().id().map(str::to_string),
            el,
        })
        .collect();
    let (secoes, assunto_e_aba) = match f.aba {
        Some(nome) => {
            let s = abas
                .into_iter()
                .find(|s| {
                    s.titulo
                        .as_deref()
                        .is_some_and(|t| t.eq_ignore_ascii_case(nome))
                })
                .ok_or_else(|| anyhow!("a aba {nome:?} não existe mais na página"))?;
            (vec![s], false)
        }
        None if abas.is_empty() => (
            vec![Secao {
                titulo: None,
                id: None,
                el: corpo,
            }],
            false,
        ),
        None => (abas, true),
    };
    let mut perguntas = Vec::new();
    for s in &secoes {
        let link = match &s.id {
            Some(id) => format!("{url}#{}", pct(id)),
            None => url.to_string(),
        };
        let assunto = if assunto_e_aba {
            s.titulo.clone()
        } else {
            None
        };
        let brutas = segmentar(&linhas(s.el));
        avisar_numeracao(f.nome, s.titulo.as_deref(), &brutas);
        for b in brutas {
            let resposta = sem_prefixo_r(&html_para_texto(&b.resposta_html, opcoes()));
            if resposta.is_empty() {
                bail!("a pergunta {:?} ficou sem resposta", b.titulo);
            }
            if b.titulo.contains(CLOAK_TEXTO) || resposta.contains(CLOAK_TEXTO) {
                bail!(
                    "sobrou um e-mail protegido do Joomla em {:?} — o plugin mudou?",
                    b.titulo
                );
            }
            perguntas.push(Pergunta {
                titulo: b.titulo,
                resposta,
                assunto: assunto.clone(),
                grupo: b.grupo,
                link: link.clone(),
            });
        }
    }
    if perguntas.is_empty() {
        bail!("nenhuma pergunta na página");
    }
    Ok(Pagina {
        nome: f.nome,
        url: url.to_string(),
        perguntas,
    })
}

fn opcoes() -> Opcoes<'static> {
    Opcoes {
        base: Some(HOST),
        quebras_do_fonte: false,
    }
}

/// O id da aba no fragmento da URL: o que não é ASCII seguro vai em `%XX` (`não-habilitado` → `n%C3%A3o-habilitado`).
fn pct(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// Numeração que pula ou repete só avisa (D-PBFAQ-7): a MDF-e não tem a 6, e isso é da página.
fn avisar_numeracao(pagina: &str, aba: Option<&str>, brutas: &[Bruta]) {
    for (grupo, nums) in numeracao_irregular(brutas) {
        println!(
            "PB FAQs: aviso — {pagina}{}{}: numeração {nums:?}",
            aba.map(|a| format!(" | {a}")).unwrap_or_default(),
            if grupo.is_empty() {
                String::new()
            } else {
                format!(" | {grupo}")
            }
        );
    }
}

/// `Página | Aba` quando a aba é o assunto; `Página [| Grupo]` nas outras (D-PBFAQ-4).
fn trilha(p: &Pagina, q: &Pergunta) -> String {
    let mut partes = vec![p.nome];
    if let Some(a) = &q.assunto {
        partes.push(a);
    }
    if !q.grupo.is_empty() {
        partes.push(&q.grupo);
    }
    partes.join(" | ")
}

/// O acervo, com a guarda que olha todas as páginas: o nome do `.md` sai de link + pergunta, e o mesmo par duas vezes
/// sobrescreveria um arquivo.
fn consolidar(paginas: &[Pagina]) -> Result<Vec<FaqRaw>> {
    let mut vistos = HashSet::new();
    let mut faqs = Vec::new();
    for p in paginas {
        for q in &p.perguntas {
            if !vistos.insert((q.link.as_str(), q.titulo.as_str())) {
                bail!("a pergunta {:?} aparece duas vezes em {}", q.titulo, q.link);
            }
            faqs.push(FaqRaw {
                pergunta: q.titulo.clone(),
                resposta: q.resposta.clone(),
                origin: trilha(p, q),
                url: q.link.clone(),
            });
        }
    }
    println!(
        "PB FAQs: {} perguntas em {} páginas",
        faqs.len(),
        paginas.len()
    );
    Ok(faqs)
}

fn piso(n: usize) -> Result<()> {
    if n < PISO {
        bail!("só {n} perguntas, abaixo do piso de {PISO} — alguma página encolheu?");
    }
    Ok(())
}

/// A árvore da aba de FAQs: raiz → um nó por página → um nó por aba-assunto (Cadastro, Cobrança Automática) ou por
/// grupo (Substituição Tributária). As perguntas sem aba-assunto nem grupo ficam no nó da página.
fn montar_arvore(paginas: &[Pagina]) -> FaqNode {
    let item = |q: &Pergunta| FaqItem {
        pergunta: q.titulo.clone(),
        resposta: q.resposta.clone(),
        url: None,
    };
    let filhos = paginas
        .iter()
        .map(|p| {
            let mut diretos = Vec::new();
            let mut nos: Vec<(String, String, Vec<FaqItem>)> = Vec::new();
            for q in &p.perguntas {
                let chave = q.assunto.clone().unwrap_or_else(|| q.grupo.clone());
                if chave.is_empty() {
                    diretos.push(item(q));
                    continue;
                }
                match nos.iter_mut().find(|(c, _, _)| *c == chave) {
                    Some((_, _, itens)) => itens.push(item(q)),
                    None => nos.push((chave, q.link.clone(), vec![item(q)])),
                }
            }
            let children: Vec<FaqNode> = nos
                .into_iter()
                .map(|(titulo, url, faq_items)| FaqNode {
                    origin: format!("{} | {titulo}", p.nome),
                    title: titulo,
                    url,
                    page_type: PageType::Faq,
                    children: Vec::new(),
                    faq_items,
                })
                .collect();
            FaqNode {
                title: p.nome.to_string(),
                url: p.url.clone(),
                page_type: if children.is_empty() {
                    PageType::Faq
                } else {
                    PageType::Menu
                },
                origin: p.nome.to_string(),
                children,
                faq_items: diretos,
            }
        })
        .collect();
    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: HOST.to_string(),
        page_type: PageType::Menu,
        origin: String::new(),
        children: filhos,
        faq_items: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // O `div.item-page` das páginas reais de 07/11/2026 (com os scripts de e-mail protegido): Cadastro (10 abas, 18
    // e-mails), Substituição Tributária (a aba de FAQ entre outras, com grupos e a 13 de negrito partido), Simples
    // Nacional (o índice antes das perguntas), SPED (a tabela de anexos), MDF-e (sem a 6), NFA-e (sem número) e
    // Cobrança Automática (três abas sem pergunta).
    const CADASTRO: &str = include_str!("../tests/fixtures/pb-faqs-cadastro.html");
    const ST: &str = include_str!("../tests/fixtures/pb-faqs-st.html");
    const SIMPLES: &str = include_str!("../tests/fixtures/pb-faqs-simples.html");
    const SPED: &str = include_str!("../tests/fixtures/pb-faqs-sped.html");
    const MDFE: &str = include_str!("../tests/fixtures/pb-faqs-mdfe.html");
    const NFAE: &str = include_str!("../tests/fixtures/pb-faqs-nfae.html");
    const COBRANCA: &str = include_str!("../tests/fixtures/pb-faqs-cobranca.html");

    fn fonte(nome: &str) -> &'static Fonte {
        FONTES.iter().find(|f| f.nome == nome).unwrap()
    }

    fn pagina(html: &str, nome: &str) -> Pagina {
        let f = fonte(nome);
        extrair(html, f, &format!("{HOST}{}", f.caminho)).unwrap()
    }

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    /// Uma página mínima no formato do portal, sem abas.
    fn corpo(html: &str) -> String {
        format!(r#"<div class="item-page"><div class="content-text">{html}</div></div>"#)
    }

    #[test]
    fn cadastro_tem_uma_secao_por_aba_com_o_link_da_aba() {
        let p = pagina(CADASTRO, "Cadastro de Contribuintes");
        assert_eq!(p.perguntas.len(), 64);
        let q = &p.perguntas[0];
        assert_eq!(q.assunto.as_deref(), Some("FAC"));
        assert!(
            q.link
                .ends_with("/info/cadastro/8-perguntas-frequentes#fac")
        );
        let abas: Vec<&str> = p
            .perguntas
            .iter()
            .filter_map(|q| q.assunto.as_deref())
            .collect();
        assert_eq!(abas.iter().collect::<HashSet<_>>().len(), 10);
        // O id com acento vai em `%XX`.
        assert!(
            p.perguntas
                .iter()
                .any(|q| q.link.ends_with("#n%C3%A3o-habilitado"))
        );
    }

    #[test]
    fn titulo_e_texto_puro_sem_a_url_do_link() {
        let p = pagina(CADASTRO, "Cadastro de Contribuintes");
        assert!(
            p.perguntas[0]
                .titulo
                .starts_with("1. Estou tentando enviar uma FAC, mas aparece a mensagem"),
            "{}",
            p.perguntas[0].titulo
        );
    }

    #[test]
    fn subtitulo_em_negrito_da_resposta_nao_abre_pergunta() {
        // REDESIM: "Sim. Mas o que fazer?" está em negrito e termina em "?", mas a aba numera as perguntas.
        let p = pagina(CADASTRO, "Cadastro de Contribuintes");
        assert!(
            p.perguntas
                .iter()
                .all(|q| q.titulo != "Sim. Mas o que fazer?")
        );
        assert!(
            p.perguntas
                .iter()
                .any(|q| q.resposta.contains("Sim. Mas o que fazer?"))
        );
    }

    #[test]
    fn email_protegido_vira_o_texto_visivel() {
        let p = pagina(CADASTRO, "Cadastro de Contribuintes");
        let todas: String = p.perguntas.iter().map(|q| q.resposta.as_str()).collect();
        assert!(!todas.contains(CLOAK_TEXTO));
        assert!(todas.contains("cadastro@sefaz.pb.gov.br"), "{todas}");
    }

    #[test]
    fn email_protegido_sem_o_script_esperado_e_erro() {
        let html = corpo(
            r#"<p><strong>1. Onde?</strong><br>Escreva para <span id="cloakabc">Este endereço de email está sendo protegido de spambots.</span><script>var x = 1;</script>.</p>"#,
        );
        let e = erro(extrair(&html, fonte("NFA-e"), "x"));
        assert!(e.contains("sobrou um e-mail protegido"), "{e}");
    }

    #[test]
    fn link_aberto_fecha_na_quebra_de_linha() {
        // O `<a href="mailto:…">` do portal só fecha depois do endereço da linha seguinte.
        let p = pagina(CADASTRO, "Cadastro de Contribuintes");
        let q = p
            .perguntas
            .iter()
            .find(|q| {
                q.resposta
                    .contains("CENTRO DE ATENDIMENTO AO CIDADÃO EM JOÃO PESSOA\n")
            })
            .unwrap();
        assert!(
            q.resposta.contains(
                "cacgr1@sefaz.pb.gov.br \"mailto:cacgr1@sefaz.pb.gov.br\"\nAv. Presidente"
            ),
            "{}",
            q.resposta
        );
    }

    #[test]
    fn st_le_so_a_aba_de_faq_e_os_grupos() {
        let p = pagina(ST, "Substituição Tributária");
        assert_eq!(p.perguntas.len(), 31);
        let mut grupos: Vec<(&str, usize)> = Vec::new();
        for q in &p.perguntas {
            match grupos.last_mut() {
                Some((g, n)) if *g == q.grupo => *n += 1,
                _ => grupos.push((&q.grupo, 1)),
            }
        }
        assert_eq!(
            grupos,
            [
                ("", 16),
                ("COMÉRCIO EXTERIOR", 9),
                ("DIFAL NÃO CONTRIBUINTE – ICMS – EC 87/15", 3),
                ("COMBUSTÍVEIS", 3)
            ]
        );
        // O título do grupo não fica no fim da resposta anterior.
        assert!(!p.perguntas[15].resposta.contains("PERGUNTAS FREQUENTES"));
        assert!(p.perguntas.iter().all(|q| q.assunto.is_none()));
        // A 3 de Combustíveis não termina em "?" (é um cenário com as perguntas A, B, C): entra pela continuação da
        // numeração (D-PAFAQ-2), e não fica mais na resposta da 2.
        let q3 = &p.perguntas[30];
        assert!(
            q3.titulo
                .starts_with("3 - Considerando uma operação de Venda de Etanol Hidratado")
        );
        assert!(
            !p.perguntas[29]
                .resposta
                .contains("Considerando uma operação de Venda de Etanol")
        );
    }

    #[test]
    fn digito_fora_do_negrito_nao_desfaz_a_pergunta() {
        // ST: `1<strong>3 - Como são classificadas…?</strong>`.
        let p = pagina(ST, "Substituição Tributária");
        assert!(
            p.perguntas[12]
                .titulo
                .starts_with("13 - Como são classificadas as BEBIDAS QUENTES")
        );
    }

    #[test]
    fn aspas_depois_do_ponto_de_interrogacao_nao_desfazem_a_pergunta() {
        // DeSTDA: `<strong>5. … O que devo fazer? </strong> “`.
        let html = corpo(
            r#"<p><strong>1. Primeira?</strong></p><p>R1.</p><p><strong>2. Segunda? </strong> “</p><p>R2.</p>"#,
        );
        let p = extrair(&html, fonte("NFA-e"), "x").unwrap();
        assert_eq!(p.perguntas.len(), 2);
        assert_eq!(p.perguntas[1].resposta, "R2.");
    }

    #[test]
    fn indice_antes_das_perguntas_fica_fora_e_o_prefixo_r_sai() {
        // Simples: o índice tem só o número em negrito (`<strong>1 - </strong>Não sou…?`).
        let p = pagina(SIMPLES, "Simples Nacional");
        assert_eq!(p.perguntas.len(), 12);
        assert!(p.perguntas[0].titulo.starts_with("1 – Não sou uma empresa"));
        assert!(
            p.perguntas
                .iter()
                .all(|q| sem_prefixo_r(&q.resposta) == q.resposta)
        );
    }

    #[test]
    fn tabela_de_anexos_nao_entra_na_resposta() {
        let p = pagina(SPED, "SPED - EFD-ICMS/IPI");
        assert_eq!(p.perguntas.len(), 42);
        let ultima = &p.perguntas[41];
        assert!(ultima.titulo.starts_with("P42."));
        assert!(!ultima.resposta.contains(" kB"), "{}", ultima.resposta);
    }

    #[test]
    fn numeracao_que_pula_so_avisa() {
        let p = pagina(MDFE, "MDF-e");
        assert_eq!(p.perguntas.len(), 17);
        assert!(p.perguntas[4].titulo.starts_with("5 -"));
        assert!(p.perguntas[5].titulo.starts_with("7 -"));
    }

    #[test]
    fn sem_numero_vale_toda_linha_em_negrito_com_interrogacao() {
        let p = pagina(NFAE, "NFA-e");
        assert_eq!(p.perguntas.len(), 16);
        assert_eq!(p.perguntas[0].titulo, "O que é NFA-e?");
        assert!(
            p.perguntas[0]
                .resposta
                .starts_with("NFA-e é a Nota Fiscal Avulsa Eletrônica")
        );
    }

    #[test]
    fn todas_as_abas_quando_a_fonte_nao_indica_uma() {
        let p = pagina(COBRANCA, "Cobrança Automática");
        assert_eq!(p.perguntas.len(), 4);
        assert!(
            p.perguntas
                .iter()
                .all(|q| q.assunto.as_deref() == Some("DIFAL EC 87/2015"))
        );
    }

    #[test]
    fn aba_que_sumiu_pagina_sem_corpo_ou_sem_pergunta_e_erro() {
        let f = Fonte {
            caminho: "/x",
            aba: Some("Não Existe"),
            nome: "X",
        };
        let e = erro(extrair(ST, &f, "x"));
        assert!(e.contains("não existe mais"), "{e}");
        let e = erro(extrair(
            "<html><body><p>Oi</p></body></html>",
            fonte("NFA-e"),
            "x",
        ));
        assert!(e.contains("não tem o corpo"), "{e}");
        let e = erro(extrair(&corpo("<p>Em breve.</p>"), fonte("NFA-e"), "x"));
        assert!(e.contains("nenhuma pergunta"), "{e}");
    }

    #[test]
    fn tabela_da_resposta_vira_linhas_com_celulas_separadas() {
        let html = corpo(
            "<p><strong>1. Qual a alíquota?</strong></p><table><tr><td>Até 10</td><td>2%</td></tr><tr><td>Acima</td><td>4%</td></tr></table>",
        );
        let p = extrair(&html, fonte("NFA-e"), "x").unwrap();
        assert_eq!(p.perguntas[0].resposta, "Até 10 2%\nAcima 4%");
    }

    #[test]
    fn pergunta_sem_resposta_e_erro() {
        let html = corpo(
            "<p><strong>1. Primeira?</strong></p><p><strong>2. Segunda?</strong></p><p>R2.</p>",
        );
        let e = erro(extrair(&html, fonte("NFA-e"), "x"));
        assert!(e.contains("\"1. Primeira?\" ficou sem resposta"), "{e}");
    }

    fn paginas() -> Vec<Pagina> {
        vec![
            pagina(CADASTRO, "Cadastro de Contribuintes"),
            pagina(ST, "Substituição Tributária"),
            pagina(NFAE, "NFA-e"),
        ]
    }

    #[test]
    fn trilha_com_a_aba_assunto_ou_o_grupo() {
        let faqs = consolidar(&paginas()).unwrap();
        assert_eq!(faqs.len(), 64 + 31 + 16);
        assert_eq!(faqs[0].origin, "Cadastro de Contribuintes | FAC");
        assert_eq!(faqs[64].origin, "Substituição Tributária");
        assert_eq!(
            faqs[80].origin,
            "Substituição Tributária | COMÉRCIO EXTERIOR"
        );
        assert_eq!(faqs[95].origin, "NFA-e");
    }

    #[test]
    fn pergunta_repetida_com_o_mesmo_link_e_erro() {
        let mut ps = paginas();
        let t = ps[2].perguntas[0].titulo.clone();
        ps[2].perguntas[1].titulo = t;
        let e = erro(consolidar(&ps));
        assert!(e.contains("aparece duas vezes"), "{e}");
    }

    #[test]
    fn abaixo_do_piso_e_erro() {
        assert!(piso(PISO).is_ok());
        assert!(erro(piso(PISO - 1)).contains("abaixo do piso"));
    }

    #[test]
    fn arvore_pagina_aba_ou_grupo() {
        let t = montar_arvore(&paginas());
        assert_eq!((t.page_type, t.children.len()), (PageType::Menu, 3));
        let cad = &t.children[0];
        assert_eq!(
            (cad.page_type, cad.children.len(), cad.faq_items.len()),
            (PageType::Menu, 10, 0)
        );
        assert_eq!(cad.children[0].title, "FAC");
        assert!(cad.children[0].url.ends_with("#fac"));
        let st = &t.children[1];
        assert_eq!((st.children.len(), st.faq_items.len()), (3, 16));
        assert_eq!(
            st.children[0].origin,
            "Substituição Tributária | COMÉRCIO EXTERIOR"
        );
        let nfae = &t.children[2];
        assert_eq!((nfae.page_type, nfae.faq_items.len()), (PageType::Faq, 16));
    }
}
