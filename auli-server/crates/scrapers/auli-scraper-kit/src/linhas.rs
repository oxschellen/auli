//! FAQ em linhas: para os portais em que a pergunta é só um trecho em negrito no meio do texto, sem estrutura própria
//! (o Joomla da PB e o do PA). A seção vira linhas — tag de bloco e `<br>` quebram — e pergunta é a linha cujo texto
//! com letras está todo em destaque e termina em "?". Estudos: `docs/faqs/ESTUDO-PB-Faqs.md` §3 e
//! `docs/faqs/ESTUDO-PA-Faqs.md` §3 (D-PBFAQ-2, D-PAFAQ-2, -3).
//!
//! - [`linhas`]: o elemento da seção → as linhas, cada uma com o texto puro, um HTML mínimo (texto, links, imagens e as
//!   tabelas inteiras, que o [`crate::html`] converte) e se está toda em destaque;
//! - [`segmentar`]: as linhas → perguntas, com o grupo ("COMÉRCIO EXTERIOR - PERGUNTAS FREQUENTES") de cada uma;
//! - [`sem_cloak`]: o e-mail protegido do Joomla → o texto que o navegador mostra (antes do parse);
//! - [`sem_prefixo_r`] e [`numeracao_irregular`]: os acabamentos da resposta e do aviso de numeração.

use std::collections::{BTreeMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;
use scraper::node::Node;
use scraper::{ElementRef, Html};

use crate::clean;

/// O e-mail protegido do Joomla: o texto "Este endereço de email está sendo protegido de spambots…" e o script que
/// monta o endereço.
static CLOAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<span id="cloak([0-9a-f]+)">[^<]*</span>\s*<script[^>]*>(.*?)</script>"#)
        .unwrap()
});
static PARTE_JS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"'([^']*)'").unwrap());
/// O que sobra no texto quando o plugin muda e o [`sem_cloak`] não acha o endereço — o chamador o usa como guarda.
pub const CLOAK_TEXTO: &str = "protegido de spambots";
/// Numeração da pergunta: "1.", "1 -", "1 –", "1)", "P1.", "11.a-", "1.1 -".
static NUM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^P?(\d+)(?:\.[a-z0-9]+)?\s*(?:[.\-–)]|\s[-–])\s*").unwrap());
/// O número seguido só de espaço ("49 Deve ser gerado…?", no SINTEGRA da BA): vale só na pergunta em destaque que
/// continua a numeração.
static NUM_SOLTO_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d+)\s+[^\W\d_]").unwrap());
/// Título de subgrupo ("COMÉRCIO EXTERIOR - PERGUNTAS FREQUENTES").
static GRUPO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)PERGUNTAS\s+(?:FREQUENTES|E\s+RESPOSTAS)").unwrap());
static SUFIXO_GRUPO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s*[-–]\s*PERGUNTAS\s+(?:FREQUENTES|E\s+RESPOSTAS)\s*$").unwrap()
});
/// O "R –", "R." ou "R:" que abre a resposta. "R$" não casa.
static PREFIXO_R_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^R\s*[-–.:]\s*").unwrap());
static LETRA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\W\d_]").unwrap());
/// A nota entre parênteses depois do "?" ("…da NF-e? (atualizado em 31/12/08)", na NF-e da BA).
static NOTA_FINAL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\([^()]*\)$").unwrap());

/// Troca cada e-mail protegido pelo texto que o navegador mostra: o `addy_text…` do script, cujos pedaços vêm entre
/// aspas, com entidades e às vezes com um `<span>` de cor. O texto visível vale mais que o `mailto`: na PB, um `mailto`
/// tem erro de digitação e o texto está certo. Sem `addy_text`, o trecho fica, e a guarda do chamador ([`CLOAK_TEXTO`])
/// pega.
pub fn sem_cloak(html: &str) -> String {
    CLOAK_RE
        .replace_all(html, |c: &regex::Captures| {
            let Ok(re) = Regex::new(&format!(
                r"var addy_text{}\s*=((?:\s*\+?\s*'[^']*')+)",
                &c[1]
            )) else {
                return c[0].to_string();
            };
            match re.captures(&c[2]) {
                Some(t) => {
                    let junto: String = PARTE_JS_RE
                        .captures_iter(&t[1])
                        .map(|p| p[1].to_string())
                        .collect();
                    let visivel: String =
                        Html::parse_fragment(&junto).root_element().text().collect();
                    escapar(clean(&visivel).as_str())
                }
                None => c[0].to_string(),
            }
        })
        .into_owned()
}

/// Uma linha da seção: o texto puro (para achar a pergunta), o HTML mínimo (para o conversor) e se todo o texto com
/// letras está em destaque.
#[derive(Debug, Default)]
pub struct Linha {
    pub texto: String,
    pub html: String,
    pub negrito: bool,
}

#[derive(Default)]
struct Escritor {
    linhas: Vec<Linha>,
    texto: String,
    html: String,
    tem_texto_fora: bool,
    /// Um `<a>` aberto na linha atual. A quebra o fecha: a PB deixa `<a href="mailto:…">` aberto até o endereço da
    /// linha seguinte, e o conversor colaria as duas linhas no texto do link ("…gov.brAv. Presidente…").
    link_aberto: bool,
}

impl Escritor {
    fn quebra(&mut self) {
        if self.link_aberto {
            self.html.push_str("</a>");
            self.link_aberto = false;
        }
        let texto = clean(&self.texto);
        if !texto.is_empty() || !self.html.trim().is_empty() {
            self.linhas.push(Linha {
                negrito: !texto.is_empty() && !self.tem_texto_fora,
                texto,
                html: std::mem::take(&mut self.html),
            });
        }
        self.texto.clear();
        self.html.clear();
        self.tem_texto_fora = false;
    }
}

/// O elemento da seção → as linhas, na ordem do documento.
pub fn linhas(el: ElementRef) -> Vec<Linha> {
    let mut e = Escritor::default();
    andar(el, false, &mut e);
    e.quebra();
    e.linhas
}

fn eh_negrito(el: ElementRef) -> bool {
    matches!(
        el.value().name(),
        "strong" | "b" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
    )
}

/// HTML → linhas. Bloco e `<br>` quebram a linha; a tabela entra inteira numa linha sua (o conversor a lê); a lista
/// de anexos do `com_attachments` não entra. No HTML da linha só ficam o texto (reescapado: o parser já decodificou),
/// os links e as imagens — o conversor do kit faz o resto.
fn andar(el: ElementRef, negrito: bool, e: &mut Escritor) {
    for filho in el.children() {
        match filho.value() {
            Node::Text(t) => {
                e.texto.push_str(t);
                e.html.push_str(&escapar(t));
                // Trecho sem letra fora do negrito (o "1" de "13" que o editor deixou de fora, o "“" depois do "?") não
                // desfaz o destaque da linha.
                if !negrito && LETRA_RE.is_match(t) {
                    e.tem_texto_fora = true;
                }
            }
            Node::Element(_) => {
                let Some(f) = ElementRef::wrap(filho) else {
                    continue;
                };
                let v = f.value();
                if v.classes().any(|c| c.starts_with("attachments")) {
                    continue;
                }
                match v.name() {
                    "script" | "style" => {}
                    "br" => e.quebra(),
                    "table" => {
                        e.quebra();
                        e.html.push_str(&f.html());
                        e.quebra();
                    }
                    "img" => {
                        if let Some(src) = v.attr("src") {
                            e.html
                                .push_str(&format!("<img src=\"{}\">", escapar_atributo(src)));
                        }
                    }
                    "a" => {
                        if let Some(href) = v.attr("href") {
                            e.html
                                .push_str(&format!("<a href=\"{}\">", escapar_atributo(href)));
                            e.link_aberto = true;
                        }
                        andar(f, negrito, e);
                        if e.link_aberto {
                            e.html.push_str("</a>");
                            e.link_aberto = false;
                        }
                    }
                    nome => {
                        let bloco = matches!(
                            nome,
                            "p" | "div"
                                | "li"
                                | "ul"
                                | "ol"
                                | "dt"
                                | "dd"
                                | "dl"
                                | "blockquote"
                                | "hr"
                                | "center"
                                | "h1"
                                | "h2"
                                | "h3"
                                | "h4"
                                | "h5"
                                | "h6"
                        );
                        if bloco {
                            e.quebra();
                        }
                        andar(f, negrito || eh_negrito(f), e);
                        if bloco {
                            e.quebra();
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn escapar(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escapar_atributo(s: &str) -> String {
    escapar(s).replace('"', "&quot;")
}

/// Uma pergunta como saiu do [`segmentar`]: o título (texto puro), o HTML da resposta (para o conversor) e o grupo
/// (vazio quando a seção não tem).
#[derive(Debug)]
pub struct Bruta {
    pub titulo: String,
    pub resposta_html: String,
    pub grupo: String,
}

/// Sem aspas e espaços do fim: a DeSTDA da PB fechava uma pergunta com `?" “`.
fn sem_fecho(s: &str) -> &str {
    s.trim_end_matches(|c: char| c.is_whitespace() || matches!(c, '"' | '“' | '”' | '\''))
}

/// Termina em "?", a menos das aspas e de uma nota entre parênteses depois dele.
fn termina_em_interrogacao(l: &Linha) -> bool {
    let t = sem_fecho(&l.texto);
    t.ends_with('?') || sem_fecho(&NOTA_FINAL_RE.replace(t, "")).ends_with('?')
}

fn numerada(l: &Linha) -> bool {
    NUM_RE.is_match(&l.texto)
}

fn numero(texto: &str) -> Option<u32> {
    NUM_RE.captures(texto).and_then(|c| c[1].parse().ok())
}

/// As linhas → perguntas. Pergunta = linha em destaque que termina em "?" (a nota entre parênteses depois dele não
/// conta); quando a maioria é numerada, só as
/// numeradas (no Cadastro da PB, "Sim. Mas o que fazer?" é subtítulo de resposta). Numa seção **sem nenhuma** pergunta
/// em destaque, valem as linhas numeradas que terminam em "?" (o PIX, o Parcelamento e as Imunidades do PA). Grupo =
/// linha em destaque sem "?" com "PERGUNTAS FREQUENTES"/"PERGUNTAS E RESPOSTAS", depois da 1ª pergunta. Resposta = as
/// linhas até a próxima pergunta ou grupo; o que vem antes da 1ª pergunta (título, introdução, índice) sai.
pub fn segmentar(linhas: &[Linha]) -> Vec<Bruta> {
    let mut perguntas: Vec<usize> = linhas
        .iter()
        .enumerate()
        .filter(|(_, l)| l.negrito && termina_em_interrogacao(l))
        .map(|(i, _)| i)
        .collect();
    if perguntas.is_empty() {
        perguntas = linhas
            .iter()
            .enumerate()
            .filter(|(_, l)| numerada(l) && termina_em_interrogacao(l))
            .map(|(i, _)| i)
            .collect();
    }
    let numeradas = perguntas.iter().filter(|&&i| numerada(&linhas[i])).count();
    if !perguntas.is_empty() && numeradas * 2 >= perguntas.len() {
        // Seção numerada: só as numeradas — e também a linha em destaque que continua a numeração sem terminar em "?"
        // ("8. A consulta … pode ser feita de duas formas:", "2- Qualquer valor deve ser retificado? Mesmo que …", no PA),
        // desde que tenha letra: o número sozinho em negrito ("10.") é item de lista dentro da resposta (o SINTEGRA da
        // BA numera os tipos de registro assim). A pergunta marcada com o número sem separador ("49 Deve…?") entra se
        // continua a numeração.
        let marcadas: HashSet<usize> = perguntas.iter().copied().collect();
        perguntas.clear();
        let mut ultimo: Option<u32> = None;
        for (i, l) in linhas.iter().enumerate() {
            let solto = || {
                NUM_SOLTO_RE
                    .captures(&l.texto)
                    .and_then(|c| c[1].parse().ok())
                    .filter(|&n| marcadas.contains(&i) && ultimo.is_some_and(|u| n == u + 1))
            };
            let Some(n) = numero(&l.texto).or_else(solto) else {
                continue;
            };
            if marcadas.contains(&i)
                || (l.negrito && LETRA_RE.is_match(&l.texto) && ultimo.is_some_and(|u| n == u + 1))
            {
                perguntas.push(i);
                ultimo = Some(n);
            }
        }
    }
    let Some(&primeira) = perguntas.first() else {
        return Vec::new();
    };
    let grupos: HashSet<usize> = linhas
        .iter()
        .enumerate()
        .filter(|&(i, l)| {
            i > primeira && l.negrito && GRUPO_RE.is_match(&l.texto) && !termina_em_interrogacao(l)
        })
        .map(|(i, _)| i)
        .collect();
    let mut marcos: Vec<usize> = perguntas
        .iter()
        .copied()
        .chain(grupos.iter().copied())
        .collect();
    marcos.sort_unstable();
    let mut out = Vec::new();
    let mut grupo = String::new();
    for (k, &i) in marcos.iter().enumerate() {
        let fim = marcos.get(k + 1).copied().unwrap_or(linhas.len());
        if grupos.contains(&i) {
            grupo = SUFIXO_GRUPO_RE.replace(&linhas[i].texto, "").into_owned();
            continue;
        }
        out.push(Bruta {
            titulo: linhas[i].texto.clone(),
            resposta_html: linhas[i + 1..fim]
                .iter()
                .map(|l| l.html.as_str())
                .collect::<Vec<_>>()
                .join("<br>"),
            grupo: grupo.clone(),
        });
    }
    out
}

/// A resposta sem o "R –", "R." ou "R:" do começo.
pub fn sem_prefixo_r(resposta: &str) -> String {
    PREFIXO_R_RE.replace(resposta, "").into_owned()
}

/// Os grupos cuja numeração não é 1, 2, 3…: `(grupo, números)`. Só serve de aviso — a MDF-e da PB não tem a 6, e
/// isso é da página.
pub fn numeracao_irregular(brutas: &[Bruta]) -> Vec<(String, Vec<u32>)> {
    let mut por_grupo: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
    for b in brutas {
        if let Some(n) = numero(&b.titulo) {
            por_grupo.entry(b.grupo.as_str()).or_default().push(n);
        }
    }
    por_grupo
        .into_iter()
        .filter(|(_, nums)| nums.iter().copied().ne(1..=nums.len() as u32))
        .map(|(g, nums)| (g.to_string(), nums))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Selector;

    fn segmentado(corpo: &str) -> Vec<Bruta> {
        let doc = Html::parse_document(&format!(
            "<html><body><div id=s>{corpo}</div></body></html>"
        ));
        let sel = Selector::parse("#s").unwrap();
        segmentar(&linhas(doc.select(&sel).next().unwrap()))
    }

    fn titulos(bs: &[Bruta]) -> Vec<&str> {
        bs.iter().map(|b| b.titulo.as_str()).collect()
    }

    #[test]
    fn pergunta_em_negrito_no_mesmo_paragrafo_da_resposta() {
        let bs = segmentado(
            "<p><b>1 - O que é?<br></b>É isto.</p><p><b>2 - E aquilo?</b><br>Aquilo.</p>",
        );
        assert_eq!(titulos(&bs), ["1 - O que é?", "2 - E aquilo?"]);
        assert_eq!(bs[0].resposta_html, "É isto.");
    }

    #[test]
    fn maioria_numerada_descarta_o_negrito_sem_numero() {
        let bs = segmentado(
            "<h4>1. Primeira?</h4><p>Resposta. <strong>Sim. Mas o que fazer?</strong></p><p>Isto.</p><h4>2. Segunda?</h4><p>R2.</p>",
        );
        assert_eq!(titulos(&bs), ["1. Primeira?", "2. Segunda?"]);
    }

    #[test]
    fn negrito_que_continua_a_numeracao_abre_pergunta_sem_interrogacao() {
        let bs = segmentado(
            "<p><b>1- Primeira?</b></p><p>R1.</p><p><b>2- Segunda? Mesmo que pouco.</b></p><p>R2.</p><p><b>3. Pode ser de duas formas:</b></p><p>a) assim.</p><p><b>5. Pulou?</b></p><p>R5.</p>",
        );
        assert_eq!(
            titulos(&bs),
            [
                "1- Primeira?",
                "2- Segunda? Mesmo que pouco.",
                "3. Pode ser de duas formas:",
                "5. Pulou?"
            ]
        );
        // O número que não continua a sequência, sem "?", fica na resposta — e a lista numerada sem negrito também.
        let bs = segmentado("<p><b>1. Um?</b></p><p><b>3. Passo três</b></p><p>2. passo dois</p>");
        assert_eq!(titulos(&bs), ["1. Um?"]);
    }

    #[test]
    fn nota_entre_parenteses_depois_do_ponto_de_interrogacao() {
        // A NF-e da BA: as três primeiras da seção têm a data da revisão depois do "?" e nenhuma antes as numera.
        let bs = segmentado(
            "<p>Introdução.</p><p><b>1. Como funciona? </b><b>(atualizado em 31/12/08)</b></p><p>Assim.</p>\
             <p><b>2. E o prazo?</b></p><p>Curto.</p><p><b>Nota (rodapé)</b></p>",
        );
        assert_eq!(
            titulos(&bs),
            [
                "1. Como funciona? (atualizado em 31/12/08)",
                "2. E o prazo?"
            ]
        );
        // O negrito com parênteses e sem "?" segue na resposta.
        assert_eq!(bs[1].resposta_html, "Curto.<br>Nota (rodapé)");
    }

    #[test]
    fn numero_sozinho_em_negrito_e_item_da_resposta() {
        // O SINTEGRA da BA: a resposta da 9 lista os tipos de registro com o número em negrito numa célula própria.
        let html = "<p><b>9. Quais registros informar?</b></p><p>Os tipos:</p><p><b>10.</b></p><p>Registro 61.</p>\
                    <p><b>11.</b></p><p>Registro 70.</p><p><b>10. Quem envia?</b></p><p>Todos.</p>";
        let bs = segmentado(html);
        assert_eq!(
            titulos(&bs),
            ["9. Quais registros informar?", "10. Quem envia?"]
        );
        assert!(bs[0].resposta_html.contains("Registro 70."));
    }

    #[test]
    fn numero_sem_separador_na_pergunta_que_continua_a_numeracao() {
        // O SINTEGRA da BA: "49 Deve…?" (sem ponto) entre a 48 e a 50. Fora da sequência, segue na resposta.
        let html = "<p><b>48. Quais registros?</b></p><p>O 61.</p><p><b>49 Deve ser um por nota?</b></p><p>Não.</p>\
                    <p><b>50. Desde quando?</b></p><p>2004.</p><p><b>2004 Mudou algo?</b></p><p>Nada.</p>";
        let bs = segmentado(html);
        assert_eq!(
            titulos(&bs),
            [
                "48. Quais registros?",
                "49 Deve ser um por nota?",
                "50. Desde quando?"
            ]
        );
        assert_eq!(bs[0].resposta_html, "O 61.");
        assert!(bs[2].resposta_html.contains("2004 Mudou algo?"));
    }

    #[test]
    fn sem_negrito_valem_as_numeradas() {
        // O Parcelamento do PA: nenhuma pergunta em negrito.
        let bs = segmentado(
            "<p>Introdução?</p><p>1.1 - Como faço para parcelar?</p><p>Assim.</p><p>1.2 - Preciso ir à SEFA?</p><p>Não.</p>",
        );
        assert_eq!(
            titulos(&bs),
            ["1.1 - Como faço para parcelar?", "1.2 - Preciso ir à SEFA?"]
        );
        assert_eq!(bs[1].resposta_html, "Não.");
    }

    #[test]
    fn com_negrito_as_numeradas_sem_negrito_nao_contam() {
        // O índice do Simples da PB: só o número em negrito; as perguntas vêm depois, inteiras em negrito.
        let bs = segmentado(
            "<ul><li><strong>1 - </strong>Não sou empresa nova?</li></ul><p><strong>1 – Não sou empresa nova?</strong></p><p>R.</p>",
        );
        assert_eq!(titulos(&bs), ["1 – Não sou empresa nova?"]);
    }

    #[test]
    fn grupo_vira_rotulo_e_sai_da_resposta() {
        let bs = segmentado(
            "<p><b>1 - Geral?</b></p><p>R1.</p><p><b>COMÉRCIO EXTERIOR - PERGUNTAS FREQUENTES</b></p><p><b>1 - Exterior?</b></p><p>R2.</p>",
        );
        assert_eq!(bs[0].resposta_html, "R1.");
        assert_eq!(
            (bs[1].grupo.as_str(), bs[0].grupo.as_str()),
            ("COMÉRCIO EXTERIOR", "")
        );
    }

    #[test]
    fn digito_e_aspas_fora_do_negrito_nao_desfazem_a_pergunta() {
        let bs = segmentado(
            "<p>1<strong>3 - Treze?</strong></p><p>R.</p><p><strong>14. Catorze? </strong> “</p><p>R.</p>",
        );
        assert_eq!(titulos(&bs), ["13 - Treze?", "14. Catorze? “"]);
    }

    #[test]
    fn link_aberto_fecha_na_quebra() {
        let bs = segmentado(
            r#"<p><b>1. Onde?</b></p><p><a href="mailto:a@x.gov.br"><strong>a@x.gov.br</strong><br>Av. Brasil, 1</a></p>"#,
        );
        assert_eq!(
            bs[0].resposta_html,
            r#"<a href="mailto:a@x.gov.br">a@x.gov.br</a><br>Av. Brasil, 1"#
        );
    }

    #[test]
    fn tabela_inteira_numa_linha_e_anexos_fora() {
        let bs = segmentado(
            "<p><b>1. Qual?</b></p><table><tr><td>a</td></tr></table><div class=\"attachmentsContainer\">x.pdf 215 kB</div>",
        );
        assert_eq!(
            bs[0].resposta_html,
            "<table><tbody><tr><td>a</td></tr></tbody></table>"
        );
    }

    #[test]
    fn email_protegido_vira_o_texto_visivel() {
        let html = r#"<p>Escreva a <span id="cloak9f">Este endereço de email está sendo protegido de spambots.</span><script type='text/javascript'>
            var addy9f = 'c&#97;d' + '&#64;'; var addy_text9f = '<span style="color: #000;">c&#97;d</span>' + '&#64;' + 'x&#46;br';</script>.</p>"#;
        assert_eq!(sem_cloak(html), "<p>Escreva a cad@x.br.</p>");
        // Sem `addy_text`, fica como está (e a guarda do chamador pega).
        let sem = r#"<span id="cloak1">Este endereço de email está sendo protegido de spambots.</span><script>x</script>"#;
        assert!(sem_cloak(sem).contains(CLOAK_TEXTO));
    }

    #[test]
    fn prefixo_r_e_numeracao_irregular() {
        assert_eq!(sem_prefixo_r("R – Sim."), "Sim.");
        assert_eq!(sem_prefixo_r("R$ 4.800,00"), "R$ 4.800,00");
        let bs = segmentado(
            "<p><b>1 - A?</b></p><p>r</p><p><b>2 - B?</b></p><p>r</p><p><b>4 - D?</b></p><p>r</p>",
        );
        assert_eq!(
            numeracao_irregular(&bs),
            vec![(String::new(), vec![1, 2, 4])]
        );
    }
}
