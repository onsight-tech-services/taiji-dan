//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use taiji_engine_types::substate::SubstateAddress;
use taiji_template_lib::{
    models::{Amount, ResourceAddress},
    resource::ResourceType,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VaultModel {
    pub account_address: SubstateAddress,
    pub address: SubstateAddress,
    pub resource_address: ResourceAddress,
    pub resource_type: ResourceType,
    pub balance: Amount,
    pub token_symbol: Option<String>,
}
