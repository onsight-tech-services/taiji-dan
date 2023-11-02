//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use diesel::{Queryable, QueryableByName};
use taiji_dan_common_types::NodeAddressable;
use taiji_dan_storage::{consensus_models, StorageError};
use time::PrimitiveDateTime;

use crate::{schema::quorum_certificates, serialization::deserialize_json};

#[derive(Debug, Clone, Queryable, QueryableByName)]
pub struct QuorumCertificate {
    pub id: i32,
    pub qc_id: String,
    pub block_id: String,
    pub json: String,
    pub created_at: PrimitiveDateTime,
}

impl<TAddr: NodeAddressable> TryFrom<QuorumCertificate> for consensus_models::QuorumCertificate<TAddr> {
    type Error = StorageError;

    fn try_from(value: QuorumCertificate) -> Result<Self, Self::Error> {
        deserialize_json(&value.json)
    }
}
