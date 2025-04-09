use std::{env::var, time::Duration};
use std::path::PathBuf;
use std::sync::OnceLock;

use clap::{Parser, ValueEnum};


#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum CompressionMode {
    None,
    Gzip,
}


#[derive(ValueEnum, Debug, Clone)]
pub enum StorageMode {
    None,
    File,
    Cloud,
}


#[derive(ValueEnum, Debug, Clone)]
pub enum Stage {
    Parse,
    Uses,
    Output,
}


#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value=".")]
    pub repo: PathBuf,

    #[arg(short = 'm', long, default_value="file")]
    pub storage_mode: StorageMode,

    #[arg(short = 'o', long, default_value=".territory/index")]
    pub outdir: PathBuf,

    #[arg(short, long, default_value="territory-index-scrap")]
    pub bucket: String,

    #[arg(long)]
    pub repo_id: String,

    #[arg(long)]
    pub build_id: String,

    #[arg(short, long, default_value_t=1)]
    pub writer_concurrency: usize,

    #[arg(short, long, default_value_t=8)]
    pub store_concurrency: usize,

    #[arg(long)]
    pub single_file: Vec<PathBuf>,

    #[arg(long, default_value_t=false)]
    pub index_system: bool,

    #[arg(short = 'c', long, default_value="gzip")]
    pub compression: CompressionMode,

    #[arg(long)]
    pub remove_path_prefix: Option<String>,

    #[arg(long)]
    pub status_fd: Option<std::os::fd::RawFd>,

    #[arg(long, default_value="/tmp/territory-build")]
    pub intermediate_path: PathBuf,

    #[arg(short = 'd', long, default_value=".territory/db")]
    pub db_path: PathBuf,

    #[arg(long, default_value_t=1)]
    pub par: usize,

    #[arg(long, default_value_t=1)]
    pub slice: usize,

    #[arg(long)]
    pub stage: Option<Stage>,

    #[arg(long, default_value_t=false)]
    pub fastwait: bool,

    #[arg(long, default_value_t=false)]
    pub no_references: bool,

    #[arg(long)]
    pub clang_extra_args: Option<Vec<String>>,

    #[arg(long)]
    pub fatal_missing_spans: bool,

    #[arg(long, default_value="/tmp/territory.sock")]
    pub scanner_socket_path: PathBuf,

    #[arg(long, default_value="5")]
    pub scanner_socket_timeout: usize,

    #[arg(short = 'l', long)]
    pub log_dir: Option<PathBuf>,

    #[arg(long)]
    pub uim_input: Option<PathBuf>,

    #[arg(long, default_value_t=100_000)]
    pub max_node_len: usize,
}

#[derive(Default)]
pub struct DebugCfg {
    pub print_token_node: bool,
    pub print_token_subtree: bool,
    pub definition_links: bool,
    pub types: bool,
    pub usrs: bool,
    pub print_sem_nodes: bool,
    pub print_references: bool,
    pub print_blocks: bool,
    pub print_global_defs: bool,
    pub location_filter: Option<(String, u32, u32)>,
    pub pretty_semfiles: bool,
    pub print_blob_writes: bool,
    pub fatal_missing_spans: bool,
    pub print_node_skips: bool,
    pub print_uncut_tokens: bool,
}

static DEBUG_CFG: OnceLock<DebugCfg> = OnceLock::new();

pub fn get_debug_cfg() -> &'static DebugCfg {
    DEBUG_CFG.get_or_init(|| {
        let mut cfg = DebugCfg::default();
        if let Ok(dbg_string) = var("IXDBG") {
            for key in dbg_string.split(",") {
                match key {
                    "n"   => { cfg.print_token_node = true; }
                    "pts" => { cfg.print_token_subtree = true; }
                    "d"   => { cfg.definition_links = true; }
                    "t"   => { cfg.types = true; }
                    "u"   => { cfg.usrs = true; }
                    "b"   => { cfg.print_blocks = true; }
                    "sem" => { cfg.print_sem_nodes = true; }
                    "ref" => { cfg.print_references = true; }
                    "xd"  => { cfg.print_global_defs = true; }
                    "sf"  => { cfg.pretty_semfiles = true; }
                    "o"   => { cfg.print_blob_writes = true; }
                    "s"   => { cfg.print_node_skips = true; }
                    "uc"  => { cfg.print_uncut_tokens = true; }
                    _     => { panic!("bad INDEXER_DEBUG value"); }
                }
            }
        }
        if let Ok(location_filer_str) = var("IXDBG_FILTER") {
            let re = regex::Regex::new(r"(.*):([0-9]+)-([0-9]+)").unwrap();
            let re_caps = re.captures(&location_filer_str)
                .expect("Invalid IXDBG_FILTER. Expected file:line-line");
            cfg.location_filter = Some((
                re_caps[1].to_owned(), re_caps[2].parse().unwrap(), re_caps[3].parse().unwrap()
            ));
        }
        cfg
    })
}


pub fn get_scanner_ipc_timeout() -> Duration {
    let timeout_str = var("SCANNER_IPC_TIMEOUT").unwrap_or("9000".to_string());
    Duration::from_secs(timeout_str.parse().unwrap())
}
