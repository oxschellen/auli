//! Subcomando `grafo` — monta o grafo de jurisprudência (TAREFA-CANONIZADOR, extensão).
//!
//! Terceiro passo do knowledge graph, **determinístico, sem rede, sem LLM**. Consome duas saídas
//! já no disco — `dispositivos-index.json` (do `canonizar`) e `extracao.jsonl` (temas, do `extrair`)
//! — e emite `data/<id>/extracao/grafo.json`: nós (dispositivos canônicos + temas tributários),
//! arestas (co-citação dispositivo↔dispositivo e co-ocorrência tema↔dispositivo) e um **layout**
//! force-directed já calculado (x,y em [0,1]), pronto para o visualizador consumir sem tocar em rede.
//!
//! Substitui o script one-shot que fazia isso à mão: agora regenera para qualquer estado. Idempotente
//! (init do layout é determinístico, sem PRNG externo) — re-rodar produz o mesmo `grafo.json`.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::entities::EntityConfig;
use crate::errors::Result;
use crate::extracao::extracao_dir;

// Filtros do núcleo do grafo (legibilidade). Ajustáveis; hoje espelham o one-shot do RS.
const DISP_MIN: usize = 3; // dispositivo entra se citado em ≥ isto pareceres
const DD_MIN: u32 = 2; // aresta de co-citação entra se peso ≥ isto
const TEMA_TOP: usize = 16; // nº de temas mais frequentes exibidos
const TD_MIN: u32 = 2; // aresta tema↔dispositivo entra se peso ≥ isto

/// Temas genéricos demais (conectam a quase tudo → colapsam o layout): fora do grafo.
const TEMA_STOP: &[&str] = &["icms"];

/// Uma entrada do `dispositivos-index.json` (só os campos usados; o resto é ignorado).
#[derive(Deserialize)]
struct IdxEntry {
    display: String,
    pareceres: Vec<String>,
}

/// Uma linha do `extracao.jsonl` — só `numero` + os temas.
#[derive(Deserialize)]
struct ExtracaoLinha {
    numero: String,
    extracao: ExtracaoTemas,
}
#[derive(Deserialize)]
struct ExtracaoTemas {
    temas: Vec<String>,
}

/// Um nó do grafo. `fam` só existe em dispositivo (tema é identificado por `kind`).
#[derive(Serialize)]
struct Node {
    kind: &'static str,
    id: String,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fam: Option<&'static str>,
    val: usize,
    x: f64,
    y: f64,
}

/// Uma aresta. `k`: 0 = dispositivo↔dispositivo (co-citação), 1 = tema↔dispositivo (co-ocorrência).
#[derive(Serialize)]
struct Edge {
    s: usize,
    t: usize,
    w: u32,
    k: u8,
}

#[derive(Serialize)]
struct Meta {
    pareceres: usize,
    dispositivos_total: usize,
    disp: usize,
    temas: usize,
    edges: usize,
}
#[derive(Serialize)]
struct Grafo {
    meta: Meta,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

/// Normaliza um tema (colapsa dupes conhecidos do vocabulário — amostra do controlled-vocab futuro).
fn tema_canon(t: &str) -> &str {
    match t {
        "diferencial de alíquotas" => "diferencial de alíquota",
        other => other,
    }
}

/// Família da norma a partir da chave canônica (mesma partição do visualizador).
fn familia(key: &str) -> &'static str {
    match key.split(':').next().unwrap_or("") {
        "ricms-rs" => "RICMS",
        "cf" | "ec" => "Constituição/EC",
        "convenio" | "protocolo" => "Convênio/Protocolo",
        "lei" | "lc" | "ctn" => "Lei/LC",
        "decreto" => "Decreto",
        "in" | "in-drp" | "in-re" => "Instrução Normativa",
        _ => "Outros",
    }
}

/// Escrita atômica (`.tmp` + rename) — saída derivada, reescrita inteira a cada rodada.
fn escrever_atomico(path: &Path, conteudo: &str) -> Result<()> {
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    std::fs::write(&tmp, conteudo)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn run(entity: &EntityConfig) -> Result<()> {
    let dir = extracao_dir(entity)?;
    let idx_path = dir.join("dispositivos-index.json");
    let ex_path = dir.join("extracao.jsonl");
    if !idx_path.exists() {
        return Err(format!(
            "índice de dispositivos ausente: {} — rode `auli-collections {} canonizar` antes.",
            idx_path.display(),
            entity.id
        )
        .into());
    }
    if !ex_path.exists() {
        return Err(format!(
            "extração ausente: {} — rode `extrair` antes.",
            ex_path.display()
        )
        .into());
    }

    // 1. Índice: canon_key -> (display, pareceres). Ordenado (BTreeMap) = determinístico.
    let idx: BTreeMap<String, IdxEntry> =
        serde_json::from_str(&std::fs::read_to_string(&idx_path)?)
            .map_err(|e| format!("{}: JSON inválido ({e})", idx_path.display()))?;
    let disp_total = idx.len();

    // parecer -> dispositivos (invertendo o índice) + universo de pareceres.
    let mut par2disp: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (k, e) in &idx {
        for p in &e.pareceres {
            par2disp.entry(p.as_str()).or_default().insert(k.as_str());
        }
    }
    let total_pareceres = par2disp.len();
    let val: HashMap<&str, usize> = idx
        .iter()
        .map(|(k, e)| (k.as_str(), e.pareceres.len()))
        .collect();

    // 2. parecer -> temas (normalizados, sem stoplist), + frequência por tema.
    let stop: HashSet<&str> = TEMA_STOP.iter().copied().collect();
    let mut par2tema: HashMap<String, Vec<String>> = HashMap::new();
    let mut tfreq: BTreeMap<String, usize> = BTreeMap::new();
    for l in std::fs::read_to_string(&ex_path)?.lines() {
        let l = l.trim();
        if l.is_empty() {
            continue;
        }
        let row: ExtracaoLinha = serde_json::from_str(l)
            .map_err(|e| format!("{}: linha malformada ({e})", ex_path.display()))?;
        let mut vistos = BTreeSet::new();
        let temas: Vec<String> = row
            .extracao
            .temas
            .iter()
            .map(|t| tema_canon(t))
            .filter(|t| !stop.contains(t) && vistos.insert(t.to_string()))
            .map(String::from)
            .collect();
        for t in &temas {
            *tfreq.entry(t.clone()).or_insert(0) += 1;
        }
        par2tema.insert(row.numero, temas);
    }

    // 3. Dispositivos-núcleo (≥ DISP_MIN) + arestas de co-citação (peso ≥ DD_MIN).
    let disp_ok: BTreeSet<&str> = val
        .iter()
        .filter(|&(_, &v)| v >= DISP_MIN)
        .map(|(&k, _)| k)
        .collect();
    let mut dd: BTreeMap<(&str, &str), u32> = BTreeMap::new();
    for ds in par2disp.values() {
        let cur: Vec<&str> = ds.iter().copied().filter(|d| disp_ok.contains(d)).collect();
        for i in 0..cur.len() {
            for j in i + 1..cur.len() {
                *dd.entry((cur[i], cur[j])).or_insert(0) += 1;
            }
        }
    }
    let dd: Vec<(&str, &str, u32)> = dd
        .into_iter()
        .filter(|(_, w)| *w >= DD_MIN)
        .map(|((a, b), w)| (a, b, w))
        .collect();
    // Espinha = dispositivos com ao menos uma co-citação. Os temas são overlay sobre ela: um
    // dispositivo sem par de co-citação não entra (mantém o mesmo núcleo do desenho aprovado).
    let backbone: BTreeSet<&str> = dd.iter().flat_map(|(a, b, _)| [*a, *b]).collect();

    // 4. Temas-topo (por frequência, desempate por nome = determinístico).
    let mut ranked: Vec<(&String, usize)> = tfreq.iter().map(|(t, &c)| (t, c)).collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let top_t: Vec<&str> = ranked
        .iter()
        .take(TEMA_TOP)
        .map(|(t, _)| t.as_str())
        .collect();
    let top_set: HashSet<&str> = top_t.iter().copied().collect();

    // 5. Arestas tema↔dispositivo (co-ocorrência no mesmo parecer, peso ≥ TD_MIN).
    let mut td: BTreeMap<(&str, &str), u32> = BTreeMap::new();
    for (p, ds) in &par2disp {
        if let Some(ts) = par2tema.get(*p) {
            for t in ts {
                if !top_set.contains(t.as_str()) {
                    continue;
                }
                for d in ds {
                    if backbone.contains(d) {
                        *td.entry((top_t[top_t.iter().position(|x| x == t).unwrap()], *d))
                            .or_insert(0) += 1;
                    }
                }
            }
        }
    }
    let td: Vec<(&str, &str, u32)> = td
        .into_iter()
        .filter(|(_, w)| *w >= TD_MIN)
        .map(|((t, d), w)| (t, d, w))
        .collect();

    // 6. Maior componente conexo (descarta satélites soltos que só sujam o layout).
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for (a, b, _) in &dd {
        adj.entry(a).or_default().push(b);
        adj.entry(b).or_default().push(a);
    }
    for (t, d, _) in &td {
        adj.entry(t).or_default().push(d);
        adj.entry(d).or_default().push(t);
    }
    let mut giant: HashSet<&str> = HashSet::new();
    let mut visitados: HashSet<&str> = HashSet::new();
    for &start in adj.keys() {
        if visitados.contains(start) {
            continue;
        }
        let mut comp = HashSet::new();
        let mut pilha = vec![start];
        while let Some(u) = pilha.pop() {
            if !comp.insert(u) {
                continue;
            }
            visitados.insert(u);
            if let Some(vs) = adj.get(u) {
                pilha.extend(vs.iter().copied());
            }
        }
        if comp.len() > giant.len() {
            giant = comp;
        }
    }

    // 7. Ordem estável dos nós: dispositivos da espinha (chave ordenada) depois temas.
    let mut disp_nodes: Vec<&str> = backbone
        .iter()
        .copied()
        .filter(|d| giant.contains(d))
        .collect();
    disp_nodes.sort_unstable();
    let tema_nodes: Vec<&str> = top_t
        .iter()
        .copied()
        .filter(|t| giant.contains(t))
        .collect();
    let n = disp_nodes.len() + tema_nodes.len();
    let is_tema = |i: usize| i >= disp_nodes.len();
    let mut id_of: HashMap<&str, usize> = HashMap::new();
    for (i, d) in disp_nodes.iter().enumerate() {
        id_of.insert(*d, i);
    }
    for (i, t) in tema_nodes.iter().enumerate() {
        id_of.insert(*t, disp_nodes.len() + i);
    }
    let edges: Vec<(usize, usize, u32, u8)> = dd
        .iter()
        .filter(|(a, b, _)| giant.contains(a) && giant.contains(b))
        .map(|(a, b, w)| (id_of[a], id_of[b], *w, 0u8))
        .chain(
            td.iter()
                .filter(|(t, d, _)| giant.contains(t) && giant.contains(d))
                .map(|(t, d, w)| (id_of[t], id_of[d], *w, 1u8)),
        )
        .collect();

    // 8. Layout force-directed determinístico (spring-electrical + gravidade + anti-colisão).
    let (xs, ys) = layout(n, &edges, &is_tema);

    // 9. Monta os nós com posições normalizadas.
    let max_disp = disp_nodes.iter().map(|d| val[d]).max().unwrap_or(1).max(1);
    let max_tema = tema_nodes
        .iter()
        .map(|t| tfreq[*t])
        .max()
        .unwrap_or(1)
        .max(1);
    let mut nodes = Vec::with_capacity(n);
    for (i, d) in disp_nodes.iter().enumerate() {
        nodes.push(Node {
            kind: "disp",
            id: d.to_string(),
            label: idx[*d].display.clone(),
            fam: Some(familia(d)),
            val: val[d],
            x: xs[i],
            y: ys[i],
        });
    }
    for (i, t) in tema_nodes.iter().enumerate() {
        let gi = disp_nodes.len() + i;
        nodes.push(Node {
            kind: "tema",
            id: t.to_string(),
            label: t.to_string(),
            fam: None,
            val: tfreq[*t],
            x: xs[gi],
            y: ys[gi],
        });
    }
    let _ = (max_disp, max_tema); // tamanhos são do renderer; `val` já vai no nó

    let grafo = Grafo {
        meta: Meta {
            pareceres: total_pareceres,
            dispositivos_total: disp_total,
            disp: disp_nodes.len(),
            temas: tema_nodes.len(),
            edges: edges.len(),
        },
        nodes,
        edges: edges
            .iter()
            .map(|&(s, t, w, k)| Edge { s, t, w, k })
            .collect(),
    };
    let out = dir.join("grafo.json");
    escrever_atomico(
        &out,
        &serde_json::to_string(&grafo).map_err(|e| format!("serializando: {e}"))?,
    )?;

    println!(
        "🕸️  {}: {} nós ({} dispositivos + {} temas) | {} arestas | {} pareceres",
        entity.id, n, grafo.meta.disp, grafo.meta.temas, grafo.meta.edges, total_pareceres
    );
    println!("📄 saída: {}", out.display());
    Ok(())
}

/// Simulação spring-electrical determinística. Init em círculo com jitter derivado do índice (sem
/// PRNG externo → idempotente). Temas repelem-se muito mais forte (espalha os conceitos). Devolve
/// posições normalizadas em [0,1] (centroide + raio p95).
fn layout(
    n: usize,
    edges: &[(usize, usize, u32, u8)],
    is_tema: &dyn Fn(usize) -> bool,
) -> (Vec<f64>, Vec<f64>) {
    use std::f64::consts::TAU;
    let mut px = vec![0.0f64; n];
    let mut py = vec![0.0f64; n];
    for i in 0..n {
        let ang = TAU * (i as f64) / (n as f64);
        let jitter = (((i * 97 + 13) % 100) as f64 / 100.0 - 0.5) * 0.06;
        px[i] = ang.cos() * 0.5 + jitter;
        py[i] = ang.sin() * 0.5 - jitter;
    }
    if n == 0 {
        return (px, py);
    }
    let mut vx = vec![0.0f64; n];
    let mut vy = vec![0.0f64; n];
    const REP: f64 = 0.013;
    const GRAV: f64 = 0.026;
    const DAMP: f64 = 0.85;
    for _ in 0..900 {
        let mut fx = vec![0.0f64; n];
        let mut fy = vec![0.0f64; n];
        for i in 0..n {
            for j in i + 1..n {
                let dx = px[i] - px[j];
                let dy = py[i] - py[j];
                let d2 = dx * dx + dy * dy + 1e-4;
                let d = d2.sqrt();
                let mult = if is_tema(i) && is_tema(j) {
                    5.5
                } else if is_tema(i) || is_tema(j) {
                    2.0
                } else {
                    1.0
                };
                let f = REP / d2 * mult;
                let (ux, uy) = (dx / d, dy / d);
                fx[i] += ux * f;
                fy[i] += uy * f;
                fx[j] -= ux * f;
                fy[j] -= uy * f;
            }
        }
        for &(a, b, w, k) in edges {
            let dx = px[a] - px[b];
            let dy = py[a] - py[b];
            let d = (dx * dx + dy * dy).sqrt() + 1e-4;
            let base = if k == 0 { 0.12 } else { 0.20 };
            let rest = base * (1.0 - 0.3 * (w.min(6) as f64) / 6.0);
            let strength = if k == 0 { 0.95 } else { 0.62 };
            let f = (d - rest) * strength;
            let (ux, uy) = (dx / d, dy / d);
            fx[a] -= ux * f;
            fy[a] -= uy * f;
            fx[b] += ux * f;
            fy[b] += uy * f;
        }
        for i in 0..n {
            fx[i] -= px[i] * GRAV;
            fy[i] -= py[i] * GRAV;
            vx[i] = (vx[i] + fx[i]) * DAMP;
            vy[i] = (vy[i] + fy[i]) * DAMP;
            px[i] += vx[i].clamp(-0.045, 0.045);
            py[i] += vy[i].clamp(-0.045, 0.045);
        }
    }
    // anti-colisão (raio por tipo; temas com mais espaço pessoal)
    let rr: Vec<f64> = (0..n)
        .map(|i| if is_tema(i) { 0.05 } else { 0.03 })
        .collect();
    for _ in 0..110 {
        for i in 0..n {
            for j in i + 1..n {
                let dx = px[i] - px[j];
                let dy = py[i] - py[j];
                let d = (dx * dx + dy * dy).sqrt() + 1e-6;
                let mind = rr[i] + rr[j];
                if d < mind {
                    let pu = (mind - d) / 2.0;
                    let (ux, uy) = (dx / d, dy / d);
                    px[i] += ux * pu;
                    py[i] += uy * pu;
                    px[j] -= ux * pu;
                    py[j] -= uy * pu;
                }
            }
        }
    }
    // normalização robusta: centroide + raio p95 → [0,1]
    let cx = px.iter().sum::<f64>() / n as f64;
    let cy = py.iter().sum::<f64>() / n as f64;
    let mut rad: Vec<f64> = (0..n)
        .map(|i| ((px[i] - cx).powi(2) + (py[i] - cy).powi(2)).sqrt())
        .collect();
    rad.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let scale = rad[(0.95 * n as f64) as usize].max(1e-6);
    let norm = |v: f64, c: f64| {
        ((0.5 + (v - c) / scale * 0.46).clamp(0.0, 1.0) * 10000.0).round() / 10000.0
    };
    let x = (0..n).map(|i| norm(px[i], cx)).collect();
    let y = (0..n).map(|i| norm(py[i], cy)).collect();
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_entity(tag: &str) -> (EntityConfig, PathBuf) {
        let base = std::env::temp_dir().join(format!("auli_grafo_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("raw")).unwrap();
        std::fs::create_dir_all(base.join("extracao")).unwrap();
        let e = EntityConfig {
            id: "xx".into(),
            name: "Teste".into(),
            system_prompt: String::new(),
            data_dir: base.join("raw").to_string_lossy().into_owned(),
        };
        (e, base.join("extracao"))
    }

    #[test]
    fn monta_grafo_com_temas_e_e_idempotente() {
        let (e, exdir) = temp_entity("basico");
        // 3 dispositivos co-citados em 3 pareceres + temas.
        let index = r#"{
          "ricms-rs:lI:art1": {"display":"art. 1, Livro I do RICMS/RS","ocorrencias":3,"variantes":[],"pareceres":["P1","P2","P3"]},
          "ricms-rs:lI:art2": {"display":"art. 2, Livro I do RICMS/RS","ocorrencias":3,"variantes":[],"pareceres":["P1","P2","P3"]},
          "cf:art155": {"display":"art. 155 do Constituição Federal","ocorrencias":3,"variantes":[],"pareceres":["P1","P2","P3"]}
        }"#;
        std::fs::write(exdir.join("dispositivos-index.json"), index).unwrap();
        let ex = [
            r#"{"numero":"P1","extracao":{"temas":["substituição tributária","icms"]}}"#,
            r#"{"numero":"P2","extracao":{"temas":["substituição tributária"]}}"#,
            r#"{"numero":"P3","extracao":{"temas":["substituição tributária"]}}"#,
        ]
        .join("\n");
        std::fs::write(exdir.join("extracao.jsonl"), ex).unwrap();

        run(&e).unwrap();
        let g1 = std::fs::read_to_string(exdir.join("grafo.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&g1).unwrap();
        // 3 dispositivos + 1 tema (icms caiu no stoplist).
        assert_eq!(v["meta"]["disp"], 3);
        assert_eq!(
            v["meta"]["temas"], 1,
            "só substituição tributária; icms é stopword"
        );
        // co-citação (3 pares) + tema-disp (1 tema × 3 disp) presentes; posições em [0,1].
        assert!(
            v["edges"].as_array().unwrap().iter().any(|e| e["k"] == 1),
            "tem aresta tema-disp"
        );
        for nd in v["nodes"].as_array().unwrap() {
            let (x, y) = (nd["x"].as_f64().unwrap(), nd["y"].as_f64().unwrap());
            assert!((0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y));
        }
        // idempotência: re-rodar = bytes idênticos.
        run(&e).unwrap();
        assert_eq!(
            g1,
            std::fs::read_to_string(exdir.join("grafo.json")).unwrap()
        );
    }

    #[test]
    fn erro_sem_indice_ensina_o_remedio() {
        let (e, _) = temp_entity("semidx");
        let err = run(&e).unwrap_err().to_string();
        assert!(
            err.contains("canonizar"),
            "erro deve mandar rodar canonizar: {err}"
        );
    }

    #[test]
    fn familia_particiona_por_norma() {
        assert_eq!(familia("ricms-rs:lI:art1"), "RICMS");
        assert_eq!(familia("ec:87/15"), "Constituição/EC");
        assert_eq!(familia("lei:8820/89"), "Lei/LC");
        assert_eq!(familia("in-drp:45/98"), "Instrução Normativa");
    }
}
