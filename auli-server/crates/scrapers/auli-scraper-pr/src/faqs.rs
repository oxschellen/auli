// SEFA-PR FAQs — as "Perguntas Frequentes" do Portal de Atendimento do SAC
// (`atendimento.fazenda.pr.gov.br/sacsefa/portal`), aplicação PHP à parte do portal Drupal do
// `scrape.rs`. Estudo, medições e decisões: docs/ESTUDO-PR-Faqs.md (D-PRFAQ-*).
//
// A fonte é HTML server-side: o `index` lista os referentes (temas) e cada `assuntosReferente/<id>`
// traz o referente inteiro — cards de assunto com os painéis de pergunta e a resposta íntegra. Três
// propriedades da fonte moldam o código:
//   - o servidor manda só o certificado folha → o intermediário DV R36 vai embutido (`agent`);
//   - o texto usa `<` como seta de menu e cita `<Consulta Pendencias>` → a conversão só tira as tags
//     de uma lista fechada (`html_para_texto`);
//   - não há rota por pergunta, e o mesmo texto de pergunta se repete dentro de um referente → o link
//     é a página do referente + `#heading<id>` (`link_da_pergunta`), o que torna o nome do `.md` único.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use regex::Regex;
use ureq::Agent;
use ureq::tls::{Certificate, RootCerts, TlsConfig};

use auli_contract::FaqRaw;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::http::GetOpts;
use auli_scraper_kit::{clean, decode_entities};

use crate::pareceres::{COURTESY, UA};

const HOST: &str = "https://atendimento.fazenda.pr.gov.br";
const BASE: &str = "https://atendimento.fazenda.pr.gov.br/sacsefa/portal";

/// Intermediário **Sectigo Public Server Authentication CA DV R36**, emissor do folha
/// `*.fazenda.pr.gov.br`, que o servidor não manda (D-PRFAQ-4). Não é o OV R36 do `auli-scraper-ac`.
/// Baixado do AIA do folha (`crt.sectigo.com/SectigoPublicServerAuthenticationCADVR36.crt`); válido
/// até 21/03/2036. O folha vence em 21/11/2026: se a renovação vier de outro emissor, o handshake
/// falha alto e este PEM tem de ser trocado.
const SECTIGO_DV_R36_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIGTDCCBDSgAwIBAgIQOXpmzCdWNi4NqofKbqvjsTANBgkqhkiG9w0BAQwFADBf
MQswCQYDVQQGEwJHQjEYMBYGA1UEChMPU2VjdGlnbyBMaW1pdGVkMTYwNAYDVQQD
Ey1TZWN0aWdvIFB1YmxpYyBTZXJ2ZXIgQXV0aGVudGljYXRpb24gUm9vdCBSNDYw
HhcNMjEwMzIyMDAwMDAwWhcNMzYwMzIxMjM1OTU5WjBgMQswCQYDVQQGEwJHQjEY
MBYGA1UEChMPU2VjdGlnbyBMaW1pdGVkMTcwNQYDVQQDEy5TZWN0aWdvIFB1Ymxp
YyBTZXJ2ZXIgQXV0aGVudGljYXRpb24gQ0EgRFYgUjM2MIIBojANBgkqhkiG9w0B
AQEFAAOCAY8AMIIBigKCAYEAljZf2HIz7+SPUPQCQObZYcrxLTHYdf1ZtMRe7Yeq
RPSwygz16qJ9cAWtWNTcuICc++p8Dct7zNGxCpqmEtqifO7NvuB5dEVexXn9RFFH
12Hm+NtPRQgXIFjx6MSJcNWuVO3XGE57L1mHlcQYj+g4hny90aFh2SCZCDEVkAja
EMMfYPKuCjHuuF+bzHFb/9gV8P9+ekcHENF2nR1efGWSKwnfG5RawlkaQDpRtZTm
M64TIsv/r7cyFO4nSjs1jLdXYdz5q3a4L0NoabZfbdxVb+CUEHfB0bpulZQtH1Rv
38e/lIdP7OTTIlZh6OYL6NhxP8So0/sht/4J9mqIGxRFc0/pC8suja+wcIUna0HB
pXKfXTKpzgis+zmXDL06ASJf5E4A2/m+Hp6b84sfPAwQ766rI65mh50S0Di9E3Pn
2WcaJc+PILsBmYpgtmgWTR9eV9otfKRUBfzHUHcVgarub/XluEpRlTtZudU5xbFN
xx/DgMrXLUAPaI60fZ6wA+PTAgMBAAGjggGBMIIBfTAfBgNVHSMEGDAWgBRWc1hk
lfmSGrASKgRieaFAFYghSTAdBgNVHQ4EFgQUaMASFhgOr872h6YyV6NGUV3LBycw
DgYDVR0PAQH/BAQDAgGGMBIGA1UdEwEB/wQIMAYBAf8CAQAwHQYDVR0lBBYwFAYI
KwYBBQUHAwEGCCsGAQUFBwMCMBsGA1UdIAQUMBIwBgYEVR0gADAIBgZngQwBAgEw
VAYDVR0fBE0wSzBJoEegRYZDaHR0cDovL2NybC5zZWN0aWdvLmNvbS9TZWN0aWdv
UHVibGljU2VydmVyQXV0aGVudGljYXRpb25Sb290UjQ2LmNybDCBhAYIKwYBBQUH
AQEEeDB2ME8GCCsGAQUFBzAChkNodHRwOi8vY3J0LnNlY3RpZ28uY29tL1NlY3Rp
Z29QdWJsaWNTZXJ2ZXJBdXRoZW50aWNhdGlvblJvb3RSNDYucDdjMCMGCCsGAQUF
BzABhhdodHRwOi8vb2NzcC5zZWN0aWdvLmNvbTANBgkqhkiG9w0BAQwFAAOCAgEA
YtOC9Fy+TqECFw40IospI92kLGgoSZGPOSQXMBqmsGWZUQ7rux7cj1du6d9rD6C8
ze1B2eQjkrGkIL/OF1s7vSmgYVafsRoZd/IHUrkoQvX8FZwUsmPu7amgBfaY3g+d
q1x0jNGKb6I6Bzdl6LgMD9qxp+3i7GQOnd9J8LFSietY6Z4jUBzVoOoz8iAU84OF
h2HhAuiPw1ai0VnY38RTI+8kepGWVfGxfBWzwH9uIjeooIeaosVFvE8cmYUB4TSH
5dUyD0jHct2+8ceKEtIoFU/FfHq/mDaVnvcDCZXtIgitdMFQdMZaVehmObyhRdDD
4NQCs0gaI9AAgFj4L9QtkARzhQLNyRf87Kln+YU0lgCGr9HLg3rGO8q+Y4ppLsOd
unQZ6ZxPNGIfOApbPVf5hCe58EZwiWdHIMn9lPP6+F404y8NNugbQixBber+x536
WrZhFZLjEkhp7fFXf9r32rNPfb74X/U90Bdy4lzp3+X1ukh1BuMxA/EEhDoTOS3l
7ABvc7BYSQubQ2490OcdkIzUh3ZwDrakMVrbaTxUM2p24N6dB+ns2zptWCva6jzW
r8IWKIMxzxLPv5Kt3ePKcUdvkBU/smqujSczTzzSjIoR5QqQA6lN1ZRSnuHIWCvh
JEltkYnTAH41QJ6SAWO66GrrUESwN/cgZzL4JLEqz1Y=
-----END CERTIFICATE-----
";

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. As guardas
/// (D-PRFAQ-6) rodam antes de qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = agent();

    let index = buscar(data_dir, &agent, &format!("{BASE}/index"), use_cache)?;
    let referentes = parse_index(&index)?;
    println!("PR FAQs: {} referentes no index", referentes.len());

    let mut paginas = Vec::new();
    for r in referentes {
        let html = buscar(data_dir, &agent, &url_do_referente(r.id), use_cache)?;
        let cards = parse_referente(r.id, &html)?;
        println!(
            "PR FAQs: referente {} ({}): {} assuntos, {} perguntas",
            r.id,
            r.nome,
            cards.len(),
            cards.iter().map(|c| c.paineis.len()).sum::<usize>()
        );
        paginas.push((r, cards));
    }
    let faqs = consolidar(&paginas)?;

    // Atômico pelo mesmo motivo do RS e do SC: a árvore não tem outra fonte (o snapshot achatado não
    // a reconstrói) e é o único artefato de coleta que o frontend lê direto.
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

/// Agent só para o host `atendimento.`, com o DV R36 como trust anchor do rustls (o molde do
/// `ac.rs`). O `www.fazenda.pr.gov.br` dos serviços manda a cadeia completa e segue no agent padrão.
fn agent() -> Agent {
    let cert = Certificate::from_pem(SECTIGO_DV_R36_PEM.as_bytes()).expect("DV R36 PEM válido");
    Agent::config_builder()
        .user_agent(UA)
        .timeout_global(Some(Duration::from_secs(60)))
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::new_with_certs(&[cert]))
                .build(),
        )
        .build()
        .into()
}

/// Uma requisição (D-PRFAQ-7, a lição da D-SCFAQ-4): rede sempre, gravando o cache sob a url da
/// página; com `--usecache`, só o cache, e miss é erro. Cache-first reproduziria a primeira coleta
/// em silêncio.
fn buscar(data_dir: &str, agent: &Agent, url: &str, use_cache: bool) -> Result<String> {
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", url)
            .ok_or_else(|| anyhow!("cache miss para {url} (modo --usecache, sem rede)"));
    }
    sleep(COURTESY);
    let corpo = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "PR FAQs",
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", url, &corpo);
    Ok(corpo)
}

/// Um tema do SAC. `nome` é o longo, do `index` ("IPVA - Imposto sobre…"), não o `Referente:` curto
/// do painel (D-PRFAQ-3).
struct Referente {
    id: u32,
    nome: String,
}

/// Um card de assunto da página do referente (`headPanel<id>`), com os painéis na ordem da página.
struct Card {
    titulo: String,
    paineis: Vec<Painel>,
}

/// Uma pergunta, já em texto. `id` é o `Pergunta Nº`, conferido contra o `heading<id>`.
struct Painel {
    id: u32,
    pergunta: String,
    resposta: String,
}

/// Um referente do `index`: o link da página e, no `h4` do mesmo cabeçalho, o nome longo.
static REFERENTE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)href="/sacsefa/portal/assuntosReferente/(\d+)".*?class=['"]referenteAssuntoPerguntaContrast['"]\s*>(.*?)</h4>"#,
    )
    .unwrap()
});

/// Os referentes, na ordem do `index` (D-PRFAQ-1): a fonte de verdade. Nunca varrer ids — as lacunas
/// são referentes desativados, que dão 302.
fn parse_index(html: &str) -> Result<Vec<Referente>> {
    let referentes = REFERENTE_RE
        .captures_iter(html)
        .map(|c| -> Result<Referente> {
            Ok(Referente {
                id: c[1].parse()?,
                nome: clean(&decode_entities(&c[2])),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    // (e) Sem referente não há o que coletar: é a página que mudou, não o SAC que esvaziou.
    if referentes.is_empty() {
        bail!("o index não lista nenhum referente — a página mudou?");
    }
    Ok(referentes)
}

static CARD_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"id="headPanel(\d+)""#).unwrap());
static PAINEL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"id="heading(\d+)""#).unwrap());
static TITULO_DO_CARD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<h4[^>]*>(.*?)</h4>").unwrap());
static PERGUNTA_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)class=['"]referenteAssuntoPerguntaContrast['"]\s*>(.*?)</h4>"#).unwrap()
});
static NUMERO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<b>\s*Pergunta\s+N[ºo°]\s*(\d+)\s*</b>").unwrap());
static ASSUNTO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)Assunto:(.*?)</b>").unwrap());
static RESPOSTA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<p\b[^>]*>(.*?)</p>").unwrap());

/// Corta `s` nos matches de `re` (que capturam um id): cada fatia vai do match até o próximo.
fn fatias<'a>(s: &'a str, re: &Regex) -> Result<Vec<(u32, &'a str)>> {
    let marcos = re
        .captures_iter(s)
        .map(|c| -> Result<(usize, u32)> { Ok((c.get(0).unwrap().start(), c[1].parse()?)) })
        .collect::<Result<Vec<_>>>()?;
    Ok(marcos
        .iter()
        .enumerate()
        .map(|(i, (inicio, id))| {
            let fim = marcos.get(i + 1).map_or(s.len(), |(f, _)| *f);
            (*id, &s[*inicio..fim])
        })
        .collect())
}

/// A página do referente: cards de assunto → painéis de pergunta. Aplica as guardas (a), (b) e (c) da
/// D-PRFAQ-6, que são da página.
fn parse_referente(rid: u32, html: &str) -> Result<Vec<Card>> {
    let mut cards = Vec::new();
    for (aid, card) in fatias(html, &CARD_RE)? {
        let titulo = TITULO_DO_CARD_RE
            .captures(card)
            .map(|c| clean(&html_para_texto(&c[1])))
            .ok_or_else(|| anyhow!("referente {rid}, assunto {aid}: card sem título"))?;
        let mut paineis = Vec::new();
        for (id, painel) in fatias(card, &PAINEL_RE)? {
            // Cada campo na sua metade: a pergunta pode citar "Assunto:" (a 2247, do referente 28) e a
            // resposta pode citar qualquer coisa — o regex não pode atravessar de uma para a outra.
            let (cabecalho, corpo) = painel
                .split_once("perguntaFrequentePanelBody")
                .ok_or_else(|| anyhow!("referente {rid}, heading{id}: painel sem corpo"))?;
            let campo = |re: &Regex, metade: &str, nome: &str| {
                re.captures(metade)
                    .map(|c| c[1].to_string())
                    .ok_or_else(|| {
                        anyhow!("referente {rid}, heading{id}: painel sem {nome} — o markup mudou?")
                    })
            };
            // (c) A identidade tem de conferir: o número que o painel mostra é o do seu `heading`, e
            //     o `Assunto:` do corpo é o card que o contém.
            let numero: u32 = campo(&NUMERO_RE, cabecalho, "Pergunta Nº")?.parse()?;
            if numero != id {
                bail!("referente {rid}: heading{id} traz a Pergunta Nº {numero}");
            }
            let assunto = clean(&decode_entities(&campo(&ASSUNTO_RE, corpo, "Assunto:")?));
            if assunto != titulo {
                bail!(
                    "referente {rid}, pergunta {id}: Assunto \"{assunto}\" ≠ card \"{titulo}\" ({aid})"
                );
            }
            paineis.push(Painel {
                id,
                pergunta: clean(&html_para_texto(&campo(
                    &PERGUNTA_RE,
                    cabecalho,
                    "pergunta",
                )?)),
                resposta: html_para_texto(&campo(&RESPOSTA_RE, corpo, "resposta")?),
            });
        }
        cards.push(Card { titulo, paineis });
    }

    // (a) O index diz que o referente existe e a página não traz pergunta: é a coleta que falhou (o
    //     302 de um referente desativado cai no próprio index, que não tem card), não o SAC que esvaziou.
    let parseados: usize = cards.iter().map(|c| c.paineis.len()).sum();
    if parseados == 0 {
        bail!("referente {rid}: nenhuma pergunta na página — 302 para o index ou markup mudou?");
    }
    // (b) Painel fora de card escaparia do parse sem erro: a contagem crua da página tem de bater.
    let no_html = html.matches("perguntaFrequentePanelHeader").count();
    if no_html != parseados {
        bail!(
            "referente {rid}: {no_html} painéis no HTML, {parseados} parseados — o parse perdeu algum"
        );
    }
    Ok(cards)
}

/// Todas as perguntas, na ordem do index e das páginas, com o referente e o card de cada uma.
fn perguntas(
    paginas: &[(Referente, Vec<Card>)],
) -> impl Iterator<Item = (&Referente, &Card, &Painel)> {
    paginas.iter().flat_map(|(r, cards)| {
        cards
            .iter()
            .flat_map(move |c| c.paineis.iter().map(move |p| (r, c, p)))
    })
}

/// O acervo, com as guardas (d), (f) e (g) da D-PRFAQ-6, que olham todas as páginas.
fn consolidar(paginas: &[(Referente, Vec<Card>)]) -> Result<Vec<FaqRaw>> {
    // (d) Uma pergunta mora num referente só (0 em dois em 01/11/2026). Não há dobro conhecido como no
    //     SC: número repetido é reclassificação no meio da coleta ou markup que mudou.
    let mut referente_de = BTreeMap::new();
    let mut repetidas = Vec::new();
    for (r, _, p) in perguntas(paginas) {
        if let Some(outro) = referente_de.insert(p.id, r.id) {
            repetidas.push(format!("{} (referentes {outro} e {})", p.id, r.id));
        }
    }
    if !repetidas.is_empty() {
        bail!(
            "{} número(s) de pergunta repetido(s) — {}",
            repetidas.len(),
            repetidas.join(", ")
        );
    }

    // (f) Pergunta sem texto vira `.md` sem título e vetor sem conteúdo (a D-SCFAQ-3f).
    let sem_texto: Vec<String> = perguntas(paginas)
        .filter(|(_, _, p)| p.pergunta.is_empty())
        .map(|(_, _, p)| p.id.to_string())
        .collect();
    if !sem_texto.is_empty() {
        bail!(
            "{} pergunta(s) sem texto — {}",
            sem_texto.len(),
            sem_texto.join(", ")
        );
    }

    // (g) Resposta vazia só avisa: título e link ainda são documento. Com a imagem virando url
    //     (D-PRFAQ-5), não há nenhuma em 01/11/2026.
    let sem_resposta: Vec<String> = perguntas(paginas)
        .filter(|(_, _, p)| p.resposta.is_empty())
        .map(|(_, _, p)| p.id.to_string())
        .collect();
    if !sem_resposta.is_empty() {
        eprintln!(
            "⚠️  PR FAQs: {} pergunta(s) com resposta vazia depois da conversão — {}",
            sem_resposta.len(),
            sem_resposta.join(", ")
        );
    }

    let faqs: Vec<FaqRaw> = perguntas(paginas)
        .map(|(r, c, p)| montar_faq_raw(r, c, p))
        .collect();
    println!(
        "PR FAQs: {} perguntas em {} referentes",
        faqs.len(),
        paginas.len()
    );
    Ok(faqs)
}

static IMG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<img\b[^>]*?\ssrc\s*=\s*["']([^"']*)["'][^>]*>"#).unwrap());
static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<a\b[^>]*?\shref\s*=\s*["']([^"']*)["'][^>]*>(.*?)</a>"#).unwrap()
});
static BLOCO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)</?(?:br|center|div|hr|li|ol|p|ul)\b[^>]*>").unwrap());
/// Lista fechada (D-PRFAQ-5): o vocabulário medido nas 1.321 respostas, mais folga. Qualquer outro
/// `<…>` é texto — o `<` de menu ("Cadastro < Alteração Cadastral") e o `<Consulta Pendencias>` da
/// FAQ 1775, que o `<[^>]+>` do SC apagaria.
static TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)</?(?:a|b|br|center|div|em|font|hr|i|img|li|ol|p|span|strong|sub|sup|u|ul)\b[^>]*>",
    )
    .unwrap()
});

/// HTML → texto, na convenção do SC: `<a href="url">texto</a>` vira `texto "url"`, tag de bloco vira
/// quebra de linha, entidades são decodificadas e linhas vazias descartadas. Duas diferenças: só as
/// tags da lista fechada saem, e `<img src>` vira `"<url absoluta>"`, como a âncora vazia — 3
/// respostas são só uma tabela em imagem.
fn html_para_texto(s: &str) -> String {
    let com_imagens = IMG_RE.replace_all(s, |c: &regex::Captures| {
        let src = decode_entities(c[1].trim());
        if src.starts_with('/') {
            format!("\"{HOST}{src}\"")
        } else {
            format!("\"{src}\"")
        }
    });
    let com_links = LINK_RE.replace_all(&com_imagens, |c: &regex::Captures| {
        let href = decode_entities(c[1].trim());
        let texto = clean(&decode_entities(&TAG_RE.replace_all(&c[2], "")));
        if href.is_empty() || href.starts_with('#') || href.starts_with("javascript:") {
            texto
        } else if texto.is_empty() {
            format!("\"{href}\"")
        } else {
            format!("{texto} \"{href}\"")
        }
    });
    let com_quebras = BLOCO_RE.replace_all(&com_links, "\n");
    let sem_tags = TAG_RE.replace_all(&com_quebras, "");
    decode_entities(&sem_tags)
        .lines()
        .map(clean)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn url_do_referente(rid: u32) -> String {
    format!("{BASE}/assuntosReferente/{rid}")
}

/// Não há rota por pergunta (D-PRFAQ-2): a página do referente + a âncora do cabeçalho. A âncora não
/// abre o painel (ele fica num card recolhido), mas é ela que separa as perguntas de mesmo texto no
/// mesmo referente — sem ela, o nome do `.md` colide.
fn link_da_pergunta(rid: u32, id: u32) -> String {
    format!("{}#heading{id}", url_do_referente(rid))
}

/// `<nome do referente no index> | <assunto>` — dois níveis, sempre (D-PRFAQ-3).
fn trilha(r: &Referente, c: &Card) -> String {
    format!("{} | {}", r.nome, c.titulo)
}

fn montar_faq_raw(r: &Referente, c: &Card, p: &Painel) -> FaqRaw {
    FaqRaw {
        pergunta: p.pergunta.clone(),
        resposta: p.resposta.clone(),
        origin: trilha(r, c),
        url: link_da_pergunta(r.id, p.id),
    }
}

/// A árvore da aba de FAQs: raiz → um `Menu` por referente (ordem do index) → um `Faq` por card de
/// assunto (ordem da página), cada pergunta com o próprio link. Nó sem pergunta fica de fora, a regra
/// do SC (hoje não há card vazio).
fn montar_arvore(paginas: &[(Referente, Vec<Card>)]) -> FaqNode {
    let children = paginas
        .iter()
        .filter_map(|(r, cards)| {
            let assuntos: Vec<FaqNode> = cards
                .iter()
                .filter(|c| !c.paineis.is_empty())
                .map(|c| FaqNode {
                    title: c.titulo.clone(),
                    url: url_do_referente(r.id),
                    page_type: PageType::Faq,
                    origin: trilha(r, c),
                    children: Vec::new(),
                    faq_items: c
                        .paineis
                        .iter()
                        .map(|p| FaqItem {
                            pergunta: p.pergunta.clone(),
                            resposta: p.resposta.clone(),
                            url: Some(link_da_pergunta(r.id, p.id)),
                        })
                        .collect(),
                })
                .collect();
            (!assuntos.is_empty()).then(|| FaqNode {
                title: r.nome.clone(),
                url: url_do_referente(r.id),
                page_type: PageType::Menu,
                origin: String::new(),
                children: assuntos,
                faq_items: Vec::new(),
            })
        })
        .collect();

    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: format!("{BASE}/index"),
        page_type: PageType::Menu,
        origin: String::new(),
        children,
        faq_items: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Recortes do HTML real (01/11/2026), do `<div class="maincontent">` em diante: o `index` até o 3º
    // referente (com o formulário de busca e o `<select>` dos referentes, que o parse tem de ignorar)
    // e a página inteira do referente 39 (SPED-Fiscal: 2 cards, 3 + 2 painéis).
    const INDEX: &str = include_str!("../tests/fixtures/pr-faqs-index.html");
    const REFERENTE_39: &str = include_str!("../tests/fixtures/pr-faqs-referente-39.html");

    fn sped() -> Referente {
        Referente {
            id: 39,
            nome: "SPED-Fiscal - Sistema Público de Escrituração Digital".to_string(),
        }
    }

    fn paginas() -> Vec<(Referente, Vec<Card>)> {
        vec![(sped(), parse_referente(39, REFERENTE_39).unwrap())]
    }

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn parse_index_le_os_referentes_na_ordem_com_o_nome_longo() {
        let r = parse_index(INDEX).unwrap();
        let ids: Vec<u32> = r.iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![49, 1, 3]);
        // Espaço duplo da fonte ("ADRC-ST  - …") e a indentação em volta saem no `clean`.
        assert_eq!(
            r[0].nome,
            "ADRC-ST - Arquivo Digital de Recuperação e Ressarcimento do ICMS-ST e FECOP"
        );
        assert_eq!(
            r[1].nome,
            "AIDF - Autorização para Impressão de Documentos Fiscais"
        );
    }

    #[test]
    fn index_sem_referente_e_erro() {
        assert!(erro(parse_index("<html></html>")).contains("nenhum referente"));
    }

    #[test]
    fn parse_referente_le_cards_paineis_e_resposta() {
        let cards = parse_referente(39, REFERENTE_39).unwrap();
        let resumo: Vec<(&str, Vec<u32>)> = cards
            .iter()
            .map(|c| (c.titulo.as_str(), c.paineis.iter().map(|p| p.id).collect()))
            .collect();
        assert_eq!(
            resumo,
            vec![
                ("Apresentação", vec![1333, 1334, 1335]),
                ("Demais Informações sobre SPED", vec![1336, 2595]),
            ]
        );
        assert_eq!(cards[0].paineis[0].pergunta, "O que é SPED?");
        assert!(
            cards[0].paineis[0]
                .resposta
                .starts_with("O Sistema Público de Escrituração Digital (SPED) foi instituído")
        );
        // `<br>` vira linha; o `-` do começo de cada item fica.
        assert!(
            cards[0].paineis[2]
                .resposta
                .starts_with("- CT-e (Conhecimento de Transporte Eletrônico)\n- ECD (Escrituração Contábil Digital)\n")
        );
        // `<b>` sai sem quebrar a linha.
        assert!(cards[1].paineis[1].resposta.starts_with(
            "O Decreto nº 6.022, de 22 de janeiro de 2007, instituiu o Sistema Público de Escrituração Digital (SPED) como"
        ));
    }

    #[test]
    fn pagina_do_index_nao_vira_pergunta() {
        // O 302 de um referente desativado cai no index, que reusa `perguntaFrequentePanelHeader` e
        // `heading<N>` para os cards de REFERENTE: nada disso pode virar pergunta.
        let e = erro(parse_referente(19, INDEX));
        assert!(
            e.contains("referente 19: nenhuma pergunta na página"),
            "{e}"
        );
    }

    #[test]
    fn heading_diferente_do_numero_e_erro() {
        let html = REFERENTE_39.replacen("Pergunta Nº 1334", "Pergunta Nº 1343", 1);
        let e = erro(parse_referente(39, &html));
        assert!(e.contains("heading1334 traz a Pergunta Nº 1343"), "{e}");
    }

    #[test]
    fn assunto_diferente_do_card_e_erro() {
        let html = REFERENTE_39.replacen("Assunto: Apresentação", "Assunto: Outro", 1);
        let e = erro(parse_referente(39, &html));
        assert!(
            e.contains("pergunta 1333: Assunto \"Outro\" ≠ card \"Apresentação\""),
            "{e}"
        );
    }

    #[test]
    fn pergunta_que_cita_assunto_e_numero_nao_confunde_o_parse() {
        // A 2247 (referente 28) cita `Assunto: "Outorga de..."` no texto da pergunta.
        let html = REFERENTE_39.replacen(
            "O que é SPED?",
            "O que é SPED? Assunto: \"Outorga de...\". Pergunta Nº 7",
            1,
        );
        let cards = parse_referente(39, &html).unwrap();
        assert_eq!(
            cards[0].paineis[0].pergunta,
            "O que é SPED? Assunto: \"Outorga de...\". Pergunta Nº 7"
        );
    }

    #[test]
    fn painel_fora_de_card_e_erro() {
        let html = format!(
            r#"<div class="panel-heading perguntaFrequentePanelHeader" id="heading9"></div>{REFERENTE_39}"#
        );
        let e = erro(parse_referente(39, &html));
        assert!(e.contains("6 painéis no HTML, 5 parseados"), "{e}");
    }

    #[test]
    fn numero_em_dois_referentes_e_erro() {
        let mut p = paginas();
        p.push((
            Referente {
                id: 40,
                nome: "Outro".to_string(),
            },
            parse_referente(39, REFERENTE_39).unwrap(),
        ));
        let e = erro(consolidar(&p));
        assert!(e.contains("5 número(s) de pergunta repetido(s)"), "{e}");
        assert!(e.contains("1333 (referentes 39 e 40)"), "{e}");
    }

    #[test]
    fn pergunta_sem_texto_e_erro() {
        let mut p = paginas();
        p[0].1[1].paineis[0].pergunta.clear();
        let e = erro(consolidar(&p));
        assert!(e.contains("1 pergunta(s) sem texto — 1336"), "{e}");
    }

    #[test]
    fn resposta_vazia_so_avisa() {
        let mut p = paginas();
        p[0].1[0].paineis[0].resposta.clear();
        assert_eq!(consolidar(&p).unwrap().len(), 5);
    }

    #[test]
    fn faq_raw_tem_trilha_de_dois_niveis_e_link_com_ancora() {
        let faqs = consolidar(&paginas()).unwrap();
        assert_eq!(faqs.len(), 5);
        assert_eq!(
            faqs[3].pergunta,
            "Para acessar mais informações e serviços relacionados ao SPED"
        );
        assert_eq!(
            faqs[3].origin,
            "SPED-Fiscal - Sistema Público de Escrituração Digital | Demais Informações sobre SPED"
        );
        assert_eq!(
            faqs[3].url,
            "https://atendimento.fazenda.pr.gov.br/sacsefa/portal/assuntosReferente/39#heading1336"
        );
    }

    #[test]
    fn arvore_tem_referente_assunto_e_link_por_pergunta() {
        let t = montar_arvore(&paginas());
        assert_eq!(t.title, "Perguntas frequentes");
        assert_eq!(
            t.url,
            "https://atendimento.fazenda.pr.gov.br/sacsefa/portal/index"
        );
        let sped = &t.children[0];
        assert_eq!(sped.page_type, PageType::Menu);
        assert_eq!(
            sped.url,
            "https://atendimento.fazenda.pr.gov.br/sacsefa/portal/assuntosReferente/39"
        );
        let titulos: Vec<&str> = sped.children.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(
            titulos,
            vec!["Apresentação", "Demais Informações sobre SPED"]
        );
        let apresentacao = &sped.children[0];
        assert_eq!(apresentacao.page_type, PageType::Faq);
        assert_eq!(
            apresentacao.origin,
            "SPED-Fiscal - Sistema Público de Escrituração Digital | Apresentação"
        );
        assert_eq!(apresentacao.faq_items.len(), 3);
        assert_eq!(
            apresentacao.faq_items[1].url.as_deref(),
            Some(
                "https://atendimento.fazenda.pr.gov.br/sacsefa/portal/assuntosReferente/39#heading1334"
            )
        );
        // O fio que o `parseFaqs.ts` lê: folha sem `children`, page_type como string.
        let json = serde_json::to_value(apresentacao).unwrap();
        assert_eq!(json["page_type"], "Faq");
        assert!(json.get("children").is_none());
    }

    // As respostas abaixo são as reais (01/11/2026), recortadas onde são longas.

    #[test]
    fn link_maiusculo_sem_aspas_no_target_e_quebras() {
        // FAQ 0271 (IPVA): `<A HREF=… target=_new>`, `<br><br>` e a seta `->` do texto.
        let r = r#"Solicitar a regularização de débitos de IPVA através do <A HREF="https://www.contribuinte.fazenda.pr.gov.br/ipva/faces/home" target=_new>Portal do IPVA</A> (acesse com senha do programa Nota Paraná) -> menu "Serviços" -> opção "Regularização de Débitos".<br><br>Cabe ao arrematante solicitar a transferência do veículo para seu nome ao Detran/PR.<br><br>"#;
        assert_eq!(
            html_para_texto(r),
            "Solicitar a regularização de débitos de IPVA através do Portal do IPVA \
             \"https://www.contribuinte.fazenda.pr.gov.br/ipva/faces/home\" (acesse com senha do \
             programa Nota Paraná) -> menu \"Serviços\" -> opção \"Regularização de Débitos\".\n\
             Cabe ao arrematante solicitar a transferência do veículo para seu nome ao Detran/PR."
        );
    }

    #[test]
    fn menor_que_de_menu_fica_no_texto() {
        // FAQ 2274 (referente 30): o `<` é a seta do caminho de menu. O `<b>` adiante fecharia o
        // `<[^>]+>` do SC, que apagaria o caminho até ele (o `<br>` não: vira quebra antes).
        let r = r#"A comunicação deverá ser efetuada por meio do formulário do cadastro eletrônico, disponível na área restrita do <a href="https://receita.pr.gov.br/login" target="_blank">https://receita.pr.gov.br/login</a> - Receita/PR, menu Cadastro de Contribuintes < Alteração Cadastral < Dados Cadastrais. Fica <b>dispensado o envio de qualquer documento</b>, conforme disposto no no § 9º do art 17 da NPF 092/2017."#;
        assert_eq!(
            html_para_texto(r),
            "A comunicação deverá ser efetuada por meio do formulário do cadastro eletrônico, \
             disponível na área restrita do https://receita.pr.gov.br/login \
             \"https://receita.pr.gov.br/login\" - Receita/PR, menu Cadastro de Contribuintes < \
             Alteração Cadastral < Dados Cadastrais. Fica dispensado o envio de qualquer documento, \
             conforme disposto no no § 9º do art 17 da NPF 092/2017."
        );
    }

    #[test]
    fn tag_fora_da_lista_fica_no_texto() {
        // FAQ 1775 (CADIN): `<Consulta Pendencias>` é o nome do menu do portal, escrito como tag.
        let r = r#"No endereço eletrônico <a href="https://www.fazenda.pr.gov.br/servicos/Empresa/Cadin/Consultar-se-o-CPF-ou-o-CNPJ-esta-inscrito-no-Cadin-aPo4ZB3m" target="_blank">www.cadin.pr.gov.br</a> <Consulta Pendencias>, basta digitar o número do CPF ou do CNPJ."#;
        assert_eq!(
            html_para_texto(r),
            "No endereço eletrônico www.cadin.pr.gov.br \
             \"https://www.fazenda.pr.gov.br/servicos/Empresa/Cadin/Consultar-se-o-CPF-ou-o-CNPJ-esta-inscrito-no-Cadin-aPo4ZB3m\" \
             <Consulta Pendencias>, basta digitar o número do CPF ou do CNPJ."
        );
    }

    #[test]
    fn imagem_vira_url_absoluta() {
        // FAQ 1009 (Energia Elétrica): a resposta é só a tabela em imagem.
        let r = "\n                        <img src=\"/sacsefa/images/tabelas/EESC_TCNAE1.JPG\" width=\"100%\" height=\"700\">                    ";
        assert_eq!(
            html_para_texto(r),
            "\"https://atendimento.fazenda.pr.gov.br/sacsefa/images/tabelas/EESC_TCNAE1.JPG\""
        );
        // FAQ 745: ícone no meio da frase — a mesma regra.
        assert_eq!(
            html_para_texto(
                r#"A <img src="/sacsefa/images/icon_localizar.png"> possibilita visualizar a ocorrência.<br>"#
            ),
            "A \"https://atendimento.fazenda.pr.gov.br/sacsefa/images/icon_localizar.png\" possibilita visualizar a ocorrência."
        );
    }

    #[test]
    fn agent_com_dv_r36_constroi() {
        let _ = agent(); // o PEM embutido é válido.
    }
}
