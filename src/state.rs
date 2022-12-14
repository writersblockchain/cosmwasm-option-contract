use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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
