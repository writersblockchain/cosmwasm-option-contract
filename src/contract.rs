#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, BankMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:simple-option";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    //"deps" allows us to perform storage related actions, validate addresses and query other smart contracts
    env: Env,
     //"Env" contains all of the current info we know about the blockchain state
    info: MessageInfo,
    //"info" provides access to the message metadata (i.e., sender address, the amount and type of funds)
    msg: InstantiateMsg,
    //"msg" is the MsgInstantiateContract payload, which comprises the data received from the contract creator
  //in JSON format that conforms to the InstantiateMsg struct

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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
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

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = CONFIG.load(deps.storage)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, coins, CosmosMsg};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            counter_offer: coins(40, "ETH"),
            expires: 100_000,
        };
        let info = mock_info("creator", &coins(1, "BTC"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query_config(deps.as_ref()).unwrap();
        assert_eq!(100_000, res.expires);
        assert_eq!("creator", res.owner.as_str());
        assert_eq!("creator", res.creator.as_str());
        assert_eq!(coins(1, "BTC"), res.collateral);
        assert_eq!(coins(40, "ETH"), res.counter_offer);
        println!("yo!");
        
    }

    #[test]
    fn transfer() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            counter_offer: coins(40, "ETH"),
            expires: 100_000,
        };
        let info = mock_info("creator", &coins(1, "BTC"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // random cannot transfer
        let info = mock_info("anyone", &[]);
        let err =
            execute_transfer(deps.as_mut(), mock_env(), info, "anyone".to_string()).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            e => panic!("unexpected error: {}", e),
        }

        // owner can transfer
        let info = mock_info("creator", &[]);
        let res = execute_transfer(deps.as_mut(), mock_env(), info, "someone".to_string()).unwrap();
        assert_eq!(res.attributes.len(), 2);
        assert_eq!(res.attributes[0], attr("action", "transfer"));

        // check updated properly
        let res = query_config(deps.as_ref()).unwrap();
        assert_eq!("someone", res.owner.as_str());
        assert_eq!("creator", res.creator.as_str());
    }

    #[test]
    fn execute() {
        let mut deps = mock_dependencies();

        let amount = coins(40, "ETH");
        let collateral = coins(1, "BTC");
        let expires = 100_000;
        let msg = InstantiateMsg {
            counter_offer: amount.clone(),
            expires,
        };
        let info = mock_info("creator", &collateral);

        // we can just call .unwrap() to assert this was a success
        let _ = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // set new owner
        let info = mock_info("creator", &[]);
        let _ = execute_transfer(deps.as_mut(), mock_env(), info, "owner".to_string()).unwrap();

        // random cannot execute
        let info = mock_info("creator", &amount);
        let err = execute_execute(deps.as_mut(), mock_env(), info).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            e => panic!("unexpected error: {}", e),
        }

        // expired cannot execute
        let info = mock_info("owner", &amount);
        let mut env = mock_env();
        env.block.height = 200_000;
        let err = execute_execute(deps.as_mut(), env, info).unwrap_err();
        match err {
            ContractError::OptionExpired { expired } => assert_eq!(expired, expires),
            e => panic!("unexpected error: {}", e),
        }

        // bad counter_offer cannot execute
        let msg_offer = coins(39, "ETH");
        let info = mock_info("owner", &msg_offer);
        let err = execute_execute(deps.as_mut(), mock_env(), info).unwrap_err();
        match err {
            ContractError::CounterOfferMismatch {
                offer,
                counter_offer,
            } => {
                assert_eq!(msg_offer, offer);
                assert_eq!(amount, counter_offer);
            }
            e => panic!("unexpected error: {}", e),
        }

        // proper execution
        let info = mock_info("owner", &amount);
        let res = execute_execute(deps.as_mut(), mock_env(), info).unwrap();
        assert_eq!(res.messages.len(), 2);
        assert_eq!(
            res.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "creator".into(),
                amount,
            })
        );
        assert_eq!(
            res.messages[1].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "owner".into(),
                amount: collateral,
            })
        );

        // check deleted
        let _ = query_config(deps.as_ref()).unwrap_err();
    }
}
