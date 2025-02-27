//   Copyright 2023 The Tari Project
//   SPDX-License-Identifier: BSD-3-Clause

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use tari_common_types::types::PublicKey;

use crate::fee_claim::FeeClaim;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VirtualSubstateAddress {
    CurrentEpoch,
    UnclaimedValidatorFee { epoch: u64, address: PublicKey },
}

impl Display for VirtualSubstateAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VirtualSubstateAddress::CurrentEpoch => write!(f, "Virtual(CurrentEpoch)"),
            VirtualSubstateAddress::UnclaimedValidatorFee { epoch, address } => {
                write!(
                    f,
                    "Virtual(UnclaimedValidatorFee(epoch = {}, address = {:.7}))",
                    epoch, address
                )
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VirtualSubstate {
    CurrentEpoch(u64),
    UnclaimedValidatorFee(FeeClaim),
}
