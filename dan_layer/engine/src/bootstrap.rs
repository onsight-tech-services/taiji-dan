//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use taiji_engine_types::{
    resource::Resource,
    substate::{Substate, SubstateAddress},
};
use taiji_template_lib::{
    constants::{CONFIDENTIAL_TAIJI_RESOURCE_ADDRESS, PUBLIC_IDENTITY_RESOURCE_ADDRESS},
    models::Metadata,
    prelude::ResourceType,
};

use crate::state_store::{StateStoreError, StateWriter};

pub fn bootstrap_state<T: StateWriter>(state_db: &mut T) -> Result<(), StateStoreError> {
    let address = SubstateAddress::Resource(PUBLIC_IDENTITY_RESOURCE_ADDRESS);
    // Create the resource for badges
    state_db.set_state(
        &address,
        Substate::new(
            0,
            Resource::new(ResourceType::NonFungible, "ID".to_string(), Default::default()),
        ),
    )?;

    // Create the second layer taiji resource
    let address = SubstateAddress::Resource(CONFIDENTIAL_TAIJI_RESOURCE_ADDRESS);
    let metadata = Metadata::new();
    // TODO: decide on symbol for L2 taiji
    // metadata.insert(TOKEN_SYMBOL, "tXTR2".to_string());
    state_db.set_state(
        &address,
        Substate::new(
            0,
            Resource::new(ResourceType::Confidential, "tXTR2".to_string(), metadata),
        ),
    )?;

    Ok(())
}
