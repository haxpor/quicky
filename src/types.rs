use clap::Parser;
use std::collections::HashMap;

#[derive(Debug, Parser)]
#[clap(author="by Wasin Thonkaew (wasin@wasin.io)")]
#[clap(name="quicky")]
#[clap(about="quicky lets you place limit order quickly (consider volatility of the price)", long_about=None)]
pub struct CommandlineArgs {
    #[clap(short='s', long)]
    pub symbol: String,

    /// Quantity as part of the trade operation.
    /// Positive for buy side.
    /// Negative for sell side.
    #[clap(short='q', long)]
    pub qty: i64,

    /// Whether or not to execute against testnet
    // We dont need to explicitly specify value for bool here, so just --testnet
    // is fine to make it true. Otherwise, see
    // https://github.com/clap-rs/clap/blob/master/examples/derive_ref/custom-bool.rs
    // as 'bool' type needs special care here.
    //
    // Use the following when we need to explicitly specify value
    // `#[clap(long, parse(try_from_str), default_value="false")]`
    #[clap(long="testnet", multiple_values=false, default_missing_value="true", takes_value=false)]
    pub testnet: bool,

    /// Stop-loss percentage
    #[clap(long, default_value_t=crate::defines::DEFAULT_SL_PCNT)]
    pub sl_pcnt: f64,
}

/// Status code represents the result of API related calls & its internal operations.
pub enum StatusCode {
    Success=0,
    InternalErrorGeneric,
    InternalErrorParsingRawUrl,
    InternalErrorCreatingHttpRequest,
    InternalErrorParsingJsonObject,
    InternalErrorNoTickStepAvailable,
    ErrorApiResponse,
    ErrorJsonParsing,
    ErrorNumericJsonParsing,
    MalformedAPIResponseFormat,
    ApiEmptyResult,
    ErrorIncorrectParameterValue,
}

/// `TradingContext` contains information used during trading.
/// It also contains cached information we know before hand as we don't have to
/// make unnecessary API requests which waste time.
pub struct TradingContext {
    /// Set environment variable with name BYBIT_API_KEY
    pub api_key: String,

    /// Set environment variable with name BYBIT_API_SECRET
    pub api_secret: String,

    /// Set environment variable with name BYBIT_TESTNET_API_KEY
    pub testnet_api_key: String,

    /// Set environment variable with name BYBIT_TESTNET_API_SECRET
    pub testnet_api_secret: String,

    /// Tick steps information for symbols
    pub tick_steps: HashMap<String, f64>,

    /// Stop-loss percentage
    pub stop_loss_pcnt: f64,

    /// Whether or not to execute API against testnet
    pub use_testnet: bool
}

/// Generic response structure with no result field.
/// Usually used to get to know whether response is success or not.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BybitGenericNoResultResponse {
    pub ret_code: u32,
    pub ret_msg: String,
    pub ext_code: String,
    pub ext_info: String,
}

/// Server time response from Bybit
/// NOTE: Currently we didn't use this as it is not necessary, such that we
/// can use local timestamp if local one's time synced with time server online.
// https://bybit-exchange.github.io/docs/inverse/?python--old#t-servertime
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BybitServerTimeResponse {
    pub ret_code: u32,
    pub ret_msg: String,
    pub ext_code: String,
    pub ext_info: String,
    pub time_now: String,
}

/// Result field of symbol latest information response from Bybit.
/// NOTE: Currently not used, to reduce time spent for making and waiting for
/// response of HTTP request. We hard-coded certain information of target asset
/// instead for now.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BybitLatestInformationSymbolResult {
    pub symbol: String,
    pub bid_price: String,
    pub ask_price: String,
    pub last_price: String,
    pub last_tick_direction: String,
    pub prev_price_24h: String,
    pub price_24h_pcnt: String,
    pub high_price_24h: String,
    pub low_price_24h: String,
    pub prev_price_1h: String,
    pub price_1h_pcnt: String,
    pub mark_price: String,
    pub index_price: String,
    pub open_interest: u64,
    pub open_value: String,
    pub total_turnover: String,
    pub turnover_24h: String,
    pub total_volume: u64,
    pub volume_24h: u64,
    pub funding_rate: String,
    pub predicted_funding_rate: String,
    pub next_funding_time: String,
    pub countdown_hour: u8,
    pub delivery_fee_rate: String,
    pub predicted_delivery_price: String,
    pub delivery_time: String,
}

/// Symbol latest information response from Bybit.
/// NOTE: Currently not used at the moment. See comment in `BybitLatestInformationSymbolResult`.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BybitLatestInformationSymbolResponse {
    pub ret_code: u32,
    pub ret_msg: String,
    pub ext_code: String,
    pub ext_info: String,
    pub result: Option<Vec<BybitLatestInformationSymbolResult>>, // use Option<> for error case
    pub time_now: String,
}
