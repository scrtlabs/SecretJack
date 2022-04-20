use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub game_contract_code_id: u64,
    pub game_contract_code_hash: String,
    pub secret : u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    UpdateGameAddress {
        address: HumanAddr,
    },
    Withdraw {
        amount: Uint128,
        to: HumanAddr,
    },
    WithdrawAll {}, // Dooms day command, will withdraw to an hard-coded address.
    ChangeOwner {
        new_owner: HumanAddr,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetBankBalance {},
}

/// Responses from handle function
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    ChangeOwner {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryAnswer {
    GetBankBalance {
        balance: Uint128,
    },
}
