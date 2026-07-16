//! Fixtures de aceitação portadas do harness `auli-anon-eval` — perguntas sintéticas
//! estilo NAVI, com dados fictícios (CPF/CNPJ de DV válido, gerados para teste).
//!
//! Duas travas:
//! - [`baseline_fase0`] — verde hoje: garante que o que o cloakrs nativo já pega (CPF, CNPJ
//!   numérico, e-mail) permanece anonimizado, e que a pergunta de controle não gera falso
//!   positivo. É a rede de regressão da Fase 0.
//! - [`recall_estruturado_fase1`] — `#[ignore]` por ora: exige 100% de recall sobre **todos**
//!   os identificadores estruturados. É o alvo a destravar quando os reconhecedores
//!   customizados da Fase 1 entrarem (um `#[ignore]` a menos por reconhecedor concluído).
//!
//! Nome de pessoa, razão social e endereço livre ([`Classe::Fase4`]) ficam fora de ambas as
//! travas — dependem de NER/heurística e são escopo da Fase 4.

use auli_anon::Anonimizador;

#[derive(PartialEq)]
enum Classe {
    /// Já coberto pelos reconhecedores nativos do cloakrs (locale BR).
    Fase0,
    /// Identificador estruturado a cobrir com reconhecedor customizado (Fase 1).
    Fase1,
    /// Nome/razão social/endereço — exige NER/heurística (Fase 4, fora de escopo aqui).
    Fase4,
    /// Pergunta sem PII: não deve gerar detecção alguma.
    Controle,
}

struct Fx {
    id: &'static str,
    categoria: &'static str,
    pergunta: &'static str,
    /// Trechos que NÃO podem sobreviver à anonimização (para as classes estruturadas).
    segredos: &'static [&'static str],
    classe: Classe,
}

// CPF 529.982.247-25 e CNPJ 11.222.333/0001-81 têm DV válido (fictícios).
const FIXTURES: &[Fx] = &[
    Fx { id: "01", categoria: "CPF formatado", classe: Classe::Fase0,
        pergunta: "O contribuinte de CPF 529.982.247-25 não consegue emitir a certidão de situação fiscal. Como proceder?",
        segredos: &["529.982.247-25"] },
    Fx { id: "02", categoria: "CPF sem formatação", classe: Classe::Fase0,
        pergunta: "Contribuinte CPF 52998224725 solicita parcelamento de IPVA em dívida ativa.",
        segredos: &["52998224725"] },
    Fx { id: "03", categoria: "CNPJ formatado", classe: Classe::Fase0,
        pergunta: "A empresa de CNPJ 11.222.333/0001-81 pagou em duplicidade a GA 1118. É possível compensar com ICMS?",
        segredos: &["11.222.333/0001-81"] },
    Fx { id: "04", categoria: "CNPJ sem formatação", classe: Classe::Fase0,
        pergunta: "Empresa 11222333000181 quer aderir ao RDA. Quais os requisitos?",
        segredos: &["11222333000181"] },
    Fx { id: "05", categoria: "CNPJ alfanumérico (2026)", classe: Classe::Fase1,
        pergunta: "O CNPJ 12.ABC.345/01DE-35 foi emitido no formato novo e o sistema não aceita. O que fazer?",
        segredos: &["12.ABC.345/01DE-35"] },
    Fx { id: "06", categoria: "E-mail", classe: Classe::Fase0,
        pergunta: "O contador solicita retorno no e-mail joao.contador@escritoriofiscal.com.br sobre o protocolo aberto.",
        segredos: &["joao.contador@escritoriofiscal.com.br"] },
    Fx { id: "07", categoria: "Telefone celular", classe: Classe::Fase1,
        pergunta: "Contribuinte pede contato no telefone (51) 99876-5432 para tratar do auto de lançamento.",
        segredos: &["(51) 99876-5432"] },
    Fx { id: "08", categoria: "Telefone fixo", classe: Classe::Fase1,
        pergunta: "O escritório atende no (51) 3214-5678 em horário comercial.",
        segredos: &["(51) 3214-5678"] },
    Fx { id: "09", categoria: "Inscrição Estadual (RS)", classe: Classe::Fase1,
        pergunta: "A IE 224/3210012 consta como baixada, mas a empresa segue operando. Como regularizar?",
        segredos: &["224/3210012"] },
    Fx { id: "10", categoria: "Protocolo eletrônico", classe: Classe::Fase1,
        pergunta: "Qual o andamento do protocolo eletrônico 2026/000123456 aberto no e-CAC?",
        segredos: &["2026/000123456"] },
    Fx { id: "11", categoria: "Número de GA", classe: Classe::Fase1,
        pergunta: "A GA de número 0312026000987654 foi paga com código errado. Como pedir a alteração?",
        segredos: &["0312026000987654"] },
    Fx { id: "12", categoria: "RENAVAM", classe: Classe::Fase1,
        pergunta: "O veículo RENAVAM 12345678901 aparece com IPVA em aberto já quitado.",
        segredos: &["12345678901"] },
    Fx { id: "13", categoria: "Placa Mercosul", classe: Classe::Fase1,
        pergunta: "O IPVA da placa IVW4D21 foi lançado em dobro.",
        segredos: &["IVW4D21"] },
    Fx { id: "14", categoria: "Nome de pessoa (pt-BR)", classe: Classe::Fase4,
        pergunta: "O produtor rural João da Silva Pereira quer saber como emitir nota de talão.",
        segredos: &["João da Silva Pereira"] },
    Fx { id: "15", categoria: "Razão social", classe: Classe::Fase4,
        pergunta: "A empresa Anderle Transportes Ltda não consegue gerar o QR Code do Trânsito Livre.",
        segredos: &["Anderle Transportes"] },
    Fx { id: "16", categoria: "CEP", classe: Classe::Fase1,
        pergunta: "O endereço cadastrado tem CEP 90010-150 e precisa ser atualizado.",
        segredos: &["90010-150"] },
    Fx { id: "17", categoria: "Endereço", classe: Classe::Fase4,
        pergunta: "A sede fica na Av. Mauá, 1155, Centro, Porto Alegre. Como alterar o endereço do sócio?",
        segredos: &["Av. Mauá, 1155"] },
    Fx { id: "19", categoria: "Data de nascimento", classe: Classe::Fase1,
        pergunta: "O dependente nasceu em 14/03/1998 e precisa constar na declaração do ITCD.",
        segredos: &["14/03/1998"] },
    Fx { id: "20", categoria: "Sem PII (controle)", classe: Classe::Controle,
        pergunta: "Qual o período de inadimplência para cancelar um parcelamento, regra geral?",
        segredos: &[] },
];

fn vazou<'a>(limpo: &str, segredos: &'a [&'a str]) -> Vec<&'a str> {
    segredos.iter().copied().filter(|s| limpo.contains(s)).collect()
}

/// Verde na Fase 0: o que o cloakrs nativo já cobre continua coberto; controle sem falso positivo.
#[test]
fn baseline_fase0() {
    let anon = Anonimizador::novo().expect("construir anonimizador");
    for fx in FIXTURES {
        let r = anon.anonimizar(fx.pergunta).expect("anonimizar");
        match fx.classe {
            Classe::Fase0 => {
                let v = vazou(&r.texto, fx.segredos);
                assert!(v.is_empty(), "[{}] {}: vazou {:?}", fx.id, fx.categoria, v);
            }
            Classe::Controle => {
                assert!(
                    r.mapping.entries.is_empty(),
                    "[{}] controle gerou falso positivo: {:?}",
                    fx.id, r.mapping.entries
                );
            }
            // Fase1/Fase4 ainda vazam por definição — cobertos na trava abaixo / na Fase 4.
            _ => {}
        }
    }
}

/// Alvo da Fase 1: 100% de recall sobre todos os identificadores estruturados
/// (todas as classes exceto nome/razão social/endereço). Destravar ao concluir os
/// reconhecedores customizados.
#[test]
#[ignore = "alvo da Fase 1: exige os reconhecedores customizados (§3 do plano)"]
fn recall_estruturado_fase1() {
    let anon = Anonimizador::novo().expect("construir anonimizador");
    let mut vazamentos = Vec::new();
    for fx in FIXTURES {
        if !(fx.classe == Classe::Fase0 || fx.classe == Classe::Fase1) {
            continue;
        }
        let r = anon.anonimizar(fx.pergunta).expect("anonimizar");
        for s in vazou(&r.texto, fx.segredos) {
            vazamentos.push(format!("[{}] {}: {}", fx.id, fx.categoria, s));
        }
    }
    assert!(vazamentos.is_empty(), "identificadores estruturados vazaram:\n{}", vazamentos.join("\n"));
}
