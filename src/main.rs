use isahc::prelude::*;
use url::Url;
use ring::*;
use regex::Regex;
use std::collections::HashMap;
use clap::Parser;

const TESTNET_URI_PREFIX: &str = "https://api-testnet.bybit.com";
const URI_PREFIX: &str = "https://api.bybit.com";
const DEFAULT_SL_PCNT: f64 = 0.2;

#[derive(Debug, Parser)]
#[clap(author="by Wasin Thonkaew (wasin@wasin.io)")]
#[clap(name="quicky")]
#[clap(about="quicky lets you place limit order quickly (consider volatility of the price)", long_about=None)]
struct CommandlineArgs {
    #[clap(short='s', long)]
    symbol: String,

    /// Quantity as part of the trade operation.
    /// Positive for buy side.
    /// Negative for sell side.
    #[clap(short='q', long)]
    qty: i64,

    /// Whether or not to execute against testnet
    // We dont need to explicitly specify value for bool here, so just --testnet
    // is fine to make it true. Otherwise, see
    // https://github.com/clap-rs/clap/blob/master/examples/derive_ref/custom-bool.rs
    // as 'bool' type needs special care here.
    //
    // Use the following when we need to explicitly specify value
    // `#[clap(long, parse(try_from_str), default_value="false")]`
    #[clap(long)]
    testnet: bool,

    /// Stop-loss percentage
    #[clap(long, default_value_t=DEFAULT_SL_PCNT)]
    sl_pcnt: f64,
}

/// `TradingContext` contains information used during trading.
/// It also contains cached information we know before hand as we don't have to
/// make unnecessary API requests which waste time.
struct TradingContext {
    /// Set environment variable with name BYBIT_API_KEY
    api_key: String,

    /// Set environment variable with name BYBIT_API_SECRET
    api_secret: String,

    /// Set environment variable with name BYBIT_TESTNET_API_KEY
    testnet_api_key: String,

    /// Set environment variable with name BYBIT_TESTNET_API_SECRET
    testnet_api_secret: String,

    /// Tick steps information for symbols
    tick_steps: HashMap<String, f64>,

    /// Stop-loss percentage
    stop_loss_pcnt: f64,

    /// Whether or not to execute API against testnet
    use_testnet: bool
}

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

/// Status code represents the result of API related calls & its internal operations.
enum StatusCode {
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

/// Generic response structure with no result field.
/// Usually used to get to know whether response is success or not.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct BybitGenericNoResultResponse {
    ret_code: u32,
    ret_msg: String,
    ext_code: String,
    ext_info: String,
}

/// Server time response from Bybit
/// NOTE: Currently we didn't use this as it is not necessary, such that we
/// can use local timestamp if local one's time synced with time server online.
// https://bybit-exchange.github.io/docs/inverse/?python--old#t-servertime
#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct BybitServerTimeResponse {
    ret_code: u32,
    ret_msg: String,
    ext_code: String,
    ext_info: String,
    time_now: String,
}

/// Result field of symbol latest information response from Bybit.
/// NOTE: Currently not used, to reduce time spent for making and waiting for
/// response of HTTP request. We hard-coded certain information of target asset
/// instead for now.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct BybitLatestInformationSymbolResult {
    symbol: String,
    bid_price: String,
    ask_price: String,
    last_price: String,
    last_tick_direction: String,
    prev_price_24h: String,
    price_24h_pcnt: String,
    high_price_24h: String,
    low_price_24h: String,
    prev_price_1h: String,
    price_1h_pcnt: String,
    mark_price: String,
    index_price: String,
    open_interest: u64,
    open_value: String,
    total_turnover: String,
    turnover_24h: String,
    total_volume: u64,
    volume_24h: u64,
    funding_rate: String,
    predicted_funding_rate: String,
    next_funding_time: String,
    countdown_hour: u8,
    delivery_fee_rate: String,
    predicted_delivery_price: String,
    delivery_time: String,
}

/// Symbol latest information response from Bybit.
/// NOTE: Currently not used at the moment. See comment in `BybitLatestInformationSymbolResult`.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct BybitLatestInformationSymbolResponse {
    ret_code: u32,
    ret_msg: String,
    ext_code: String,
    ext_info: String,
    result: Option<Vec<BybitLatestInformationSymbolResult>>, // use Option<> for error case
    time_now: String,
}

/// Send a quick limit order.
/// Side depends on specified `qty`. If negative, then it is sell side, otherwise
/// it is buy side.
///
/// # Arguments
/// * `context` - `TradingContext` for information that we know before hand. This
///               will reduce time in sending unnecessary API request to get
///               such information.
/// * `symbol` - symbol to create an order for
/// * `qty` - quantity. It can be negative for sell, or positive buy. If specified
///           as 0, then it will be ignored.
fn api_send_quick_limit_order(context: &TradingContext, symbol: &str, qty: i64) -> Result<(), StatusCode> {
    // We can get the price step from API, use
    // https://bybit-exchange.github.io/docs/inverse/?console#t-querysymbol
    // but that would be too time consuming.
    if !context.tick_steps.contains_key(symbol) {
        return Err(StatusCode::InternalErrorNoTickStepAvailable);
    }

    let price = api_get_current_price(context, symbol)?;

    if qty == 0 {
        return Err(StatusCode::ErrorIncorrectParameterValue);
    }

    let is_buy_side = qty > 0;
    let tick_step = context.tick_steps[symbol];
    let tick_step_value_roundup = 10.0_f64.powi(count_tick_steps(tick_step));
    let stop_loss_pcnt = context.stop_loss_pcnt;
    let target_limit_price:f64 = if is_buy_side { ((price - tick_step)*tick_step_value_roundup).round() / tick_step_value_roundup } else { ((price + tick_step)*tick_step_value_roundup).round() / tick_step_value_roundup };
    let curr_unix_timestamp = get_unix_timestamp_as_millis();
    let curr_unix_timestamp_str = curr_unix_timestamp.to_string();
    let side = if is_buy_side {"Buy"} else {"Sell"};
    let qty_abs:u64 = qty.abs() as u64;

    let stop_loss_price:f64 = if is_buy_side { ((price * (1.0 - stop_loss_pcnt/100.0))*tick_step_value_roundup).round() / tick_step_value_roundup } else { ((price * (1.0 + stop_loss_pcnt/100.0))*tick_step_value_roundup).round() / tick_step_value_roundup };

    // TODO: add into hash, then sort alphabetically
    // prepare request's parameters for private API
    let param_str = format!("api_key={api_key}&order_type=Limit&price={price}&qty={qty}&side={side}&stop_loss={stop_loss}&symbol={symbol}&time_in_force=PostOnly&timestamp={timestamp}", api_key=get_api_key(context), price=target_limit_price, qty=qty_abs.to_string(), side=side, stop_loss=stop_loss_price, symbol=symbol, timestamp=curr_unix_timestamp_str);
    let sign = sign_private_request_params(&param_str, get_api_secret(context));

    // Serialize in serde is ok to work with &str, but not Deserialize
    #[derive(Debug, serde::Serialize)]
    struct RequestObj<'a> {
        api_key: &'a str,
        order_type: &'a str,
        price: f64,
        qty: u64,
        side: &'a str,
        stop_loss: f64,
        symbol: &'a str,
        timestamp: &'a str,
        time_in_force: &'a str,
        sign: &'a str,
    }

    let request_json_obj = RequestObj {
        api_key: get_api_key(context),
        order_type: "Limit",
        price: target_limit_price,
        qty: qty_abs,
        side: side,
        stop_loss: stop_loss_price,
        symbol: symbol,
        timestamp: &curr_unix_timestamp_str,
        time_in_force: "PostOnly",
        sign: &sign,
    };
    
    let raw_url_str = get_full_uri(context.use_testnet, "/v2/private/order/create");
    let url = Url::parse(&raw_url_str);
    if let Err(_) = url {
        return Err(StatusCode::InternalErrorCreatingHttpRequest);
    }

    let request_json_obj_body = serde_json::to_vec(&request_json_obj);
    if request_json_obj_body.is_err() {
        return Err(StatusCode::InternalErrorParsingJsonObject);
    }

    let request = isahc::Request::builder()
        .method("POST")
        .uri(url.unwrap().as_str())
        .header("content-type", "application/json")
        .version_negotiation(isahc::config::VersionNegotiation::http2())
        .body(request_json_obj_body.unwrap());

    match isahc::send(request.unwrap()) {
        Ok(mut res) => {
            match res.json::<BybitGenericNoResultResponse>() {
                Ok(json) => {
                    if json.ret_code == 0 { return Ok(()); } else { return Err(StatusCode::ErrorApiResponse); }
                }
                Err(e) => {
                    eprintln!("{:?}", e);
                    Err(StatusCode::ErrorJsonParsing)
                }
            }
        },
        Err(_) => {
            Err(StatusCode::ErrorApiResponse)
        }
    }
}

/// Get current price of the specified `symbol`.
///
/// # Arguments
/// * `context` - `TradingContext` for context information used in trading
/// * `symbol` - symbol to get the current price (current price is **last traded price**)
fn api_get_current_price(context: &TradingContext, symbol: &str) -> Result<f64, StatusCode> {

    let raw_url_str = get_full_uri(context.use_testnet, &("/v2/public/tickers?symbol=".to_owned() + symbol));
    let url = Url::parse(&raw_url_str);
    if let Err(_) = url {
        return Err(StatusCode::InternalErrorParsingRawUrl);
    }

    let request = isahc::Request::builder()
        .method("GET")
        .uri(url.unwrap().as_str())
        .header("content-type", "application/json")
        .version_negotiation(isahc::config::VersionNegotiation::http2())
        .body(());
    if let Err(_) = request {
        return Err(StatusCode::InternalErrorCreatingHttpRequest);
    }

    match isahc::send(request.unwrap()) {
        Ok(mut res) => {
            match res.json::<BybitLatestInformationSymbolResponse>() {
                Ok(json) => {
                    // early return if error
                    if json.ret_code != 0 {
                        eprintln!("Error: {}", json.ret_msg);
                        return Err(StatusCode::ErrorApiResponse);
                    }

                    // guarantee to have result for success case, safe to unwrap
                    let result = json.result.unwrap();

                    if result.len() == 0 {
                        return Err(StatusCode::ApiEmptyResult);
                    }

                    match result[0].last_price.parse::<f64>() {
                        Ok(price) => Ok(price),
                        Err(_) => Err(StatusCode::ErrorNumericJsonParsing)
                    }
                },
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    Err(StatusCode::ErrorJsonParsing)
                }
            }
        },
        Err(_) => Err(StatusCode::ErrorApiResponse)
    }
}

/// Get server time from Bybit server through api
/// In success, return timestamp in milliseconds. Otherwise return `StatusCode`.
/// **Note**: This is blocking call waiting for response back from API request.
///
/// Ref: Bybit server time - https://bybit-exchange.github.io/docs/inverse/#t-servertime
///
/// Currently we don't use this to reduce time in making an additional HTTP request
/// to just get a server's timestamp to satisfy Bybit side. But we can just get
/// our local timestamp and use it just fine if our local one has time synced
/// properly.
///
/// # Arguments
/// * `context` - `TradingContext` for context information used in trading
fn api_get_bybit_timestamp(context: &TradingContext) -> Result<u64, StatusCode> {
    let raw_url_str = get_full_uri(context.use_testnet, "/v2/public/time");
    let url = Url::parse(&raw_url_str);
    if let Err(_) = url {
        return Err(StatusCode::InternalErrorParsingRawUrl);
    }

    let request = isahc::Request::builder()
        .method("GET")
        .uri(url.unwrap().as_str())
        .header("content-type", "application/json")
        .version_negotiation(isahc::config::VersionNegotiation::http2())
        .body(());
    if let Err(_) = request {
        return Err(StatusCode::InternalErrorCreatingHttpRequest);
    }

    match isahc::send(request.unwrap()) {
        Ok(mut res) => {
            match res.json::<BybitServerTimeResponse>() {
                Ok(json) => {
                    parse_time_now(&json.time_now)
                },
                Err(_) => Err(StatusCode::ErrorJsonParsing)
            }
        },
        Err(_) => Err(StatusCode::ErrorApiResponse),
    }
}

/// Parse string of time now.
///
/// # Arguments
/// * `time_now_str` - `String` of time now to be parsed
fn parse_time_now(time_now_str: &str) -> Result<u64, StatusCode> {
    // Form the correct pattern before returning
    //
    // timestamp returned as millisecond.nanoseconds
    // we will get seconds.first-3-digit-of-nanoseconds from returned
    // response from API
    let regex = Regex::new(r"(\d+)\.(\d{3})\d{3}").unwrap();
    let results = regex.captures_iter(time_now_str).filter_map(|cap| {
        let groups = (cap.get(1), cap.get(2));
        match groups {
            (Some(seconds), Some(millis)) => {
                let mut seconds_copy = seconds.as_str().to_owned();
                seconds_copy.push_str(millis.as_str());
                Some(seconds_copy.parse().unwrap())
            },
            _ => None
        }
    });

    let collected_results: Vec<u64> = results.collect();
    match collected_results.first() {
        Some(res) => Ok(*res),
        None => Err(StatusCode::MalformedAPIResponseFormat)
    }
}

/// Sign a specified string associated with the secret string via HMAC-SHA256
/// algorithm.
fn sign_private_request_params(str: &str, secret: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let signed = hmac::sign(&key, str.as_bytes());
    assert!(hmac::verify(&key, str.as_bytes(), signed.as_ref()).is_ok());

    signed.as_ref().iter().map(|x| format!("{:02x}", x)).collect::<String>()
}

/// Print on stderr from the input `StatusCode`.
/// It won't do anything for `StatusCode::Success`.
///
/// # Arguments
/// * `code` - `StatusCode`
fn print_error_if_necessary(code: StatusCode) {
    match code {
        StatusCode::InternalErrorCreatingHttpRequest => eprintln!("Error: internal error creating http request"),
        StatusCode::InternalErrorParsingRawUrl => eprintln!("Error: internal error parsing a raw url"),
        StatusCode::ErrorJsonParsing => eprintln!("Error: parsing json"),
        StatusCode::ErrorApiResponse => eprintln!("Error: received error in api response"),
        StatusCode::InternalErrorGeneric => eprintln!("Error: internal generic error"),
        StatusCode::MalformedAPIResponseFormat => eprintln!("Error: malformed result from API response"),
        StatusCode::ApiEmptyResult => eprintln!("Error: API has empty result"),
        StatusCode::ErrorNumericJsonParsing => eprintln!("Error: numeric Json parsing error"),
        StatusCode::InternalErrorNoTickStepAvailable => eprintln!("Error: no tick steps available for specified symbol"),
        _ => {}
    }
}

/// Start measuring time. Suitable for wall-clock time measurement.
/// This is mainly used to measure time of placing a limit order onto Bybit.
fn measure_start(start: &mut std::time::Instant) {
    *start = std::time::Instant::now();
}

/// Mark the end of the measurement of time performance.
/// Return result in seconds, along with printing the elapsed time if `also_print`
/// is `true`.
fn measure_end(start: &std::time::Instant, also_print: bool) -> f64 {
    let elapsed = start.elapsed().as_secs_f64();
    if also_print {
        println!("(elapsed = {:.2} secs)", elapsed);
    }
    elapsed
}

/// Ref https://stackoverflow.com/a/44378174/571227
/// Instant doesn't provide the way.
fn get_unix_timestamp_as_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let start = SystemTime::now();
    let duration_since_epoch = start.duration_since(UNIX_EPOCH);
    match duration_since_epoch {
        Ok(dur) => dur.as_millis(),
        Err(_) => 0
    }
}

/// Internal function to count the steps of the specified value.
/// Ex. 0.0001 has 4 steps.
fn count_tick_steps(value: f64) -> i32 {
    if value >= 1.0 {
        return 0;
    }

    let mut count = 0;
    let mut value_copy = value;

    while value_copy < 1.0 {
        value_copy = value_copy * 10.0;
        count = count + 1;
    }

    count
}

/// Get API key from `TradingContext`.
fn get_api_key(context: &TradingContext) -> &str {
    if context.use_testnet { &context.testnet_api_key } else { &context.api_key }
}

/// Get API secret from `TradingContext`.
fn get_api_secret(context: &TradingContext) -> &str {
    if context.use_testnet { &context.testnet_api_secret } else { &context.api_secret }
}

/// Form the full URI from specified `end_point` and whether or not it is meant
/// to be using on testnet as specified by `use_testnet`.
fn get_full_uri(use_testnet: bool, end_point: &str) -> String {
    format!("{prefix}{end_point}", prefix=if use_testnet { TESTNET_URI_PREFIX } else { URI_PREFIX }, end_point=end_point)
}

fn main() {    
    // parse arguments via clap
    let cmd_args = CommandlineArgs::parse();

    println!("{:?}", cmd_args);

    // construct trading context with some which specified via command line's arguments,
    // and the less with default values.
    let trading_context = TradingContext {
        use_testnet: cmd_args.testnet,
        stop_loss_pcnt: cmd_args.sl_pcnt,
        ..Default::default()
    };

    let mut start = std::time::Instant::now();
    measure_start(&mut start);
 
    match api_send_quick_limit_order(&trading_context, &cmd_args.symbol, cmd_args.qty) {
        Ok(_) => {
            println!("done");
            measure_end(&start, true);
        }
        Err(e) => print_error_if_necessary(e)
    }
}
