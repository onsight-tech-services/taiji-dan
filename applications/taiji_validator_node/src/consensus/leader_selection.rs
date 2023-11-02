//    Copyright 2023 OnSight Tech Services LLC
//    SPDX-License-Identifier: BSD-3-Clause

use taiji_consensus::traits::LeaderStrategy;
use taiji_dan_common_types::{committee::Committee, NodeAddressable, NodeHeight};

#[derive(Debug, Clone, Copy, Default)]
pub struct RoundRobinLeaderStrategy;
impl RoundRobinLeaderStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl<TAddr: NodeAddressable> LeaderStrategy<TAddr> for RoundRobinLeaderStrategy {
    fn calculate_leader(&self, committee: &Committee<TAddr>, height: NodeHeight) -> u32 {
        (height.0 % committee.members.len() as u64) as u32
    }
}
