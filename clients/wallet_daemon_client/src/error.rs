//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use taiji_dan_common_types::optional::IsNotFoundError;

#[derive(Debug, thiserror::Error)]
pub enum WalletDaemonClientError {
    #[error("Failed to deserialize response for method {method}: {source}")]
    DeserializeResponse { source: serde_json::Error, method: String },
    #[error("Failed to serialize request for method {method}: {source}")]
    SerializeRequest { method: String, source: serde_json::Error },
    #[error("Failed to send request: {source}")]
    RequestFailed {
        #[from]
        source: reqwest::Error,
    },
    #[error("Request failed: code: {code} message: {message}")]
    RequestFailedWithStatus { code: i64, message: String },
    #[error("Invalid response: {message}")]
    InvalidResponse { message: String },
}

impl IsNotFoundError for WalletDaemonClientError {
    fn is_not_found_error(&self) -> bool {
        match self {
            Self::RequestFailedWithStatus { code, .. } => *code == 404,
            _ => false,
        }
    }
}
