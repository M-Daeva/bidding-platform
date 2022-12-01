use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Coin;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Bid {},
    Close {},
    Retract { receiver: Option<String> },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(QueryTotalBidResponse)]
    QueryTotalBid { address: String },

    #[returns(QueryTotalBidResponse)]
    QueryHighestBid {},

    #[returns(QueryWinnerResponse)]
    QueryWinner {},
}

#[cw_serde]
pub struct QueryTotalBidResponse {
    pub value: Coin,
}

#[cw_serde]
pub struct QueryHighestBidResponse {
    pub address: String,
    pub value: Coin,
}

#[cw_serde]
pub struct QueryWinnerResponse {
    pub address: String,
}
