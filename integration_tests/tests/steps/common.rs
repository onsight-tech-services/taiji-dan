//  Copyright 2022 OnSight Tech Services LLC
//  SPDX-License-Identifier: BSD-3-Clause

use cucumber::when;
use tari_crypto::tari_utilities::hex::Hex;

use crate::TaijiWorld;

#[when(expr = "I convert commitment {word} into {word} address")]
async fn when_i_convert_commitment_into_address(world: &mut TaijiWorld, commitment_name: String, new_name: String) {
    let commitment = world
        .commitments
        .get(&commitment_name)
        .unwrap_or_else(|| panic!("Commitment {} not found", commitment_name));
    let address = format!("commitment_{}", commitment.to_hex());
    world.addresses.insert(new_name, address);
}
