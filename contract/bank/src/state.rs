use std::any::type_name;

use cosmwasm_std::{HumanAddr, StdResult, Storage, ReadonlyStorage, StdError};
use cosmwasm_storage::{ReadonlySingleton, Singleton};
use serde::{de::DeserializeOwned, Serialize};
use secret_toolkit::serialization::{Bincode2, Serde};

static KEY_OWNER: &[u8] = b"owner";
static PENDING_GAME_ADDRESS: &[u8] = b"pending";
static KEY_GAME_ADDRESS: &[u8] = b"gameaddress";
static KEY_BANK_ADDRESS: &[u8] = b"bankaddress";

pub fn set_pending_game_address<S: Storage>(storage: &mut S) -> StdResult<()> {
    let marker = "True".to_string();
    Singleton::new(storage, PENDING_GAME_ADDRESS).save(&marker)?;
    Ok(())
}

pub fn unset_pending_game_address<S: Storage>(storage: &mut S) -> StdResult<()> {
    let marker = "False".to_string();
    Singleton::new(storage, PENDING_GAME_ADDRESS).save(&marker)?;
    Ok(())
}

pub fn is_pending_game_address<S: Storage>(storage: &S) -> StdResult<bool> {
    let is_pending : String = ReadonlySingleton::new(storage, PENDING_GAME_ADDRESS).load()?;
    Ok(is_pending.eq("True"))
}

pub fn store_owner<S: Storage>(storage: &mut S, data: &HumanAddr) -> StdResult<()> {
    Singleton::new(storage, KEY_OWNER).save(data)?;
    Ok(())
}

pub fn read_owner<S: Storage>(storage: &S) -> StdResult<HumanAddr> {
    ReadonlySingleton::new(storage, KEY_OWNER).load()
}

pub fn store_game_address<S: Storage>(storage: &mut S, data: &HumanAddr) -> StdResult<()> {
    Singleton::new(storage, KEY_GAME_ADDRESS).save(data)?;
    Ok(())
}

pub fn read_game_address<S: Storage>(storage: &S) -> StdResult<HumanAddr> {
    ReadonlySingleton::new(storage, KEY_GAME_ADDRESS).load()
}

pub fn store_bank_address<S: Storage>(storage: &mut S, data: &HumanAddr) -> StdResult<()> {
    Singleton::new(storage, KEY_BANK_ADDRESS).save(data)?;
    Ok(())
}

pub fn read_bank_address<S: Storage>(storage: &S) -> StdResult<HumanAddr> {
    ReadonlySingleton::new(storage, KEY_BANK_ADDRESS).load()
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

