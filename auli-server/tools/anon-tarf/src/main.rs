//! `anon-tarf` — roda o `auli-anon` sobre o `## corpo` dos acórdãos do TARF-RS e mede.
//!
//! **One-shot, fora do pipeline.** Não escreve em `data/rs/docs/tarf/` nem em pack nenhum: produz
//! uma árvore paralela anonimizada e quatro relatórios, todos em diretório novo.
//!
//! O acórdão é um **corpus auto-rotulado**: as linhas `RECORRENTE(S):` / `RECORRIDO(S):` do corpo
//! *dizem* qual é a entidade. Daí sai um gold set de milhares de nomes reais sem uma linha de
//! anotação manual — e, nos 56% em que a própria fonte já substituiu o nome por `(...)`, sai o
//! **controle negativo**: ali o bloco de identificação está provadamente sem nome de parte, então
//! detecção naquele bloco é falso positivo **com rótulo**.
//!
//! O que ele mede, e por que assim:
//!
//! - **recall por coorte, nunca agregado** — com sufixo societário (a Fase 4 deve pegar) e sem
//!   sufixo (`main` não tem nada) são populações opostas; a média das duas não descreve nenhuma;
//! - **duas taxas por documento** — mascarado *na linha rotulada* × mascarado *fora dela*. A
//!   segunda mede capacidade; a primeira é o teto do que a anonimização estrutural pegaria de graça;
//! - **mineração e medição em metades disjuntas** do corpus, por hash do `doc_path` (não por ordem
//!   alfabética, que correlaciona com ano e com layout).
//!
//! Uso: `anon-tarf [<raiz-do-data>] [<saída>]` — padrões `../data` e `../data/anon-tarf`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{Context, Result};
use auli_anon::Anonimizador;
use auli_contract::{arquivo, mddoc};
use regex::Regex;

/// Rótulos do bloco de identificação. As duas eras do TARF põem o valor na mesma linha (até 2015)
/// ou depois de uma quebra (de 2020 em diante); as formas plurais sem parênteses (`RECORRENTES:`)
/// respondem por 463 documentos e ficaram de fora da primeira apuração — omiti-las produziria um
/// gold set enviesado para uma época, que é o pior modo de falha aqui.
static RE_RECORRENTE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bRECORRENTE\s?S?\s?\(?S?\)?\s*:").unwrap());
static RE_BLOCO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(RECURSO|RECORRENTE|RECORRID[AO]|PROCESSO|PROCE\s?SSO|AUTO DE LANÇAMENTO|DECISÃO DE 1ª INSTÂNCIA|PROCEDÊNCIA)\s?S?\s?\(?S?\)?\s*:")
        .unwrap()
});
/// Sufixo societário ou termo de segmento óbvio — o porteiro que separa as duas coortes.
static RE_PJ: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(ltda|s/?a\b|s\.a\b|eireli|me\b|epp\b|cia\b|com[ée]rcio|ind[úu]stria|transportes?|distribuidora|supermercado|servi[çc]os)")
        .unwrap()
});
/// Partes que não são PII: a Fazenda é sempre uma das partes, e os anafóricos referem a outra linha.
static RE_NAO_PII: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(fazenda estadual|as mesmas|o mesmo|a mesma|os mesmos)").unwrap()
});

/// Uma detecção, achatada para `String`/`usize`/`f64`. **Nenhum tipo do `cloakrs` é nomeado aqui**
/// — é o teste da fronteira de reuso: se este achatamento não desse, seria o achado da TAREFA.
struct Deteccao {
    classe: String,
    reconhecedor: String,
    confianca: f64,
    ini: usize,
    fim: usize,
    original: String,
}

/// O que a linha rotulada de um acórdão declara.
struct Rotulo {
    /// Span do valor no `## corpo` — a "linha rotulada", para separar as duas taxas.
    valor: (usize, usize),
    /// Fim do bloco de identificação: até onde o controle negativo vale.
    fim_bloco: usize,
    /// Partes privadas declaradas (sem Fazenda nem anafórico). Vazio quando saneado na fonte.
    entidades: Vec<String>,
    /// A fonte já substituiu o nome por `(...)`.
    saneado: bool,
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let raiz = PathBuf::from(args.next().unwrap_or_else(|| "../data".into()));
    let saida = PathBuf::from(
        args.next()
            .unwrap_or_else(|| raiz.join("anon-tarf").to_string_lossy().into_owned()),
    );
    let docs = raiz.join("rs/docs/tarf");
    std::fs::create_dir_all(saida.join("docs")).context("criando a árvore de saída")?;

    let anon = Anonimizador::novo().map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut arquivos: Vec<PathBuf> = std::fs::read_dir(&docs)
        .with_context(|| format!("lendo {}", docs.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    arquivos.sort();
    println!("📄 {} acórdãos em {}", arquivos.len(), docs.display());

    let mut m = Medidas::default();
    for p in &arquivos {
        // Um documento por vez: ler, anonimizar, escrever, soltar. Memória constante, como o resto
        // do pipeline — são ~22 mil acórdãos inteiros.
        let doc_path = format!("docs/tarf/{}", p.file_name().unwrap().to_string_lossy());
        let texto = std::fs::read_to_string(p)?;
        let (_, _, corpo) = match mddoc::parse_doc(&texto) {
            Ok(t) => t,
            Err(e) => {
                m.parse_falhou.push(format!("{doc_path}\t{e}"));
                continue;
            }
        };
        let a = anon
            .anonimizar(&corpo)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        // Round-trip em escala: o teste mais severo que essa propriedade já recebeu. Uma falha aqui
        // é defeito no crate encontrado em dado real, então o relatório traz o ponto exato da
        // divergência — sem ele, `doc_path` sozinho manda alguém procurar agulha em 30 KB.
        let voltou = anon.restaurar(&a.texto, &a.mapping);
        if voltou != corpo {
            m.round_trip_falhou.push(format!(
                "{doc_path}\n{}",
                primeira_diferenca(&corpo, &voltou)
            ));
        }
        let dets: Vec<Deteccao> = a
            .mapping
            .entries
            .iter()
            .map(|e| Deteccao {
                classe: classe_do_placeholder(&e.placeholder),
                reconhecedor: e.recognizer_id.clone(),
                confianca: e.confidence,
                ini: e.span_start,
                fim: e.span_end,
                original: e.original.clone(),
            })
            .collect();
        arquivo::escrever_atomico(&saida.join("docs").join(p.file_name().unwrap()), &a.texto)?;
        m.contabilizar(&doc_path, &corpo, &dets, extrair_rotulo(&corpo));
    }

    m.escrever(&saida)?;
    println!("✅ relatórios em {}", saida.display());
    Ok(())
}

/// `[NOME_3]` → `NOME`. A classe vem do placeholder porque o `auli-anon` **não reexporta** o
/// `EntityType` do `cloakrs`: ler o enum exigiria depender da biblioteca de baixo direto. É o único
/// atrito encontrado na fronteira, e ele tem contorno de uma linha — por isso não virou pedido de
/// mudança na biblioteca.
fn classe_do_placeholder(p: &str) -> String {
    let s = p.trim_start_matches('[').trim_end_matches(']');
    s.rsplit_once('_').map_or(s, |(c, _)| c).to_string()
}

/// Onde o texto restaurado passou a divergir do original, com contexto dos dois lados.
fn primeira_diferenca(orig: &str, volta: &str) -> String {
    let i = orig
        .bytes()
        .zip(volta.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| orig.len().min(volta.len()));
    let janela = |s: &str| {
        let ini = s.floor_char_boundary(i.saturating_sub(50));
        let fim = s.ceil_char_boundary((i + 50).min(s.len()));
        s[ini..fim].replace('\n', "⏎")
    };
    format!(
        "    byte {i} (original {} bytes, restaurado {})\n    orig : {}\n    volta: {}",
        orig.len(),
        volta.len(),
        janela(orig),
        janela(volta)
    )
}

/// Extrai o que a linha `RECORRENTE` declara, e até onde vai o bloco de identificação.
fn extrair_rotulo(corpo: &str) -> Option<Rotulo> {
    let m = RE_RECORRENTE.find(corpo)?;
    // O valor é a primeira linha não vazia depois dos dois-pontos — cobre as duas eras de layout.
    let resto = &corpo[m.end()..];
    let (rel_ini, linha) = resto
        .char_indices()
        .filter(|(i, _)| *i == 0 || resto.as_bytes()[i - 1] == b'\n')
        .map(|(i, _)| (i, resto[i..].split('\n').next().unwrap_or("")))
        .find(|(_, l)| !l.trim().is_empty())?;
    let bruto = linha.trim();
    let ini = m.end() + rel_ini + (linha.len() - linha.trim_start().len());
    let fim_bloco = RE_BLOCO
        .find_iter(corpo)
        .map(|b| b.end())
        .max()
        .unwrap_or(m.end())
        .max(ini + bruto.len());
    let saneado = bruto.contains("(...)") || bruto.contains("(…)");
    // 11,2% dos valores trazem duas partes unidas por ` E `. Quebrar aqui é o que permite medir por
    // entidade em vez de por linha — e dá, de graça, um conjunto de teste real e rotulado para a
    // falha nº 2 da Fase 5 (o conector `e` encadeando itens de lista).
    let entidades = bruto
        .split(" E ")
        .flat_map(|p| p.split(" e "))
        .map(str::trim)
        .filter(|p| !p.is_empty() && !RE_NAO_PII.is_match(p) && !p.contains("(...)"))
        .map(str::to_string)
        .collect();
    Some(Rotulo {
        valor: (ini, ini + bruto.len()),
        fim_bloco,
        entidades,
        saneado,
    })
}

/// Metade do corpus a que o documento pertence. Hash FNV-1a do `doc_path`, **não** ordem
/// alfabética: a ordem correlaciona com ano e, por consequência, com layout — minerar numa era e
/// medir na outra é a mesma armadilha de ajustar ao conjunto de teste, só que disfarçada.
fn metade(doc_path: &str) -> usize {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in doc_path.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    (h % 2) as usize
}

#[derive(Default)]
struct Medidas {
    docs: usize,
    parse_falhou: Vec<String>,
    round_trip_falhou: Vec<String>,
    /// classe → (detecções, documentos que a tiveram)
    por_classe: BTreeMap<String, (usize, usize)>,
    /// reconhecedor → (detecções, soma das confianças, mínima, máxima)
    por_reconhecedor: BTreeMap<String, (usize, f64, f64, f64)>,
    /// coorte × posição → (esperadas, cobertas)
    recall: BTreeMap<(&'static str, &'static str), (usize, usize)>,
    /// Uma detecção cobrindo duas entidades esperadas: acerto para as duas, contado à parte.
    fundidas: usize,
    saneados: usize,
    /// Falso positivo COM rótulo: detecção de nome/razão social no bloco de um doc saneado.
    fp_rotulado: BTreeMap<String, usize>,
    /// Miss no gold set: entidade declarada que nenhuma detecção cobriu, fora da linha rotulada.
    misses: Vec<String>,
    /// Token → nº de valores DISTINTOS que o contêm (metade de mineração). Termo de segmento é o
    /// que aparece em muitos nomes diferentes; nome próprio aparece em poucos.
    tokens: HashMap<String, BTreeSet<String>>,
    ano_saneado: BTreeMap<String, (usize, usize)>,
}

impl Medidas {
    fn contabilizar(
        &mut self,
        doc_path: &str,
        corpo: &str,
        dets: &[Deteccao],
        rot: Option<Rotulo>,
    ) {
        self.docs += 1;
        let mut classes_vistas = BTreeSet::new();
        for d in dets {
            let e = self.por_classe.entry(d.classe.clone()).or_default();
            e.0 += 1;
            if classes_vistas.insert(d.classe.clone()) {
                e.1 += 1;
            }
            let r = self
                .por_reconhecedor
                .entry(d.reconhecedor.clone())
                .or_insert((0, 0.0, f64::MAX, f64::MIN));
            r.0 += 1;
            r.1 += d.confianca;
            r.2 = r.2.min(d.confianca);
            r.3 = r.3.max(d.confianca);
        }
        let Some(rot) = rot else { return };
        // O ano é o sufixo de dois dígitos do slug (`acordao-001-25.md`). Nem todo nome termina
        // assim — há um `-fisico.md` no acervo —, e um sufixo não numérico entra como `??` em vez de
        // virar um ano inventado.
        let ano = doc_path
            .rsplit('-')
            .next()
            .unwrap_or("")
            .trim_end_matches(".md");
        let ano = if ano.len() == 2 && ano.bytes().all(|b| b.is_ascii_digit()) {
            format!("20{ano}")
        } else {
            "??".to_string()
        };
        let a = self.ano_saneado.entry(ano).or_default();
        a.1 += 1;

        if rot.saneado {
            self.saneados += 1;
            a.0 += 1;
            // Controle negativo: o bloco não tem nome de parte, por declaração da fonte. Qualquer
            // NOME/RAZAO_SOCIAL ali é FP **com rótulo** — o que torna automática a metade da
            // medição que a TAREFA dava como necessariamente manual.
            for d in dets.iter().filter(|d| d.ini < rot.fim_bloco) {
                if d.classe.contains("NOME") || d.classe.contains("RAZAO") {
                    *self.fp_rotulado.entry(d.original.clone()).or_default() += 1;
                }
            }
            return;
        }

        for ent in &rot.entidades {
            let coorte = if RE_PJ.is_match(ent) {
                "pessoa jurídica"
            } else {
                "sem sufixo"
            };
            if metade(doc_path) == 0 {
                for t in ent.split_whitespace().filter(|t| t.chars().count() > 3) {
                    self.tokens
                        .entry(t.to_lowercase())
                        .or_default()
                        .insert(ent.clone());
                }
            }
            // Cada ocorrência do valor no corpo é um caso: a que cai na linha rotulada mede o teto
            // da anonimização estrutural; as de fora medem capacidade do reconhecedor.
            for (pos, _) in corpo.match_indices(ent.as_str()) {
                let na_linha = pos >= rot.valor.0 && pos < rot.valor.1;
                let posicao = if na_linha {
                    "linha rotulada"
                } else {
                    "texto livre"
                };
                let coberta = dets.iter().any(|d| d.ini < pos + ent.len() && pos < d.fim);
                let r = self.recall.entry((coorte, posicao)).or_default();
                r.0 += 1;
                r.1 += usize::from(coberta);
                if !coberta && !na_linha {
                    self.misses.push(format!("{doc_path}\tmiss\t{ent}"));
                }
            }
        }
        // Detecção única cobrindo duas entidades declaradas: acerto para as duas, e sinalizada.
        if rot.entidades.len() > 1 {
            self.fundidas += dets
                .iter()
                .filter(|d| {
                    rot.entidades
                        .iter()
                        .filter(|e| {
                            corpo
                                .get(d.ini..d.fim)
                                .is_some_and(|s| s.contains(e.as_str()))
                        })
                        .count()
                        > 1
                })
                .count();
        }
    }

    fn escrever(&self, saida: &Path) -> Result<()> {
        let mut r = String::new();
        writeln!(r, "RELATÓRIO — anon-tarf ({} acórdãos)\n", self.docs)?;
        writeln!(r, "ROUND-TRIP (anonimizar → restaurar == original)")?;
        writeln!(
            r,
            "  falhas: {} de {}\n",
            self.round_trip_falhou.len(),
            self.docs
        )?;
        for f in self.round_trip_falhou.iter().take(20) {
            writeln!(r, "    {f}")?;
        }
        writeln!(r, "DETECÇÕES POR CLASSE")?;
        for (c, (n, d)) in &self.por_classe {
            writeln!(r, "  {c:<16} {n:>7} detecções em {d:>6} documentos")?;
        }
        writeln!(
            r,
            "\nDETECÇÕES POR RECONHECEDOR (confiança média / mín / máx)"
        )?;
        for (k, (n, soma, min, max)) in &self.por_reconhecedor {
            writeln!(
                r,
                "  {k:<34} {n:>7}   {:.2} / {min:.2} / {max:.2}",
                soma / *n as f64
            )?;
        }
        writeln!(
            r,
            "\nRECALL POR COORTE E POSIÇÃO (nunca agregado — as coortes são opostas)"
        )?;
        for ((coorte, pos), (esp, cob)) in &self.recall {
            let pct = if *esp > 0 {
                100.0 * *cob as f64 / *esp as f64
            } else {
                0.0
            };
            writeln!(
                r,
                "  {coorte:<16} {pos:<16} {cob:>6}/{esp:<6} = {pct:>5.1}%"
            )?;
        }
        writeln!(
            r,
            "\n  detecções fundidas (uma cobrindo duas entidades): {}",
            self.fundidas
        )?;
        writeln!(
            r,
            "\nCONTROLE NEGATIVO — {} documentos saneados na fonte; FP com rótulo no bloco: {}",
            self.saneados,
            self.fp_rotulado.values().sum::<usize>()
        )?;
        for (t, n) in ordenar(&self.fp_rotulado).iter().take(25) {
            writeln!(r, "  {n:>5}×  {t}")?;
        }
        writeln!(r, "\nSANEADO POR ANO (saneados / total)")?;
        for (ano, (s, t)) in &self.ano_saneado {
            writeln!(
                r,
                "  {ano:<6}{s:>6}/{t:<6} = {:>5.1}%",
                100.0 * *s as f64 / *t as f64
            )?;
        }
        if !self.parse_falhou.is_empty() {
            writeln!(r, "\nPARSE FALHOU: {}", self.parse_falhou.len())?;
        }
        arquivo::escrever_atomico(&saida.join("relatorio.txt"), &r)?;

        // Termos de segmento: token que aparece em MUITOS nomes distintos. Minerados só na metade 0;
        // o recall acima é medido nas duas, e o relatório diz qual metade produziu cada número.
        let mut segs: Vec<(usize, &String)> = self
            .tokens
            .iter()
            .map(|(t, v)| (v.len(), t))
            .filter(|(n, _)| *n >= 5)
            .collect();
        segs.sort_by_key(|(n, _)| std::cmp::Reverse(*n));
        let mut s = String::from("# termo\tnomes distintos (metade 0, mineração)\n");
        for (n, t) in &segs {
            writeln!(s, "{t}\t{n}")?;
        }
        arquivo::escrever_atomico(&saida.join("dicionario-segmentos.txt"), &s)?;

        // Stop-list do domínio: o que o anonimizador mascarou nos blocos provadamente sem nome.
        // Semeada com as duas armadilhas mais frequentes do acervo, que são rotuladas por definição.
        let mut sl = String::from("FAZENDA ESTADUAL\nAS MESMAS\n");
        for (t, n) in ordenar(&self.fp_rotulado) {
            writeln!(sl, "{t}\t{n}")?;
        }
        arquivo::escrever_atomico(&saida.join("stoplist-dominio.txt"), &sl)?;

        // Discordâncias: é isto que vai para revisão humana, não uma amostra aleatória. O rótulo
        // já decidiu o resto.
        let mut d = String::from("# doc_path\ttipo\ttexto\n");
        for l in &self.misses {
            writeln!(d, "{l}")?;
        }
        arquivo::escrever_atomico(&saida.join("discordancias.tsv"), &d)?;
        Ok(())
    }
}

fn ordenar(m: &BTreeMap<String, usize>) -> Vec<(&String, usize)> {
    let mut v: Vec<(&String, usize)> = m.iter().map(|(k, n)| (k, *n)).collect();
    v.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    v
}
