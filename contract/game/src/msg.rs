use cosmwasm_std::{HumanAddr, Uint128};
use rs_poker::core::Card;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub bank_address: HumanAddr,
    pub owner: HumanAddr,
    pub bank_code_hash: String,
    pub secret: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum GameState {
    NoPlayers,
    PlayerTurn { player_seat: u8, is_first: bool, turn_start_time: u64 },
    DealerTurn,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerDeck {
    pub cards: Vec<Card>,
    pub total_value: u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum PlayerState {
    NotPlaying,
    Bid,
    Hit,
    Hold,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    pub address: HumanAddr,
    pub deck: Option<PlayerDeck>,
    pub state: PlayerState,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PublicTableData {
    pub players_count: u8,
    pub players: [Player; 6],
    pub dealer_deck: Option<PlayerDeck>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Table {
    pub public_table_data: PublicTableData,
    pub state: GameState,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GameDeck {
    pub deck: Vec<Card>,
    pub next_free_card : u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Bid {
        amount: Uint128,
        seat: u8,
    },
    Hold {
        seat: u8,
    },
    Sit {
        secret: u64,
        seat: u8,
    },
    Stand {
        seat: u8,
    },
    Kick {
        target: HumanAddr,
        seat: u8,
    },
    Hit {
        seat: u8,
    },
    // Administrative
    ChangeOwner {
        new_owner: HumanAddr,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetUserBalance {
        address: HumanAddr,
    }
}

/// Responses from handle function
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {

}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryAnswer {
    GetUserBalance {
        balance: Uint128,
    },
}
