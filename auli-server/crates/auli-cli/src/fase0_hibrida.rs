//! **Fase 0 do `docs/Estudo-rag-dense.md`** — a medição da busca híbrida (denso + esparso do
//! BGE-M3) no acervo real. Ferramenta de laboratório: só existe em `cfg(test)`, não entra no binário,
//! não escreve em `data/` e não muda nada do `update`, do motor ou do servidor.
//!
//! **Por que um módulo de teste dentro do crate**, e não um teste em `tests/` como o `ab_faq_pr.rs`:
//! a medição tem de ser feita sobre a key que a produção indexou, e a key só nasce em
//! [`update::preparar`], que é `pub(crate)`. Abrir a visibilidade faria do `preparar` API pública do
//! crate por causa de uma ferramenta de medição; daqui ele é alcançável sem tocar no `update`. E a
//! key não é recomposta por outro caminho: cada documento tem o `hash_hex(key)` conferido contra o
//! `key_hash` do registro no pack — qualquer diferença derruba a rodada.
//!
//! Dois passos, cada um um teste `#[ignore]`:
//!
//! 1. [`fase0_calcular`] — esparso das keys, gravado em `AULI_F0_SAIDA/<id>-<kind>.sparse.json`.
//!    Rodar o binário de teste sob `/usr/bin/time -v` dá o tempo e o pico de RSS (M5).
//! 2. [`fase0_medir`] — M1 a M6 e a impressão lado a lado, lendo os packs e os esparsos do passo 1.
//!
//! Variáveis: `AULI_F0_ENTIDADE` (diretório da entidade **na cópia de laboratório**, com `docs/` e
//! `packs/`; o id é o nome do diretório), `AULI_F0_SAIDA`, `AULI_F0_KINDS` (só no passo 1; padrão:
//! as cinco), `EMBED_CACHE_DIR`, `EMBED_THREADS` (padrão 16, o da produção) e `AULI_F0_MUTACAO=1`
//! (a verificação por mutação: embaralha a associação documento ↔ vetor; o M1 e o M2 têm de ir ao
//! chão).
//!
//! ```text
//! AULI_F0_ENTIDADE=<lab>/rs AULI_F0_SAIDA=<lab>/saida EMBED_CACHE_DIR=../../models \
//!   cargo test -p auli-cli --release --lib fase0_hibrida::fase0_medir -- --ignored --nocapture
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use auli_contract::{Documento, Kind};
use auli_core::embed::Embedder;
use auli_core::manifest::hash_hex;
use auli_core::sparse::{EmbedderEsparso, VetorEsparso};
use vector_store::{CollectionData, cosine_distance, read_collection_file};

use crate::update;

/// A fusão principal (D-HIB-3): RRF com `k = 60` e as 100 primeiras posições de cada lista.
const K_RRF: f64 = 60.0;
const PROF_RRF: usize = 100;
/// A grade de sensibilidade da fusão: os dois parâmetros que o estudo manda medir.
const GRADE_K: [f64; 3] = [20.0, 60.0, 100.0];
const GRADE_PROF: [usize; 4] = [10, 20, 50, 100];

/// Os termos do M3/M4. O mantenedor não forneceu lista (14/11/2026), então fica o "conluio" do
/// estudo de agosto; "crédito presumido" é o exemplo de ementa do próprio estudo e cobre o acento,
/// que o "conluio" não tem.
const TERMOS: [&str; 2] = ["conluio", "crédito presumido"];

struct Cfg {
    entidade: String,
    dir: PathBuf,
    saida: PathBuf,
    cache: PathBuf,
    threads: usize,
}

impl Cfg {
    fn do_ambiente() -> Option<Self> {
        let Ok(dir) = std::env::var("AULI_F0_ENTIDADE") else {
            eprintln!("AULI_F0_ENTIDADE não setada — pulando");
            return None;
        };
        let dir = PathBuf::from(dir);
        let entidade = dir
            .file_name()
            .expect("AULI_F0_ENTIDADE sem nome de diretório")
            .to_string_lossy()
            .into_owned();
        let saida = PathBuf::from(std::env::var("AULI_F0_SAIDA").expect("AULI_F0_SAIDA"));
        let cache = std::env::var("EMBED_CACHE_DIR").unwrap_or_else(|_| "../../models".into());
        let threads = std::env::var("EMBED_THREADS")
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(16);
        Some(Self {
            entidade,
            dir,
            saida,
            cache: cache.into(),
            threads,
        })
    }

    fn arquivo_esparso(&self, kind: Kind) -> PathBuf {
        self.saida
            .join(format!("{}-{}.sparse.json", self.entidade, kind.as_str()))
    }

    fn arquivo_pack(&self, kind: Kind) -> PathBuf {
        self.dir
            .join("packs")
            .join(format!("{}-{}.json", self.entidade, kind.as_str()))
    }
}

/// Uma coleção como a produção a indexou: os documentos na ordem do `preparar` (que é a ordem de
/// `id`) e, alinhado a eles, o vetor denso **do pack**.
struct Colecao {
    kind: Kind,
    docs: Vec<Documento>,
    densos: Vec<Vec<f32>>,
}

/// Lê a árvore pelo `preparar` e casa cada documento com o registro do pack, **conferindo a key**.
fn carregar(cfg: &Cfg, kind: Kind) -> Option<Colecao> {
    let docs = update::preparar(&cfg.entidade, &cfg.dir.join("docs"), kind)
        .unwrap_or_else(|e| panic!("preparar {kind:?}: {e:?}"))?;
    let pack: CollectionData<serde::de::IgnoredAny> =
        read_collection_file(cfg.arquivo_pack(kind)).expect("pack");
    let mut por_id: HashMap<String, _> = pack
        .records
        .into_iter()
        .map(|r| (r.id.clone(), r))
        .collect();
    let mut densos = Vec::with_capacity(docs.len());
    for d in &docs {
        let id = d.doc_path();
        let r = por_id
            .remove(&id)
            .unwrap_or_else(|| panic!("`{id}` está na árvore e não está no pack"));
        let h = hash_hex(d.text_to_embed.as_bytes());
        assert_eq!(
            r.key_hash.as_deref(),
            Some(h.as_str()),
            "`{id}`: a key do `preparar` não é a que o pack indexou — a medição não estaria sobre \
             o que a produção vê"
        );
        densos.push(r.embedding);
    }
    assert!(
        por_id.is_empty(),
        "{kind:?}: {} registros do pack sem documento na árvore",
        por_id.len()
    );
    println!(
        "✔ {kind:?}: {} documentos, key_hash conferido em todos",
        docs.len()
    );
    Some(Colecao { kind, docs, densos })
}

fn ler_esparsos(cfg: &Cfg, c: &Colecao) -> Vec<VetorEsparso> {
    let caminho = cfg.arquivo_esparso(c.kind);
    let lidos: Vec<(String, VetorEsparso)> = serde_json::from_slice(
        &std::fs::read(&caminho).unwrap_or_else(|e| panic!("{}: {e}", caminho.display())),
    )
    .expect("esparso do laboratório");
    assert_eq!(
        lidos.len(),
        c.docs.len(),
        "{:?}: contagem do esparso",
        c.kind
    );
    lidos
        .into_iter()
        .zip(&c.docs)
        .map(|((id, v), d)| {
            assert_eq!(id, d.doc_path(), "{:?}: esparso fora de ordem", c.kind);
            v
        })
        .collect()
}

/// `VmRSS` e `VmHWM` do processo, em GB.
fn memoria() -> (f64, f64) {
    let status = std::fs::read_to_string("/proc/self/status").expect("/proc/self/status");
    let campo = |nome: &str| {
        status
            .lines()
            .find(|l| l.starts_with(nome))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|kb| kb.parse::<f64>().ok())
            .map(|kb| kb / 1024.0 / 1024.0)
            .unwrap_or(f64::NAN)
    };
    (campo("VmRSS:"), campo("VmHWM:"))
}

/// Passo 1: o esparso das keys, gravado no laboratório.
#[test]
#[ignore = "Fase 0 do Estudo-rag-dense: precisa da cópia de laboratório e do modelo"]
fn fase0_calcular() {
    let Some(cfg) = Cfg::do_ambiente() else {
        return;
    };
    let kinds: Vec<Kind> = match std::env::var("AULI_F0_KINDS") {
        Ok(lista) => lista
            .split(',')
            .map(|k| {
                *Kind::TODOS
                    .iter()
                    .find(|x| x.as_str() == k.trim())
                    .unwrap_or_else(|| panic!("kind desconhecido: {k}"))
            })
            .collect(),
        Err(_) => Kind::TODOS.to_vec(),
    };
    std::fs::create_dir_all(&cfg.saida).expect("AULI_F0_SAIDA");
    let esparso = EmbedderEsparso::new(cfg.cache.clone(), cfg.threads).expect("esparso");
    let (mut total, mut tempo) = (0usize, Duration::ZERO);
    for kind in kinds {
        let Some(c) = carregar(&cfg, kind) else {
            println!("— {kind:?}: a entidade não tem esta coleção");
            continue;
        };
        let keys: Vec<String> = c.docs.iter().map(|d| d.text_to_embed.clone()).collect();
        let t = Instant::now();
        let vetores = esparso.embed_sparse(keys).expect("embed_sparse");
        let dt = t.elapsed();
        let arquivo: Vec<(String, VetorEsparso)> =
            c.docs.iter().map(|d| d.doc_path()).zip(vetores).collect();
        std::fs::write(
            cfg.arquivo_esparso(kind),
            serde_json::to_vec(&arquivo).expect("json"),
        )
        .expect("gravar o esparso");
        println!(
            "⏱ {kind:?}: esparso de {} keys em {:.1} s",
            c.docs.len(),
            dt.as_secs_f64()
        );
        total += c.docs.len();
        tempo += dt;
    }
    let (rss, hwm) = memoria();
    println!(
        "⏱ total: {total} keys em {:.1} s de esparso | RSS {rss:.2} GB, pico {hwm:.2} GB",
        tempo.as_secs_f64()
    );
}

/// A linha de base do M1, só com o denso e sem o esparso: as perguntas das FAQs contra o pack da
/// entidade. Existe para rodar também sobre um pack ANTIGO (o de 07/11/2026, das 2.056 FAQs do
/// piloto), e separar "o acervo mudou" de "a ferramenta mede outra coisa" quando o número de hoje
/// não bate com o do piloto.
#[test]
#[ignore = "Fase 0 do Estudo-rag-dense: precisa de uma cópia de laboratório e do modelo"]
fn fase0_linha_de_base() {
    let Some(cfg) = Cfg::do_ambiente() else {
        return;
    };
    let c = carregar(&cfg, Kind::Faqs).expect("a entidade não tem FAQs");
    let denso = Embedder::new(cfg.cache.clone(), cfg.threads).expect("denso");
    let mut p = Placar::default();
    for (alvo, d) in c.docs.iter().enumerate() {
        let q = denso
            .embed_dense(vec![d.titulo.clone()])
            .expect("denso")
            .remove(0);
        p.somar(posicao(&ranking_denso(&c.densos, &q), alvo));
    }
    println!(
        "\nlinha de base — {} FAQs, só denso: acerto em 1º | entre os 10 | MRR@10 = {}",
        p.n,
        p.celula()
    );
}

// ── ordenação e fusão ────────────────────────────────────────────────────────────────────────────

/// Índices em ordem de distância cosseno crescente — a ordem do motor. Empate pelo índice, que é a
/// ordem de `id`.
fn ranking_denso(densos: &[Vec<f32>], q: &[f32]) -> Vec<usize> {
    let mut v: Vec<(f32, usize)> = densos
        .iter()
        .enumerate()
        .map(|(i, e)| (cosine_distance(e, q), i))
        .collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    v.into_iter().map(|(_, i)| i).collect()
}

/// Índices em ordem de pontuação esparsa decrescente. **Só quem tem algum token em comum** com a
/// pergunta entra: pontuação zero é "o índice lexical não achou", não a última posição.
fn ranking_esparso(esparsos: &[VetorEsparso], q: &VetorEsparso) -> Vec<usize> {
    let mut v: Vec<(f32, usize)> = esparsos
        .iter()
        .enumerate()
        .map(|(i, e)| (q.pontuar(e), i))
        .filter(|(s, _)| *s > 0.0)
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().map(|(_, i)| i).collect()
}

/// RRF: `Σ 1 / (k + posição)` sobre as `prof` primeiras posições de cada lista. Empate pelo índice.
fn rrf(denso: &[usize], esparso: &[usize], k: f64, prof: usize) -> Vec<usize> {
    let mut s: HashMap<usize, f64> = HashMap::new();
    for lista in [denso, esparso] {
        for (pos, &i) in lista.iter().take(prof).enumerate() {
            *s.entry(i).or_default() += 1.0 / (k + (pos + 1) as f64);
        }
    }
    let mut v: Vec<(f64, usize)> = s.into_iter().map(|(i, x)| (x, i)).collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().map(|(_, i)| i).collect()
}

fn posicao(lista: &[usize], alvo: usize) -> Option<usize> {
    lista.iter().position(|&i| i == alvo).map(|p| p + 1)
}

#[derive(Default, Clone)]
struct Placar {
    n: usize,
    h1: usize,
    h10: usize,
    rr10: f64,
}

impl Placar {
    fn somar(&mut self, pos: Option<usize>) {
        self.n += 1;
        if let Some(p) = pos
            && p <= 10
        {
            self.h10 += 1;
            self.rr10 += 1.0 / p as f64;
            if p == 1 {
                self.h1 += 1;
            }
        }
    }

    fn celula(&self) -> String {
        let n = self.n.max(1) as f64;
        format!(
            "{:.1}% ({}) | {:.1}% ({}) | {:.4}",
            100.0 * self.h1 as f64 / n,
            self.h1,
            100.0 * self.h10 as f64 / n,
            self.h10,
            self.rr10 / n
        )
    }
}

/// O resultado de uma família de consultas: as três colunas e a grade da fusão.
#[derive(Default)]
struct Familia {
    denso: Placar,
    esparso: Placar,
    hibrido: Placar,
    grade: Vec<((f64, usize), Placar)>,
    /// Quantas vezes o alvo entrou no top 10 híbrido sem estar no top 10 denso, e o contrário.
    ganhos: usize,
    perdas: usize,
    t_q_denso: Vec<f64>,
    t_q_esparso: Vec<f64>,
    t_scan_denso: Vec<f64>,
    t_scan_esparso: Vec<f64>,
}

impl Familia {
    fn imprimir(&self, nome: &str) {
        println!("\n### {nome} — {} consultas", self.denso.n);
        println!("| coluna | acerto em 1º | entre os 10 | MRR@10 |");
        println!("|---|---|---|---|");
        println!("| só denso | {} |", self.denso.celula());
        println!("| só esparso | {} |", self.esparso.celula());
        println!(
            "| híbrido (RRF k={K_RRF}, prof={PROF_RRF}) | {} |",
            self.hibrido.celula()
        );
        println!(
            "top 10: o híbrido ganhou {} e perdeu {} alvos em relação ao denso",
            self.ganhos, self.perdas
        );
        println!("grade da fusão:");
        for ((k, prof), p) in &self.grade {
            println!("  k={k:>3} prof={prof:>3}: {}", p.celula());
        }
    }
}

fn mediana(v: &[f64]) -> f64 {
    percentil(v, 0.5)
}

fn percentil(v: &[f64], p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    s[((s.len() - 1) as f64 * p).round() as usize]
}

/// Mede uma família: cada documento é consultado por `consulta(doc)` e o acerto é ele mesmo.
fn medir_familia(
    c: &Colecao,
    esparsos: &[VetorEsparso],
    denso: &Embedder,
    esparso: &EmbedderEsparso,
    consulta: impl Fn(&Documento) -> String,
) -> Familia {
    let mut f = Familia {
        grade: GRADE_K
            .iter()
            .flat_map(|&k| GRADE_PROF.iter().map(move |&p| ((k, p), Placar::default())))
            .collect(),
        ..Default::default()
    };
    for (alvo, doc) in c.docs.iter().enumerate() {
        let q = consulta(doc);
        let t = Instant::now();
        let qd = denso.embed_dense(vec![q.clone()]).expect("denso").remove(0);
        f.t_q_denso.push(t.elapsed().as_secs_f64() * 1e3);
        let t = Instant::now();
        let qs = esparso.embed_sparse(vec![q]).expect("esparso").remove(0);
        f.t_q_esparso.push(t.elapsed().as_secs_f64() * 1e3);

        let t = Instant::now();
        let rd = ranking_denso(&c.densos, &qd);
        f.t_scan_denso.push(t.elapsed().as_secs_f64() * 1e3);
        let t = Instant::now();
        let rs = ranking_esparso(esparsos, &qs);
        f.t_scan_esparso.push(t.elapsed().as_secs_f64() * 1e3);
        let rh = rrf(&rd, &rs, K_RRF, PROF_RRF);

        let (pd, ph) = (posicao(&rd, alvo), posicao(&rh, alvo));
        f.denso.somar(pd);
        f.esparso.somar(posicao(&rs, alvo));
        f.hibrido.somar(ph);
        let no10 = |p: Option<usize>| p.is_some_and(|p| p <= 10);
        match (no10(pd), no10(ph)) {
            (false, true) => f.ganhos += 1,
            (true, false) => f.perdas += 1,
            _ => {}
        }
        for ((k, prof), p) in f.grade.iter_mut() {
            p.somar(posicao(&rrf(&rd, &rs, *k, *prof), alvo));
        }
    }
    f
}

/// Caixa e acento fora, como a busca literal da aba.
fn dobrar(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

fn titulo_curto(d: &Documento) -> String {
    let t = format!("{} — {}", d.titulo, d.ementa);
    t.chars().take(110).collect()
}

/// Passo 2: M1–M6 e a impressão lado a lado.
#[test]
#[ignore = "Fase 0 do Estudo-rag-dense: precisa da cópia de laboratório, do passo 1 e do modelo"]
fn fase0_medir() {
    let Some(cfg) = Cfg::do_ambiente() else {
        return;
    };
    let mutacao = std::env::var("AULI_F0_MUTACAO").is_ok_and(|v| v == "1");

    // ── M5: memória dos dois encoders, antes de qualquer pack ──
    let (rss0, _) = memoria();
    let denso = Embedder::new(cfg.cache.clone(), cfg.threads).expect("denso");
    denso
        .embed_dense(vec!["aquecimento".into()])
        .expect("denso");
    let (rss1, hwm1) = memoria();
    let esparso = EmbedderEsparso::new(cfg.cache.clone(), cfg.threads).expect("esparso");
    esparso
        .embed_sparse(vec!["aquecimento".into()])
        .expect("esparso");
    let (rss2, hwm2) = memoria();
    println!("\n## M5 — memória dos encoders");
    println!("processo vazio: RSS {rss0:.2} GB");
    println!("+ denso: RSS {rss1:.2} GB (pico {hwm1:.2} GB)");
    println!("+ esparso (segunda sessão): RSS {rss2:.2} GB (pico {hwm2:.2} GB)");

    let mut cols: Vec<(Colecao, Vec<VetorEsparso>)> = Vec::new();
    for kind in Kind::TODOS {
        if let Some(c) = carregar(&cfg, kind) {
            let e = ler_esparsos(&cfg, &c);
            cols.push((c, e));
        }
    }

    // ── M6 sobre o dado real: uma amostra recalculada em outra ordem e composição ──
    println!("\n## M6 — determinismo sobre o acervo");
    for (c, es) in &cols {
        let amostra: Vec<usize> = (0..c.docs.len()).step_by(47).rev().collect();
        let textos = amostra
            .iter()
            .map(|&i| c.docs[i].text_to_embed.clone())
            .collect();
        let de_novo = esparso.embed_sparse(textos).expect("esparso");
        for (&i, v) in amostra.iter().zip(&de_novo) {
            assert_eq!(
                &es[i],
                v,
                "M6 FALHOU: `{}` recalculado deu outro esparso — sem determinismo não há pack \
                 reprodutível, a fase para aqui",
                c.docs[i].doc_path()
            );
        }
        println!(
            "{:?}: {} documentos recalculados em ordem inversa, todos bit a bit iguais",
            c.kind,
            amostra.len()
        );
    }

    // ── M5: tamanho do esparso ──
    println!("\n## M5 — tamanho do esparso");
    println!(
        "| coleção | docs | não nulos: mín | mediana | P99 | máx | esparso JSON | binário (nnz×8) | pack hoje | JSON/pack |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|");
    let (mut tot_json, mut tot_bin, mut tot_pack) = (0u64, 0u64, 0u64);
    for (c, es) in &cols {
        let nnz: Vec<f64> = es.iter().map(|v| v.indices.len() as f64).collect();
        let json = std::fs::metadata(cfg.arquivo_esparso(c.kind))
            .unwrap()
            .len();
        let bin: u64 = es.iter().map(|v| v.indices.len() as u64 * 8).sum();
        let pack = std::fs::metadata(cfg.arquivo_pack(c.kind)).unwrap().len();
        let mb = |b: u64| b as f64 / 1e6;
        println!(
            "| {:?} | {} | {} | {} | {} | {} | {:.1} MB | {:.1} MB | {:.1} MB | {:.0}% |",
            c.kind,
            es.len(),
            percentil(&nnz, 0.0),
            mediana(&nnz),
            percentil(&nnz, 0.99),
            percentil(&nnz, 1.0),
            mb(json),
            mb(bin),
            mb(pack),
            100.0 * json as f64 / pack as f64
        );
        tot_json += json;
        tot_bin += bin;
        tot_pack += pack;
    }
    println!(
        "| total | | | | | | {:.1} MB | {:.1} MB | {:.1} MB | {:.0}% |",
        tot_json as f64 / 1e6,
        tot_bin as f64 / 1e6,
        tot_pack as f64 / 1e6,
        100.0 * tot_json as f64 / tot_pack as f64
    );

    // ── M3: o teto lexical, e quanto dele está na key ──
    println!("\n## M3 — o termo na key × só no corpo (sem caixa e sem acento)");
    println!("| termo | coleção | na key | só no corpo |");
    println!("|---|---|---|---|");
    for termo in TERMOS {
        let t = dobrar(termo);
        for (c, _) in &cols {
            let na_key = c
                .docs
                .iter()
                .filter(|d| dobrar(&d.text_to_embed).contains(&t))
                .count();
            let so_corpo = c
                .docs
                .iter()
                .filter(|d| !dobrar(&d.text_to_embed).contains(&t) && dobrar(&d.corpo).contains(&t))
                .count();
            if na_key + so_corpo > 0 {
                println!("| {termo} | {:?} | {na_key} | {so_corpo} |", c.kind);
            }
        }
    }

    // ── M4: caixa e acento ──
    println!("\n## M4 — caixa e acento");
    let variantes = |t: &str| {
        let sem: String = dobrar(t);
        vec![t.to_string(), t.to_uppercase(), sem]
    };
    for termo in TERMOS {
        let vs = variantes(termo);
        for v in &vs {
            let toks = esparso.tokens(v).expect("tokens");
            let s: Vec<String> = toks.iter().map(|(id, p)| format!("{p}={id}")).collect();
            println!("`{v}` → {}", s.join(" "));
        }
        let e = esparso.embed_sparse(vs.clone()).expect("esparso");
        println!(
            "pontuação `{}`×`{}` = {:.4} | `{}`×`{}` = {:.4} | `{}`×`{}` = {:.4} (consigo mesmo)",
            vs[0],
            vs[1],
            e[0].pontuar(&e[1]),
            vs[0],
            vs[2],
            e[0].pontuar(&e[2]),
            vs[0],
            vs[0],
            e[0].pontuar(&e[0]),
        );
        // Os documentos com o termo na key, separados pela grafia: os que o têm escrito como a
        // pergunta (minúsculas, com acento) e os que só o têm de outro jeito (a ementa em maiúsculas,
        // ou sem acento). Onde cada grupo cai quando a pergunta é o termo em minúsculas.
        let qd = denso
            .embed_dense(vec![termo.to_string()])
            .expect("denso")
            .remove(0);
        let alvo = dobrar(termo);
        println!(
            "| coleção | grafia na key | docs | top 10 denso | top 10 esparso | top 10 híbrido | esparso = 0 |"
        );
        println!("|---|---|---|---|---|---|---|");
        for (c, es) in &cols {
            let rd = ranking_denso(&c.densos, &qd);
            let rs = ranking_esparso(es, &e[0]);
            let rh = rrf(&rd, &rs, K_RRF, PROF_RRF);
            for (grupo, exata) in [("a da pergunta", true), ("só outra (caixa/acento)", false)] {
                let docs: Vec<usize> = (0..c.docs.len())
                    .filter(|&i| {
                        let k = &c.docs[i].text_to_embed;
                        dobrar(k).contains(&alvo) && k.contains(termo) == exata
                    })
                    .collect();
                if docs.is_empty() {
                    continue;
                }
                let no10 = |l: &[usize]| {
                    docs.iter()
                        .filter(|&&i| posicao(l, i).is_some_and(|p| p <= 10))
                        .count()
                };
                let zero = docs.iter().filter(|&&i| posicao(&rs, i).is_none()).count();
                println!(
                    "| {:?} | {grupo} | {} | {} | {} | {} | {zero} |",
                    c.kind,
                    docs.len(),
                    no10(&rd),
                    no10(&rs),
                    no10(&rh)
                );
            }
        }
    }

    // ── a verificação por mutação: cada documento com o vetor do vizinho ──
    if mutacao {
        println!("\n⚠️  MUTAÇÃO LIGADA: documento i com os vetores do documento i+1");
        for (c, es) in cols.iter_mut() {
            c.densos.rotate_left(1);
            es.rotate_left(1);
        }
    }
    let col = |k: Kind| cols.iter().find(|(c, _)| c.kind == k);

    // ── M1: as perguntas das FAQs ──
    let mut todas: Vec<(&str, Kind, Familia)> = Vec::new();
    if let Some((c, es)) = col(Kind::Faqs) {
        let mut vistos: HashMap<String, usize> = HashMap::new();
        for d in &c.docs {
            *vistos.entry(d.titulo.trim().to_string()).or_default() += 1;
        }
        let repetidas: usize = vistos.values().filter(|&&n| n > 1).sum();
        println!(
            "\n## M1 — FAQs: {repetidas} das {} perguntas têm texto idêntico a outra FAQ (teto do acerto em 1º)",
            c.docs.len()
        );
        todas.push((
            "M1 — FAQs (pergunta → a própria FAQ)",
            Kind::Faqs,
            medir_familia(c, es, &denso, &esparso, |d| d.titulo.clone()),
        ));
    }
    // ── M2: pareceres e acórdãos perguntados pelo título ──
    for (kind, nome, minusculas) in [
        (
            Kind::Pareceres,
            "M2 — pareceres (título → o próprio parecer)",
            false,
        ),
        (Kind::Tarf, "M2 — TARF (título → o próprio acórdão)", false),
        // O mesmo título como quem atende digita: em minúsculas e sem acento.
        (
            Kind::Pareceres,
            "M2 — pareceres, título em minúsculas e sem acento",
            true,
        ),
        (
            Kind::Tarf,
            "M2 — TARF, título em minúsculas e sem acento",
            true,
        ),
    ] {
        if let Some((c, es)) = col(kind) {
            let consulta = |d: &Documento| {
                if minusculas {
                    dobrar(&d.titulo)
                } else {
                    d.titulo.clone()
                }
            };
            todas.push((nome, kind, medir_familia(c, es, &denso, &esparso, consulta)));
        }
    }
    for (nome, _, f) in &todas {
        f.imprimir(nome);
    }

    // ── M5: tempos por pergunta ──
    println!("\n## M5 — tempo por pergunta (mediana, ms)");
    println!(
        "| família | vetorizar denso | vetorizar esparso | varrer denso | varrer esparso | coleção |"
    );
    println!("|---|---|---|---|---|---|");
    for (nome, kind, f) in &todas {
        println!(
            "| {nome} | {:.1} | {:.1} | {:.2} | {:.2} | {} docs |",
            mediana(&f.t_q_denso),
            mediana(&f.t_q_esparso),
            mediana(&f.t_scan_denso),
            mediana(&f.t_scan_esparso),
            col(*kind).map(|(c, _)| c.docs.len()).unwrap_or(0)
        );
    }

    // ── a impressão lado a lado: sem a lista do mantenedor, só o "conluio" ──
    println!("\n## Lado a lado — `conluio`, top 10 (✓ = o termo está na key)");
    let q = "conluio";
    let qd = denso.embed_dense(vec![q.into()]).expect("denso").remove(0);
    let qs = esparso
        .embed_sparse(vec![q.into()])
        .expect("esparso")
        .remove(0);
    for kind in [Kind::Tarf, Kind::Pareceres] {
        let Some((c, es)) = col(kind) else { continue };
        let rd = ranking_denso(&c.densos, &qd);
        let rs = ranking_esparso(es, &qs);
        let rh = rrf(&rd, &rs, K_RRF, PROF_RRF);
        println!(
            "\n{kind:?} — o esparso pontua {} documentos acima de zero",
            rs.len()
        );
        for (rotulo, lista) in [("denso", &rd), ("híbrido", &rh)] {
            println!("{rotulo}:");
            for (p, &i) in lista.iter().take(10).enumerate() {
                let d = &c.docs[i];
                let marca = if dobrar(&d.text_to_embed).contains(q) {
                    "✓"
                } else {
                    " "
                };
                println!("  {:>2}. {marca} {}", p + 1, titulo_curto(d));
            }
        }
    }
}
