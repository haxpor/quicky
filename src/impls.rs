use crate::types::TradingContext;
use crate::defines::*;

use std::collections::HashMap;

/// Provide default values for `TradingContext`
impl Default for TradingContext {
    fn default() -> TradingContext {
        TradingContext {
            // panic if required api-keys/api-secrets are not set
            api_key: std::env::var("BYBIT_API_KEY").expect("Required env variable BYBIT_API_KEY to be set"),
            api_secret: std::env::var("BYBIT_API_SECRET").expect("Required env variable BYBIT_API_SECRET to be set"),
            testnet_api_key: std::env::var("BYBIT_TESTNET_API_KEY").expect("Required env variable BYBIT_TESTNET_API_KEY to be set"),
            testnet_api_secret: std::env::var("BYBIT_TESTNET_API_SECRET").expect("Required env variable BYBIT_TESTNET_API_SECRET to be set"),
            tick_steps: HashMap::from([
                                      ("XRPUSD".to_string(), 0.0001)
            ]),
            stop_loss_pcnt: DEFAULT_SL_PCNT,
            use_testnet: true,      // default for safety use testnet
        }
    }
}
