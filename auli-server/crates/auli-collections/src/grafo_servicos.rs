//! Subcomando `grafo-servicos` — monta o grafo da carta de serviços.
//!
//! Terceiro passo, **determinístico, sem rede, sem LLM**, espelho do `grafo` dos pareceres. Consome
//! `sistemas-index.json` (do `canonizar-servicos`), `servicos-extracao.jsonl` (temas, do
//! `extrair-servicos`) e a árvore `docs/servicos/*.md` (o overlay de público) e emite
//! `data/<id>/extracao/grafo-servicos.json`: nós (sistemas canônicos + temas), arestas (co-menção
//! sistema↔sistema e co-ocorrência tema↔sistema) e o layout force-directed já calculado.
//!
//! **Os limiares NÃO são os dos pareceres** (G-SERV-1). A malha de serviços é muito mais rala que a
//! de jurisprudência: no RS são 415 menções para 133 sistemas, com só 14 pares co-mencionados ≥2×
//! (contra 719 dispositivos densamente co-citados). Copiar `DISP_MIN=3, DD_MIN=2` deixaria ~10 nós.
//! Os valores abaixo foram medidos contra o acervo — ver a tabela no `docs/`.
//!
//! O `fam` do nó de sistema é o **público predominante** dos serviços que o citam (Cidadãos,
//! Empresas, …), relido da árvore: é o overlay determinístico que o LLM nunca viu, de propósito —
//! se público/classe fossem ao prompt, os temas ecoariam a taxonomia do portal.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use auli_contract::mddoc;

use crate::domain::entities::EntityConfig;
use crate::errors::Result;
use crate::extracao::extracao_dir;
use crate::grafo::{LayoutParams, layout};

// Filtros do núcleo (legibilidade), calibrados sobre o acervo do RS — ver o cabeçalho do módulo.
const SIS_MIN: usize = 2; // sistema entra se citado em ≥ isto serviços
const SS_MIN: u32 = 1; // aresta sistema↔sistema entra se peso ≥ isto (a malha não sobrevive a 2)
const TEMA_TOP: usize = 24; // nº de temas mais frequentes exibidos
const TS_MIN: u32 = 1; // aresta tema↔sistema entra se peso ≥ isto

/// Temas genéricos demais (ligam-se a quase tudo → colapsam o layout): fora do grafo.
const TEMA_STOP: &[&str] = &[
    "consulta",
    "serviço",
    "servico",
    "atendimento",
    "informação",
];

/// Uma entrada do `sistemas-index.json` (só os campos usados).
#[derive(Deserialize)]
struct IdxEntry {
    display: String,
    servicos: Vec<String>,
}

/// Uma linha do `servicos-extracao.jsonl` — só o título + os temas.
#[derive(Deserialize)]
struct ExtracaoLinha {
    titulo: String,
    extracao: ExtracaoTemas,
}
#[derive(Deserialize)]
struct ExtracaoTemas {
    temas: Vec<String>,
}

/// Um nó do grafo. `fam` (público predominante) só existe em sistema.
#[derive(Serialize)]
struct Node {
    kind: &'static str,
    id: String,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fam: Option<String>,
    val: usize,
    x: f64,
    y: f64,
}

/// Uma aresta. `k`: 0 = sistema↔sistema (co-menção), 1 = tema↔sistema (co-ocorrência).
#[derive(Serialize)]
struct Edge {
    s: usize,
    t: usize,
    w: u32,
    k: u8,
}

#[derive(Serialize)]
struct Meta {
    servicos: usize,
    servicos_com_sistema: usize,
    sistemas_total: usize,
    sis: usize,
    temas: usize,
    edges: usize,
}
#[derive(Serialize)]
struct Grafo {
    meta: Meta,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

/// Escrita atômica (`.tmp` + rename) — saída derivada, reescrita inteira a cada rodada.
fn escrever_atomico(path: &Path, conteudo: &str) -> Result<()> {
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    std::fs::write(&tmp, conteudo)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Lê a árvore `docs/servicos` e devolve `titulo -> públicos` (o mesmo serviço é ofertado a mais de
/// um público). Árvore ausente NÃO é erro: o overlay é opcional e o grafo sai sem `fam`.
fn publicos_por_servico(entity: &EntityConfig) -> Result<HashMap<String, BTreeSet<String>>> {
    let base = Path::new(&entity.data_dir)
        .parent()
        .ok_or_else(|| format!("data_dir sem pai: {}", entity.data_dir))?;
    let dir = base.join("docs").join("servicos");
    let mut out: HashMap<String, BTreeSet<String>> = HashMap::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entrada in std::fs::read_dir(&dir)? {
        let caminho = entrada?.path();
        if !caminho.is_file() || caminho.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let texto = std::fs::read_to_string(&caminho)?;
        let (header, _resumo, _corpo) = mddoc::parse_doc(&texto)
            .map_err(|e| format!("`{}` não parseia ({e})", caminho.display()))?;
        out.entry(header.titulo).or_default().insert(
            auli_contract::partes_trilha_servico(&header.trilha)
                .0
                .to_string(),
        );
    }
    Ok(out)
}

pub fn run(entity: &EntityConfig) -> Result<()> {
    let dir = extracao_dir(entity)?;
    let idx_path = dir.join("sistemas-index.json");
    let ex_path = dir.join("servicos-extracao.jsonl");
    if !idx_path.exists() {
        return Err(format!(
            "índice de sistemas ausente: {} — rode `auli-collections {} canonizar-servicos` antes.",
            idx_path.display(),
            entity.id
        )
        .into());
    }
    if !ex_path.exists() {
        return Err(format!(
            "extração ausente: {} — rode `extrair-servicos` antes.",
            ex_path.display()
        )
        .into());
    }

    // 1. Índice: chave -> (display, serviços). BTreeMap = determinístico.
    let idx: BTreeMap<String, IdxEntry> =
        serde_json::from_str(&std::fs::read_to_string(&idx_path)?)
            .map_err(|e| format!("{}: JSON inválido ({e})", idx_path.display()))?;
    let sis_total = idx.len();

    // serviço -> sistemas (invertendo o índice).
    let mut serv2sis: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (k, e) in &idx {
        for s in &e.servicos {
            serv2sis.entry(s.as_str()).or_default().insert(k.as_str());
        }
    }
    let com_sistema = serv2sis.len();
    let val: HashMap<&str, usize> = idx
        .iter()
        .map(|(k, e)| (k.as_str(), e.servicos.len()))
        .collect();

    // 2. serviço -> temas (sem stoplist) + frequência por tema. O universo de serviços é o da
    //    extração (todos), não o dos que citam sistema.
    let stop: HashSet<&str> = TEMA_STOP.iter().copied().collect();
    let mut serv2tema: HashMap<String, Vec<String>> = HashMap::new();
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
            .map(|t| t.trim().to_lowercase())
            .filter(|t| !t.is_empty() && !stop.contains(t.as_str()) && vistos.insert(t.clone()))
            .collect();
        for t in &temas {
            *tfreq.entry(t.clone()).or_insert(0) += 1;
        }
        serv2tema.insert(row.titulo, temas);
    }
    let total_servicos = serv2tema.len();

    // 3. Sistemas-núcleo (≥ SIS_MIN serviços) + arestas de co-menção (peso ≥ SS_MIN).
    let sis_ok: BTreeSet<&str> = val
        .iter()
        .filter(|&(_, &v)| v >= SIS_MIN)
        .map(|(&k, _)| k)
        .collect();
    let mut ss: BTreeMap<(&str, &str), u32> = BTreeMap::new();
    for ks in serv2sis.values() {
        let cur: Vec<&str> = ks.iter().copied().filter(|k| sis_ok.contains(k)).collect();
        for i in 0..cur.len() {
            for j in i + 1..cur.len() {
                *ss.entry((cur[i], cur[j])).or_insert(0) += 1;
            }
        }
    }
    let ss: Vec<(&str, &str, u32)> = ss
        .into_iter()
        .filter(|(_, w)| *w >= SS_MIN)
        .map(|((a, b), w)| (a, b, w))
        .collect();
    // Espinha = sistemas com ao menos uma co-menção; os temas são overlay sobre ela.
    let backbone: BTreeSet<&str> = ss.iter().flat_map(|(a, b, _)| [*a, *b]).collect();

    // 4. Temas-topo (frequência, desempate por nome = determinístico).
    let mut ranked: Vec<(&String, usize)> = tfreq.iter().map(|(t, &c)| (t, c)).collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let top_t: Vec<&str> = ranked
        .iter()
        .take(TEMA_TOP)
        .map(|(t, _)| t.as_str())
        .collect();
    let top_set: HashSet<&str> = top_t.iter().copied().collect();

    // 5. Arestas tema↔sistema (co-ocorrência no mesmo serviço, peso ≥ TS_MIN).
    let mut ts: BTreeMap<(&str, &str), u32> = BTreeMap::new();
    for (s, ks) in &serv2sis {
        if let Some(temas) = serv2tema.get(*s) {
            for t in temas {
                if !top_set.contains(t.as_str()) {
                    continue;
                }
                let t_ref = top_t[top_t.iter().position(|x| x == t).unwrap()];
                for k in ks {
                    if backbone.contains(k) {
                        *ts.entry((t_ref, *k)).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    let ts: Vec<(&str, &str, u32)> = ts
        .into_iter()
        .filter(|(_, w)| *w >= TS_MIN)
        .map(|((t, k), w)| (t, k, w))
        .collect();

    // 6. Maior componente conexo (descarta satélites soltos que só sujam o layout).
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for (a, b, _) in &ss {
        adj.entry(a).or_default().push(b);
        adj.entry(b).or_default().push(a);
    }
    for (t, k, _) in &ts {
        adj.entry(t).or_default().push(k);
        adj.entry(k).or_default().push(t);
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

    // 7. Ordem estável dos nós: sistemas da espinha (chave ordenada) depois temas.
    let mut sis_nodes: Vec<&str> = backbone
        .iter()
        .copied()
        .filter(|k| giant.contains(k))
        .collect();
    sis_nodes.sort_unstable();
    let tema_nodes: Vec<&str> = top_t
        .iter()
        .copied()
        .filter(|t| giant.contains(t))
        .collect();
    let n = sis_nodes.len() + tema_nodes.len();
    let is_tema = |i: usize| i >= sis_nodes.len();
    let mut id_of: HashMap<&str, usize> = HashMap::new();
    for (i, k) in sis_nodes.iter().enumerate() {
        id_of.insert(*k, i);
    }
    for (i, t) in tema_nodes.iter().enumerate() {
        id_of.insert(*t, sis_nodes.len() + i);
    }
    let edges: Vec<(usize, usize, u32, u8)> = ss
        .iter()
        .filter(|(a, b, _)| giant.contains(a) && giant.contains(b))
        .map(|(a, b, w)| (id_of[a], id_of[b], *w, 0u8))
        .chain(
            ts.iter()
                .filter(|(t, k, _)| giant.contains(t) && giant.contains(k))
                .map(|(t, k, w)| (id_of[t], id_of[k], *w, 1u8)),
        )
        .collect();

    // 8. Layout force-directed determinístico — a mesma física do grafo de jurisprudência, com a
    //    repulsão dos temas afrouxada: lá são 16 temas para 97 normas, aqui 21 para 22 sistemas, e
    //    o valor original empurrava todos os sistemas para um nó central ilegível.
    //    O espaço pessoal acompanha o tamanho DESENHADO (∝ √val, como no visualizador): aqui `val`
    //    vai de 2 a 75, e um raio constante deixava os dois hubs — Protocolo Eletrônico e Portal
    //    e-CAC, co-mencionados 47× — literalmente um por cima do outro.
    let max_sis = sis_nodes.iter().map(|k| val[k]).max().unwrap_or(1).max(1);
    let max_tema = tema_nodes
        .iter()
        .map(|t| tfreq[*t])
        .max()
        .unwrap_or(1)
        .max(1);
    let raios: Vec<f64> = (0..n)
        .map(|i| {
            if is_tema(i) {
                let v = tfreq[tema_nodes[i - sis_nodes.len()]] as f64 / max_tema as f64;
                0.028 + 0.030 * v.sqrt()
            } else {
                let v = val[sis_nodes[i]] as f64 / max_sis as f64;
                0.022 + 0.075 * v.sqrt()
            }
        })
        .collect();
    let params = LayoutParams {
        rep_tema_tema: 2.5,
        rep_tema_outro: 1.4,
        rep_escala: 2.2,
        // Mola do tipo 0 mais longa E mais frouxa: os dois hubs (Protocolo Eletrônico e Portal
        // e-CAC, co-mencionados 47×) aparecem em quase todo serviço, então ocupam a mesma posição
        // estrutural e a mola padrão os funde num borrão só. Afrouxá-la não os separa muito — a
        // normalização final reescala o desenho —, mas desafoga o miolo o bastante para os dois
        // ficarem legíveis lado a lado.
        rest_intra: 0.36,
        forca_intra: 0.42,
    };
    let (xs, ys) = layout(n, &edges, &is_tema, &raios, &params);

    // 9. Overlay determinístico: público predominante dos serviços que citam o sistema.
    let publicos = publicos_por_servico(entity)?;
    let predominante = |chave: &str| -> Option<String> {
        let mut contagem: BTreeMap<&str, usize> = BTreeMap::new();
        for s in &idx[chave].servicos {
            for p in publicos.get(s).into_iter().flatten() {
                *contagem.entry(p.as_str()).or_insert(0) += 1;
            }
        }
        // Desempate: contagem desc, depois nome asc — função do acervo, não da ordem de leitura.
        contagem
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0)))
            .map(|(p, _)| p.to_string())
    };

    let mut nodes = Vec::with_capacity(n);
    for (i, k) in sis_nodes.iter().enumerate() {
        nodes.push(Node {
            kind: "sis",
            id: (*k).to_string(),
            label: idx[*k].display.clone(),
            fam: predominante(k),
            val: val[k],
            x: xs[i],
            y: ys[i],
        });
    }
    for (i, t) in tema_nodes.iter().enumerate() {
        let gi = sis_nodes.len() + i;
        nodes.push(Node {
            kind: "tema",
            id: (*t).to_string(),
            label: (*t).to_string(),
            fam: None,
            val: tfreq[*t],
            x: xs[gi],
            y: ys[gi],
        });
    }

    let grafo = Grafo {
        meta: Meta {
            servicos: total_servicos,
            servicos_com_sistema: com_sistema,
            sistemas_total: sis_total,
            sis: sis_nodes.len(),
            temas: tema_nodes.len(),
            edges: edges.len(),
        },
        nodes,
        edges: edges
            .iter()
            .map(|&(s, t, w, k)| Edge { s, t, w, k })
            .collect(),
    };
    let out = dir.join("grafo-servicos.json");
    escrever_atomico(
        &out,
        &serde_json::to_string(&grafo).map_err(|e| format!("serializando: {e}"))?,
    )?;

    println!(
        "🕸️  {}: {} nós ({} sistemas + {} temas) | {} arestas | {} serviços ({} citam algum sistema)",
        entity.id,
        n,
        grafo.meta.sis,
        grafo.meta.temas,
        grafo.meta.edges,
        total_servicos,
        com_sistema
    );
    println!("📄 saída: {}", out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_entity(tag: &str) -> (EntityConfig, PathBuf) {
        let base =
            std::env::temp_dir().join(format!("auli_grafo_serv_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("raw")).unwrap();
        std::fs::create_dir_all(base.join("extracao")).unwrap();
        std::fs::create_dir_all(base.join("docs").join("servicos")).unwrap();
        let e = EntityConfig {
            id: "xx".into(),
            name: "Teste".into(),
            system_prompt: String::new(),
            data_dir: base.join("raw").to_string_lossy().into_owned(),
        };
        (e, base.join("extracao"))
    }

    fn escrever_servico(entity: &EntityConfig, arquivo: &str, titulo: &str, tipo: &str) {
        let base = Path::new(&entity.data_dir).parent().unwrap();
        let header = mddoc::DocHeader {
            titulo: titulo.into(),
            trilha: auli_contract::trilha_servico(tipo, "Cadastro"),
            ementa: String::new(),
            link: "https://exemplo".into(),
            orgao: Some("RECEITA".into()),
            resumo_info: None,
        };
        std::fs::write(
            base.join("docs")
                .join("servicos")
                .join(format!("{arquivo}.md")),
            mddoc::render_doc(&header, None, "corpo"),
        )
        .unwrap();
    }

    #[test]
    fn monta_grafo_com_temas_publico_e_e_idempotente() {
        let (e, exdir) = temp_entity("basico");
        // 3 sistemas co-mencionados em 3 serviços.
        let index = r#"{
          "e-cac": {"display":"Portal e-CAC","ocorrencias":3,"variantes":[],"servicos":["S1","S2","S3"]},
          "protocolo-eletronico": {"display":"Protocolo Eletrônico","ocorrencias":3,"variantes":[],"servicos":["S1","S2","S3"]},
          "dte": {"display":"DTE","ocorrencias":2,"variantes":[],"servicos":["S1","S2"]}
        }"#;
        std::fs::write(exdir.join("sistemas-index.json"), index).unwrap();
        let ex = [
            r#"{"titulo":"S1","extracao":{"temas":["parcelamento","consulta"]}}"#,
            r#"{"titulo":"S2","extracao":{"temas":["parcelamento"]}}"#,
            r#"{"titulo":"S3","extracao":{"temas":["parcelamento"]}}"#,
        ]
        .join("\n");
        std::fs::write(exdir.join("servicos-extracao.jsonl"), ex).unwrap();
        // A árvore dá o overlay: S1/S2 de Empresas, S3 de Cidadãos → predominante = Empresas.
        escrever_servico(&e, "s1", "S1", "Empresas");
        escrever_servico(&e, "s2", "S2", "Empresas");
        escrever_servico(&e, "s3", "S3", "Cidadãos");

        run(&e).unwrap();
        let g1 = std::fs::read_to_string(exdir.join("grafo-servicos.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&g1).unwrap();
        assert_eq!(v["meta"]["sis"], 3);
        assert_eq!(
            v["meta"]["temas"], 1,
            "só parcelamento; consulta é stopword"
        );
        assert_eq!(v["meta"]["servicos"], 3);
        assert!(
            v["edges"].as_array().unwrap().iter().any(|e| e["k"] == 1),
            "tem aresta tema-sistema"
        );
        let ecac = v["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == "e-cac")
            .unwrap();
        assert_eq!(
            ecac["fam"], "Empresas",
            "público predominante vem da árvore"
        );
        for nd in v["nodes"].as_array().unwrap() {
            let (x, y) = (nd["x"].as_f64().unwrap(), nd["y"].as_f64().unwrap());
            assert!((0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y));
        }
        // Idempotência: re-rodar = bytes idênticos.
        run(&e).unwrap();
        assert_eq!(
            g1,
            std::fs::read_to_string(exdir.join("grafo-servicos.json")).unwrap()
        );
    }

    #[test]
    fn erro_sem_indice_ensina_o_remedio() {
        let (e, _) = temp_entity("semidx");
        let err = run(&e).unwrap_err().to_string();
        assert!(
            err.contains("canonizar-servicos"),
            "erro deve mandar rodar o canonizador: {err}"
        );
    }

    #[test]
    fn arvore_ausente_nao_e_erro_o_overlay_e_opcional() {
        let (e, exdir) = temp_entity("semarvore");
        std::fs::remove_dir_all(
            Path::new(&e.data_dir)
                .parent()
                .unwrap()
                .join("docs")
                .join("servicos"),
        )
        .unwrap();
        let index = r#"{
          "a": {"display":"A","ocorrencias":2,"variantes":[],"servicos":["S1","S2"]},
          "b": {"display":"B","ocorrencias":2,"variantes":[],"servicos":["S1","S2"]}
        }"#;
        std::fs::write(exdir.join("sistemas-index.json"), index).unwrap();
        std::fs::write(
            exdir.join("servicos-extracao.jsonl"),
            "{\"titulo\":\"S1\",\"extracao\":{\"temas\":[\"x\"]}}\n{\"titulo\":\"S2\",\"extracao\":{\"temas\":[\"x\"]}}\n",
        )
        .unwrap();
        run(&e).unwrap();
        let v: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(exdir.join("grafo-servicos.json")).unwrap(),
        )
        .unwrap();
        let a = v["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == "a")
            .unwrap();
        assert!(a.get("fam").is_none(), "sem árvore, o nó sai sem `fam`");
    }
}
