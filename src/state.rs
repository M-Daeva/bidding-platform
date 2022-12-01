use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::{Item, Map};

pub const STATE: Item<State> = Item::new("state");

#[cw_serde]
pub struct State {
    pub owner: Addr,
    pub round_number: u128,
}

// key - round number
pub const BIDDING_ROUNDS: Map<u128, BiddingRound> = Map::new("biddind_rounds");

#[cw_serde]
pub struct BiddingRound {
    pub top_bidder: Addr,
    pub highest_bid: Coin,
    pub winner: Option<Addr>,
}

// key - player address
pub const PLAYERS: Map<&Addr, Player> = Map::new("players");

#[cw_serde]
pub struct Player {
    pub retractable_amount: Coin,
}

// key - player address
pub const BIDS: Map<&Addr, Vec<Bid>> = Map::new("bids");

#[cw_serde]
pub struct Bid {
    pub value: Coin,
}
