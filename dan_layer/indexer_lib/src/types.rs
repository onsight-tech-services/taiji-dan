//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use serde::{Deserialize, Serialize};
use taiji_engine_types::substate::{Substate, SubstateAddress};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NonFungibleSubstate {
    pub index: u64,
    pub address: SubstateAddress,
    pub substate: Substate,
}
