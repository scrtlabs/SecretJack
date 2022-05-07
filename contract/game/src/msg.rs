use cosmwasm_std::{HumanAddr, Uint128};
use rs_poker::core::Card;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub bank_address: HumanAddr,
    pub bank_code_hash: String,
    pub secret: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GameState {
    NoPlayers,
    PlayerTurn { player_seat: u8, is_first: bool, turn_start_time: u64 },
    DealerTurn,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerHand {
    pub cards: Vec<Card>,
    pub total_value: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PlayerState {
    NotPlaying,
    Bid,
    Hit,
    Hold,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    pub address: HumanAddr,
    pub hand: Option<PlayerHand>,
    pub state: PlayerState,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Table {
    pub players_count: u8,
    pub players: [Player; 6],
    pub dealer_hand: Option<PlayerHand>,
    pub state: GameState,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GameDeck {
    pub deck: Vec<Card>,
    pub next_free_card : u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerResult {
    pub address: HumanAddr,
    pub won : bool,
    pub score : u8,
    pub reward: Uint128,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Scores {
    pub players: [Option<PlayerResult>; 6],
    pub dealer : PlayerResult,
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetUserBalance {
        address: HumanAddr,
    },
    GetTable { },
    GetLastScore { }
}

/// Responses from handle function
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {

}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    GetUserBalance {
        balance: Uint128,
    },
    GetTable {
        table: Vec<u8>,
    },
    GetLastScore {
        last_score: Vec<u8>,
    }
}
