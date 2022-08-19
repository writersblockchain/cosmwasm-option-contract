use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Coin}; 
use crate::state::State;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    //The owner, creator, and collateral variables all come from MessageInfo. 

    //MessageInfo includes a "sender" variable and a "funds" variable. 'sender' is the address that initiated the action (i.e. the message). 'funds' are the funds that are sent to the contract as part of `MsgInstantiateContract`. The transfer is processed in bank before the contract is executed such that the new balance is visible during contract execution.
    pub counter_offer: Vec<Coin>, 
    pub expires: u64, 
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
 Transfer { recipient: String},
  // Owner can transfer the option to a new owner. 'recipient' is a String that is the new owner's wallet address 
 Execute {},
 // Owner executes unexpired option to execute and get the collateral
 Burn {},
  //Burn will release the collateral if the option is expired
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
   Config{}, 
}

// We define a custom struct for each query response. In this case, the query response is the State struct, imported from state.rs  
pub type ConfigResponse = State;
