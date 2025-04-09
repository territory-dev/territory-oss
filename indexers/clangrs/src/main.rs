use std::fs::{create_dir_all, File};
use std::os::fd::FromRawFd;
use std::sync::Arc;

use clap::Parser;
use log::info;
use simplelog::{ColorChoice, CombinedLogger, LevelFilter, SharedLogger, TermLogger, TerminalMode, WriteLogger};

use clangrs::args::{Args, Stage};
use clangrs::parse_stage::{parse_stage, parse_stage_with_stores};
use clangrs::serial_stage::serial_stage_with_stores;
use clangrs::uses_stage::{uses_stage, uses_stage_with_store};
use clangrs::output_stage::{output_stage, output_stage_with_stores};
use clangrs::intermediate_model::sqlite;


#[tokio::main]
async fn main() {
    let args = Args::parse();
    if let Some(fd) = args.status_fd {
        let mut _file = unsafe { File::from_raw_fd(fd) };
    }

    let mut loggers: Vec<Box<dyn SharedLogger>> = vec![
            TermLogger::new(
                LevelFilter::Debug,
                simplelog::Config::default(),
                TerminalMode::Mixed,
                ColorChoice::Auto),
    ];
    if let Some(log_dir) = &args.log_dir {
        create_dir_all(&log_dir).unwrap();

        loggers.push(
            WriteLogger::new(
                LevelFilter::Info,
                simplelog::Config::default(),
                File::create(log_dir.join("index")).unwrap()));
    }
    CombinedLogger::init(loggers).unwrap();

    info!(
        "clangrs indexer starting. build sha: {}",
        option_env!("BUILD_SHA").unwrap_or("unknown build"));

    if let Some(uim_input) = &args.uim_input {
        let p = uim_input.clone();
        clangrs::uim::index_uim(args, &p).await;
        return;
    }

    match args.stage {
        None => {
            run_stages(&args).await;
        },

        Some(Stage::Parse) => {
            parse_stage(&args);
        },

        Some(Stage::Uses) => {
            uses_stage(&args);
        },

        Some(Stage::Output) => {
            output_stage(&args).await;
        },
    }
}


pub async fn run_stages(args: &Args) {
    let mut t = clangrs::timers::Timers::new();

    let mut store = t.timed("create tables", || {
        info!("opening db: {:?}", args.db_path);
        let store = sqlite::new_from_args::<sqlite::SqliteGSMWriter, sqlite::SqliteUMWriter>(args);
        info!("preparing tables");
        store.create_tables();
        store
    });

    t.timed("parse stage", || {
        parse_stage_with_stores(args, &mut store)
    });

    let mut store = sqlite::new_with_conn(store.conn);
    t.async_timed("uses stage", async {
        uses_stage_with_store(&args, &mut store);
    }).await;

    let conn = Arc::clone(&store.conn);
    let store = sqlite::new_with_conn(Arc::clone(&conn));
    t.async_timed("output stage", async {
        output_stage_with_stores(args, store).await;
    }).await;

    let store = sqlite::new_with_conn(Arc::clone(&conn));
    t.async_timed("serial stage", async {
        serial_stage_with_stores(args, store).await;
    }).await;

    t.dump();
}
