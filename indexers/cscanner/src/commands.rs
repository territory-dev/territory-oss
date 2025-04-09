use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::{PathBuf, Path};

use serde_json::Value;

use crate::ast::ClangCommand;
use crate::ipc::ScanCommandsArgs;


fn strip_cc_path<'a>(repo: &Path, args: &ScanCommandsArgs, path: &'a PathBuf) -> &'a Path {
        let path = if let Some(prefix) = &args.remove_path_prefix {
            path.strip_prefix(prefix).unwrap_or_else(|_| {
                println!("specified prefix {} not present in path {:?}", prefix, path);
                path
            })
        } else {
            &path
        };
        path.strip_prefix(repo).unwrap_or(&path)
}


fn ccs_empty(repo: &Path) -> Result<bool, &'static str> {
    // we need to parse JSON ourselves for this check due to a bug in rust clang lib
    let path = repo.join("compile_commands.json");
    let mut buf = String::new();
    File::open(path).or(Err("could not open file"))?
        .read_to_string(&mut buf).or(Err("could not read"))?;
    let ccs: Value = serde_json::from_str(&buf).or(Err("parse error"))?;
    let arr = ccs.as_array().ok_or("malformed compile_commands.json, array expected")?;
    Ok(arr.is_empty())
}


fn drop_args(cc: &mut Vec<String>) {
    cc.retain(|arg| {
        !arg.starts_with("-frandomize-layout-seed-file")
    });
}


pub fn scan_commands(cc_dir: &Path, args: &ScanCommandsArgs) -> Result<Vec<ClangCommand>, String> {
    let mut result = Vec::new();

    let lib = clang_sys::get_library();
    if let Some(lib) = lib {
        println!("libclang {:?} at {:?}", lib.version(), lib.path());
    } else {
        println!("libclang not loaded");
    };

    match ccs_empty(cc_dir) {
        Ok(false) => {},
        Ok(true) => {
            return Err("compile_commands.json empty".into());
        },
        Err(s) => {
            return Err(format!("failed to load compile_commands.json (in {:?}): {}", cc_dir, s));
        }
    }

    let comp_db = clang::CompilationDatabase::from_directory(".").unwrap();
    let comp_commands = comp_db.get_all_compile_commands();
    let mut cc_vec = comp_commands.get_commands();
    if !args.single_file.is_empty() {
        let mut paths: HashSet<PathBuf> = args.single_file.iter().map(|p| std::fs::canonicalize(p).unwrap()).collect();
        cc_vec.retain(|cmd| {
            paths.remove(&cmd.get_filename())
        });
        if !paths.is_empty() {
            return Err(format!("requested files not in compile commands: {:?}", paths));
        }
    }

    for (i, cmd) in cc_vec.iter().enumerate() {
        let cmd_filename = cmd.get_filename();
        let cmd_filename_str = cmd_filename.to_string_lossy();

        let cargs = cmd.get_arguments();
        let mut cargs: Vec<_> = cargs.into_iter().filter(|arg| arg != &cmd_filename_str).collect();
        drop_args(&mut cargs);
        if let Some(extra_args) = &args.clang_extra_args {
            cargs.append(&mut extra_args.clone());
        }

        let cmd_directory = cmd.get_directory();
        let directory = strip_cc_path(cc_dir, args, &cmd_directory);
        let directory = std::env::current_dir().unwrap().join(directory);

        let file = strip_cc_path(cc_dir, args, &cmd_filename);
        let file = directory.join(file);

        result.push(ClangCommand {
            index: i as u64 + 1,
            file,
            directory,
            args: cargs,
        });
    }

    Ok(result)
}


