#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    coin, entry_point, to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult,
};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{
        ExecuteMsg, InstantiateMsg, QueryHighestBidResponse, QueryMsg, QueryTotalBidResponse,
        QueryWinnerResponse,
    },
    state::{Bid, BiddingRound, Player, State, BIDDING_ROUNDS, BIDS, PLAYERS, STATE},
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:bidding-platform";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DENOM: &str = "uatom";
const FEE_DIV: u128 = 20;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let value = match info.funds.iter().find(|x| x.denom == DENOM) {
        Some(x) => x.to_owned(),
        _ => coin(0, DENOM),
    };

    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut fee = 0_u128;

    let owner = match msg.owner {
        Some(x) => {
            let round_owner = deps.api.addr_validate(&x)?;

            if info.sender != round_owner {
                fee = value.amount.u128() / FEE_DIV;

                msgs.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: round_owner.to_string(),
                    amount: vec![coin(fee, DENOM)],
                }));
            }

            round_owner
        }
        _ => info.sender.clone(),
    };

    let init_bid = Bid {
        value: coin(value.amount.u128() - fee, DENOM),
    };

    let player = Player {
        retractable_amount: coin(init_bid.value.amount.u128(), DENOM),
    };

    let bidding_round = BiddingRound {
        top_bidder: info.sender.clone(),
        highest_bid: init_bid.value.clone(),
        winner: None,
    };

    let initial_state = State {
        owner,
        round_number: 0,
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    BIDS.save(deps.storage, &info.sender, &vec![init_bid])?;
    PLAYERS.save(deps.storage, &info.sender, &player)?;
    BIDDING_ROUNDS.save(deps.storage, initial_state.round_number, &bidding_round)?;
    STATE.save(deps.storage, &initial_state)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Bid {} => bid(deps, info),
        ExecuteMsg::Close {} => close(deps, info),
        ExecuteMsg::Retract { receiver } => retract(deps, info, receiver),
    }
}

pub fn bid(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let value = match info.funds.iter().find(|x| x.denom == DENOM) {
        Some(x) => x.to_owned(),
        _ => coin(0, DENOM),
    };
    let fee = value.amount.u128() / FEE_DIV;

    let state = STATE.load(deps.storage)?;
    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: state.owner.to_string(),
        amount: vec![coin(fee, DENOM)],
    });

    let bid = Bid {
        value: coin(value.amount.u128() - fee, DENOM),
    };
    let mut bids = match BIDS.load(deps.storage, &info.sender) {
        Ok(x) => x,
        _ => vec![],
    };

    let mut player = match PLAYERS.load(deps.storage, &info.sender) {
        Ok(x) => x,
        _ => Player {
            retractable_amount: coin(0, DENOM),
        },
    };

    let mut bidding_round = BIDDING_ROUNDS.load(deps.storage, state.round_number)?;
    let bids_sum = bid.value.amount.u128()
        + bids
            .iter()
            .fold(0_u128, |acc, cur| acc + cur.value.amount.u128());

    if bids_sum <= bidding_round.highest_bid.amount.u128() {
        return Err(ContractError::SmallBid {});
    }

    bidding_round.top_bidder = info.sender.clone();
    bidding_round.highest_bid = coin(bids_sum, DENOM);
    bids.push(bid);
    player.retractable_amount = coin(bids_sum, DENOM);

    BIDS.save(deps.storage, &info.sender, &bids)?;
    PLAYERS.save(deps.storage, &info.sender, &player)?;
    BIDDING_ROUNDS.save(deps.storage, state.round_number, &bidding_round)?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("method", "bid"))
}

pub fn close(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    let mut bidding_round = BIDDING_ROUNDS.load(deps.storage, state.round_number)?;

    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: state.owner.to_string(),
        amount: vec![coin(bidding_round.highest_bid.amount.u128(), DENOM)],
    });

    bidding_round.winner = Some(bidding_round.top_bidder.clone());
    BIDDING_ROUNDS.save(deps.storage, state.round_number, &bidding_round)?;

    // state.round_number += 1;
    // BIDDING_ROUNDS.save(
    //     deps.storage,
    //     state.round_number,
    //     &BiddingRound {
    //         top_bidder: state.owner.clone(),
    //         highest_bid: coin(0, DENOM),
    //         winner: None,
    //     },
    // )?;

    // STATE.save(deps.storage, &state)?;

    let player = Player {
        retractable_amount: coin(0, DENOM),
    };
    PLAYERS.save(deps.storage, &bidding_round.top_bidder, &player)?;

    BIDS.clear(deps.storage);

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("method", "close"))
}

pub fn retract(
    deps: DepsMut,
    info: MessageInfo,
    receiver: Option<String>,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let bidding_round = BIDDING_ROUNDS.load(deps.storage, state.round_number)?;

    if bidding_round.winner.is_none() {
        return Err(ContractError::BiddingIsOpen {});
    }

    let player = match PLAYERS.load(deps.storage, &info.sender) {
        Ok(x) => x,
        _ => {
            return Err(ContractError::PlayerIsNotFound {});
        }
    };

    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: match receiver {
            Some(x) => deps.api.addr_validate(&x)?.to_string(),
            _ => info.sender.to_string(),
        },
        amount: vec![player.retractable_amount],
    });

    let player = Player {
        retractable_amount: coin(0, DENOM),
    };
    PLAYERS.save(deps.storage, &bidding_round.top_bidder, &player)?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("method", "retract"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryTotalBid { address } => query_total_bid(deps, address),
        QueryMsg::QueryHighestBid {} => query_highest_bid(deps),
        QueryMsg::QueryWinner {} => query_winner(deps),
    }
}

pub fn query_total_bid(deps: Deps, address: String) -> StdResult<Binary> {
    let bids = match BIDS.load(deps.storage, &deps.api.addr_validate(&address)?) {
        Ok(x) => x,
        _ => vec![],
    };

    let bids_sum = bids
        .iter()
        .fold(0_u128, |acc, cur| acc + cur.value.amount.u128());

    to_binary(&QueryTotalBidResponse {
        value: coin(bids_sum, DENOM),
    })
}

pub fn query_highest_bid(deps: Deps) -> StdResult<Binary> {
    let state = STATE.load(deps.storage)?;
    let bidding_round = BIDDING_ROUNDS.load(deps.storage, state.round_number)?;

    to_binary(&QueryHighestBidResponse {
        address: bidding_round.top_bidder.to_string(),
        value: bidding_round.highest_bid,
    })
}

pub fn query_winner(deps: Deps) -> StdResult<Binary> {
    let state = STATE.load(deps.storage)?;
    let bidding_round = BIDDING_ROUNDS.load(deps.storage, state.round_number)?;
    let address = match bidding_round.winner {
        Some(x) => x.to_string(),
        _ => "".to_string(),
    };

    to_binary(&QueryWinnerResponse { address })
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{Addr, Coin, Empty};
    use cw_multi_test::{App, Contract, ContractWrapper, Executor};

    use super::{
        coin, execute, instantiate, query, ExecuteMsg, QueryHighestBidResponse, QueryMsg,
        QueryTotalBidResponse, QueryWinnerResponse, DENOM,
    };

    fn get_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(execute, instantiate, query);
        Box::new(contract)
    }

    #[test]
    fn test_bid() {
        let owner = Addr::unchecked("owner");
        let joe = Addr::unchecked("joe");

        let mut app = App::new(|router, _api, storage| {
            router
                .bank
                .init_balance(storage, &joe, vec![coin(100, DENOM)])
                .unwrap();
        });

        let contract_id = app.store_code(get_contract());

        let contract_addr = app
            .instantiate_contract(contract_id, owner, &Empty {}, &[], "bidding platform", None)
            .unwrap();

        app.execute_contract(
            joe,
            contract_addr.clone(),
            &ExecuteMsg::Bid {},
            &[coin(100, DENOM)],
        )
        .unwrap();

        let resp: QueryTotalBidResponse = app
            .wrap()
            .query_wasm_smart(
                contract_addr,
                &QueryMsg::QueryTotalBid {
                    address: "joe".to_string(),
                },
            )
            .unwrap();

        assert_eq!(
            resp,
            QueryTotalBidResponse {
                value: coin(95, DENOM)
            }
        );
    }

    #[test]
    fn test_double_bid() {
        let owner = Addr::unchecked("owner");
        let joe = Addr::unchecked("joe");
        let stable_joe = Addr::unchecked("stable joe");

        let mut app = App::new(|router, _api, storage| {
            router
                .bank
                .init_balance(storage, &joe, vec![coin(100, DENOM)])
                .unwrap();

            router
                .bank
                .init_balance(storage, &stable_joe, vec![coin(200, DENOM)])
                .unwrap();
        });

        let contract_id = app.store_code(get_contract());

        let contract_addr = app
            .instantiate_contract(contract_id, owner, &Empty {}, &[], "bidding platform", None)
            .unwrap();

        app.execute_contract(
            joe,
            contract_addr.clone(),
            &ExecuteMsg::Bid {},
            &[coin(100, DENOM)],
        )
        .unwrap();

        app.execute_contract(
            stable_joe.clone(),
            contract_addr.clone(),
            &ExecuteMsg::Bid {},
            &[coin(200, DENOM)],
        )
        .unwrap();

        let resp: QueryHighestBidResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::QueryHighestBid {})
            .unwrap();

        assert_eq!(
            resp,
            QueryHighestBidResponse {
                address: stable_joe.to_string(),
                value: coin(190, DENOM)
            }
        );
    }

    #[test]
    fn test_close() {
        let owner = Addr::unchecked("owner");
        let joe = Addr::unchecked("joe");
        let stable_joe = Addr::unchecked("stable joe");

        let mut app = App::new(|router, _api, storage| {
            router
                .bank
                .init_balance(storage, &joe, vec![coin(100, DENOM)])
                .unwrap();

            router
                .bank
                .init_balance(storage, &stable_joe, vec![coin(200, DENOM)])
                .unwrap();
        });

        let contract_id = app.store_code(get_contract());

        let contract_addr = app
            .instantiate_contract(
                contract_id,
                owner.clone(),
                &Empty {},
                &[],
                "bidding platform",
                None,
            )
            .unwrap();

        app.execute_contract(
            joe,
            contract_addr.clone(),
            &ExecuteMsg::Bid {},
            &[coin(100, DENOM)],
        )
        .unwrap();

        app.execute_contract(
            stable_joe.clone(),
            contract_addr.clone(),
            &ExecuteMsg::Bid {},
            &[coin(200, DENOM)],
        )
        .unwrap();

        app.execute_contract(owner, contract_addr.clone(), &ExecuteMsg::Close {}, &[])
            .unwrap();

        let resp: QueryWinnerResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::QueryWinner {})
            .unwrap();

        assert_eq!(
            resp,
            QueryWinnerResponse {
                address: stable_joe.to_string(),
            }
        );
    }

    #[test]
    fn test_retract() {
        let owner = Addr::unchecked("owner");
        let joe = Addr::unchecked("joe");
        let stable_joe = Addr::unchecked("stable joe");

        let mut app = App::new(|router, _api, storage| {
            router
                .bank
                .init_balance(storage, &joe, vec![coin(100, DENOM)])
                .unwrap();

            router
                .bank
                .init_balance(storage, &stable_joe, vec![coin(200, DENOM)])
                .unwrap();
        });

        let contract_id = app.store_code(get_contract());

        let contract_addr = app
            .instantiate_contract(
                contract_id,
                owner.clone(),
                &Empty {},
                &[],
                "bidding platform",
                None,
            )
            .unwrap();

        app.execute_contract(
            joe.clone(),
            contract_addr.clone(),
            &ExecuteMsg::Bid {},
            &[coin(100, DENOM)],
        )
        .unwrap();

        app.execute_contract(
            stable_joe.clone(),
            contract_addr.clone(),
            &ExecuteMsg::Bid {},
            &[coin(200, DENOM)],
        )
        .unwrap();

        app.execute_contract(owner, contract_addr.clone(), &ExecuteMsg::Close {}, &[])
            .unwrap();

        app.execute_contract(
            joe,
            contract_addr,
            &ExecuteMsg::Retract {
                receiver: Some(stable_joe.to_string()),
            },
            &[],
        )
        .unwrap();

        let resp: Vec<Coin> = app.wrap().query_all_balances(stable_joe).unwrap();

        assert_eq!(resp, vec![coin(95, DENOM)]);
    }
}
