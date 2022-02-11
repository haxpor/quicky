# quicky

A command line program to allow day trader to quickly place a **limit** order instead of convenient but more fee of market order.

Usually we don't have to quickly place a market order in order to ensure we get into the position. But if we take advantage of volatility of the price, eventually we will have a position with limit order.

Some of exchanges have convenient UI to allow traders to quickly place a limit/market order, some don't. Anyway, even with availability of such convenient UI, traders still need to precisely get the nearest price (mostly a tick step away from the current trade price up or down) by hovering the mouse cursor, or fallback to use normal order form but time needed in order to find the price, and fill it into the form.

Thus `quicky` will help speed things up.

---

It takes advantage of ByBit exchange because limit order has a rebate fee of -0.025% compared to market order of 0.075%. Thus placing a limit order will minimize the fee, and maximize profit at the end. Thus `quicky` is specifically implemented to work with ByBit exchange.

# Setup

* Create API on Bybit exchange bot for mainnet, and testnet, and setup permission accordingly to only what is needed for your bot
* Define the following environment variables (on Linux via `~/.bash_aliases`, etc), and make sure you source the file
    * `BYBIT_API_KEY` - API key for mainnet
    * `BYBIT_API_SECRET` - API secret for mainnet
    * `BYBIT_TESTNET_API_KEY` - API key for **testnet**
    * `BYBIT_TESTNET_API_SECRET` - API secret for **testnet**
* `cargo build --release` - Better to build and use release build, minimize time as much as possible apart from HTTP request we would be definitely doing
* `cargo run -- -s XRPUSD -q 1 --testnet` or locate `quicky` binary and execute it like `quicky -s XRPUSD -q 1 --testnet`

# Usage

Following is output from `--help`.

```
quicky 
by Wasin Thonkaew (wasin@wasin.io)
quicky lets you place limit order quickly (consider volatility of the price)

USAGE:
    quicky [OPTIONS] --symbol <SYMBOL> --qty <QTY>

OPTIONS:
    -h, --help                 Print help information
    -q, --qty <QTY>            Quantity as part of the trade operation. Positive for buy side.
                               Negative for sell side
    -s, --symbol <SYMBOL>      
        --sl-pcnt <SL_PCNT>    Stop-loss percentage [default: 0.2]
        --testnet              Whether or not to execute against testnet
```

# Features

* Specifically work with derivatives (inverse perpetual) on ByBit exchange (for now only with `XRPUSD`, hint define tick step at `tick_steps` to support more assets)
* Allow to place limit buy/sell with specified quantity & stop-loss without a need to know the price, it will automatically find the nearest (as of tick step of such crypto asset) up or down from the current trade price
* Able to switch to trade on mainnet and testnet via `--testnet` flag at command line
* Trading context e.g. stop-loss percentage, (more to come in the future), etc are customized via command line's arguments

# Disclaimer

Use this program at your own risk. I take no responsibility towards damage or loss from using it
as a tool for investment. Please consider this program and its source code as educational purposethat might be useful for your case and situation at hands. The behaviors of the program are not meant to be a recommended investment strategy, please kindly do your due diligence.

# License
MIT, Wasin Thonkaew
