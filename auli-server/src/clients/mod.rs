// Adapters for external services and the in-process vector store / embedder: one module per concern.

pub mod embedder;
pub mod ingest;
pub mod llm;
pub mod vector_store;
