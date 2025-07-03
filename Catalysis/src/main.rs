//! A program that computes DeFi metrics (ICR, liquidation threshold, LTV) for a BTC-collateralized
//! loan system, committing the results as public values in a zkVM.

#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::SolType;
use alloy_primitives::Address;

use fibonacci_lib::PublicValuesCatalyst;

pub fn main() {
    // ✅ Step 1: Read 20-byte Ethereum address from zkVM input
    let raw_address = sp1_zkvm::io::read::<[u8; 20]>(); // Ethereum-style address (20 bytes)
    let user_address = Address::from_slice(&raw_address); // Convert to alloy Address

    // ✅ Step 2: Read reward amount
    let reward_amount = sp1_zkvm::io::read::<u32>(); // Reward amount in USD

    // ✅ Step 3: Populate the public values struct
    let public_values = PublicValuesCatalyst {
        user_address,
        reward_amount,
    };

    // ✅ Step 4: ABI-encode the public values
    let bytes = PublicValuesCatalyst::abi_encode(&public_values);

    // ✅ Step 5: Commit encoded values to the zkVM
    sp1_zkvm::io::commit_slice(&bytes);
}
