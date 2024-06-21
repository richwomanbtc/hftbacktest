use std::{env, fs, process};

use algo::gridtrading;
use hftbacktest::{
    connector::binancefutures::{BinanceFutures, Endpoint},
    live::{LiveBot, LoggingRecorder},
    prelude::{Bot, HashMapMarketDepth},
};
use yaml_rust::YamlLoader;
mod algo;

fn load_config(path: &str) -> Vec<yaml_rust::Yaml> {
    let f = fs::read_to_string(path);
    let s = f.unwrap().to_string();
    let docs = YamlLoader::load_from_str(&s).unwrap();
    docs
}

fn prepare_live(
    api_key: &str,
    secret: &str,
    symbol: &str,
    tick_size: f32,
    lot_size: f32,
) -> LiveBot<HashMapMarketDepth> {
    let binance_futures = BinanceFutures::builder()
        .endpoint(Endpoint::Public)
        .api_key(api_key)
        .secret(secret)
        // .order_prefix(symbol)
        .build()
        .unwrap();

    let mut hbt = LiveBot::builder()
        .register("binancefutures", binance_futures)
        .add("binancefutures", symbol, tick_size, lot_size)
        .depth(|asset| HashMapMarketDepth::new(asset.tick_size, asset.lot_size))
        .build()
        .unwrap();

    hbt.run().unwrap();
    hbt
}

fn main() {
    tracing_subscriber::fmt::init();

    let args = env::args().collect::<Vec<String>>();
    let symbol = args[1].clone();

    let api_key = match env::var("API_KEY") {
        Ok(val) => val,
        Err(err) => {
            println!("{}: {}", err, "API_KEY");
            process::exit(1);
        }
    };

    let secret = match env::var("SECRET") {
        Ok(val) => val,
        Err(err) => {
            println!("{}: {}", err, "SECRET");
            process::exit(1);
        }
    };
    println!("./examples/config_{}.yaml", symbol);
    let config = load_config(format!("./examples/config_{}.yaml", symbol).as_str());
    let relative_half_spread = config[0]["relative_half_spread"].as_f64().unwrap();
    let relative_grid_interval = config[0]["relative_grid_interval"].as_f64().unwrap();
    let grid_num = config[0]["grid_num"].as_i64().unwrap();
    let min_grid_step = config[0]["min_grid_step"].as_f64().unwrap();
    let order_qty = config[0]["order_qty"].as_f64().unwrap();
    let tick_size = config[0]["tick_size"].as_f64().unwrap();
    let lot_size = config[0]["lot_size"].as_f64().unwrap();

    let skew = relative_half_spread / grid_num as f64;
    let max_position = grid_num as f64 * order_qty;

    let mut hbt = prepare_live(
        &api_key,
        &secret,
        symbol.as_str(),
        tick_size as f32,
        lot_size as f32,
    );

    println!("config {:?}", config);
    // process::exit(0);

    let mut recorder = LoggingRecorder::new();
    gridtrading(
        &mut hbt,
        &mut recorder,
        relative_half_spread,
        relative_grid_interval,
        grid_num as usize,
        min_grid_step,
        skew,
        order_qty,
        max_position,
    )
    .unwrap();
    hbt.close().unwrap();
}
