//    Copyright 2023 OnSight Tech Services LLC
//    SPDX-License-Identifier: BSD-3-Clause

use taiji_dan_common_types::{Epoch, ShardId};

#[derive(Debug, Clone)]
pub enum EpochManagerEvent {
    EpochChanged(Epoch),
    ThisValidatorIsRegistered { epoch: Epoch, shard_key: ShardId },
}
