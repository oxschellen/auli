use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PageType {
    Menu,
    Faq,
    Geral,
}

impl From<&str> for PageType {
    fn from(s: &str) -> Self {
        match s {
            "Menu" => Self::Menu,
            "Faq" => Self::Faq,
            _ => Self::Geral,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FaqItem {
    pub pergunta: String,
    pub resposta: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SiteNode {
    pub title: String,
    pub url: String,
    pub page_type: PageType,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub origin: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<SiteNode>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub faq_items: Vec<FaqItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub title: String,
    pub url: String,
    pub page_type: PageType,
    pub description: String, // If PageType::Menu is a description - If PageType::FAQ is the answer
}

impl Entry {
    pub fn new(title: String, url: String, page_type: PageType, description: String) -> Self {
        Self {
            title,
            url,
            page_type,
            description,
        }
    }
}

// // ==========================

// #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// pub enum TipoDoc {
//     Servicos,
//     FAQs,
//     Pareceres,
//     Counteudo,
//     Notas,
// }

// #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// pub struct Docs {
//     pub tipo: TipoDoc,
//     pub filename: String,
//     pub url: String,
// }

// // Tipos de Documentos: Servicos, FAQs, Pareceres, Conteúdo, Notas
// #[derive(Serialize, Deserialize, Debug)]
// pub struct TipoDocumentos {

// }

// // Tipos de Servicos:
// #[derive(Serialize, Deserialize, Debug)]
// pub struct TipoServicos {
//     pub tipo: String,
//     pub filename: String,
//     pub url: String,
// }

// #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// pub enum FAQs {
//     Menu,
//     Faq,
//     Geral,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct TipoFaqs {
//     pub tipo: String,
//     pub filename: String,
//     pub url: String,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct Service {
//     /// Sequential ID for reference (starts from 1)
//     pub id: usize,
//     /// Service number extracted from the URL query param `servico=NNNN`
//     pub service_number: String,
//     /// Category type (e.g. "Cidadãos", "Empresas")
//     pub tipo: String,
//     /// Service class/group from the card title
//     pub classe: String,
//     /// Originating organ label from the card
//     pub orgao: String,
//     /// URL link for the service
//     pub link: String,
//     /// Human-readable title
//     pub titulo: String,
//     /// Description (reserved for future use)
//     pub descricao: String,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct Faq {
//     pub tipo: TipoFaqs,
//     pub titulo: String,
//     pub url: String,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct FaqsOutro {
//     /// Sequential ID for reference (starts from 1)
//     pub id: usize,
//     /// FAQ number extracted from the URL query param `faq=NNNN`
//     pub faq_number: String,
//     /// Category type (e.g. "Cidadãos", "Empresas")
//     pub tipo: String,
//     /// FAQ class/group from the card title
//     pub classe: String,
//     /// Originating organ label from the card
//     pub orgao: String,
//     /// URL link for the FAQ
//     pub link: String,
//     /// Human-readable title
//     pub titulo: String,
//     /// Description (reserved for future use)
//     pub descricao: String,
// }
