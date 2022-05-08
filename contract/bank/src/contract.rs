use cosmwasm_std::{to_binary, Api, Binary, Env, Extern, HandleResponse, HandleResult, InitResponse, Querier, StdError, StdResult, Storage, Uint128, HumanAddr, CosmosMsg, BankMsg, Coin, WasmMsg};
use crate:: {
    msg::{HandleAnswer, HandleMsg, InitMsg, QueryAnswer, QueryMsg},
    state:: {store_owner, read_owner},
};
use crate::state::{read_game_address, store_game_address, read_bank_address, store_bank_address, set_pending_game_address, is_pending_game_address, unset_pending_game_address};
mod game_msg;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    store_owner(&mut deps.storage, &env.message.sender)?;
    store_bank_address(&mut deps.storage, &env.contract.address)?;

    let game_contract_label = "SJ-Game".to_string() + env.contract.address.as_str();
    let mut messages = vec![];
    messages.extend(vec![CosmosMsg::Wasm(WasmMsg::Instantiate {
        code_id: u64::from(msg.game_contract_code_id),
        callback_code_hash: msg.game_contract_code_hash.to_lowercase(),
        msg: to_binary(&game_msg::InitMsg {
            bank_address: env.contract.address.clone(),
            bank_code_hash: env.contract_code_hash.as_str().to_string(),
            secret: msg.secret,
        })?,
        send: vec![],
        label: game_contract_label,
    })]);

    set_pending_game_address(&mut deps.storage)?;
    Ok(InitResponse {
        messages,
        log: vec![],
    })
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
        data: Some(to_binary(&HandleAnswer::ChangeOwner {})?),
    })
}

pub fn withdraw<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128,
    send_to: HumanAddr,
    all: bool,
) -> HandleResult {
    let to: HumanAddr;

    if all {
        check_owner(deps, &env)?;
        to = read_owner(&deps.storage)?;
    } else {
        if read_game_address(&deps.storage)? != env.message.sender {
            return Err(StdError::generic_err("Only game contract can withdraw funds".to_string()));
        }

        to = send_to;
    }

    let curr_balance = deps.querier.query_balance(read_bank_address(&deps.storage)?, "uscrt")?.amount;
    if amount > curr_balance {
        return Err(StdError::generic_err(format!(
            "Insufficient bank balance, asked for: {}, balance is: {}",
            amount,
            curr_balance
        )));
    }

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: env.contract.address,
            to_address: to,
            amount: vec![Coin::new(amount.u128(), "uscrt")],
        })],
        log: vec![],
        data: None,
    })
}

pub fn update_game_address<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    address: HumanAddr,
) -> HandleResult {
    store_game_address(&mut deps.storage, &address)?;
    unset_pending_game_address(&mut deps.storage)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: None,
    })
}


pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::UpdateGameAddress { address } => update_game_address(deps, address),
        _ => after_initialization_transaction(deps, env, msg),
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
pub fn after_initialization_transaction<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    if is_pending_game_address(&deps.storage)? {
        return Err(StdError::generic_err("Transaction made before contract was fully initialized".to_string()));
    }

    match msg {
        HandleMsg::PayToWinner { amount, to } => withdraw(deps, env, amount,  to,false),
        _ => administrative_transaction(deps, env, msg),
    }
}

pub fn administrative_transaction<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    check_owner(deps, &env)?;
    match msg {
        HandleMsg::ChangeOwner {new_owner} => change_owner(deps, &env, new_owner),
        HandleMsg::EmergencyWithdrawAll {} => withdraw(
            deps,
            env.clone(),
            // In this case, make sure you take all the balance away, don't use the data in the state
            deps.querier.query_balance(env.contract.address, "uscrt")?.amount,
            read_owner(&deps.storage)?,
            true),
        _ => panic!("Used non-administrative transaction as an administrative transaction"),
    }
}

fn get_bank_balance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<Binary> {
    let mut balance = deps.querier.query_balance(read_bank_address(&deps.storage)?, "uscrt")?.amount;
    balance = Uint128::from(balance.u128() - (balance.u128() / 10) as u128); // 10% of the money is stored for the contract owner.
    Ok(to_binary(&QueryAnswer::GetBankBalance { balance })?)
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetBankBalance {} => get_bank_balance(deps),
    }
}
