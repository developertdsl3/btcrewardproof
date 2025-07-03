use alloy_sol_types::{sol, SolType};

sol! {
    /// The public values encoded as a struct that can be deserialized in Solidity.
    struct PublicValuesStruct {
        uint32 icr;
        uint32 collateral_amount;
        uint32 liquidation_threshold;
        uint32 real_time_ltv;
    }

    struct PublicValuesIcr{
        uint32 icr;
        uint32 collateral_amount;
       
}
    struct PublicValuesLiquidation{
        uint32 liquidation_threshold;
        
    }
    struct PublicValuesLtv{
        uint32 real_time_ltv;
        
    }

    struct PublicValuesAddress {
        address user_address;
    }

    struct PublicValuesCatalyst{
        address user_address;
        uint32 reward_amount;
    }

    
}

const SCALE: u64 = 100; // Supports two decimal places

/// Calculate the ICR and USD value of the collateral.
pub fn calculate_icr(
    collateral_amount: u32,
    debt_amount: u32,
    btc_price_usd: u32,
) -> (u32, u32) {

    let collateral_usd = (collateral_amount as u64 * btc_price_usd as u64) as u32;

    let icr = if debt_amount == 0 {
        0
    } else {
        ((collateral_amount as u64 * btc_price_usd as u64 * SCALE) / debt_amount as u64) as u32
    };

    (icr, collateral_usd)
}

/// Calculate the liquidation threshold from collateral value and minimum ICR.
pub fn calculate_liquidation_threshold(
    collateral_amount: u32,
    btc_price_usd: u32,
    min_icr: u32,
) -> u32 {
    if min_icr == 0 {
        return 0;
    }

    let collateral_value = collateral_amount as u64 * btc_price_usd as u64;
    (collateral_value / min_icr as u64) as u32
}

/// Calculate real-time LTV (Loan-to-Value ratio).
pub fn real_time_ltv(
    debt_amount: u32,
    collateral_amount: u32,
    btc_price_usd: u32,
) -> u32 {
    if collateral_amount == 0 || btc_price_usd == 0 {
        return 0;
    }
    let collateral_value = collateral_amount as u64 * btc_price_usd as u64;
    ((debt_amount as u64 * SCALE) / collateral_value) as u32
}


