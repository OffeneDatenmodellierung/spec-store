pub mod baseline;
pub mod structured;
pub mod vector;

pub use baseline::BaselineStore;
pub use structured::StructuredStore;
pub use vector::{embed_text, LocalVectorStore, SearchResult, VectorRecord};
