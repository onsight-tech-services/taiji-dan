//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use taiji_template_lib::{
    models::VaultId,
    prelude::{Metadata, NonFungibleId},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NonFungibleToken {
    pub vault_id: VaultId,
    pub nft_id: NonFungibleId,
    pub metadata: Metadata,
    pub is_burned: bool,
    pub token_symbol: String,
}
