// Copyright 2022 OnSight Tech Services LLC
// SPDX-License-Identifier: BSD-3-Clause

mod bootstrap;
pub mod fees;
pub mod flow;
pub mod function_definitions;
pub mod packager;
pub mod runtime;
pub mod state_store;
pub mod traits;
pub mod transaction;
pub mod wasm;

pub use bootstrap::bootstrap_state;
pub use taiji_template_abi as abi;

pub mod base_layer_hashers {
    use blake2::{digest::consts::U32, Blake2b};
    use tari_crypto::hasher;
    // TODO: DRY - This should always be the same as the base layer hasher
    hasher!(
        Blake2b<U32>,
        ConfidentialOutputHasher,
        "com.taiji.layer_two.confidential_output",
        1,
        confidential_output_hasher
    );
}
