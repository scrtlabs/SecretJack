use std::any::type_name;

use cosmwasm_std::{HumanAddr, StdResult, Storage, Uint128, ReadonlyStorage, StdError};
use cosmwasm_storage::{ReadonlySingleton, Singleton};
use serde::{de::DeserializeOwned, Serialize};
use secret_toolkit::serialization::{Bincode2, Serde};
use serde_json_wasm as serde_json;
use crate::msg::{Table, GameDeck};

static KEY_OWNER: &[u8] = b"owner";
static KEY_BANK_CODE_HASH: &[u8] = b"bankcodehash";
static KEY_GAME_ADDRESS: &[u8] = b"gameaddress";
static KEY_BANK_ADDRESS: &[u8] = b"bankddress";
static KEY_TABLE: &[u8] = b"table";
static KEY_DECK: &[u8] = b"deck";
static KEY_SECRET: &[u8] = b"secret";

pub fn store_owner<S: Storage>(storage: &mut S, data: &HumanAddr) -> StdResult<()> {
    Singleton::new(storage, KEY_OWNER).save(data)?;
    Ok(())
}

pub fn read_owner<S: Storage>(storage: &S) -> StdResult<HumanAddr> {
    ReadonlySingleton::new(storage, KEY_OWNER).load()
}

pub fn store_secret<S: Storage>(storage: &mut S, data: &u64) -> StdResult<()> {
    Singleton::new(storage, KEY_SECRET).save(data)?;
    Ok(())
}

pub fn read_secret<S: Storage>(storage: &S) -> StdResult<u64> {
    ReadonlySingleton::new(storage, KEY_SECRET).load()
}

pub fn store_table<S: Storage>(storage: &mut S, data: &Table) -> StdResult<()> {
    storage.set(KEY_TABLE, &serde_json::to_vec(data).unwrap());
    Ok(())
}

pub fn read_table<S: Storage>(storage: &S) -> StdResult<Table> {
    return serde_json::from_slice(&storage.get(KEY_TABLE).unwrap()).unwrap();
}

pub fn store_deck<S: Storage>(storage: &mut S, data: &GameDeck) -> StdResult<()> {
    storage.set(KEY_DECK, &serde_json::to_vec(data).unwrap());
    Ok(())
}

pub fn read_deck<S: Storage>(storage: &S) -> StdResult<GameDeck> {
    return serde_json::from_slice(&storage.get(KEY_DECK).unwrap()).unwrap();
}

pub fn store_game_address<S: Storage>(storage: &mut S, data: &HumanAddr) -> StdResult<()> {
    Singleton::new(storage, KEY_GAME_ADDRESS).save(data)?;
    Ok(())
}

pub fn read_game_address<S: Storage>(storage: &S) -> StdResult<HumanAddr> {
    ReadonlySingleton::new(storage, KEY_GAME_ADDRESS).load()
}

pub fn add_user_balance<S: Storage>(storage: &mut S, address:HumanAddr , balance: Uint128) -> StdResult<()> {
    let key = "balance".to_string() + address.as_str();
    let loaded_balance : StdResult<Uint128> = load(storage, key.as_bytes());
    let balance = match  loaded_balance{
        Ok(value) => value + balance,
        Err(_) => balance,
    };

    save(storage, key.as_bytes(), &balance)?;

    Ok(())
}

pub fn read_user_balance<S: Storage>(storage: &S, address: HumanAddr) -> StdResult<Uint128> {
    let key = "balance".to_string() + address.as_str();
    Ok(match load(storage, key.as_bytes()) {
        Ok(value) => value,
        Err(_) => Uint128::from(0 as u128),
    })
}

pub fn save<T: Serialize, S: Storage>(storage: &mut S, key: &[u8], value: &T) -> StdResult<()> {
    storage.set(key, &Bincode2::serialize(value)?);
    Ok(())
}

pub fn load<T: DeserializeOwned, S: ReadonlyStorage>(storage: &S, key: &[u8]) -> StdResult<T> {
    Bincode2::deserialize(
        &storage
            .get(key)
            .ok_or_else(|| StdError::not_found(type_name::<T>()))?,
    )
}

pub fn may_load<T: DeserializeOwned, S: ReadonlyStorage>(storage: &S, key: &[u8]) -> StdResult<Option<T>> {
    match storage.get(key) {
        Some(value) => Bincode2::deserialize(&value).map(Some),
        None => Ok(None),
    }
}

pub fn store_bank_address<S: Storage>(storage: &mut S, data: &HumanAddr) -> StdResult<()> {
    Singleton::new(storage, KEY_BANK_ADDRESS).save(data)?;
    Ok(())
}

pub fn read_bank_address<S: Storage>(storage: &S) -> StdResult<HumanAddr> {
    ReadonlySingleton::new(storage, KEY_BANK_ADDRESS).load()
}

pub fn store_player_secret<S: Storage>(storage: &mut S, seat: u8, data: &u64) -> StdResult<()> {
    let key = "secret".to_string() + seat.to_string().as_str();
    Singleton::new(storage, key.as_bytes()).save(data)?;
    Ok(())
}

pub fn read_player_secret<S: Storage>(storage: &S, seat: u8) -> StdResult<u64> {
    let key = "secret".to_string() + seat.to_string().as_str();
    ReadonlySingleton::new(storage, key.as_bytes()).load()
}

pub fn store_bank_code_hash<S: Storage>(storage: &mut S, data: &String) -> StdResult<()> {
    Singleton::new(storage, KEY_BANK_CODE_HASH).save(data)?;
    Ok(())
}

pub fn read_bank_code_hash<S: Storage>(storage: &S) -> StdResult<String> {
    ReadonlySingleton::new(storage, KEY_BANK_CODE_HASH).load()
}

