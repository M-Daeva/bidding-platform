use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },

    #[error("The bid is too small")]
    SmallBid {},

    #[error("Sender does not have access permissions!")]
    Unauthorized {},

    #[error("Bidding is not closed!")]
    BiddingIsOpen {},

    #[error("Player is not found!")]
    PlayerIsNotFound {},
}
