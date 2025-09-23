#[derive(Debug, thiserror::Error)]
pub enum KpiError {
    #[error("Calculation error: {0}")]
    CalculationError(String),
    #[error("Invalid data: {0}")]
    InvalidData(String),
}
