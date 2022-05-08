use cosmwasm_std::{to_binary, Api, Binary, Env, Extern, HandleResponse, HandleResult, InitResponse, Querier, StdError, StdResult, Storage, Uint128, HumanAddr, CosmosMsg, BankMsg, Coin, WasmMsg, WasmQuery, QueryRequest, debug_print};
use rand::prelude::SliceRandom;
use rand::SeedableRng;
use rs_poker::core::{Card, Deck, Value};
use crate:: {
    msg::{PlayerHand, HandleMsg, InitMsg, QueryAnswer, QueryMsg, Table, Player, PlayerState, GameDeck, GameState, Scores, PlayerResult},
    state:: {read_raw_scores, store_scores, read_secret, zero_user_balance, store_secret, store_table, read_table, read_raw_table, read_user_balance, read_bank_address, read_bank_code_hash, add_user_balance, store_bank_address, store_bank_code_hash, store_game_address, read_deck, store_deck, read_player_secret, store_player_secret},
};
use rand_chacha::ChaChaRng;
use sha2::{Digest, Sha256};

mod bank_msg;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    store_bank_address(&mut deps.storage, &msg.bank_address)?;
    store_game_address(&mut deps.storage, &env.contract.address)?;
    store_bank_code_hash(&mut deps.storage, &msg.bank_code_hash)?;
    store_secret(&mut deps.storage, &msg.secret)?;

    let table = Table {
        players_count: 0,
        players: [Player{ address: Default::default(), hand: None, state: PlayerState::NotPlaying }, Player{ address: Default::default(), hand: None, state: PlayerState::NotPlaying },
                  Player{ address: Default::default(), hand: None, state: PlayerState::NotPlaying }, Player{ address: Default::default(), hand: None, state: PlayerState::NotPlaying },
                  Player{ address: Default::default(), hand: None, state: PlayerState::NotPlaying }, Player{ address: Default::default(), hand: None, state: PlayerState::NotPlaying }, ],
        dealer_hand: None,
        state: GameState::NoPlayers
    };

    store_table(&mut deps.storage, &table)?;

    let scores = Scores {
        players: [None, None, None, None,None,None],
        dealer: PlayerResult {
            address: HumanAddr::default(),
            won: false,
            score: 0,
            reward: Uint128::from(0 as u128)
        }
    };

    store_scores(&mut deps.storage, &scores)?;

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
    let prev_game_state = table.state.clone();
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

    let player= get_player(&mut table, Some(&env.message.sender), seat)?;
    let prev_player_state = player.state.clone();
    match prev_player_state {
        PlayerState::Bid | PlayerState::Hit => {
            player.state = PlayerState::Hold;
        }
        _ => {}
    }

    on_player_state_change(deps, player,  &prev_player_state, &PlayerState::Hold)?;
    advance_to_next_player(deps, &env, &mut table, seat, false)?;
    on_game_state_change(deps, &env, &mut table, &prev_game_state, &mut msgs)?;

    store_table(&mut deps.storage, &table)?;

    Ok(HandleResponse {
        messages: msgs,
        log: vec![],
        data: None
    })
}

pub fn get_player<'a>(
    table: &'a mut Table,
    address: Option<&HumanAddr>,
    seat: u8,
) -> StdResult<&'a mut Player> {
    if seat >= 6 {
        return Err(StdError::generic_err("No such seat"))
    }

    let player : &mut Player = table.players.get_mut(usize::from(seat)).unwrap();
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

pub fn get_player_seat(table: &Table, address: &HumanAddr) -> StdResult<u8> {
    for seat in 0..6 {
        if table.players[usize::from(seat)].address == address.clone() {
            return Ok(seat);
        }
    }

    Err(StdError::not_found("Player"))
}

pub fn add_player<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    table: &mut Table,
    address: &HumanAddr,
    seat: u8,
    secret: u64,
) -> StdResult<()> {
    if seat >= 6 {
        return Err(StdError::generic_err("No such seat"))
    }

    if get_player_seat(&*table, address).is_ok() {
        return Err(StdError::generic_err("Player already seated"))
    }

    let player : &mut Player = table.players.get_mut(usize::from(seat)).unwrap();
    if !player.address.is_empty() {
        return Err(StdError::generic_err("Seat already taken"))
    }

    player.address = address.clone();
    table.players_count += 1;
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

    let player : &mut Player = table.players.get_mut(usize::from(seat)).unwrap();
    if !player.address.eq(&address) {
        return Err(StdError::generic_err("Wrong address for seated player"))
    }

    player.address = Default::default();
    player.hand = None;
    player.state = PlayerState::NotPlaying;

    table.players_count -= 1;

    zero_user_balance(&mut deps.storage, &player.address)?;

    let empty_player_secret: u64 = 0;
    store_player_secret(&mut deps.storage, seat, &empty_player_secret)?;

    Ok(())
}

pub fn get_player_score(deck: &PlayerHand) -> u8 {
    if deck.total_value > 21 {
        return deck.total_value;
    }

    let mut result: u8 = deck.total_value;

    for card in deck.cards.iter() {
        if card.value == Value::Ace {
            if result + 10 <= 21 {
                result += 10;
            }
        }
    }

    return result;
}

pub fn play_dealer<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    table: &mut Table
) -> StdResult<()> {
    let mut deck = read_deck(&deps.storage)?;
    let mut dealer_deck = PlayerHand { cards: vec![], total_value: 0 };
    debug_print("Playing dealer");
    while get_player_score(&dealer_deck) < 17 {
        debug_print(format!("Dealer score is {}", get_player_score(&dealer_deck)));
        dealer_deck.cards.push(deck.deck[usize::from(deck.next_free_card)]);
        dealer_deck.total_value += get_card_value(&deck.deck[usize::from(deck.next_free_card)]);
        deck.next_free_card += 1;
    }



    store_deck(&mut deps.storage, &deck)?;
    table.dealer_hand = Some(dealer_deck);

    Ok(())
}

pub fn game_roundup<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    table: &mut Table,
    out_msgs: &mut Vec<CosmosMsg>,
) -> StdResult<()> {
    let dealer_score = get_player_score(table.dealer_hand.as_ref().unwrap());
    let mut scores = Scores { players: [None,None,None,None,None,None],
        dealer: PlayerResult{ address: HumanAddr::default(), won: false, score: dealer_score, reward:Uint128::from(0 as u128) } };

    for seat in 0..6 {
        let player = get_player(table, None, seat)?;
        if player.address.is_empty() {
            continue;
        }

        match player.state {
            PlayerState::Hold => {
                let player_balance = read_user_balance(&deps.storage, &player.address)?;
                let player_score = get_player_score(player.hand.as_ref().unwrap());
                if (player_score <= 21) && (( player_score > dealer_score) || dealer_score > 21)  {
                    let mut player_award = player_balance.u128();
                    if player_score == 21 {
                        player_award = ((player_award * 125) / 100) as u128;
                    }
                    out_msgs.push( CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: read_bank_address(&deps.storage)?,
                        callback_code_hash: read_bank_code_hash(&deps.storage)?,
                        msg: to_binary(&bank_msg::HandleMsg::PayToWinner {
                            amount: Uint128::from(player_award),
                            to: player.address.clone(),
                        })?,
                        send: vec![],
                    }));
                    out_msgs.push(CosmosMsg::Bank(BankMsg::Send {
                        from_address: env.contract.address.clone(),
                        to_address: player.address.clone(),
                        amount: vec![Coin::new(player_balance.u128(), "uscrt")],
                    }));

                    scores.players[usize::from(seat)] = Some(PlayerResult{ address: player.address.clone(), won: true, score: player_score, reward: Uint128::from(player_award) });
                } else {
                    out_msgs.push(CosmosMsg::Bank(BankMsg::Send {
                        from_address: env.contract.address.clone(),
                        to_address: read_bank_address(&deps.storage)?,
                        amount: vec![Coin::new(player_balance.u128(), "uscrt")],
                    }));

                    scores.players[usize::from(seat)] = Some(PlayerResult{ address: player.address.clone(), won: false, score: player_score , reward: player_balance});
                }

                zero_user_balance(&mut deps.storage, &player.address)?;
            }
            _ => { continue; }
        }

        player.state = PlayerState::NotPlaying;
    }

    store_scores(&mut deps.storage, &scores)?;

    Ok(())
}

pub fn on_game_state_change<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    table: &mut Table,
    prev_state: &GameState,
    out_msgs: &mut Vec<CosmosMsg>
) -> StdResult<()> {
    match prev_state {
        GameState::NoPlayers => {
            match table.state {
                GameState::PlayerTurn { player_seat: _, is_first: _, turn_start_time: _ } => { start_new_round(deps, env, table)?;},
                _ => { return Err(StdError::generic_err(format!("Unexpected state from NoPlayers to {:?}", table.state))); }
            }
        }
        GameState::PlayerTurn { player_seat: _, is_first: _, turn_start_time: _ } => {
            match table.state {
                GameState::PlayerTurn { player_seat: _, is_first: _, turn_start_time: _ } => { return Ok(()); },
                GameState::DealerTurn => {

                    play_dealer(deps, table)?;
                    game_roundup(deps, env, table, out_msgs)?;
                    start_new_round(deps, env, table)?;

                    return Ok(());
                },
                GameState::NoPlayers => { return Ok(()); }
            }
        }
        GameState::DealerTurn => { return Err(StdError::generic_err(format!("Unexpected state from DealerTurn to {:?}", table.state))); }
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
    let player_hand : &mut PlayerHand = player.hand.as_mut().unwrap();
    if player_hand.total_value >= 21 {
        return Err(StdError::generic_err(format!("Player can't hit when having this score: {}", player_hand.total_value)));
    }

    player_hand.cards.push(deck.deck[usize::from(deck.next_free_card)]);
    player_hand.total_value += get_card_value(&deck.deck[usize::from(deck.next_free_card)]);
    debug_print(format!("Player hit new card total_value is {} cards count is {} next_free card is {} card value {}",
                        player_hand.total_value, player_hand.cards.len(), deck.next_free_card,
                        get_card_value(&deck.deck[usize::from(deck.next_free_card)]))
    );
    deck.next_free_card += 1;
    store_deck(&mut deps.storage, &deck)?;

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
                    debug_print(format!("Deck size is: {} next free card is: {}, ", deck.deck.len(), deck.next_free_card));
                    let player_deck = PlayerHand {
                        cards: vec![deck.deck[usize::from(deck.next_free_card)], deck.deck[usize::from(deck.next_free_card + 1)]],
                        total_value: get_card_value(&deck.deck[usize::from(deck.next_free_card)]) + get_card_value(&deck.deck[usize::from(deck.next_free_card + 1)]),
                    };

                    deck.next_free_card += 2;
                    store_deck(&mut deps.storage, &deck)?;

                    player.hand = Some(player_deck);
                },
                _ => { return Err(StdError::generic_err(format!("Unexpected state from NotPlaying to {:?}", player.state))); }
            }
        },
        PlayerState::Bid => {
            match new_state {
                PlayerState::Hit => {
                    on_player_hit(deps, player)?;
                },
                PlayerState::Hold => { return Ok(()); },
                _ => { return Err(StdError::generic_err(format!("Unexpected state from Bid to {:?}", new_state))); }
            }
        },
        PlayerState::Hit => {
            match new_state {
                PlayerState::Hit => {
                    on_player_hit(deps, player)?;
                },
                PlayerState::Hold => { return Ok(()); },
                _ => { return Err(StdError::generic_err(format!("Unexpected state from Hit to {:?}", new_state))); }
                }
        }
        PlayerState::Hold => { return Err(StdError::generic_err(format!("Unexpected state from Hold to {:?}", player.state))); }
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
    secret: u64,
) -> StdResult<[u8; 32]> {
    let mut combined_secret : Vec<u8> = vec![];

    combined_secret.extend(&secret.to_be_bytes());
    for seat in 0..6 {
        if !table.players[usize::from(seat)].address.is_empty() {
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
    for seat in 0..6 {
        let player = get_player(table, None, seat)?;
        player.hand = None;
    }

    match get_first_player_to_play(table) {
        Ok(seat) => {
            table.state = GameState::PlayerTurn {
                player_seat: seat,
                is_first: true,
                turn_start_time: env.block.time
            };

            let secret = read_secret(&deps.storage)?;
            let seed = get_random_seed(deps, table, secret)?;
            store_secret(&mut deps.storage, &(secret + 1))?;

            let mut rng = ChaChaRng::from_seed(seed);
            let mut deck = GameDeck {
                deck: vec![],
                next_free_card: 0
            };
            deck.deck = Deck::default().into_iter().collect();
            deck.deck.shuffle(&mut rng);

            store_deck(&mut deps.storage, &deck)?;
        },
        Err(_) =>  {
            table.dealer_hand = None;
            table.state = GameState::NoPlayers;
        },
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
            start_new_round(deps, env, table)?;
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

    let mut msgs: Vec<CosmosMsg> = vec![];

    add_player(deps, &mut table,&env.message.sender, seat, secret)?;
    match prev_state {
        GameState::NoPlayers => {
            table.state = GameState::PlayerTurn { player_seat: seat, is_first: true, turn_start_time: 0 };
            on_game_state_change(deps, &env, &mut table, &prev_state, &mut msgs)?;
        },
        _ => {},
    }

    store_table(&mut deps.storage, &table)?;
    Ok(HandleResponse {
        messages: msgs,
        log: vec![],
        data: None
    })
}

pub fn stand<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    seat: u8,
) -> HandleResult {
    let mut table = read_table(&deps.storage)?;
    let mut msgs: Vec<CosmosMsg> = vec![];

    let player = get_player(&mut table, None, seat)?;
    if player.address.is_empty() {
        return Err(StdError::generic_err(format!("Seat {} is empty", seat)))
    }

    if player.address != env.message.sender {
        return Err(StdError::generic_err(format!("Player is not seated in seat {}", seat)))
    }

    match player.state {
        PlayerState::NotPlaying => {}
        _ => { return Err(StdError::generic_err("Player can't stand while playing")) }
    }

    remove_player(deps, &mut table, &env.message.sender, seat)?;
    advance_to_next_player(deps, &env, &mut table, seat, true)?;

    on_game_state_change(deps, &env, &mut table, &GameState::PlayerTurn { player_seat: seat, is_first: true, turn_start_time: 0 }, &mut msgs)?;
    store_table(&mut deps.storage, &table)?;

    Ok(HandleResponse {
        messages: msgs,
        log: vec![],
        data: None
    })
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

            let mut deck = read_deck(&deps.storage)?;
            table.dealer_hand = Some(PlayerHand { cards: vec![], total_value: 0 });
            table.dealer_hand.as_mut().unwrap().cards.push(deck.deck[usize::from(deck.next_free_card)]);
            table.dealer_hand.as_mut().unwrap().total_value += get_card_value(&deck.deck[usize::from(deck.next_free_card)]);
            deck.next_free_card += 1;
            store_deck(&mut deps.storage, &deck)?;

        },
        _ => return Err(StdError::generic_err("Player can bid only on his turn"))
    }

    if amount == Uint128::from(0 as u128) {
        return Err(StdError::generic_err("Amount should be set"));
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

    let player = get_player(&mut table, Some(&env.message.sender), seat)?;
    player.state = PlayerState::Bid;

    add_user_balance(&mut deps.storage, &env.message.sender, amount)?;

    on_player_state_change(deps, player, &PlayerState::NotPlaying, &PlayerState::Bid)?;
    table.state = GameState::PlayerTurn {player_seat: seat, is_first: false, turn_start_time: env.block.time};


    store_table(&mut deps.storage, &table)?;

    Ok(HandleResponse::default())
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

    let mut player= get_player(&mut table, Some(&env.message.sender), seat)?;
    let prev_player_state = player.state.clone();
    match prev_player_state {
        PlayerState::Bid | PlayerState::Hit => {
            player.state = PlayerState::Hit;
        }
        _ => {}
    }

    on_player_state_change(deps, &mut player,&prev_player_state, &PlayerState::Hit)?;

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
    let prev_state = table.state.clone();
    let mut msgs: Vec<CosmosMsg> = vec![];

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
    advance_to_next_player(deps, &env, &mut table, seat, true)?;

    on_game_state_change(deps, &env, &mut table, &prev_state, &mut msgs)?;
    store_table(&mut deps.storage, &table)?;

    Ok(HandleResponse {
        messages: msgs,
        log: vec![],
        data: None
    })
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
    }
}

fn get_user_balance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: &HumanAddr,
) -> StdResult<Binary> {
    let balance = read_user_balance(&deps.storage, address)?;
    Ok(to_binary(&QueryAnswer::GetUserBalance { balance })?)
}

fn get_table_data<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<Binary> {
    let table = read_raw_table(&deps.storage)?;
    Ok(to_binary(&QueryAnswer::GetTable { table })?)
}

fn get_last_score<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<Binary> {
    let scores = read_raw_scores(&deps.storage)?;
    Ok(to_binary(&QueryAnswer::GetLastScore { last_score: scores })?)
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetUserBalance {address} => get_user_balance(deps, &address),
        QueryMsg::GetTable { } => get_table_data(deps),
        QueryMsg::GetLastScore { } => get_last_score(deps),
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{Coin, from_binary};
    use serde::Serialize;
    use crate::msg::{HandleMsg, InitMsg, QueryMsg, HandleAnswer, QueryAnswer};
    use crate::msg::GameState::NoPlayers;

    fn validate_game_state(table: &Table, expected_state: GameState) -> bool {
        match table.state {
            GameState::NoPlayers =>
                match expected_state {
                    GameState::NoPlayers => true,
                    GameState::PlayerTurn { .. } => false,
                    GameState::DealerTurn => false,
                },
            GameState::PlayerTurn { player_seat, is_first, turn_start_time: _ } =>
                match expected_state {
                    GameState::NoPlayers => false,
                    GameState::PlayerTurn { player_seat: e_player_seat, is_first: e_is_first, turn_start_time: _ } =>
                        (player_seat == e_player_seat) && (is_first == e_is_first),
                    GameState::DealerTurn => false,
                },
            GameState::DealerTurn =>
                match expected_state {
                    GameState::NoPlayers => false,
                    GameState::PlayerTurn { .. } => false,
                    GameState::DealerTurn => true,
                },
        }
    }

    #[test]
    fn test_sit() {
        let mut deps = mock_dependencies(20,  &[Coin {
            denom: "uscrt".to_string(),
            amount: Uint128(10000000),
        }]);

        let msg = InitMsg{
            bank_address: Default::default(),
            bank_code_hash: "".to_string(),
            secret: 1234
        };
        let env = mock_env("sit", &[]);

        let _init_res = init(&mut deps, env.clone(), msg).unwrap();

        // No previous players
        assert!(validate_game_state(&read_table(&deps.storage).unwrap(), GameState::NoPlayers));
        let msg = HandleMsg::Sit { secret: 4321, seat: 4 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_err(), "Seat should have been already taken");

        assert!(validate_game_state(&read_table(&deps.storage).unwrap(), GameState::PlayerTurn {
            player_seat: 4,
            is_first: true,
            turn_start_time: 0
        }));

        let msg = HandleMsg::Sit { secret: 4321, seat: 5 };

        let nenv = mock_env("new_sit", &[]);
        let res = handle(&mut deps, nenv.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", format!("{:?}", res.unwrap_err())));

        assert!(validate_game_state(&read_table(&deps.storage).unwrap(), GameState::PlayerTurn {
            player_seat: 4,
            is_first: true,
            turn_start_time: 0
        }));
    }

    #[test]
    fn test_stand() {
        let mut deps = mock_dependencies(20,  &[Coin {
            denom: "uscrt".to_string(),
            amount: Uint128(10000000),
        }]);

        let msg = InitMsg{
            bank_address: Default::default(),
            bank_code_hash: "".to_string(),
            secret: 1234
        };
        let env = mock_env("unsit", &[]);

        let _init_res = init(&mut deps, env.clone(), msg).unwrap();


        // No other players
        let msg = HandleMsg::Sit { secret: 4321, seat: 4 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        assert!(validate_game_state(&read_table(&deps.storage).unwrap(), GameState::PlayerTurn {
            player_seat: 4,
            is_first: true,
            turn_start_time: 0
        }));

        let msg = HandleMsg::Stand { seat: 5 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_err(), "Seat wasn't occupied");

        let msg = HandleMsg::Stand { seat: 4 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        assert!(validate_game_state(&read_table(&deps.storage).unwrap(), GameState::NoPlayers));

        let msg = HandleMsg::Sit { secret: 4321, seat: 4 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        let nenv = mock_env("new_unsit", &[]);
        let msg = HandleMsg::Sit { secret: 4321, seat: 5 };

        let res = handle(&mut deps, nenv.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        let msg = HandleMsg::Stand { seat: 4 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        assert!(validate_game_state(&read_table(&deps.storage).unwrap(), GameState::PlayerTurn {
            player_seat: 5,
            is_first: true,
            turn_start_time: 0
        }));
    }

    #[test]
    fn test_stand() {
        let mut deps = mock_dependencies(20,  &[Coin {
            denom: "uscrt".to_string(),
            amount: Uint128(10000000),
        }]);

        let msg = InitMsg{
            bank_address: Default::default(),
            bank_code_hash: "".to_string(),
            secret: 1234
        };
        let env = mock_env("unsit", &[]);

        let _init_res = init(&mut deps, env.clone(), msg).unwrap();


        // No other players
        let msg = HandleMsg::Sit { secret: 4321, seat: 4 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        assert!(validate_game_state(&read_table(&deps.storage).unwrap(), GameState::PlayerTurn {
            player_seat: 4,
            is_first: true,
            turn_start_time: 0
        }));

        let msg = HandleMsg::Stand { seat: 5 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_err(), "Seat wasn't occupied");

        let msg = HandleMsg::Stand { seat: 4 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        assert!(validate_game_state(&read_table(&deps.storage).unwrap(), GameState::NoPlayers));

        let msg = HandleMsg::Sit { secret: 4321, seat: 4 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        let nenv = mock_env("new_unsit", &[]);
        let msg = HandleMsg::Sit { secret: 4321, seat: 5 };

        let res = handle(&mut deps, nenv.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        let msg = HandleMsg::Stand { seat: 4 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        assert!(validate_game_state(&read_table(&deps.storage).unwrap(), GameState::PlayerTurn {
            player_seat: 5,
            is_first: true,
            turn_start_time: 0
        }));
    }

    #[test]
    fn test_stand() {
        let mut deps = mock_dependencies(20,  &[Coin {
            denom: "uscrt".to_string(),
            amount: Uint128(10000000),
        }]);

        let msg = InitMsg{
            bank_address: Default::default(),
            bank_code_hash: "".to_string(),
            secret: 1234
        };
        let env = mock_env("unsit", &[]);

        let _init_res = init(&mut deps, env.clone(), msg).unwrap();


        // No other players
        let msg = HandleMsg::Sit { secret: 4321, seat: 4 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        assert!(validate_game_state(&read_table(&deps.storage).unwrap(), GameState::PlayerTurn {
            player_seat: 4,
            is_first: true,
            turn_start_time: 0
        }));

        let msg = HandleMsg::Stand { seat: 5 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_err(), "Seat wasn't occupied");

        let msg = HandleMsg::Stand { seat: 4 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        assert!(validate_game_state(&read_table(&deps.storage).unwrap(), GameState::NoPlayers));

        let msg = HandleMsg::Sit { secret: 4321, seat: 4 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        let nenv = mock_env("new_unsit", &[]);
        let msg = HandleMsg::Sit { secret: 4321, seat: 5 };

        let res = handle(&mut deps, nenv.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        let msg = HandleMsg::Stand { seat: 4 };

        let res = handle(&mut deps, env.clone(), msg.clone());
        assert!(res.is_ok(), format!("{:?}", res.unwrap_err()));

        assert!(validate_game_state(&read_table(&deps.storage).unwrap(), GameState::PlayerTurn {
            player_seat: 5,
            is_first: true,
            turn_start_time: 0
        }));
    }

    //Query tests
    #[test]
    fn test_get_user_balance() {
        let mut deps = mock_dependencies(20,  &[Coin {
            denom: "uscrt".to_string(),
            amount: Uint128(10000000),
        }]);

        let msg = InitMsg{
            bank_address: Default::default(),
            bank_code_hash: "".to_string(),
            secret: 0
        };
        let env = mock_env("user_balance", &[]);

        let _init_res = init(&mut deps, env, msg).unwrap();

        let msg = HandleMsg::GetCookie{};
        let env = mock_env("cookie", &[]);
        let res: HandleAnswer = from_binary(&handle(&mut deps, env.clone(), msg).unwrap().data.unwrap()).unwrap();

        match res {
            HandleAnswer::GetCookie{ status: _, ref cookie } => {
                assert!(ensure_success(&res));
                assert_eq!(&env.message.sender.to_string(), cookie);

                let msg = QueryMsg::GetUserCalculations {user_cookie: String::from(cookie)};

                let q_res: QueryAnswer = from_binary(&query(&mut deps,  msg).unwrap()).unwrap();
                match q_res {
                    QueryAnswer::GetUserCalculations {ref status, calculations} => {
                        assert_eq!(0, calculations.len());
                        assert!(!status.is_empty());
                    }
                }

                let msg = HandleMsg::Add { eq: EquationVariables { x: Uint128(10 as u128), y: Uint128(20 as u128) } };
                handle(&mut deps, env, msg).ok();

                let msg = QueryMsg::GetUserCalculations {user_cookie: String::from(cookie)};

                let q_res: QueryAnswer = from_binary(&query(&mut deps,  msg).unwrap()).unwrap();
                match q_res {
                    QueryAnswer::GetUserCalculations {ref status, calculations} => {
                        assert_eq!(1, calculations.len());
                        assert!(status.is_empty());
                    }
                }
            }
            _ => panic!("HandleAnswer for GetCookie should be GetCookie"),
        }
    }
}


