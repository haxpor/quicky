mod types;
mod util;
mod impls;
mod defines;

use clap::Parser;
use types::*;
use util::*;

fn main() {    
    // parse arguments via clap
    let cmd_args = CommandlineArgs::parse();

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
