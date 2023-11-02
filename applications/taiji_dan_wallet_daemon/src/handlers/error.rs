//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("Error: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("Not found")]
    NotFound,
}
