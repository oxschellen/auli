// As FAQs que o portal publica em PDF (D-RSFAQ-7): o item do menu redireciona (303) para o arquivo, e a caminhada lê o
// texto (`kit::pdf`, `pdftotext -layout`) em vez do HTML. Os PDFs de perguntas são cinco (a lista em `main.rs`); o
// "Manual para Declaração de ITCD" também é PDF, mas é manual, não perguntas, e fica fora (decisão do mantenedor).
//
// Um leitor só para os cinco moldes do Word:
// - cabeçalho e rodapé: as linhas que se repetem em metade das páginas ou mais (o endereço do GES, "Estado do Rio Grande
//   do Sul / Secretaria da Fazenda / Receita Estadual") saem, e o número de página também;
// - sumário: o que vem até a última linha com pontilhado e página ("…… 7") sai — e ele não é lista-mestra: o de Bebidas
//   não tem a 5.10 nem a 5.11;
// - a numeração do corpo manda: "N.M." em capítulos "N. TÍTULO EM CAIXA ALTA" (os GES de Bebidas e Comunicações), ou
//   "N." / "N)" corrida, com "Seção N: …" opcional (NFCom, Convênio 201/17, Autorregularização). A pergunta é a linha com
//   o número seguinte e vai até o "?"; a resposta, até a próxima pergunta, seção ou capítulo. Uma seção pode recomeçar
//   repetindo o último número (a 2 da NFCom recomeça no 7) — aviso. Item numerado que não chega a um "?" não é pergunta
//   ("14) Legislação compilada:", no Convênio): fica fora, com aviso;
// - a resposta perde o "R.:"/"R:" do começo, e a frase que a quebra de página corta continua no mesmo parágrafo.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use crate::errors::Result;

static SECAO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(Seção \d+:\s*\S.*?)\s*$").unwrap());
/// O capítulo dos PDFs dos GES: "2. LEGISLAÇÃO SUBSTITUIÇÃO TRIBUTÁRIA" (sem minúscula nem "?").
static CAPITULO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(\d+)\.\s+[^a-zà-ú?]{3,}$").unwrap());
/// "1.", "1)", "1.1." no começo da linha.
static NUM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(\d+(?:\.\d+)*)[.)]\s+\S").unwrap());
static HIERARQUICA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*1\.1\.\s").unwrap());
/// A entrada do sumário: pontilhado e o número da página.
static SUMARIO_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\.{5,}\s*\d+\s*$").unwrap());
static RESPOSTA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^R\.?:\s*").unwrap());
static PAGINA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d+$").unwrap());
/// O item de lista que abre a linha — marcador ("•", "-"), alínea ("b)"), inciso ("II –") ou item numerado ("2 –",
/// "1)") — abre parágrafo.
static ITEM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[•▪●\-−–]|\d+\s*[-–)]|[a-z]\)|[IVXL]+\s*[-–])\s").unwrap());

pub struct Secao {
    /// Vazio no PDF sem seções nem capítulos.
    pub titulo: String,
    pub perguntas: Vec<Pergunta>,
}

pub struct Pergunta {
    pub pergunta: String,
    pub resposta: String,
    /// A página do PDF em que a pergunta começa (o link leva `#page=N`).
    pub pagina: usize,
}

/// As seções e perguntas do texto do PDF. Erro quando o molde muda: nenhuma pergunta, seção sem pergunta, pergunta sem
/// resposta, ou texto solto entre o título de uma seção e a 1ª pergunta dela.
pub fn secoes(texto: &str) -> Result<Vec<Secao>> {
    let linhas = sem_sumario(sem_cabecalho_nem_rodape(texto));
    let numeracao = Numeracao {
        hierarquica: linhas.iter().any(|(_, l)| HIERARQUICA_RE.is_match(l)),
    };
    let mut secoes: Vec<Secao> = Vec::new();
    let mut ultimo: Option<Vec<u32>> = None;
    let mut i = 0;
    while i < linhas.len() {
        let (pagina, l) = linhas[i];
        let titulo = SECAO_RE.captures(l).map(|c| c[1].to_string()).or_else(|| {
            numeracao
                .capitulo_seguinte(l, ultimo.as_deref())
                .then(|| l.to_string())
        });
        if let Some(t) = titulo {
            secoes.push(Secao {
                titulo: normalizar(&t),
                perguntas: Vec::new(),
            });
            i += 1;
            continue;
        }
        let secao_vazia = secoes.last().is_some_and(|s| s.perguntas.is_empty());
        let n = match numero(l) {
            Some(n) if numeracao.proximos(ultimo.as_deref()).contains(&n) => n,
            Some(n) if !numeracao.hierarquica && secao_vazia && ultimo.as_ref() == Some(&n) => {
                println!(
                    "RS FAQs: aviso — PDF: a \"{}\" recomeça no número {}",
                    secoes.last().map_or("", |s| s.titulo.as_str()),
                    n[0]
                );
                n
            }
            _ if l.trim().is_empty() || secoes.is_empty() => {
                // Antes da 1ª pergunta ou seção: o título e a introdução do documento.
                i += 1;
                continue;
            }
            _ => return Err(format!("PDF: texto fora de pergunta: {:?}", l.trim()).into()),
        };
        ultimo = Some(n);
        let fronteira = |l: &str| numeracao.fronteira(l, ultimo.as_deref());
        let mut pergunta = vec![l.trim()];
        i += 1;
        while !pergunta.last().is_some_and(|p| p.ends_with('?'))
            && linhas.get(i).is_some_and(|(_, l)| {
                !l.trim().is_empty() && !RESPOSTA_RE.is_match(l.trim()) && !fronteira(l)
            })
        {
            pergunta.push(linhas[i].1.trim());
            i += 1;
        }
        let inicio = i;
        while i < linhas.len() && !fronteira(linhas[i].1) {
            i += 1;
        }
        let pergunta = normalizar(&pergunta.join(" "));
        if !pergunta.ends_with('?') {
            println!("RS FAQs: aviso — PDF: {pergunta:?} não é pergunta — fica fora");
            continue;
        }
        let resposta = paragrafos(&linhas[inicio..i]);
        if resposta.is_empty() {
            return Err(format!("PDF: a pergunta {pergunta:?} ficou sem resposta").into());
        }
        if secoes.is_empty() {
            secoes.push(Secao {
                titulo: String::new(),
                perguntas: Vec::new(),
            });
        }
        if let Some(s) = secoes.last_mut() {
            s.perguntas.push(Pergunta {
                pergunta,
                resposta,
                pagina,
            });
        }
    }
    if secoes.is_empty() {
        return Err("PDF: nenhuma pergunta no texto".into());
    }
    if let Some(s) = secoes.iter().find(|s| s.perguntas.is_empty()) {
        return Err(format!("PDF: a \"{}\" não tem pergunta", s.titulo).into());
    }
    Ok(secoes)
}

/// As linhas com a página de cada uma, sem as que se repetem em metade das páginas ou mais (cabeçalho e rodapé; ao
/// menos 3 páginas e 10 caracteres, para não levar uma resposta curta repetida) e sem o número de página.
fn sem_cabecalho_nem_rodape(texto: &str) -> Vec<(usize, &str)> {
    let paginas: Vec<&str> = texto.split('\u{c}').collect();
    let mut vezes: HashMap<String, usize> = HashMap::new();
    for p in &paginas {
        let unicas: HashSet<String> = p
            .lines()
            .map(normalizar)
            .filter(|l| l.chars().count() >= 10)
            .collect();
        for l in unicas {
            *vezes.entry(l).or_default() += 1;
        }
    }
    let com_texto = paginas.iter().filter(|p| !p.trim().is_empty()).count();
    let repetidas: HashSet<String> = vezes
        .into_iter()
        .filter(|&(_, n)| n >= 3 && n * 2 >= com_texto)
        .map(|(l, _)| l)
        .collect();
    paginas
        .iter()
        .enumerate()
        .flat_map(|(i, p)| p.lines().map(move |l| (i + 1, l)))
        .filter(|(_, l)| {
            let n = normalizar(l);
            !repetidas.contains(&n) && !PAGINA_RE.is_match(&n)
        })
        .collect()
}

/// O corpo: o que vem depois da última entrada do sumário (pontilhado e página), quando há sumário.
fn sem_sumario(linhas: Vec<(usize, &str)>) -> Vec<(usize, &str)> {
    match linhas.iter().rposition(|(_, l)| SUMARIO_RE.is_match(l)) {
        Some(k) => linhas[k + 1..].to_vec(),
        None => linhas,
    }
}

fn normalizar(l: &str) -> String {
    l.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// O número que abre a linha: "1." e "1)" → [1]; "1.1." → [1, 1]. Só vale o que está em [`Numeracao::proximos`], que
/// tem o nível da numeração do PDF: o capítulo "2. TÍTULO" de uma hierárquica e o "18.1." de uma corrida não casam.
fn numero(l: &str) -> Option<Vec<u32>> {
    NUM_RE.captures(l)?[1]
        .split('.')
        .map(|p| p.parse().ok())
        .collect()
}

/// A numeração do PDF: "N.M." em capítulos (hierárquica) ou "N." / "N)" corrida.
struct Numeracao {
    hierarquica: bool,
}

impl Numeracao {
    fn capitulo(&self, l: &str) -> Option<u32> {
        if !self.hierarquica {
            return None;
        }
        CAPITULO_RE.captures(l).and_then(|c| c[1].parse().ok())
    }

    /// Os números que podem vir depois de `ultimo`.
    fn proximos(&self, ultimo: Option<&[u32]>) -> Vec<Vec<u32>> {
        match (ultimo, self.hierarquica) {
            (None, true) => vec![vec![1, 1]],
            (None, false) => vec![vec![1]],
            (Some(u), true) => vec![vec![u[0], u[1] + 1], vec![u[0] + 1, 1]],
            (Some(u), false) => vec![vec![u[0] + 1]],
        }
    }

    fn capitulo_seguinte(&self, l: &str, ultimo: Option<&[u32]>) -> bool {
        self.capitulo(l) == Some(ultimo.map_or(1, |u| u[0] + 1))
    }

    /// Onde a pergunta aberta termina: a próxima pergunta, seção ou capítulo.
    fn fronteira(&self, l: &str, ultimo: Option<&[u32]>) -> bool {
        SECAO_RE.is_match(l)
            || self.capitulo_seguinte(l, ultimo)
            || numero(l).is_some_and(|n| self.proximos(ultimo).contains(&n))
    }
}

/// As linhas da resposta em parágrafos, sem o "R.:"/"R:" que abre a resposta: linha vazia fecha o parágrafo, e a linha seguinte junta-se à anterior com um
/// espaço (sem ele depois de um hífen colado — a URL partida, "https://dfe-" / "portal…"), a menos que abra um item de
/// lista. Na virada de página, a frase que não terminou continua no mesmo parágrafo.
fn paragrafos(linhas: &[(usize, &str)]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut aberto = false;
    let mut pagina_anterior = 0;
    for &(pagina, l) in linhas {
        let t = l.split_whitespace().collect::<Vec<_>>().join(" ");
        if t.is_empty() {
            aberto = false;
            continue;
        }
        let continua = !ITEM_RE.is_match(&t)
            && (aberto
                || pagina != pagina_anterior
                    && out
                        .last()
                        .is_some_and(|p| !p.ends_with(['.', ':', ';', '!', '?'])));
        let t = if out.is_empty() {
            RESPOSTA_RE.replace(&t, "").into_owned()
        } else {
            t
        };
        if t.is_empty() {
            continue;
        }
        match out.last_mut() {
            Some(p) if continua => {
                let hifen_colado = p
                    .strip_suffix('-')
                    .is_some_and(|antes| antes.ends_with(char::is_alphanumeric));
                if !hifen_colado {
                    p.push(' ');
                }
                p.push_str(&t);
            }
            _ => out.push(t),
        }
        aberto = true;
        pagina_anterior = pagina;
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // O `pdftotext -layout` dos PDFs reais de 07/11/2026.
    /// NFCom: 5 páginas, "Seção N:" e "N." corrida (a Seção 2 recomeça no 7), cabeçalho no topo.
    const NFCOM: &str = include_str!("../../tests/fixtures/rs-faqs-nfcom.txt");
    /// GES Bebidas: 32 páginas, sumário (sem a 5.10 e a 5.11), capítulos e "N.M.", "R.:", rodapé com o endereço.
    const BEBIDAS: &str = include_str!("../../tests/fixtures/rs-faqs-bebidas.txt");
    /// GES Comunicações: 16 páginas, o mesmo molde; nem toda resposta abre com "R.:".
    const COMUNICACOES: &str = include_str!("../../tests/fixtures/rs-faqs-comunicacoes.txt");
    /// Convênio ICMS 201/17: 8 páginas, "N)" recuado, sem seção; o 14 é a lista de links.
    const CONVENIO: &str = include_str!("../../tests/fixtures/rs-faqs-convenio-201-17.txt");
    /// Autorregularização: 6 páginas, sumário, "N." sem seção, "R:".
    const AUTORREGULARIZACAO: &str =
        include_str!("../../tests/fixtures/rs-faqs-autorregularizacao.txt");

    fn todas(ss: &[Secao]) -> Vec<&Pergunta> {
        ss.iter().flat_map(|s| &s.perguntas).collect()
    }

    fn achar<'a>(ss: &'a [Secao], prefixo: &str) -> &'a Pergunta {
        todas(ss)
            .into_iter()
            .find(|p| p.pergunta.starts_with(prefixo))
            .unwrap_or_else(|| panic!("sem a pergunta {prefixo:?}"))
    }

    #[test]
    fn bebidas_real() {
        let ss = secoes(BEBIDAS).unwrap();
        let contagem: Vec<usize> = ss.iter().map(|s| s.perguntas.len()).collect();
        assert_eq!(contagem, [6, 3, 6, 18, 26, 1, 12, 8, 3]);
        assert_eq!(
            ss[4].titulo,
            "5. BENEFÍCIO FISCAL ÀS MICROCERVEJARIAS GAÚCHAS"
        );
        // A 5.10 e a 5.11 estão no corpo, não no sumário.
        assert!(achar(&ss, "5.10. ").pergunta.ends_with('?'));
        achar(&ss, "5.11. ");
        // O "R.:" sai; o rodapé (endereço e página) não entra em resposta nenhuma.
        let p = achar(&ss, "1.1. ");
        assert!(
            p.resposta
                .starts_with("O Grupo Especializado Setorial de Bebidas")
        );
        assert!(
            todas(&ss)
                .iter()
                .all(|p| !p.resposta.contains("Siqueira Campos"))
        );
        // A 4.6 abre a página 12.
        let p = achar(&ss, "4.6. ");
        assert_eq!(p.pagina, 12);
        assert!(
            p.pergunta.ends_with(
                "Como irei calcular o valor do ICMS devido por substituição tributária?"
            )
        );
    }

    #[test]
    fn comunicacoes_real() {
        let ss = secoes(COMUNICACOES).unwrap();
        let contagem: Vec<usize> = ss.iter().map(|s| s.perguntas.len()).collect();
        assert_eq!(contagem, [10, 24, 2]);
        assert_eq!(ss[1].titulo, "2. CONVÊNIOS ICMS 115/03 E 201/17");
        // A linha seguinte da 1.8 começa por "22)" — não é pergunta nem fronteira.
        assert_eq!(
            achar(&ss, "1.8. ").pergunta,
            "1.8. Como faço para consultar a autenticidade das notas fiscais eletrônicas (modelo 21 ou 22) emitidas \
             por prestador de serviços de comunicação ou telecomunicação?"
        );
        // Resposta sem "R.:".
        assert!(
            achar(&ss, "1.3. ")
                .resposta
                .starts_with("As hipóteses de estorno")
        );
        assert!(
            todas(&ss)
                .iter()
                .all(|p| !p.resposta.contains("Seção de Coordenação dos GES"))
        );
    }

    #[test]
    fn convenio_201_real() {
        let ss = secoes(CONVENIO).unwrap();
        assert_eq!(ss.len(), 1);
        assert_eq!(ss[0].titulo, "");
        assert_eq!(ss[0].perguntas.len(), 13);
        // O 14 ("Legislação compilada:") é a lista de links: nem pergunta nem resto da 13.
        let p13 = &ss[0].perguntas[12];
        assert!(p13.pergunta.starts_with("13) Qual é a penalidade"));
        assert!(!p13.resposta.contains("Legislação compilada"));
        assert!(
            todas(&ss)
                .iter()
                .all(|p| !p.resposta.contains("Secretaria da Fazenda"))
        );
        assert_eq!(
            ss[0].perguntas[1].pergunta,
            "2) Quando ocorre a hipótese de dispensa de envio do arquivo e qual procedimento para enviar tal dispensa?"
        );
    }

    #[test]
    fn autorregularizacao_real() {
        let ss = secoes(AUTORREGULARIZACAO).unwrap();
        assert_eq!((ss.len(), ss[0].titulo.as_str()), (1, ""));
        assert_eq!(ss[0].perguntas.len(), 16);
        assert!(
            ss[0].perguntas[0]
                .resposta
                .starts_with("Trata-se de procedimento preventivo")
        );
        // A 5 continua na página seguinte, sem o rodapé no meio.
        let p5 = &ss[0].perguntas[4];
        assert!(p5.resposta.contains("2) Já na segunda situação"));
        assert!(!p5.resposta.contains("Av. Mauá"));
    }

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn o_pdf_real_da_nfcom() {
        let ss = secoes(NFCOM).unwrap();
        let titulos: Vec<&str> = ss.iter().map(|s| s.titulo.as_str()).collect();
        assert_eq!(
            titulos,
            [
                "Seção 1: Conceito, Obrigatoriedade e Natureza do Documento",
                "Seção 2: Faturamento, Cobrança e Tipos de Serviço",
                "Seção 3: Tipos de Faturamento e Fluxo de Emissão",
                "Seção 4: Correções e Substituição",
                "Seção 5: Escrituração Fiscal Digital (EFD-ICMS/IPI)",
            ]
        );
        let contagem: Vec<usize> = ss.iter().map(|s| s.perguntas.len()).collect();
        // A Seção 2 recomeça no 7: são 25 perguntas numeradas até a 24.
        assert_eq!(contagem, [7, 6, 5, 3, 4]);
        assert!(ss[1].perguntas[0].pergunta.starts_with("7. O valor total"));
        let todas: Vec<&Pergunta> = ss.iter().flat_map(|s| &s.perguntas).collect();
        assert!(todas.iter().all(|p| p.pergunta.ends_with('?')));
        assert!(
            todas
                .iter()
                .all(|p| !p.resposta.contains("Secretaria da Fazenda"))
        );
        // A pergunta em duas linhas é uma só.
        assert_eq!(
            ss[1].perguntas[2].pergunta,
            "9. Como tratar Serviços de Valor Adicionado (SVA) e outros serviços não sujeitos ao ICMS?"
        );
        // As páginas: a 1 e a 4 na 1ª, a 24 (sem recuo, na última) na 5ª.
        assert_eq!(
            (todas[0].pagina, todas[3].pagina, todas[24].pagina),
            (1, 1, 5)
        );
        assert!(
            todas[24]
                .pergunta
                .starts_with("24. Como a NFCom de Substituição")
        );
    }

    #[test]
    fn resposta_em_paragrafos_atravessando_a_pagina() {
        let ss = secoes(NFCOM).unwrap();
        let p2: Vec<&str> = ss[0].perguntas[1].resposta.lines().collect();
        assert_eq!(p2.len(), 3);
        assert!(p2[0].ends_with("conforme exemplo a seguir:"));
        // A URL partida no fim da linha volta inteira; o espaçamento justificado vira um espaço.
        assert_eq!(
            p2[1],
            "O acesso ao CCC pode ser realizado no seguinte link: https://dfe-portal.svrs.rs.gov.br/Nfe/Ccc"
        );
        // A 4 continua na página 2 no meio da frase, sem o cabeçalho: um parágrafo só.
        let p4 = &ss[0].perguntas[3].resposta;
        assert!(!p4.contains('\n'));
        assert!(p4.contains("a representação gráfica que pode ser impressa"));
        // A 17 termina a página 3; a resposta começa na 4.
        assert!(
            ss[2].perguntas[4]
                .resposta
                .starts_with("Não. A NFCom e o boleto/fatura")
        );
    }

    #[test]
    fn molde_mudado_e_erro() {
        assert!(erro(secoes("Só texto.\n")).contains("nenhuma pergunta"));
        assert!(
            erro(secoes("Seção 1: A\n\nSeção 2: B\n1. P?\n  R.\n"))
                .contains("Seção 1: A\" não tem")
        );
        // O item que não é pergunta sai; a seção que fica sem nenhuma é erro.
        assert!(
            erro(secoes("Seção 1: A\n1. Sem interrogação\n\n  R.\n")).contains("não tem pergunta")
        );
        assert!(erro(secoes("Seção 1: A\n1. P?\n\n2. Q?\n  R.\n")).contains("sem resposta"));
        // O número fora da sequência logo depois da seção é texto solto, não pergunta perdida em silêncio.
        assert!(erro(secoes("Seção 1: A\n3. P?\n  R.\n")).contains("fora de pergunta"));
    }

    #[test]
    fn so_o_hifen_colado_junta_sem_espaco() {
        let ss = secoes(
            "Seção 1: A\n1. P?\n  Veja https://dfe-\n  portal.rs e o item -\n  o próximo.\n",
        )
        .unwrap();
        assert_eq!(
            ss[0].perguntas[0].resposta,
            "Veja https://dfe-portal.rs e o item - o próximo."
        );
    }

    #[test]
    fn frase_terminada_na_virada_de_pagina_abre_paragrafo() {
        let ss = secoes("Seção 1: A\n1. P?\n  Primeira frase.\n\u{c}\n  Segunda.\n").unwrap();
        assert_eq!(ss[0].perguntas[0].resposta, "Primeira frase.\nSegunda.");
    }

    #[test]
    fn cabecalho_e_rodape_sao_as_linhas_repetidas_em_metade_das_paginas() {
        // 8 páginas: o rodapé em 5 sai, com o número de página; a linha longa em 3 (menos da metade) e a curta em 4
        // (menos de 10 caracteres) ficam.
        let texto = (1..=8)
            .map(|n| {
                let mut p = String::new();
                if n == 1 {
                    p += "1. P?\n  R.: Início.\n";
                }
                if n <= 3 {
                    p += "  Linha de três páginas.\n";
                }
                if (2..=5).contains(&n) {
                    p += "  Não.\n";
                }
                if n <= 5 {
                    p += "  Rodapé da Receita, Av. X\n";
                }
                p + &format!("{n}\n")
            })
            .collect::<Vec<_>>()
            .join("\u{c}");
        let r = &secoes(&texto).unwrap()[0].perguntas[0].resposta;
        assert!(r.starts_with("Início."));
        assert!(!r.contains("Rodapé") && !r.contains(char::is_numeric));
        assert_eq!(r.matches("Linha de três páginas.").count(), 3);
        assert_eq!(r.matches("Não.").count(), 4);
    }

    #[test]
    fn o_sumario_sai() {
        let ss =
            secoes("Sumário\n1. Primeira? ............ 2\n\u{c}1. Primeira?\nR.: Sim.\n").unwrap();
        assert_eq!(ss[0].perguntas.len(), 1);
        assert_eq!(ss[0].perguntas[0].resposta, "Sim.");
        assert_eq!(ss[0].perguntas[0].pagina, 2);
    }

    #[test]
    fn titulo_sem_interrogacao_para_no_r_e_na_proxima_pergunta() {
        // Sem o "R.:" como limite, o título levaria a resposta até o "?" dela; sem a fronteira, levaria a próxima
        // pergunta.
        let ss =
            secoes("1. Título\nR.: Sim, e então?\n2. Outra?\n  R.\n3. Mais um\n4. Última?\n  S.\n")
                .unwrap();
        let titulos: Vec<&str> = ss[0]
            .perguntas
            .iter()
            .map(|p| p.pergunta.as_str())
            .collect();
        assert_eq!(titulos, ["2. Outra?", "4. Última?"]);
    }

    #[test]
    fn item_de_lista_abre_paragrafo() {
        let ss = secoes(
            "1. P?\n  Os casos:\n  • um\n  continua;\n  b) dois\n  II – três\n  2 - quatro\n",
        )
        .unwrap();
        assert_eq!(
            ss[0].perguntas[0].resposta,
            "Os casos:\n• um continua;\nb) dois\nII – três\n2 - quatro"
        );
    }

    #[test]
    fn numero_fora_da_sequencia_dentro_da_resposta_e_da_resposta() {
        let ss = secoes("Seção 1: A\n1. P?\n  Itens:\n  3. o terceiro\n2. Q?\n  R.\n").unwrap();
        assert_eq!(ss[0].perguntas.len(), 2);
        assert_eq!(ss[0].perguntas[0].resposta, "Itens: 3. o terceiro");
    }
}
