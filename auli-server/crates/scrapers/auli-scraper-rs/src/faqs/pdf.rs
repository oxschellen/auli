// As FAQs que o portal publica em PDF (D-RSFAQ-7): o item do menu redireciona (303) para o arquivo, e a caminhada lê o
// texto (`kit::pdf`, `pdftotext -layout`) em vez do HTML. Hoje só o `/faq-nfcom`, as "Perguntas e Respostas sobre a
// NFCom" do GES Comunicações (nov/2025). O "Manual para Declaração de ITCD" também redireciona para um PDF, mas é
// manual, não perguntas, e fica fora (decisão do mantenedor, 07/11/2026).
//
// O molde: "Seção N: Título" abre seção; a pergunta é a linha "N. …" com o número seguinte e vai até o "?" (pode ter
// duas linhas); a resposta, até a próxima pergunta ou seção. A numeração corre pelo documento inteiro, e uma seção pode
// começar repetindo o último número (a 2 recomeça no 7) — aviso. O cabeçalho de página ("Estado do Rio Grande do Sul /
// Secretaria da Fazenda / Receita Estadual") sai; a resposta que a quebra de página corta no meio da frase continua no
// mesmo parágrafo.

use std::sync::LazyLock;

use regex::Regex;

use crate::errors::Result;

static SECAO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(Seção \d+:\s*\S.*?)\s*$").unwrap());
static NUM_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(\d+)\.\s+\S").unwrap());
/// O cabeçalho que o Word repete no topo de cada página.
const CABECALHO: [&str; 3] = [
    "Estado do Rio Grande do Sul",
    "Secretaria da Fazenda",
    "Receita Estadual",
];

pub struct Secao {
    pub titulo: String,
    pub perguntas: Vec<Pergunta>,
}

pub struct Pergunta {
    pub pergunta: String,
    pub resposta: String,
    /// A página do PDF em que a pergunta começa (o link leva `#page=N`).
    pub pagina: usize,
}

/// As seções e perguntas do texto do PDF. Erro quando o molde muda: nenhuma seção, seção sem pergunta, pergunta que não
/// termina em "?", pergunta sem resposta, ou texto fora de pergunta depois da 1ª seção.
pub fn secoes(texto: &str) -> Result<Vec<Secao>> {
    let linhas: Vec<(usize, &str)> = texto
        .split('\u{c}')
        .enumerate()
        .flat_map(|(i, p)| p.lines().map(move |l| (i + 1, l)))
        .filter(|(_, l)| !CABECALHO.contains(&l.trim()))
        .collect();
    let mut secoes: Vec<Secao> = Vec::new();
    let mut ultimo = 0;
    let mut i = 0;
    while i < linhas.len() {
        let (pagina, l) = linhas[i];
        if let Some(c) = SECAO_RE.captures(l) {
            secoes.push(Secao {
                titulo: c[1].split_whitespace().collect::<Vec<_>>().join(" "),
                perguntas: Vec::new(),
            });
            i += 1;
            continue;
        }
        // Antes da 1ª seção: o título do documento.
        let Some(secao) = secoes.last_mut() else {
            i += 1;
            continue;
        };
        let n = match numero(l) {
            Some(n) if n == ultimo + 1 => n,
            Some(n) if n == ultimo && secao.perguntas.is_empty() => {
                println!(
                    "RS FAQs: aviso — PDF: a \"{}\" recomeça no número {n}",
                    secao.titulo
                );
                n
            }
            _ if l.trim().is_empty() => {
                i += 1;
                continue;
            }
            _ => return Err(format!("PDF: texto fora de pergunta: {:?}", l.trim()).into()),
        };
        ultimo = n;
        let mut pergunta = vec![l.trim()];
        while !pergunta.last().is_some_and(|p| p.ends_with('?')) {
            i += 1;
            match linhas.get(i) {
                Some((_, l)) if !l.trim().is_empty() && numero(l).is_none() => {
                    pergunta.push(l.trim())
                }
                _ => return Err(format!("PDF: a pergunta {n} não termina em \"?\"").into()),
            }
        }
        i += 1;
        let inicio = i;
        while i < linhas.len()
            && !SECAO_RE.is_match(linhas[i].1)
            && numero(linhas[i].1) != Some(ultimo + 1)
        {
            i += 1;
        }
        let resposta = paragrafos(&linhas[inicio..i]);
        if resposta.is_empty() {
            return Err(format!("PDF: a pergunta {n} ficou sem resposta").into());
        }
        secao.perguntas.push(Pergunta {
            pergunta: pergunta
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
            resposta,
            pagina,
        });
    }
    if secoes.is_empty() {
        return Err("PDF: nenhuma \"Seção N:\" no texto".into());
    }
    if let Some(s) = secoes.iter().find(|s| s.perguntas.is_empty()) {
        return Err(format!("PDF: a \"{}\" não tem pergunta", s.titulo).into());
    }
    Ok(secoes)
}

fn numero(l: &str) -> Option<u32> {
    NUM_RE.captures(l).and_then(|c| c[1].parse().ok())
}

/// As linhas da resposta em parágrafos: linha vazia fecha o parágrafo, e a linha seguinte junta-se à anterior com um
/// espaço (sem ele depois de um hífen colado — a URL partida, "https://dfe-" / "portal…"). Na virada de página, a frase
/// que não terminou continua no mesmo parágrafo.
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
        let continua = aberto
            || (pagina != pagina_anterior
                && out
                    .last()
                    .is_some_and(|p| !p.ends_with(['.', ':', ';', '!', '?'])));
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

    /// O `pdftotext -layout` do PDF real de 07/11/2026 (5 páginas, 5 seções).
    const NFCOM: &str = include_str!("../../tests/fixtures/rs-faqs-nfcom.txt");

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
        assert!(erro(secoes("1. Sem seção?\n   Sim.\n")).contains("nenhuma"));
        assert!(
            erro(secoes("Seção 1: A\n\nSeção 2: B\n1. P?\n  R.\n"))
                .contains("Seção 1: A\" não tem")
        );
        assert!(erro(secoes("Seção 1: A\n1. Sem interrogação\n\n  R.\n")).contains("não termina"));
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
        let ss =
            secoes("Seção 1: A\n1. P?\n  Primeira frase.\n\u{c}Receita Estadual\n\n  Segunda.\n")
                .unwrap();
        assert_eq!(ss[0].perguntas[0].resposta, "Primeira frase.\nSegunda.");
    }

    #[test]
    fn numero_fora_da_sequencia_dentro_da_resposta_e_da_resposta() {
        let ss = secoes("Seção 1: A\n1. P?\n  Itens:\n  3. o terceiro\n2. Q?\n  R.\n").unwrap();
        assert_eq!(ss[0].perguntas.len(), 2);
        assert_eq!(ss[0].perguntas[0].resposta, "Itens: 3. o terceiro");
    }
}
