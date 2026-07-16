//! Fixtures de aceitação portadas do harness `auli-anon-eval` — perguntas sintéticas
//! estilo NAVI, com dados fictícios (CPF/CNPJ de DV válido, gerados para teste).
//!
//! Duas travas:
//! - [`regressao_coberto`] — todo identificador cujo reconhecedor existe (`coberto: true`)
//!   permanece anonimizado, e a pergunta de controle não gera falso positivo.
//! - [`recall_estruturado_fase1`] — Fase 1 concluída: 100% de recall sobre **todos** os
//!   identificadores estruturados.
//!
//! Nome de pessoa, razão social e endereço livre ([`Classe::NomeRazaoEndereco`]) ficam fora de
//! ambas as travas — dependem de NER/heurística e são escopo da Fase 4.

use auli_anon::Anonimizador;

#[derive(PartialEq)]
enum Classe {
    /// Identificador estruturado (alvo de recall da Fase 1).
    Estruturado,
    /// Nome/razão social/endereço — exige NER/heurística (Fase 4, fora de escopo aqui).
    NomeRazaoEndereco,
    /// Pergunta sem PII: não deve gerar detecção alguma.
    Controle,
}

struct Fx {
    id: &'static str,
    categoria: &'static str,
    pergunta: &'static str,
    /// Trechos que NÃO podem sobreviver à anonimização.
    segredos: &'static [&'static str],
    classe: Classe,
    /// `true` quando já existe reconhecedor para esta entidade (trava de regressão).
    coberto: bool,
}

// CPF 529.982.247-25 e CNPJ 11.222.333/0001-81 têm DV válido (fictícios).
const FIXTURES: &[Fx] = &[
    Fx { id: "01", categoria: "CPF formatado", classe: Classe::Estruturado, coberto: true,
        pergunta: "O contribuinte de CPF 529.982.247-25 não consegue emitir a certidão de situação fiscal. Como proceder?",
        segredos: &["529.982.247-25"] },
    Fx { id: "02", categoria: "CPF sem formatação", classe: Classe::Estruturado, coberto: true,
        pergunta: "Contribuinte CPF 52998224725 solicita parcelamento de IPVA em dívida ativa.",
        segredos: &["52998224725"] },
    Fx { id: "03", categoria: "CNPJ formatado", classe: Classe::Estruturado, coberto: true,
        pergunta: "A empresa de CNPJ 11.222.333/0001-81 pagou em duplicidade a GA 1118. É possível compensar com ICMS?",
        segredos: &["11.222.333/0001-81"] },
    Fx { id: "04", categoria: "CNPJ sem formatação", classe: Classe::Estruturado, coberto: true,
        pergunta: "Empresa 11222333000181 quer aderir ao RDA. Quais os requisitos?",
        segredos: &["11222333000181"] },
    Fx { id: "05", categoria: "CNPJ alfanumérico (2026)", classe: Classe::Estruturado, coberto: true,
        pergunta: "O CNPJ 12.ABC.345/01DE-35 foi emitido no formato novo e o sistema não aceita. O que fazer?",
        segredos: &["12.ABC.345/01DE-35"] },
    Fx { id: "06", categoria: "E-mail", classe: Classe::Estruturado, coberto: true,
        pergunta: "O contador solicita retorno no e-mail joao.contador@escritoriofiscal.com.br sobre o protocolo aberto.",
        segredos: &["joao.contador@escritoriofiscal.com.br"] },
    Fx { id: "07", categoria: "Telefone celular", classe: Classe::Estruturado, coberto: true,
        pergunta: "Contribuinte pede contato no telefone (51) 99876-5432 para tratar do auto de lançamento.",
        segredos: &["(51) 99876-5432"] },
    Fx { id: "08", categoria: "Telefone fixo", classe: Classe::Estruturado, coberto: true,
        pergunta: "O escritório atende no (51) 3214-5678 em horário comercial.",
        segredos: &["(51) 3214-5678"] },
    Fx { id: "09", categoria: "Inscrição Estadual (RS)", classe: Classe::Estruturado, coberto: true,
        pergunta: "A IE 224/3210012 consta como baixada, mas a empresa segue operando. Como regularizar?",
        segredos: &["224/3210012"] },
    Fx { id: "10", categoria: "Protocolo eletrônico", classe: Classe::Estruturado, coberto: true,
        pergunta: "Qual o andamento do protocolo eletrônico 2026/000123456 aberto no e-CAC?",
        segredos: &["2026/000123456"] },
    Fx { id: "11", categoria: "Número de GA", classe: Classe::Estruturado, coberto: true,
        pergunta: "A GA de número 0312026000987654 foi paga com código errado. Como pedir a alteração?",
        segredos: &["0312026000987654"] },
    Fx { id: "12", categoria: "RENAVAM", classe: Classe::Estruturado, coberto: true,
        pergunta: "O veículo RENAVAM 12345678901 aparece com IPVA em aberto já quitado.",
        segredos: &["12345678901"] },
    Fx { id: "13", categoria: "Placa Mercosul", classe: Classe::Estruturado, coberto: true,
        pergunta: "O IPVA da placa IVW4D21 foi lançado em dobro.",
        segredos: &["IVW4D21"] },
    Fx { id: "14", categoria: "Nome de pessoa (pt-BR)", classe: Classe::NomeRazaoEndereco, coberto: false,
        pergunta: "O produtor rural João da Silva Pereira quer saber como emitir nota de talão.",
        segredos: &["João da Silva Pereira"] },
    Fx { id: "15", categoria: "Razão social", classe: Classe::NomeRazaoEndereco, coberto: false,
        pergunta: "A empresa Anderle Transportes Ltda não consegue gerar o QR Code do Trânsito Livre.",
        segredos: &["Anderle Transportes"] },
    Fx { id: "16", categoria: "CEP", classe: Classe::Estruturado, coberto: true,
        pergunta: "O endereço cadastrado tem CEP 90010-150 e precisa ser atualizado.",
        segredos: &["90010-150"] },
    Fx { id: "17", categoria: "Endereço", classe: Classe::NomeRazaoEndereco, coberto: false,
        pergunta: "A sede fica na Av. Mauá, 1155, Centro, Porto Alegre. Como alterar o endereço do sócio?",
        segredos: &["Av. Mauá, 1155"] },
    Fx { id: "19", categoria: "Data de nascimento", classe: Classe::Estruturado, coberto: true,
        pergunta: "O dependente nasceu em 14/03/1998 e precisa constar na declaração do ITCD.",
        segredos: &["14/03/1998"] },
    Fx { id: "20", categoria: "Sem PII (controle)", classe: Classe::Controle, coberto: false,
        pergunta: "Qual o período de inadimplência para cancelar um parcelamento, regra geral?",
        segredos: &[] },
];

fn vazou<'a>(limpo: &str, segredos: &'a [&'a str]) -> Vec<&'a str> {
    segredos.iter().copied().filter(|s| limpo.contains(s)).collect()
}

/// Regressão: todo reconhecedor já implementado (`coberto`) segue mascarando; controle limpo.
#[test]
fn regressao_coberto() {
    let anon = Anonimizador::novo().expect("construir anonimizador");
    for fx in FIXTURES {
        let r = anon.anonimizar(fx.pergunta).expect("anonimizar");
        if fx.coberto {
            let v = vazou(&r.texto, fx.segredos);
            assert!(v.is_empty(), "[{}] {}: vazou {:?}", fx.id, fx.categoria, v);
        }
        if fx.classe == Classe::Controle {
            assert!(
                r.mapping.entries.is_empty(),
                "[{}] controle gerou falso positivo: {:?}",
                fx.id, r.mapping.entries
            );
        }
    }
}

/// Fase 1 concluída: 100% de recall sobre todo identificador estruturado (todas as classes
/// exceto nome/razão social/endereço, que são da Fase 4).
#[test]
fn recall_estruturado_fase1() {
    let anon = Anonimizador::novo().expect("construir anonimizador");
    let mut vazamentos = Vec::new();
    for fx in FIXTURES {
        if fx.classe != Classe::Estruturado {
            continue;
        }
        let r = anon.anonimizar(fx.pergunta).expect("anonimizar");
        for s in vazou(&r.texto, fx.segredos) {
            vazamentos.push(format!("[{}] {}: {}", fx.id, fx.categoria, s));
        }
    }
    assert!(vazamentos.is_empty(), "identificadores estruturados vazaram:\n{}", vazamentos.join("\n"));
}
