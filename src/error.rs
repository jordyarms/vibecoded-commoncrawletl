use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },

    #[error("CSV error: {source}")]
    Csv {
        #[from]
        source: csv::Error,
    },

    #[error("JSON error: {source}")]
    Json {
        #[from]
        source: serde_json::Error,
    },

    #[error("URL parse error: {source}")]
    Url {
        #[from]
        source: url::ParseError,
    },

    #[error("N-Quads parse error at line {line}: {message}")]
    NQuadsParse { line: usize, message: String },

    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Missing required input: {description}")]
    MissingInput { description: String },

    #[error("Phase {phase} has not been completed yet")]
    PhaseNotComplete { phase: u8 },

    #[error("{message}")]
    Other { message: String },
}

pub type Result<T> = std::result::Result<T, Error>;
