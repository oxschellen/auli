//! Fixtures de aceitação portadas do harness `auli-anon-eval` — perguntas sintéticas
//! estilo NAVI, com dados fictícios (CPF/CNPJ de DV válido, gerados para teste).
//!
//! Quatro travas:
//! - [`regressao_coberto`] — todo identificador cujo reconhecedor existe (`coberto: true`)
//!   permanece anonimizado, e a pergunta de controle não gera falso positivo.
//! - [`recall_estruturado_fase1`] — Fase 1 concluída: 100% de recall sobre **todos** os
//!   identificadores estruturados.
//! - [`recall_nao_estruturado_fase4`] — Fase 4: as entidades sem forma canônica
//!   ([`Classe::NomeRazaoEndereco`]) também não vazam. Diferente da Fase 1, a meta aqui é
//!   **recall parcial documentado**: as heurísticas cobrem o caso ancorado, não todos.
//! - [`controle_fase4_zero_falsos_positivos`] — o preço da Fase 4 não pode ser falso positivo:
//!   frases de referência típicas seguem sem detecção alguma.

use auli_anon::Anonimizador;

#[derive(PartialEq)]
enum Classe {
    /// Identificador estruturado (alvo de recall da Fase 1).
    Estruturado,
    /// Nome/razão social/endereço — sem forma canônica, cobertos por heurística na Fase 4.
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
    Fx {
        id: "01",
        categoria: "CPF formatado",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "O contribuinte de CPF 529.982.247-25 não consegue emitir a certidão de situação fiscal. Como proceder?",
        segredos: &["529.982.247-25"],
    },
    Fx {
        id: "02",
        categoria: "CPF sem formatação",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "Contribuinte CPF 52998224725 solicita parcelamento de IPVA em dívida ativa.",
        segredos: &["52998224725"],
    },
    Fx {
        id: "03",
        categoria: "CNPJ formatado",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "A empresa de CNPJ 11.222.333/0001-81 pagou em duplicidade a GA 1118. É possível compensar com ICMS?",
        segredos: &["11.222.333/0001-81"],
    },
    Fx {
        id: "04",
        categoria: "CNPJ sem formatação",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "Empresa 11222333000181 quer aderir ao RDA. Quais os requisitos?",
        segredos: &["11222333000181"],
    },
    Fx {
        id: "05",
        categoria: "CNPJ alfanumérico (2026)",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "O CNPJ 12.ABC.345/01DE-35 foi emitido no formato novo e o sistema não aceita. O que fazer?",
        segredos: &["12.ABC.345/01DE-35"],
    },
    Fx {
        id: "06",
        categoria: "E-mail",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "O contador solicita retorno no e-mail joao.contador@escritoriofiscal.com.br sobre o protocolo aberto.",
        segredos: &["joao.contador@escritoriofiscal.com.br"],
    },
    Fx {
        id: "07",
        categoria: "Telefone celular",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "Contribuinte pede contato no telefone (51) 99876-5432 para tratar do auto de lançamento.",
        segredos: &["(51) 99876-5432"],
    },
    Fx {
        id: "08",
        categoria: "Telefone fixo",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "O escritório atende no (51) 3214-5678 em horário comercial.",
        segredos: &["(51) 3214-5678"],
    },
    Fx {
        id: "09",
        categoria: "Inscrição Estadual (RS)",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "A IE 224/3210012 consta como baixada, mas a empresa segue operando. Como regularizar?",
        segredos: &["224/3210012"],
    },
    Fx {
        id: "10",
        categoria: "Protocolo eletrônico",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "Qual o andamento do protocolo eletrônico 2026/000123456 aberto no e-CAC?",
        segredos: &["2026/000123456"],
    },
    Fx {
        id: "11",
        categoria: "Número de GA",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "A GA de número 0312026000987654 foi paga com código errado. Como pedir a alteração?",
        segredos: &["0312026000987654"],
    },
    Fx {
        id: "12",
        categoria: "RENAVAM",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "O veículo RENAVAM 12345678901 aparece com IPVA em aberto já quitado.",
        segredos: &["12345678901"],
    },
    Fx {
        id: "13",
        categoria: "Placa Mercosul",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "O IPVA da placa IVW4D21 foi lançado em dobro.",
        segredos: &["IVW4D21"],
    },
    Fx {
        id: "14",
        categoria: "Nome de pessoa (pt-BR)",
        classe: Classe::NomeRazaoEndereco,
        coberto: true,
        pergunta: "O produtor rural João da Silva Pereira quer saber como emitir nota de talão.",
        segredos: &["João da Silva Pereira"],
    },
    Fx {
        id: "15",
        categoria: "Razão social",
        classe: Classe::NomeRazaoEndereco,
        coberto: true,
        pergunta: "A empresa Anderle Transportes Ltda não consegue gerar o QR Code do Trânsito Livre.",
        segredos: &["Anderle Transportes"],
    },
    Fx {
        id: "16",
        categoria: "CEP",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "O endereço cadastrado tem CEP 90010-150 e precisa ser atualizado.",
        segredos: &["90010-150"],
    },
    Fx {
        id: "17",
        categoria: "Endereço",
        classe: Classe::NomeRazaoEndereco,
        coberto: true,
        pergunta: "A sede fica na Av. Mauá, 1155, Centro, Porto Alegre. Como alterar o endereço do sócio?",
        segredos: &["Av. Mauá, 1155"],
    },
    Fx {
        id: "19",
        categoria: "Data de nascimento",
        classe: Classe::Estruturado,
        coberto: true,
        pergunta: "O dependente nasceu em 14/03/1998 e precisa constar na declaração do ITCD.",
        segredos: &["14/03/1998"],
    },
    Fx {
        id: "20",
        categoria: "Sem PII (controle)",
        classe: Classe::Controle,
        coberto: false,
        pergunta: "Qual o período de inadimplência para cancelar um parcelamento, regra geral?",
        segredos: &[],
    },
];

/// Frases de referência típicas de uma resposta da Auli — o maior risco de falso positivo da
/// Fase 4, porque são cheias de sequência capitalizada institucional, sigla em caixa alta e
/// gatilho de nome sem nome depois. **Nenhuma pode gerar detecção.**
///
/// Cada uma corresponde a um falso positivo que existiu de fato durante a implementação: sem a
/// regra de Titlecase, `PARCELAMENTO ICMS ME` virava razão social; sem a stoplist,
/// `Simples Nacional ME` e `Vossa Senhoria` também.
const CONTROLES_FASE4: &[&str] = &[
    "Consulte a Receita Estadual do Rio Grande do Sul pelo Portal de Atendimento.",
    "O Simples Nacional exige Inscrição Estadual ativa.",
    "Emita a Nota Fiscal Eletrônica no portal.",
    "O contribuinte deve apresentar a documentação.",
    "Você me disse que a rua não tem número.",
    "O regime do Simples Nacional ME impede o crédito?",
    "PARCELAMENTO ICMS ME em duplicidade.",
    "O contribuinte Vossa Senhoria já foi notificado.",
    "Veja a Instrução Normativa RE 45/2019 e o Decreto 37.699/97.",
    "O fato gerador ocorreu na Rodovia BR-116 km 23.",
];

fn vazou<'a>(limpo: &str, segredos: &'a [&'a str]) -> Vec<&'a str> {
    segredos
        .iter()
        .copied()
        .filter(|s| limpo.contains(s))
        .collect()
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
                fx.id,
                r.mapping.entries
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
    assert!(
        vazamentos.is_empty(),
        "identificadores estruturados vazaram:\n{}",
        vazamentos.join("\n")
    );
}

/// Fase 4: as entidades sem forma canônica marcadas `coberto` não vazam.
///
/// A meta é **recall parcial documentado**, não os 100% da Fase 1: as heurísticas cobrem o caso
/// ancorado (razão social com sufixo societário, endereço com logradouro + número, nome depois de
/// gatilho) e assumem deixar passar o resto — nome solto, nome de um token, gatilho distante.
#[test]
fn recall_nao_estruturado_fase4() {
    let anon = Anonimizador::novo().expect("construir anonimizador");
    let mut vazamentos = Vec::new();
    for fx in FIXTURES {
        if fx.classe != Classe::NomeRazaoEndereco || !fx.coberto {
            continue;
        }
        let r = anon.anonimizar(fx.pergunta).expect("anonimizar");
        for s in vazou(&r.texto, fx.segredos) {
            vazamentos.push(format!("[{}] {}: {}", fx.id, fx.categoria, s));
        }
    }
    assert!(
        vazamentos.is_empty(),
        "entidades não estruturadas vazaram:\n{}",
        vazamentos.join("\n")
    );
}

/// O controle é a trava mais importante da Fase 4: heurística de nome/razão/endereço é a maior
/// fonte de falso positivo do crate, e mascarar texto institucional degradaria a resposta sem
/// proteger ninguém.
#[test]
fn controle_fase4_zero_falsos_positivos() {
    let anon = Anonimizador::novo().expect("construir anonimizador");
    for frase in CONTROLES_FASE4 {
        let r = anon.anonimizar(frase).expect("anonimizar");
        assert!(
            r.mapping.entries.is_empty(),
            "falso positivo em {frase:?}: {:?}",
            r.mapping.entries
        );
        // Redundante com o assert acima por construção, mas é o que o usuário veria no log.
        assert_eq!(&r.texto, frase, "o texto de controle foi alterado");
    }
}

/// Sobreposição nome × razão social (D-ANON-4.7): em `João Pereira Ltda` precedido do gatilho
/// `sócio`, os dois reconhecedores disparam sobre spans que se sobrepõem — o nome dentro da razão.
///
/// Quem resolve é o `deduplicate` do cloakrs (span mais longo vence; empate, maior confiança), não
/// código nosso. O teste existe justamente por isso: é uma premissa emprestada de uma dependência
/// pinada em `=0.3.0`, e um bump que mudasse a regra passaria despercebido sem esta trava.
#[test]
fn sobreposicao_nome_dentro_de_razao_social() {
    let anon = Anonimizador::novo().expect("construir anonimizador");
    let r = anon
        .anonimizar("o sócio João Pereira Ltda consta no cadastro")
        .expect("anonimizar");
    assert_eq!(
        r.mapping.entries.len(),
        1,
        "spans sobrepostos deveriam colapsar em um: {:?}",
        r.mapping.entries
    );
    assert!(
        r.texto.contains("[RAZAO_SOCIAL_1]"),
        "o span mais longo (razão social) deveria vencer: {}",
        r.texto
    );
    assert!(
        !r.texto.contains("[NOME_"),
        "o span mais curto (nome) não pode sobreviver: {}",
        r.texto
    );
}

/// Ciclo completo com as três entidades novas: a pergunta sai com os placeholders da Fase 4 e a
/// resposta do LLM volta com os valores originais. É a fronteira do LLM (Fase 3) exercitada com
/// o que a Fase 4 acrescentou.
#[test]
fn restore_fase4() {
    let anon = Anonimizador::novo().expect("construir anonimizador");
    let pergunta = "o contribuinte João da Silva Pereira, da Anderle Transportes Ltda, \
                    na Av. Mauá, 1155, pediu baixa";
    let r = anon.anonimizar(pergunta).expect("anonimizar");
    for placeholder in ["[NOME_1]", "[RAZAO_SOCIAL_1]", "[ENDERECO_1]"] {
        assert!(
            r.texto.contains(placeholder),
            "faltou {placeholder} em: {}",
            r.texto
        );
    }

    let resposta_llm =
        "Informe a [RAZAO_SOCIAL_1] que o pedido de [NOME_1], com sede na [ENDERECO_1], foi aceito.";
    let restaurado = anon.restaurar(resposta_llm, &r.mapping);
    for original in [
        "João da Silva Pereira",
        "Anderle Transportes Ltda",
        "Av. Mauá, 1155",
    ] {
        assert!(
            restaurado.contains(original),
            "restore perdeu {original:?}: {restaurado}"
        );
    }
}
