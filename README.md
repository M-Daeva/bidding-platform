#Bidding platform
Create a smart contract for bidding procedure. The project should be a public git repository created by yourself, send us the repository address.

At instantion, user opens a bid for some offchain commodity. Bid will be happening using only single native token (for eg. ATOM). Contract owner is optionally provided by its creator - if missing, contract creator is considered its owner.

After contract is instantiated, any user other than the contract owner can raise his bid by sending tokens to the contract with the bid {} message. When the message is called, part of the tokens send are immediately considered
bidding commission and should be transferred to contract owner. It is up to you to figure out how to calculate commission.

The total bid of the user is considered to be a sum of all bids performed minus all the commissions. When user raises his bid, it should success only if his total bid is the highest of all other users bids. If it is less or the same as
the highest, bidding should fail.

Owner can close {} the bidding at any time. When the bidding is closed, address with the highest total bid is considered the bidding winner. The whole bidding of his is transferred to the contract owner.

After the bidding is closed, everyone who bid and didn't win the bidding, can retract {} all his funds. Additionally the retract message should have an optional friend receiver being an address where the sender biddings should be send. So retract {} sends all senders bids (minus commissions) to his account. The retract { "receiver": "addr" } should send all the sender bids to the "addr" account.

Additionally - all the information kept on the contract should be queryable in reasonable manner. The most important queries are: the given addr total bid, the highest bid at the current time (who and how much), if the bidding is closed, who won the bid (if it is closed).

The contract should contain some tests using multitests framework, but I do not expect any particular coverage - 2-3 main flow tests should be enough.

##Example
There is the bidding created at bidding_contract address. alex is sending bid {} message with 15 atoms. The highest bid right now is 15 atoms by alex. Now ann is sending bid {} message with 17 atoms. The highest bid is 17 atoms by ann, and total bid by alex is 15 atoms. Now ann is sending another bid {} message with 2 atoms. Now the highest bid is 19 atoms by ann, and total of alex is 15 atoms. Then alex sends bid {} message with 1 atom - this message fails, as it would leave alex at 16 atoms bid total, which is not the highest right now. He has to send more than 5 atoms. alex sends another bid {} with 5 atoms. It makes the highest bid being 20 atoms by alex, and ann has total of 19 atoms bid. The close {} is send by contract owner - alex wins the bid, 20 atoms are send to bid owner from bidding_contract. ann can claim her atoms back calling retract {} message, optionally putting a receiver field there to point where funds should be send back to.

##Hint
The cw_storage_plus::Map<Key, Value> utility would be a great tool to keep total bids.