use cosmwasm_std::{to_binary, Api, Binary, Env, Extern, HandleResponse, HandleResult, InitResponse, Querier, StdError, StdResult, Storage, Uint128, HumanAddr, CosmosMsg, BankMsg, Coin, WasmMsg, WasmQuery, QueryRequest};
use rand::prelude::SliceRandom;
use rand::SeedableRng;
use rs_poker::core::{Card, Deck, Value};
use crate:: {
    msg::{HandleMsg, InitMsg, QueryAnswer, QueryMsg, Table, PublicTableData, Player, PlayerState, GameDeck, GameState},
    state:: {store_owner, read_owner, store_table, read_table, read_user_balance, read_bank_address, read_bank_code_hash, add_user_balance, store_bank_address, store_bank_code_hash, store_game_address, read_deck, store_deck, read_player_secret, store_player_secret},
};
use rand_chacha::ChaChaRng;
use sha2::{Digest, Sha256};
use crate::msg::PlayerDeck;
use crate::state::{read_secret, store_secret};

mod bank_msg;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    store_owner(&mut deps.storage, &msg.owner)?;
    store_bank_address(&mut deps.storage, &msg.bank_address)?;
    store_game_address(&mut deps.storage, &env.contract.address)?;
    store_bank_code_hash(&mut deps.storage, &msg.bank_code_hash)?;
    store_secret(&mut deps.storage, &msg.secret);
    
    let table = Table {
        public_table_data: PublicTableData {
            players_count: 0,
            players: [Player{ address: Default::default(), deck: None, state: PlayerState::NotPlaying }, Player{ address: Default::default(), deck: None, state: PlayerState::NotPlaying },
                      Player{ address: Default::default(), deck: None, state: PlayerState::NotPlaying }, Player{ address: Default::default(), deck: None, state: PlayerState::NotPlaying },
                      Player{ address: Default::default(), deck: None, state: PlayerState::NotPlaying }, Player{ address: Default::default(), deck: None, state: PlayerState::NotPlaying }, ],
            dealer_deck: None
        },
        state: GameState::NoPlayers
    };

    store_table(&mut deps.storage, &table)?;

    let deck = GameDeck {
        deck: vec![],
        next_free_card: 0
    };

    store_deck(&mut deps.storage, &deck)?;

    let mut messages = vec![];
    messages.extend(vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: msg.bank_address,
        callback_code_hash: msg.bank_code_hash,
        msg: to_binary(&bank_msg::HandleMsg::UpdateGameAddress {
            address: env.contract.address.clone(),
        })?,
        send: vec![],
    })]);


    Ok(InitResponse {
        messages,
        log: vec![],
    })
}

pub fn hold<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    seat: u8,
) -> HandleResult {
    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut table = read_table(&deps.storage)?;
    let mut prev_game_state = table.state.clone();
    match prev_game_state{
        GameState::PlayerTurn { player_seat, is_first , turn_start_time: _} => {
            if player_seat != seat {
                return Err(StdError::generic_err("Player can hold only on his turn"))
            }

            if is_first {
                return Err(StdError::generic_err("Player can't hold before bidding"))
            }
        },
        _ => return Err(StdError::generic_err("Player can hold only on his turn"))
    }

    let player= get_player(&mut table, Some(env.message.sender), seat)?;
    let prev_player_state = player.state.clone();
    match prev_player_state {
        PlayerState::Bid | PlayerState::Hit => {
            player.state = PlayerState::Hold;
        }
        _ => {}
    }

    on_player_state_change(deps, player,  &prev_player_state, &PlayerState::Hold);
    advance_to_next_player(deps, &env, &mut table, seat, false);
    on_game_state_change(deps, &env, &mut table, &prev_game_state, &table.state, &mut msgs);

    store_table(&mut deps.storage, &table)?;

    // msgs.push( CosmosMsg::Wasm(WasmMsg::Execute {
    //     contract_addr: read_bank_address(&deps.storage)?,
    //     callback_code_hash: read_bank_code_hash(&deps.storage)?,
    //     msg: to_binary(&bank_msg::HandleMsg::Withdraw {
    //         amount: Uint128::from(1000000 as u128),
    //         to: env.message.sender,
    //     })?,
    //     send: vec![],
    // }));

    Ok(HandleResponse {
        messages: msgs,
        log: vec![],
        data: None
    })
}

pub fn get_player(
    table: &mut Table,
    address: Option<HumanAddr>,
    seat: u8,
) -> StdResult<&mut Player> {
    if seat >= 6 {
        return Err(StdError::generic_err("No such seat"))
    }

    let player : &mut Player = table.public_table_data.players.get_mut(usize::from(seat)).unwrap();
    match address {
        Some(player_address) => {
            if !player.address.eq(&player_address) {
                return Err(StdError::generic_err("Wrong address for seated player"))
            }
        },
        None => {},
    };

    Ok(player)
}

pub fn get_player_seat(table: &Table, address: HumanAddr) -> StdResult<u8> {
    for seat in 0..6 {
        if table.public_table_data.players[usize::from(seat)].address == address {
            return Ok(seat);
        }
    }

    Err(StdError::not_found("Player"))
}

pub fn add_player<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    table: &mut Table,
    address: HumanAddr,
    seat: u8,
    secret: u64,
) -> StdResult<()> {
    if seat >= 6 {
        return Err(StdError::generic_err("No such seat"))
    }

    let player : &mut Player = table.public_table_data.players.get_mut(usize::from(seat)).unwrap();
    if !player.address.is_empty() {
        return Err(StdError::generic_err("Seat already taken"))
    }

    if get_player_seat(table, address.clone()).is_ok() {
        return Err(StdError::generic_err("Player already seated"))
    }

    player.address = address;
    table.public_table_data.players_count += 1;
    store_player_secret(&mut deps.storage, seat, &secret)?;

    Ok(())
}

pub fn remove_player<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    table: &mut Table,
    address: &HumanAddr,
    seat: u8,
) -> StdResult<()> {
    if seat >= 6 {
        return Err(StdError::generic_err("No such seat"));
    }

    let player : &mut Player = table.public_table_data.players.get_mut(usize::from(seat)).unwrap();
    if !player.address.eq(&address) {
        return Err(StdError::generic_err("Wrong address for seated player"))
    }

    player.address = Default::default();
    table.public_table_data.players_count -= 1;

    let empty_player_secret: u64 = 0;
    store_player_secret(&mut deps.storage, seat, &empty_player_secret)?;

    Ok(())
}

pub fn on_game_state_change<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    table: &mut Table,
    prev_state: &GameState,
    new_state: &GameState,
    out_msgs: &mut Vec<CosmosMsg>
) -> StdResult<()> {
    match prev_state {
        GameState::NoPlayers => {
            match new_state {
                GameState::PlayerTurn { player_seat: _, is_first: _, turn_start_time: _ } => { start_new_round(deps, env, table)?;},
                _ => { return Err(StdError::generic_err("Unexpected state")); }
            }
        }
        GameState::PlayerTurn { player_seat, is_first, turn_start_time } => {
            match new_state {
                GameState::PlayerTurn { player_seat: _, is_first: _, turn_start_time: _ } => Ok(()),
                GameState::DealerTurn => {
                    // TODO: Dealer logic
                },
                _ => { return Err(StdError::generic_err("Unexpected state")); }
            }
        }
        GameState::DealerTurn => { return Err(StdError::generic_err("Unexpected state")); }
    }

    Ok(())
}

pub fn get_card_value(card : &Card) -> u8 {
    match card.value {
        Value::Two => 2,
        Value::Three => 3,
        Value::Four => 4,
        Value::Five => 5,
        Value::Six => 6,
        Value::Seven => 7,
        Value::Eight => 8,
        Value::Nine => 9,
        Value::Ten => 10,
        Value::Jack => 10,
        Value::Queen => 10,
        Value::King => 10,
        Value::Ace => 1
    }
}

pub fn on_player_hit<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    player: &mut Player
) -> StdResult<()> {
    let mut deck = read_deck(&deps.storage)?;
    player.deck.unwrap().cards.push(deck.deck[usize::from(deck.next_free_card)]);
    player.deck.unwrap().total_value += get_card_value(&deck.deck[usize::from(deck.next_free_card)]);
    deck.next_free_card += 1;
    store_deck(&mut deps.storage, &deck);

    if player.deck.unwrap().total_value > 21 {
        player.state = PlayerState::Hold;
    }

    Ok(())
}

pub fn on_player_state_change<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    player: &mut Player,
    prev_state: &PlayerState,
    new_state: &PlayerState
) -> StdResult<()> {
    match prev_state {
        PlayerState::NotPlaying => {
            match new_state {
                PlayerState::Bid => {
                    let mut deck = read_deck(&deps.storage)?;
                    let player_deck = PlayerDeck {
                        cards: vec![deck.deck[usize::from(deck.next_free_card)], deck.deck[usize::from(deck.next_free_card + 1)]],
                        total_value: get_card_value(&deck.deck[usize::from(deck.next_free_card)]) + get_card_value(&deck.deck[usize::from(deck.next_free_card + 1)]),
                    };

                    deck.next_free_card += 2;
                    store_deck(&mut deps.storage, &deck);

                    player.deck = Some(player_deck);
                },
                _ => { return Err(StdError::generic_err("Unexpected state")); }
            }
        },
        PlayerState::Bid => {
            match new_state {
                PlayerState::Hit => {
                    on_player_hit(deps, player);
                },
                PlayerState::Hold => { return Ok(()); },
                _ => { return Err(StdError::generic_err("Unexpected state")); }
            }
        },
        PlayerState::Hit => {
            match new_state {
                PlayerState::Hit => {
                    on_player_hit(deps, player);
                },
                PlayerState::Hold => { return Ok(()); },
                _ => { return Err(StdError::generic_err("Unexpected state")); }
                }
        }
        PlayerState::Hold => { return Err(StdError::generic_err("Unexpected state")); }
            }

    Ok(())
}

pub fn get_first_player_to_play(table: &mut Table) -> StdResult<u8> {
    for seat in 0..6 {
        let player = get_player(table, None, seat)?;
        if !player.address.is_empty() {
            return match player.state {
                PlayerState::NotPlaying => Ok(seat),
                _ => Err(StdError::generic_err("Unexpected player turn")),
            }
        }
    }

    return Err(StdError::not_found("player"));
}

pub fn get_random_seed<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    table: &Table,
) -> StdResult<[u8; 32]> {
    let mut combined_secret = read_secret(&deps.storage)?.to_be_bytes();
    for seat in 0..6 {
        if !table.public_table_data.players[usize::from(seat)].address.is_empty() {
            combined_secret.extend(&read_player_secret(&deps.storage, seat)?.to_be_bytes());
        }
    }

    let seed: [u8; 32] = Sha256::digest(&combined_secret).into();
    Ok(seed)
}

pub fn start_new_round<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    table: &mut Table
) -> StdResult<()> {
    match get_first_player_to_play(table) {
        Ok(seat) => {
            table.state = GameState::PlayerTurn {
                player_seat: seat,
                is_first: true,
                turn_start_time: env.block.time
            };

            let seed = get_random_seed(deps, table)?;
            let mut rng = ChaChaRng::from_seed(seed);
            let mut deck = GameDeck {
                deck: vec![],
                next_free_card: 0
            };
            deck.deck = Deck::default().into_iter().collect();
            deck.deck.shuffle(&mut rng);

            store_deck(&mut deps.storage, &deck);
        },
        Err(_) =>  table.state = GameState::NoPlayers,
    }

    Ok(())
}

pub fn advance_to_next_player<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env, table: &mut Table,
    current_seat: u8,
    should_start_new_round: bool
) -> StdResult<()> {
    let mut next_seat = current_seat;
    for seat in (current_seat+1)..6 {
        let player = get_player(table, None, seat)?;
        if !player.address.is_empty() {
            match player.state {
                PlayerState::NotPlaying => {
                    next_seat = seat;
                    table.state = GameState::PlayerTurn {player_seat: next_seat, is_first: true, turn_start_time: env.block.time }
                },
                _ => return Err(StdError::generic_err("Unexpected player turn")),
            }
        }
    }

    if next_seat == current_seat {
        if is_any_player_holding(table)? {
            table.state = GameState::DealerTurn;
        } else if should_start_new_round {
            start_new_round(deps, env, table);
        }
    }

    Ok(())
}

pub fn is_any_player_holding(table: &mut Table) -> StdResult<bool> {
    let mut found = false;
    for seat in 0..6 {
        let player = get_player(table, None, seat)?;
        if !player.address.is_empty() {
            match player.state {
                PlayerState::Hold => {
                    found = true;
                    break;
                }
                _ => { },
            };
        }
    }

    Ok(found)
}

pub fn sit<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    seat: u8,
    secret: u64,
) -> HandleResult {
    let mut table = read_table(&deps.storage)?;
    let prev_state = table.state.clone();

    let player = get_player(&mut table, None, seat)?;
    if !player.address.is_empty() {
        return Err(StdError::generic_err(format!("Seat {} was already taken", seat)))
    }


    add_player(deps, &mut table,env.message.sender, seat, secret)?;
    match prev_state {
        GameState::NoPlayers => {
            table.state = GameState::PlayerTurn { player_seat: seat, is_first: true, turn_start_time: 0 };
            on_game_state_change(deps, &env, &mut table, &prev_state, &table.state, None);
        },
        _ => {},
    }

    store_table(&mut deps.storage, &table)?;
    Ok(HandleResponse::default())
}

pub fn stand<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    seat: u8,
) -> HandleResult {
    let mut table = read_table(&deps.storage)?;

    match table.state {
        GameState::PlayerTurn { player_seat, is_first , turn_start_time: _} => {
            if player_seat != seat {
                return Err(StdError::generic_err("Player can stand only on his turn"))
            }

            if !is_first {
                return Err(StdError::generic_err("Player can stand only on his first turn"))
            }


        },
        _ => return Err(StdError::generic_err("Player can stand only on his turn"))
    }

    remove_player(deps, &mut table, &env.message.sender, seat)?;
    advance_to_next_player(deps, &env, &mut table, seat, true);

    on_game_state_change(deps, &env, &mut table, &GameState::PlayerTurn { player_seat: seat, is_first: true, turn_start_time: 0 }, &table.state, None);
    store_table(&mut deps.storage, &table)?;
    Ok(HandleResponse::default())
}


pub fn bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    seat: u8,
    amount: Uint128,
) -> HandleResult {
    let mut table = read_table(&deps.storage)?;

    match table.state {
        GameState::PlayerTurn { player_seat, is_first, turn_start_time } => {
            if player_seat != seat {
                return Err(StdError::generic_err("Player can bid only on his turn"))
            }

            if !is_first {
                return Err(StdError::generic_err("Player can bid only on his first turn"))
            }

            table.state = GameState::PlayerTurn {player_seat, is_first: false, turn_start_time};
        },
        _ => return Err(StdError::generic_err("Player can bid only on his turn"))
    }

    if !env.message.sent_funds.contains(&Coin{ denom: "uscrt".to_string(), amount }) {
        return Err(StdError::generic_err(format!(
            "Wrong amount sent. Requested amount is {} uscrt",
            amount,
        )));
    }

    let response = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        callback_code_hash: read_bank_code_hash(&deps.storage)?,
        contract_addr: read_bank_address(&deps.storage)?,
        msg: to_binary(&bank_msg::QueryMsg::GetBankBalance {})?,
    }))?;

    match response {
        bank_msg::QueryAnswer::GetBankBalance { balance } => {
            let max_bid_allowed = ((balance.u128() * 100) / (125 * 6)) as u128;
            if amount.u128() > max_bid_allowed {
                return Err(StdError::generic_err(format!(
                    "Max bid allowed is {} uscrt",
                    max_bid_allowed,
                )));
            }
        }
    }

    let mut player = get_player(&mut table, Some(env.message.sender.clone()), seat)?;
    player.state = PlayerState::Bid;

    add_user_balance(&mut deps.storage, env.message.sender, amount)?;

    table.state = GameState::PlayerTurn {player_seat: seat, is_first: false, turn_start_time: env.block.time};
    on_player_state_change(deps, &mut player, &PlayerState::NotPlaying, &PlayerState::Bid);

    store_table(&mut deps.storage, &table)?;

    Ok(HandleResponse::default())
    // Ok(HandleResponse {
    //     messages:  vec![CosmosMsg::Bank(BankMsg::Send {
    //         from_address: env.contract.address,
    //         to_address: read_bank_address(&deps.storage)?,
    //         amount: vec![Coin::new(amount.u128(), "uscrt")],
    //     })],
    //     log: vec![],
    //     data: None,
    // })
}

pub fn hit<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    seat: u8,
) -> HandleResult {
    let mut table = read_table(&deps.storage)?;

    match table.state {
        GameState::PlayerTurn { player_seat, is_first , turn_start_time: _} => {
            if player_seat != seat {
                return Err(StdError::generic_err("Player can hit only on his turn"))
            }

            if is_first {
                return Err(StdError::generic_err("Player can't hit before bidding"))
            }
        },
        _ => return Err(StdError::generic_err("Player can hit only on his turn"))
    }

    let mut player= get_player(&mut table, Some(env.message.sender), seat)?;
    let prev_player_state = player.state.clone();
    match prev_player_state {
        PlayerState::Bid | PlayerState::Hit => {
            player.state = PlayerState::Hit;
        }
        _ => {}
    }

    on_player_state_change(deps, &mut player,&prev_player_state, &PlayerState::Hit);

    store_table(&mut deps.storage, &table)?;
    Ok(HandleResponse::default())
}

pub fn kick<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    target: HumanAddr,
    seat: u8,
) -> HandleResult {
    let mut table = read_table(&deps.storage)?;

    match table.state {
        GameState::PlayerTurn { player_seat, is_first: _, turn_start_time } => {
            if player_seat != seat {
                return Err(StdError::generic_err("Player can kick only playing player"))
            }

            if (env.block.time - turn_start_time) < 300 { // 5 minutes
                return Err(StdError::generic_err("Player can be kicked only after 5 minutes of idle time"))
            }

        },
        _ => return Err(StdError::generic_err("Player can bid only on his turn"))
    }

    remove_player(deps, &mut table, &target, seat)?;
    advance_to_next_player(deps, &env, &mut table, seat, true);
    store_table(&mut deps.storage, &table)?;

    Ok(HandleResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Hold {seat} => hold(deps, env, seat),
        HandleMsg::Bid {seat, amount} => bid(deps, env, seat, amount),
        HandleMsg::Sit {seat, secret} => sit(deps, env, seat, secret),
        HandleMsg::Stand {seat} => stand(deps, env, seat),
        HandleMsg::Kick { target, seat } => kick(deps, env, target, seat),
        HandleMsg::Hit { seat } => hit(deps, env, seat),
        _ => administrative_transaction(deps, env, msg),
    }
}

fn check_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
) -> StdResult<()> {
    let owner = read_owner(&deps.storage)?;
    if owner != env.message.sender {
        Err(StdError::unauthorized())
    } else {
        Ok(())
    }
}

pub fn change_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: &Env,
    new_owner: HumanAddr,
) -> HandleResult {
    store_owner(&mut deps.storage, &new_owner)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: None,
    })
}

pub fn administrative_transaction<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    check_owner(deps, &env)?;

    match msg {
        HandleMsg::ChangeOwner {new_owner} => change_owner(deps, &env, new_owner),
        _ => panic!("Used non-administrative transaction as an administrative transaction"),
    }
}

fn get_user_balance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: HumanAddr,
) -> StdResult<Binary> {
    let balance = read_user_balance(&deps.storage, address)?;
    Ok(to_binary(&QueryAnswer::GetUserBalance { balance })?)
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetUserBalance {address} => get_user_balance(deps, address),
    }
}
