use std::error::Error;

use clap::Parser;

use cscanner::Args;


pub fn main() -> Result<(), Box<dyn Error>> {
    clang_sys::load().unwrap();
    let lib = clang_sys::get_library();
    if let Some(lib) = lib {
        println!("libclang {:?} at {:?}", lib.version(), lib.path());
    } else {
        panic!("libclang not loaded");
    };

    let args = Args::parse();
    println!("cscanner {:?}", args);

    if args.dump_ccs {
        return cscanner::dump_ccs(&args);
    }

    let sock = cscanner::connect(&args)?;
    cscanner::scanner_loop(&sock, &args)
}
