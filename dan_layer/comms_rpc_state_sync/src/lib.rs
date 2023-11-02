//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

//! # Comms RPC State Sync Protocol
//!
//! ```mermaid
//! sequenceDiagram
//!     participant A as Client
//!     participant B as Server
//!  A->>B: CheckSync
//!  B->>A: SyncStatus
//! ```

mod error;
mod manager;

pub use error::*;
pub use manager::*;
