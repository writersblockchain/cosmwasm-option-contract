## simple-option is a cosmwasm contract that allows users to buy or sell an asset at an agreed-upon price and date.

This markdown document summarizes the simple-option contract's state.rs, msg.rs, and contract.rs files.

### state.rs

```rust
pub struct State {
    //We store 2 Coin variables - collateral and counter_offer. Coin is a struct that consists of a denom (String) and an amount (Uint128)

    //The variable 'expires' is a u64 and is the block height. So a future block height is set as the option expiration date.   
    pub creator: Addr,
    pub owner: Addr, 
    pub collateral: Vec<Coin>, 
    pub counter_offer: Vec<Coin>, 
    pub expires: u64, 
}

pub const CONFIG_KEY: &str = "config";
// Item stores one typed item at the given key. So CONFIG is storing the State struct to the given key "CONFIG_KEY"
pub const CONFIG: Item<State> = Item::new(CONFIG_KEY);
```

Here I created a test to show that Item storage is working as expected: 

```rust
#[cfg(test)]
mod test {
    use super::*;
    //you can use super:: to reach one level up the tree from your current location
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::coins;

    #[test]
    //to only run this test, run "cargo test save_and_load"
    fn save_and_load() {
        let mut store = MockStorage::new();
        assert_eq!(CONFIG.may_load(&store).unwrap(), None);

        let cfg = State {
            creator: Addr::unchecked("creator"),
            owner: Addr::unchecked("owner"),
            collateral: coins(40, "ETH"), 
            counter_offer: coins(40, "ETH"), 
            expires: 1234, 
        };
        CONFIG.save(&mut store, &cfg).unwrap();
        assert_eq!(cfg, CONFIG.load(&store).unwrap());
    }
}
```

#### msg.rs

```rust
pub struct InstantiateMsg {
    //The owner, creator, and collateral variables all come from MessageInfo. 

    //MessageInfo includes a "sender" variable and a "funds" variable. 'sender' is the address that initiated the action (i.e. the message). 'funds' are the funds that are sent to the contract as part of `MsgInstantiateContract`. The transfer is processed in bank before the contract is executed such that the new balance is visible during contract execution.
    pub counter_offer: Vec<Coin>, 
    pub expires: u64, 
}

pub enum ExecuteMsg {
 Transfer { recipient: String},
  // Owner can transfer the option to a new owner. 'recipient' is a String that is the new owner's wallet address 
 Execute {},
 // Owner can post counter_offer on unexpired option to execute and get the collateral
 Burn {},
  //Burn will release the collateral if the option is expired
}

pub enum QueryMsg {
   Config{}, 
}

// We define a custom struct for each query response. In this case, the query response is the State struct, imported from state.rs  
pub type ConfigResponse = State;
```

#### contract.rs

```rust
pub fn instantiate(
    deps: DepsMut,
    //"deps" allows us to perform storage related actions, validate addresses and query other smart contracts
    env: Env,
     //"Env" contains all of the current info we know about the blockchain state
    info: MessageInfo,
    //"info" provides access to the message metadata (i.e., sender address, the amount and type of funds)
    msg: InstantiateMsg,
    //"msg" is the MsgInstantiateContract payload, which comprises the data received from the contract creator in JSON format that conforms to the InstantiateMsg struct

) -> Result<Response, ContractError> {
    //If the option is expired, we return a generic contract error, otherwise, we store the state and return a success code:
    if msg.expires <= env.block.height {
        return Err(ContractError::OptionExpired {
            expired: msg.expires,
        });
    }

    let state = State {
        creator: info.sender.clone(),
        owner: info.sender.clone(), 
        collateral: info.funds,
         //collateral is the funds sent by the contract creator. 
        counter_offer: msg.counter_offer,
        expires: msg.expires,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &state)?;

    Ok(Response::default())
}

pub fn execute(
    //implement rust's pattern matching so the contract can either transfer, execute, or burn an option
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Transfer { recipient } => execute_transfer(deps, env, info, recipient),
        ExecuteMsg::Execute {} => execute_execute(deps, env, info),
        ExecuteMsg::Burn {} => execute_burn(deps, env, info),
    }
}

pub fn execute_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
) -> Result<Response, ContractError> {
    // ensure msg sender is the owner by loading the contract state and checking that the wallet address calling execute_transfer is the same wallet address that created the option
    let mut state = CONFIG.load(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }
    // set new owner on state and save it to the contract state 
    state.owner = deps.api.addr_validate(&recipient)?;
    CONFIG.save(deps.storage, &state)?;

    let res =
    //add the response to the cosmos sdk event logs
        Response::new().add_attributes([("action", "transfer"), ("owner", recipient.as_str())]);
    Ok(res)
}

pub fn execute_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // ensure msg sender is the owner
    let state = CONFIG.load(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }
    // ensure the option is not expired by checking if the current block height is greater than or equal to the 'expires' block height set in the contract state. If the option is expired, throw an error 
    if env.block.height >= state.expires {
        return Err(ContractError::OptionExpired {
            expired: state.expires,
        });
    }
    // ensure sending proper counter_offer
    if info.funds != state.counter_offer {
        return Err(ContractError::CounterOfferMismatch {
            offer: info.funds,
            counter_offer: state.counter_offer,
        });
    }
    // release counter_offer to creator
    let mut res = Response::new();
    res = res.add_message(BankMsg::Send {
        to_address: state.creator.to_string(),
        amount: state.counter_offer,
    });
    // release collateral to sender
    res = res.add_message(BankMsg::Send {
        //BankMsg refers to the message types of the bank module. It defines a method for sending coins from one account to another account.
        to_address: state.owner.to_string(),
        amount: state.collateral,
    });

    // delete the option
    CONFIG.remove(deps.storage);

    res = res.add_attribute("action", "execute");
    Ok(res)
}

pub fn execute_burn(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // ensure option is expired
    let state = CONFIG.load(deps.storage)?;
    if env.block.height < state.expires {
        return Err(ContractError::OptionNotExpired {
            expires: state.expires,
        });
    }

    // release collateral to creator. Since the option has expired, the collateral is returned to the owner. 
    let mut res = Response::new();
    res = res.add_message(BankMsg::Send {
        to_address: state.creator.to_string(),
        amount: state.collateral,
    });

    // delete the option
    CONFIG.remove(deps.storage);

    res = res.add_attribute("action", "burn");
    Ok(res)
}
```



