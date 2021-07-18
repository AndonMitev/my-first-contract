use cosmwasm_std::Coin;
use cosmwasm_std::CosmosMsg;
use cosmwasm_std::{
    Api, BankMsg, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier, StdError,
    StdResult, Storage,
};

use hex::decode;
use sha2::{Digest, Sha256};

use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    if msg.secret_hash.len() != 64 {
        return Err(StdError::generic_err("Invalid secret hash length"));
    }

    let state = State {
        buyer: deps.api.canonical_address(&msg.buyer)?,
        seller: deps.api.canonical_address(&msg.seller)?,
        expiration: msg.expiration,
        value: msg.value,
        secret_hash: msg.secret_hash,
    };

    config(&mut deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Claim { secret } => try_claim(deps, env, secret),
        HandleMsg::Refund {} => try_refund(deps, env),
    }
}

pub fn try_claim<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    secret: String,
) -> StdResult<HandleResponse> {
    if secret.len() != 64 {
        return Err(StdError::generic_err("Invalid secret length"));
    }
    let state = config_read(&deps.storage).load()?;

    let mut hasher = Sha256::default();
    let message: Vec<u8> = decode(secret).expect("Invalid Hex String");

    hasher.update(&message);

    let secret_hash: String = format!("{:x}", hasher.finalize());

    if state.secret_hash != secret_hash {
        return Err(StdError::generic_err("Invalid secret"));
    }

    let balances: Vec<Coin> = deps.querier.query_all_balances(&env.contract.address)?;

    let sum_balance: u128 = balances.iter().map(|b| b.amount.u128()).sum();

    if sum_balance == 0 {
        return Err(StdError::generic_err("Balance is 0"));
    }

    let buyer = deps.api.human_address(&state.buyer)?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: env.contract.address,
            to_address: HumanAddr::from(buyer),
            amount: balances,
        })],
        log: vec![],
        data: None,
    })
}

pub fn try_refund<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let state = config_read(&deps.storage).load()?;

    if env.block.time < (state.expiration as u64) {
        return Err(StdError::generic_err("Swap is not expired"));
    }

    let balances: Vec<Coin> = deps.querier.query_all_balances(&env.contract.address)?;

    let sum_balance: u128 = balances.iter().map(|b| b.amount.u128()).sum();

    if sum_balance == 0 {
        return Err(StdError::generic_err("Balance is 0"));
    }

    let seller = deps.api.human_address(&state.seller)?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: env.contract.address,
            to_address: HumanAddr::from(seller),
            amount: balances,
        })],
        log: vec![],
        data: None,
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    _: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        _ => return Err(StdError::generic_err("Query not implemented")),
    }
}
