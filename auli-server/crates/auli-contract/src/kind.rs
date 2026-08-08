//! [`Kind`] — **o enum único das coleções de documentos** (D-A1).
//!
//! Antes desta struct, o nome de uma coleção era uma string literal repetida em oito lugares: o
//! caminho da árvore no `auli update`, o sufixo do pack, as consts do MCP, o rótulo do bloco RAG, o
//! título do README do bundle, o argumento da CLI do índice. Errar uma delas produz o pior tipo de
//! defeito — o que compila, roda e só aparece na tela do usuário (um `docs/tarf` lido para dentro do
//! pack de pareceres, um README chamando "tarf" de "tarf").
//!
//! O que este enum é: as coleções que têm **árvore `docs/<kind>/` e struct de contrato**. Não é a
//! tabela de parâmetros de recuperação (`auli_core::corpus`, que também carrega `notas` — um kind de
//! rota sem fonte nem árvore) nem o vocabulário de rótulos do log. É o degrau que o
//! `ESTUDO-colecoes-trait` pedia: coleção nova = uma variante aqui, e o `match` exaustivo aponta
//! cada ponto a preencher.

/// Uma coleção de documentos do Auli.
///
/// `Copy` porque é um discriminante: passa por valor em toda parte, e nenhum chamador precisa
/// clonar. A ordem das variantes é a de **exibição** (serviços e faqs primeiro, jurisprudência
/// depois) — é a ordem em que o `auli update` vetoriza e em que [`Kind::TODOS`] lista.
///
/// A serialização usa o MESMO texto do [`Kind::as_str`] — o vocabulário é um só, e um JSON com
/// `"kind":"Tarf"` seria um segundo nome para a mesma coisa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Servicos,
    Faqs,
    Pareceres,
    Tarf,
}

impl Kind {
    /// Todas as coleções, na ordem de exibição. Existe para que quem itera não redigite a lista —
    /// acrescentar uma variante acima a inclui em todos os laços de uma vez.
    pub const TODOS: [Kind; 4] = [Kind::Servicos, Kind::Faqs, Kind::Pareceres, Kind::Tarf];

    /// O nome da coleção. **É o mesmo string em todos os papéis**, de propósito: o subdiretório da
    /// árvore (`docs/<kind>/`), o sufixo da coleção vetorial (`<id>-<kind>`), o nome do pack, o
    /// parâmetro da rota `/v1/{kind}/list`, o rótulo do `registry.toml` e o argumento da CLI.
    ///
    /// Essa unificação é doutrina antiga do projeto (ver `corpus::Collection::kind`), e é por isso
    /// que **não existe um `dir()` separado**: um segundo método devolvendo o mesmo texto seria o
    /// primeiro lugar onde os dois poderiam divergir.
    ///
    /// `const fn` para que consts de outros crates possam ser definidas a partir dela — é o que
    /// permite o MCP declarar `const KIND: &str = Kind::Pareceres.as_str()` em vez de repetir o
    /// literal.
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Servicos => "servicos",
            Kind::Faqs => "faqs",
            Kind::Pareceres => "pareceres",
            Kind::Tarf => "tarf",
        }
    }

    /// Como UM documento desta coleção é rotulado no bloco de contexto RAG
    /// (`## documento {i}: {rotulo}` — D-MARC-2). Singular: rotula um documento, não a coleção.
    pub fn rotulo(self) -> &'static str {
        match self {
            Kind::Servicos => "Serviço",
            Kind::Faqs => "FAQ",
            Kind::Pareceres => "Parecer",
            Kind::Tarf => "Acórdão TARF",
        }
    }

    /// Como os documentos são chamados no plural, para relatórios e contagens ("372 pareceres",
    /// "3800 acórdãos"). Não é derivável do [`Kind::rotulo`] em português.
    pub fn plural(self) -> &'static str {
        match self {
            Kind::Servicos => "serviços",
            Kind::Faqs => "perguntas frequentes",
            Kind::Pareceres => "pareceres",
            Kind::Tarf => "acórdãos",
        }
    }

    /// Título de exibição da coleção — o que vai no README da pasta do bundle e nos cabeçalhos.
    /// Difere do [`Kind::rotulo`] porque ali se nomeia um documento e aqui, o acervo inteiro.
    pub fn titulo(self) -> &'static str {
        match self {
            Kind::Servicos => "Serviços",
            Kind::Faqs => "Perguntas Frequentes",
            Kind::Pareceres => "Pareceres",
            Kind::Tarf => "Acórdãos TARF",
        }
    }

    /// Verdadeiro para as coleções de **jurisprudência** — as que prometem uma seção `## resumo` em
    /// cada `.md` e cujo `text_to_embed` fica cego sem ela (D-FMT-7).
    ///
    /// É também o teste de "é jurisprudência?" usado pelo subcomando `indice`, que só deriva índice
    /// leve para estas. Serviços e faqs nunca têm `## resumo`, e isso é estado normal deles.
    pub fn exige_resumo(self) -> bool {
        match self {
            Kind::Pareceres | Kind::Tarf => true,
            Kind::Servicos | Kind::Faqs => false,
        }
    }

    /// `None` para nome desconhecido — o chamador erra alto em vez de adivinhar a coleção.
    pub fn parse(s: &str) -> Option<Self> {
        Kind::TODOS.into_iter().find(|k| k.as_str() == s)
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todos_cobre_as_quatro_e_o_parse_e_inverso_do_as_str() {
        assert_eq!(Kind::TODOS.len(), 4);
        for k in Kind::TODOS {
            assert_eq!(Kind::parse(k.as_str()), Some(k), "{k}");
        }
    }

    #[test]
    fn serializa_com_o_mesmo_texto_do_as_str() {
        // Um segundo nome para a mesma coisa (`"Tarf"` vs `"tarf"`) daria dois vocabulários.
        for k in Kind::TODOS {
            let json = serde_json::to_string(&k).unwrap();
            assert_eq!(json, format!("\"{}\"", k.as_str()), "{k}");
            assert_eq!(serde_json::from_str::<Kind>(&json).unwrap(), k);
        }
    }

    #[test]
    fn nome_desconhecido_e_none() {
        // O ponto do `parse`: nome errado não vira coleção nenhuma. `acordaos` é o engano provável
        // (é o plural do TARF), e `services` é a grafia inglesa que saiu do vocabulário.
        for nome in ["acordaos", "services", "notas", "TARF", "", " tarf"] {
            assert_eq!(Kind::parse(nome), None, "{nome:?}");
        }
    }

    #[test]
    fn os_nomes_sao_distintos_entre_si() {
        // Duas variantes com o mesmo `as_str` fariam o `parse` devolver sempre a primeira, e uma
        // coleção leria a árvore da outra em silêncio.
        let mut nomes: Vec<&str> = Kind::TODOS.iter().map(|k| k.as_str()).collect();
        nomes.sort_unstable();
        nomes.dedup();
        assert_eq!(nomes.len(), 4);
    }

    #[test]
    fn so_a_jurisprudencia_exige_resumo() {
        assert!(Kind::Pareceres.exige_resumo());
        assert!(Kind::Tarf.exige_resumo());
        assert!(!Kind::Servicos.exige_resumo());
        assert!(!Kind::Faqs.exige_resumo());
    }

    #[test]
    fn rotulo_e_titulo_nao_caem_no_nome_cru() {
        // A regressão que isto pega: variante nova entrando sem passar pelos `match` de texto, e o
        // README saindo com "tarf/ — 3800 documentos (tarf)".
        for k in Kind::TODOS {
            assert_ne!(k.rotulo(), k.as_str(), "{k}");
            assert_ne!(k.titulo(), k.as_str(), "{k}");
        }
        // O `plural` fica de fora do laço: em `pareceres` ele COINCIDE com o nome da coleção, e é
        // assim mesmo — o nome já é o plural. Os outros três são conferidos um a um.
        assert_eq!(Kind::Pareceres.plural(), Kind::Pareceres.as_str());
        assert_eq!(Kind::Tarf.plural(), "acórdãos");
        assert_eq!(Kind::Servicos.plural(), "serviços");
        assert_eq!(Kind::Faqs.plural(), "perguntas frequentes");
    }
}
