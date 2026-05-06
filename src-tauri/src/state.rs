use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadingStatus {
    Loading,
    Ready,
    Error(String),
}
