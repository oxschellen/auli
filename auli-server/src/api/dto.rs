// Types for interaction with the Web UI
use derive_more::Display;
use serde::{Deserialize, Serialize};

// Struct for input Questions
#[derive(Debug, Display, Serialize, Deserialize)]
#[display("question: {}", question)]
pub struct Question {
    pub question: String,
    // Target entity (state) id. Missing/empty -> default entity ("rs").
    #[serde(default)]
    pub entity: Option<String>,
}

// Query string for GET data-management routes, e.g. `?entity=rs`.
#[derive(Debug, Deserialize)]
pub struct EntityQuery {
    #[serde(default)]
    pub entity: Option<String>,
}
// Struct for output of Questions
#[derive(Debug, Display, Serialize, Deserialize)]
#[display("{} {} {}", status, question, answer)]
pub struct QuestionResponse {
    pub status: String,
    pub question: String,
    pub answer: String,
}
// Struct for output of Answers
#[derive(Debug, Display, Serialize, Deserialize)] // Struct for Output of Answers
#[display("question: {}\nanswer: {}", question, answer)]
pub struct Answer {
    pub question: String,
    pub answer: String,
}

// Struct for input of FAQs web input form
#[derive(Debug, Display, Serialize, Deserialize)]
#[display("text: {}", text)]
pub struct InputServices {
    pub text: String,
    // Target entity (state) id. Missing/empty -> default entity ("rs").
    #[serde(default)]
    pub entity: Option<String>,
}
