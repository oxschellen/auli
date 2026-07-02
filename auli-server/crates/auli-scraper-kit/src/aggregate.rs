use std::collections::HashMap;

use auli_contract::{Ocorrencia, ServicoRaw};

use crate::Servico;

/// Serviços agrupados por público (rótulo do público, serviços daquele público), na ordem de
/// exibição — a entrada de [`aggregate_servicos`].
pub type PerPublicoServicos = Vec<(String, Vec<Servico>)>;

/// Agrega serviços per-público em um registro por `link`, acumulando uma [`Ocorrencia`]
/// (público×classe) por listagem, na ordem de descoberta (itera os públicos na ordem dada e, dentro
/// de cada um, os serviços na ordem da lista). Um serviço sob 2+ classes vira 2+ ocorrências (v2 —
/// preserva o caso multi-classe). `descricao` vira o corpo limpo (sem o header `tipo/classe/titulo`).
pub fn aggregate_servicos(inputs: &PerPublicoServicos) -> Vec<ServicoRaw> {
    let mut items: Vec<ServicoRaw> = Vec::new();
    let mut pos: HashMap<String, usize> = HashMap::new();

    for (publico, servicos) in inputs {
        for s in servicos {
            let ocorrencia = Ocorrencia { publico: publico.clone(), classe: s.classe.clone() };
            if let Some(&i) = pos.get(&s.link) {
                items[i].ocorrencias.push(ocorrencia);
                continue;
            }
            pos.insert(s.link.clone(), items.len());
            items.push(ServicoRaw {
                titulo: s.titulo.clone(),
                descricao: descricao_body(&s.descricao),
                link: s.link.clone(),
                orgao: s.orgao.clone(),
                ocorrencias: vec![ocorrencia],
            });
        }
    }
    items
}

/// O corpo da descrição sem as 3 linhas de header `tipo / classe / titulo` que o scraper prepende
/// (esses campos viram colunas próprias no snapshot). Descrição vazia/curta rende corpo vazio.
pub fn descricao_body(descricao: &str) -> String {
    descricao
        .lines()
        .skip(3)
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_records_ocorrencias_per_link_in_discovery_order() {
        let svc = |link: &str, classe: &str| Servico {
            id: 0,
            tipo: String::new(), // irrelevante: o público vem do rótulo do input
            classe: classe.into(),
            orgao: "O".into(),
            link: link.into(),
            titulo: "T".into(),
            descricao: "tipo\nclasse\ntitulo\ncorpo".into(),
        };
        let inputs = vec![
            // l2 aparece sob 2 classes no MESMO público (multi-classe) e depois em outro público.
            ("Cidadãos".to_string(), vec![svc("l1", "A"), svc("l2", "A"), svc("l2", "B")]),
            ("Empresas".to_string(), vec![svc("l2", "A"), svc("l3", "A")]),
        ];

        let out = aggregate_servicos(&inputs);

        // Um registro por link, na ordem de primeira ocorrência (l1, l2, l3).
        assert_eq!(out.iter().map(|s| s.link.as_str()).collect::<Vec<_>>(), ["l1", "l2", "l3"]);
        // l2: uma ocorrência por listagem, na ordem de descoberta.
        let l2 = out.iter().find(|s| s.link == "l2").unwrap();
        let ocs: Vec<_> = l2.ocorrencias.iter().map(|o| (o.publico.as_str(), o.classe.as_str())).collect();
        assert_eq!(ocs, [("Cidadãos", "A"), ("Cidadãos", "B"), ("Empresas", "A")]);
        // descricao = corpo limpo, sem as 3 linhas de header.
        assert_eq!(out[0].descricao, "corpo");
    }

    #[test]
    fn descricao_body_drops_three_header_lines() {
        assert_eq!(descricao_body("tipo\nclasse\ntitulo\ncorpo\nmais"), "corpo\nmais");
        assert_eq!(descricao_body("so\nduas"), "");
    }
}
