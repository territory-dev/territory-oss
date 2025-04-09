#![feature(io_error_more)]

pub mod source;
pub mod ast;
pub mod ipc;
pub mod commands;

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::cmp::min;
use std::fs::File;
use std::env::{set_current_dir, current_dir};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use clap::Parser;
use ipc::{log, LockGrant, ScanOpts};
use itertools::Itertools;
use clang::EntityKind;
use if_chain::if_chain;
use rand::{thread_rng, RngCore};

use source::cur_hash;
use territory_core::{
    AbsolutePath, GToken, Location, NodeKind, Offset, RelativePath, TokenKind
};
use crate::source::{
    RangeLocations,
    clang_file_path, curloc, find_root, from_clang_location, from_clang_token_kind,
};
use crate::ast::{TransportID, Sem, Block, ClangCommand, ClangNodeContext, ClangTokenContext, LocalDefinitionLocation};
use crate::ipc::{USDriverConn, ScannerSays, DriverConn, DriverSays, Control};


#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub repo_path : PathBuf,

    #[arg(short, long)]
    pub compile_commands_dir : PathBuf,

    #[arg(short, long)]
    pub sock: PathBuf,

    #[arg(long)]
    pub chroot: Option<PathBuf>,

    #[arg(long)]
    pub setuid: Option<u32>,

    #[arg(long)]
    pub setgid: Option<u32>,

    #[arg(long, default_value="120")]
    pub socket_timeout: u64,

    #[arg(long)]
    pub dump_ccs: bool,
}


pub fn connect(args: &Args) -> Result<UnixStream, Box<dyn Error>> {
    println!("connecting to driver on {:?}", &args.sock);
    let loop_start_time = Instant::now();
    let conn_timeout = Duration::from_secs(args.socket_timeout);

    let sock = loop {
        match UnixStream::connect(&args.sock) {
            Ok(sock) => { break sock; },
            Err(e) => {
                if loop_start_time.elapsed() > conn_timeout {
                    return Err(format!("error connecting to driver on {:?}: {}", args.sock, e).into());
                }
                std::thread::sleep(std::time::Duration::from_millis(50))
            }
        }
    };

    println!("connected to driver");

    Ok(sock)
}


pub fn scanner_loop(sock: &UnixStream, args: &Args) -> Result<(), Box<dyn Error>> {
    let dc = Arc::new(Mutex::new(DriverConn::new(&sock)));

    source::with_clang(
        &args.compile_commands_dir,
        &args.chroot,
        args.setuid,
        args.setgid,
        |idx| -> Result<(), Box<dyn Error>> {
            loop {
                let resp = {
                    let mut l = dc.lock()?;
                    l.send(ScannerSays::Control(Control::Next))?;
                    l.receive()
                };
                match resp? {
                    DriverSays::Finish => {
                        return Ok(());
                    }
                    DriverSays::Again => {
                        continue;
                    }
                    DriverSays::ScanCommands(scan_commands_args) => {
                        let commands = match commands::scan_commands(&args.compile_commands_dir, &scan_commands_args) {
                            Ok(cs) => cs,
                            Err(msg) => {
                                log(&dc, &msg);
                                vec![]
                            },
                        };
                        let mut l = dc.lock().unwrap();
                        l.send(ScannerSays::Control(Control::GotCommands { commands }))?;
                    },
                    DriverSays::ClangCommand { command, opts } => {
                        log(&dc, &format!("[{}/{}] {} {:?}", command.index, opts.total_count, command.file.to_string_lossy(), command.args));
                        let i = command.index;
                        let res = scan_clang_command(Arc::clone(&dc), &args.compile_commands_dir, &idx, command, &opts);
                        if let Err(e) = res {
                            log(&dc, &format!("error when processing command {}: {}", i, e));
                        }
                    },
                    _ => { panic!("unexpected response") }
                }
            }
    }).unwrap();

    Ok(())
}


pub fn dump_ccs(args: &Args) -> Result<(), Box<dyn Error>> {
    source::with_clang(
        &args.compile_commands_dir,
        &args.chroot,
        args.setuid,
        args.setgid,
        |_idx| -> Result<(), Box<dyn Error>> {

            let commands = commands::scan_commands(
                &args.compile_commands_dir,
                &ipc::ScanCommandsArgs {
                    single_file: vec![], remove_path_prefix: None, clang_extra_args: None
                });
            println!("{:#?}", commands);

            Ok(())
        }).unwrap();
    Ok(())
}


#[derive(Debug, Copy, Clone)]
struct Annotated<'a> {
    tok: clang::token::Token<'a>,
    cur: Option<clang::Entity<'a>>,
    start: Location,
    end: Location,
}


pub fn cut_tu<'tu, 'p>(
    driver_conn: Arc<Mutex<USDriverConn>>,
    repo_path: &'tu Path,
    files_in_tu: &HashSet<RelativePath>,
    file: clang::source::File<'tu>,
    tu: &'tu clang::TranslationUnit<'tu>,
    opts: &ScanOpts
) {
    let lock_grant = lock_files(&driver_conn, files_in_tu.clone());

    let mut visited = HashSet::new();
    cut_file(Arc::clone(&driver_conn), repo_path, file, tu, &lock_grant, &mut visited, opts);
}

fn lock_files(
    driver_conn: &Arc<Mutex<USDriverConn>>,
    paths: HashSet<RelativePath>,
) -> LockGrant {
    let lock_all = ScannerSays::Locking(ipc::Locking::LockAll { paths });

    let mut conn = driver_conn.lock().unwrap();
    conn.send(lock_all).unwrap();
    let resp = conn.receive().unwrap();
    let ipc::DriverSays::LockResponse(lock_grant) = resp else {
        panic!("unexpected driver response: {:?}", resp);
    };

    lock_grant
}

fn collect_file_tree<'tu>(
    repo_path: &'tu Path,
    result: &mut HashSet<RelativePath>,
    tu: &'tu clang::TranslationUnit<'tu>,
    file: &clang::source::File<'tu>
) {
    let path = clang_file_path(&file);
    let rel_path = path.to_relative(&repo_path);
    if result.contains(&rel_path) { return; }
    result.insert(rel_path);

    for incl in file.get_includes() {
        if let Some(f) = incl.get_file() {
            collect_file_tree(&repo_path, result, tu, &f);
        }
    }
}


enum FileType {
    C,
    Asm,
}

fn file_type<'tu>(
    file: clang::source::File<'tu>,
) -> FileType {
    const ASM_EXTS: [&str; 3] = ["S", "s", "asm"];
    let p = file.get_path();
    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
    if ASM_EXTS.iter().any(|x| ext == *x) {
        FileType::Asm
    } else {
        FileType::C
    }
}

pub fn scan_clang_command(
    driver_conn: Arc<Mutex<USDriverConn>>,
    repo_path: &Path,
    clang_index: &clang::Index,
    command: ClangCommand,
    opts: &ScanOpts,
) -> Result<(), Box<dyn Error>> {
    let slice = 0; // TODO

    let original_directory = current_dir()?;
    if let Err(e) = set_current_dir(&command.directory) {
        let msg = format!("{}: could not change dir: {:?}", command.file.to_string_lossy(), e);
        log(&driver_conn, &msg);
        return Err(format!("[{}] {}", slice, msg).into());
    }
    {
        let mut wd_lock = source::WOKRDIR.lock().unwrap();
        *wd_lock = Some(command.directory);
    }

    if ! command.file.exists() {
        let msg = format!("{}: source file does not exist", command.file.to_string_lossy());
        log(&driver_conn, &msg);
        set_current_dir(original_directory)?;
        return Err(format!("[{}] {}", slice, msg).into());
    }

    let parse_result = source::parse(clang_index, &command.file, &command.args);

    let res = match parse_result {
        Ok(tu) => {
            for diag in tu.get_diagnostics() {
                log(&driver_conn, &diag.formatter().format());
                // log(&driver_conn, &format!(
                //     "{:?} {:?} {}",
                //     diag.get_severity(),
                //     diag.get_location(),
                //     diag.get_text(),
                // ));
            }
            // let start = std::time::Instant::now();
            let f = tu.get_file(&command.file).expect(&format!("file missing from TU: {:?}", command.file));

            let mut files_in_tu = HashSet::new();
            collect_file_tree(&repo_path, &mut files_in_tu, &tu, &f);

            cut_tu(Arc::clone(&driver_conn), repo_path, &files_in_tu, f, &tu, opts);
            // let cut_elapsed = start.elapsed();

            // log(&driver_conn, "writing result");
            // let start = std::time::Instant::now();
            {
                let mut l = driver_conn.lock().unwrap();
                l.send(ScannerSays::Control(Control::TUDone { source_set: files_in_tu }))?;
                let res = l.receive()?;
                assert!(res.is_continue());
            }
            // let elapsed = start.elapsed();
            // log(&driver_conn, &format!("parsing took: {:?}", parse_elapsed));
            // log(&driver_conn, &format!("cutting took: {:?}", cut_elapsed));
            // log(&driver_conn, &format!("write took: {:?}", elapsed));
            Ok(())
        },
        Err(e) => {
            let msg = format!("{}: Parse error: {:?}", command.file.to_string_lossy(), e);
            log(&driver_conn, &msg);
            Err(format!("[{}] {}", slice, msg).into())
        }
    };

    set_current_dir(original_directory)?;

    res
}


fn cut_file<'tu, 'p>(
    driver_conn: Arc<Mutex<USDriverConn>>,
    repo_path: &'tu Path,
    file: clang::source::File<'tu>,
    tu: &'tu clang::TranslationUnit<'tu>,
    lock_grant: &LockGrant,
    visited: &mut HashSet<RelativePath>,
    opts: &ScanOpts,
) {
    let ft = file_type(file);

    let path = clang_file_path(&file);
    let rel_path = path.to_relative(repo_path);

    if !opts.index_system && !rel_path.is_in_repo() {
        log(&driver_conn, &format!("skipping system file: {path}"));
        return;
    }


    if lock_grant.already_processed.contains(&rel_path) || visited.contains(&rel_path){
        return;
    }
    visited.insert(rel_path);

    for incl in file.get_includes() {
        if let Some(f) = incl.get_file() {
            cut_file(Arc::clone(&driver_conn), repo_path, f, tu, lock_grant, visited, opts);
        } else {
            log(&driver_conn, &format!("expected #include to point to a file: {}", curloc(repo_path, &incl)));
        }
    }

    println!("INSPECT {:?}", path);

    // read raw text
    let mut text = String::new();
    let Ok(mut file_) = File::open(&path) else {
        log(&driver_conn, &format!("failed to open: {path}"));
        return;
    };
    let Ok(flen) = file_.read_to_string(&mut text) else {
        log(&driver_conn, &format!("failed to read: {path}"));
        return;
    };
    let Ok(flen) = flen.try_into() else {
        log(&driver_conn, &format!("file too long: {path}"));
        return;
    };
    if flen > 5_000_000 {
        log(&driver_conn, &format!("file too long: {path}"));
        return;
    }

    let start_loc = file.get_offset_location(0);
    let end_loc = file.get_offset_location(flen);
    let rng = clang::source::SourceRange::new(start_loc, end_loc);

    let toks = rng.tokenize();
    let annotated = annotate(&tu, &toks);
    if annotated.is_empty() {
        log(&driver_conn, &format!("empty: {}", path));
        return;
    }

    match ft {
        FileType::C => {
            let cuts = vec![Cut {
                preamble: Location::zero(),
                start: Location::zero(),
                end: annotated.last().unwrap().end,
                cur: tu.get_entity(),
            }];
            let mut result = Vec::new();
            cut_nodes(driver_conn, repo_path, file, tu, &mut result, opts, path, &text, &annotated, &cuts, None, 0);
            // assert!(result.is_empty(), "nest_level=0 should not write results to array");
        },
        FileType::Asm => whole_file(driver_conn, repo_path, tu, opts, flen, path, text, &annotated)
    }
}


fn whole_file<'tu>(
    driver_conn: Arc<Mutex<USDriverConn>>,

    repo_path: &Path,
    tu: &'tu clang::TranslationUnit<'tu>,
    opts: &ScanOpts,

    flen: u32,
    path: AbsolutePath,
    text: String,
    annotated: &[Annotated<'tu>],
) {
    let rel_path = path.to_relative(repo_path);

    let root = tu.get_entity();
    let mut block_sems = HashMap::new();
    let mut tokens = Vec::new(); // TODO

    for Annotated { tok, cur, start, end } in annotated {
        let gt = GToken {
            offset: start.off,
            line: start.line,
            text: text[start.off as usize..end.off as usize].into(),
            type_: from_clang_token_kind(tok.get_kind()),
            context: ClangTokenContext::Token {
                // tok: *tok,
                sem: cur.map(|cur| {
                    let h = cur_hash(&cur);
                    if block_sems.contains_key(&h) { return h; }
                    let sem = cur_to_sem(repo_path, &cur, &vec![]);  // TODO: get definition context?
                    block_sems.insert(h, sem);
                    return h;
                }),
                start: *start,
                end: *end,
            },
        };
        tokens.push(gt);
        if tokens.len() >= opts.max_block_len {
            log(&driver_conn, &format!(
                "truncating, node too long: {}:{}:{} - {}:{}",
                rel_path,
                start.line, start.col,
                end.line, end.col,
            ));
            let gt = GToken {
                offset: end.off,
                line: end.line,
                text: "<TRUNCATED>".to_string(),
                type_: TokenKind::Comment,
                context: ClangTokenContext::Token { sem: None, start: *end, end: *end },
            };
            tokens.push(gt);
            break;
        }
    }

    let mut rng = thread_rng();
    let transport_key = rng.next_u64();
    let mut source_file_node = Block {
        transport_key,

        kind: NodeKind::SourceFile,
        member_of: None,
        start: Location::zero(),
        end: annotated.last().unwrap().end,
        sems: block_sems,
        text: tokens,
        context: ClangNodeContext {
            abs_path: path.clone(),
            relative_path: rel_path.clone(),
            // tu: &tu,
            root: cur_to_sem(repo_path, &root, &get_definition_context(root)),
            end_offset: Some(flen),
            nested: None,
            nest_level: 0,
            is_forward_decl: false,
        },
    };
    insert_whitespace(&mut source_file_node, &text);
    {
        let mut l = driver_conn.lock().unwrap();
        l.send(ScannerSays::Control(Control::GotBlock { block: source_file_node })).unwrap();
        let rcvd = l.receive().unwrap();
        assert!(rcvd.is_block_received());
    }
}


#[derive(Debug)]
struct Cut<'tu> {
    /// whatever should be included in a node but not cut reccursively
    preamble: Location,

    start: Location,
    end: Location,
    cur: clang::Entity<'tu>,
}


fn make_cut_points_for_file<'tu>(
    file: clang::source::File<'tu>,
    tu: &'tu clang::TranslationUnit<'tu>,
    nested_in: Option<clang::Entity<'tu>>,
    annotated: &[Annotated<'tu>],
) -> Vec<Cut<'tu>> {
    let mut state_cur = None;
    let mut state_pre = None;
    let mut state_start = None;
    let mut state_end = None;

    let mut result = Vec::new();

    let tu_root = tu.get_entity();
    let under = nested_in.unwrap_or(tu_root);

    // let scissors_dbg = get_debug_cfg().print_blocks;

    for (i, at) in annotated.iter().enumerate() {
        // print_dbg(path, at);

        // some template<> tokens are missing from clang tokenization
        // if state_pre.is_none() && (
        //     at.cur.is_none()  || at.cur.is_some_and(|cur| find_root(cur, tu_root) == cur)
        //     // true
        // ) && at.tok.get_kind() == clang::token::TokenKind::Keyword && at.tok.get_spelling() == "template" {
        //     state_pre = Some(at.start);
        // }
        // Reset preamble if we are in a nested block and the parent block node is just starting.
        // Avoid some cases where template<> of the outer block would get caught into the preamble.
        // if nested_in.and_then(|e| e.start(&file)) == Some(at.start) { state_pre = None; }

        if let (Some(tok_cur), None) = (at.cur, &state_cur) {
            if should_start_block(&tok_cur, &nested_in) {
                state_cur = Some(tok_cur);
                state_start = Some(at.start);
                state_end = tok_cur.end(&file);
                // if scissors_dbg { println!("start {:?}:{:?}", state_start, state_end); }
            }
        }

        // backtrack if cur covers prior nodes
        if_chain!(
            if let Some(state_start_loc) = state_start;

            if let Some(cur) = at.cur;
            let root = find_root(cur, under);

            if let Some(new_start) = root.start(&file);
            if new_start <= state_start_loc;

            then {
                while let Some(Cut { end: prev_end, .. }) = result.last() {
                    if new_start > *prev_end { break; }
                    result.pop();
                }

                state_start = Some(new_start);
                state_cur = Some(root);

                let new_end = root.end(&file);
                let should_update_end = match (new_end, state_end) {
                    (Some(re), Some(se)) if re > se => true,
                    (Some(_), None)                 => true,
                    _                               => false
                };
                if should_update_end { state_end = new_end; }
                // if scissors_dbg { println!("expand {:?}:{:?}", state_start, state_end); }
            }
        );

        if let Some(sc) = state_cur {
            let should_cut_block =
                if let Some(next_at) = annotated.get(i + 1) {
                    if let Some(end) = state_end {
                        // we know when the cur ends
                        next_at.start.off >= end.off
                    } else if let Some(nc) = next_at.cur {
                        // strange case of cross-file cur
                        nc != sc &&
                        nc.get_lexical_parent() == Some(tu_root)
                    } else {
                        false
                    }
                } else {
                    // last tok
                    true
                };

            if should_cut_block {
                let start = state_start.unwrap_or(Location::zero());
                result.push(Cut { cur: sc, preamble: state_pre.unwrap_or(start), start, end: at.end });
                // if scissors_dbg { println!("cut {:?}:{:?}", start, at.end); }
                state_cur = None;
                state_pre = None;
                state_start = None;
                state_end = None;
            }
        }

    }

    result
}


fn cut_nodes<'tu>(
    driver_conn: Arc<Mutex<USDriverConn>>,

    repo_path: &Path,
    file: clang::source::File<'tu>,
    tu: &'tu clang::TranslationUnit<'tu>,
    // sent_sems: HashSet<TransportID>,
    result: &mut Vec<(Location, Location, TransportID)>,
    opts: &ScanOpts,

    path: AbsolutePath,
    text: &str,
    annotated: &[Annotated<'tu>],
    cuts: &[Cut<'tu>],

    nested_in: Option<clang::Entity<'tu>>,
    nest_level: usize,

) {
    let mut rng = thread_rng();

    if annotated.is_empty() {
        println!("empty: {}", path);
        return;
    }

    let rel_path = path.to_relative(repo_path);

    let mut atoks = annotated.iter();
    for cut in cuts {
        let mut block_sems = HashMap::new();
        let mut gtoks = Vec::new();
        while let Some(at) = atoks.next() {

            if at.start.off >= cut.preamble.off {
                let mut definition_context: Vec<String> = if let Some(cur) = at.cur {
                    get_definition_context(cur)
                } else { vec![] };

                if definition_context.is_empty() {
                    // Not exact (wrong for multiple decls node) but parent info is missing for macros
                    definition_context = cut.cur.get_name().map(|n| n.into()).into_iter().collect()
                };

                let gt = GToken {
                    offset: at.start.off,
                    line: at.start.line,
                    text: text[at.start.off as usize..at.end.off as usize].into(),
                    type_: from_clang_token_kind(at.tok.get_kind()),
                    context: ClangTokenContext::Token {
                        // tok: at.tok,
                        sem: at.cur.map(|cur| {
                            let h = cur_hash(&cur);
                            if block_sems.contains_key(&h) { return h; }
                            let sem = cur_to_sem(repo_path, &cur, &definition_context);
                            block_sems.insert(h, sem);
                            return h;
                        }),
                        start: at.start,
                        end: at.end,
                    },
                };
                gtoks.push(gt);
                if gtoks.len() >= opts.max_block_len {
                    log(&driver_conn, &format!(
                        "truncating, node too long: {}:{}:{} - {}:{}",
                        rel_path,
                        cut.start.line, cut.start.col,
                        cut.end.line, cut.end.col,
                    ));
                    let gt = GToken {
                        offset: at.end.off,
                        line: at.end.line,
                        text: "<TRUNCATED>".to_string(),
                        type_: TokenKind::Comment,
                        context: ClangTokenContext::Token { sem: None, start: at.end, end: at.end },
                    };
                    gtoks.push(gt);
                    break;
                }
            }

            if at.end.off >= cut.end.off {
                break;
            }

        };

        let transport_key = rng.next_u64();
        let mut block = Block {
            transport_key,

            kind: if nest_level == 0 {
                NodeKind::SourceFile
            } else {
                match cut.cur.get_kind() {
                    EntityKind::ClassDecl     |
                    EntityKind::ClassTemplate => NodeKind::Class,
                    _                         => NodeKind::Definition,
                }
            },
            member_of: if nest_level <= 1 { None } else {
                nested_in.and_then(|ncur| ncur.get_name()).map(|s| s.into())
            },
            start: cut.preamble,
            end: cut.end,
            // pre_comment,
            sems: block_sems,
            text: gtoks,
            context: ClangNodeContext {
                abs_path: path.clone(),
                relative_path: rel_path.clone(),
                root: cur_to_sem(repo_path, &cut.cur, &get_definition_context(cut.cur)),
                end_offset: Some(cut.end.off),
                nested: None,
                nest_level,
                is_forward_decl: {
                    let d = cut.cur.get_definition();
                    d.is_some() && d != Some(cut.cur)
                }
            },
        };

        insert_whitespace(&mut block, &text);
        if [NodeKind::SourceFile, NodeKind::Class].contains(&block.kind) && nest_level < 3 {
            let mut nested_result = Vec::new();

            // if get_debug_cfg().print_token_node {
            //     println!(
            //         "running nested cut for class: {}", curloc(repo_path, &block.context.root)
            //     );
            // }


            let new_slice = slice_annotated(annotated, cut.start.off, cut.end.off);
            if nest_level > 0 && new_slice.len() == annotated.len() {
                println!(
                    "nested slice covers full original slice: {}:{}-{}",
                    path.to_string(), cut.start.line, cut.end.line);
            } else {
                let cuts = make_cut_points_for_file(file, tu, (nest_level > 0).then_some(cut.cur), &new_slice);
                cut_nodes(
                    Arc::clone(&driver_conn),
                    repo_path,
                    file,
                    tu,
                    &mut nested_result,
                    opts,
                    path.clone(),
                    text,
                    &new_slice,
                    &cuts,
                    Some(cut.cur),
                    nest_level + 1,
                );
                block.context.nested = Some(nested_result);

                elide_nested(&mut block);
            }
        }

        // if get_debug_cfg().print_blocks {
        //     let mut l = std::io::stdout().lock();
        //     territory_core::pretty_print::gnode(&mut l, &block).unwrap();
        // }
        {
            let mut l = driver_conn.lock().unwrap();
            l.send(ScannerSays::Control(Control::GotBlock { block })).unwrap();
            let rcvd = l.receive().unwrap();
            assert!(rcvd.is_block_received());
        }
        if nest_level > 0 {
            result.push((cut.preamble, cut.end, transport_key));
        }
    }

    if nested_in.is_none() {
        // check_uncut(&path, path_id, annotated, result);
    }
}


fn should_start_block(cur: &clang::Entity, nested_in: &Option<clang::Entity<'_>>) -> bool {
    use EntityKind::*;

    let ck = cur.get_kind();
    if [
        Namespace,
        LinkageSpec,
    ].contains(&ck) { return false; }

    if let Some(under) = nested_in {
        // if *cur == *container { return false; }
        let r = find_root(*cur, *under);
        return [Method, Constructor, Destructor, FunctionTemplate].contains(&r.get_kind()) && r.is_definition();
    }

    true
}

fn annotate<'a>(
    tu: &'a clang::TranslationUnit<'a>,
    toks: &[clang::token::Token<'a>],
) -> Vec<Annotated<'a>> {
    let at = tu.annotate(toks);
    toks.into_iter().zip(at).map(|(tok, cur)| {
        let rng = tok.get_range();
        let cstart = rng.get_start().get_spelling_location();
        let start = from_clang_location(&cstart);
        let cend = rng.get_end().get_spelling_location();
        let end = from_clang_location(&cend);

        Annotated {
            tok: *tok,
            cur,
            start, end
        }
    }).collect()
}


fn insert_whitespace(b: &mut Block, raw_file: &str) {
    let mut v: Vec<GToken<ClangTokenContext>> = Vec::with_capacity(b.text.capacity() * 2);

    std::mem::swap(&mut v, &mut b.text);

    let mut off = b.start.off;
    let mut line = b.start.line;
    for slice_tok in v {
        if let ClangTokenContext::Token { start, end, .. } = &slice_tok.context {
            let pre_space = &raw_file[off as usize..start.off as usize];
            let tend = end.off;
            let tendl = end.line;

            if pre_space.len() > 0 {
                let gt = GToken {
                    offset: off,
                    line,
                    type_: TokenKind::WS,
                    text: pre_space.into(),
                    context: ClangTokenContext::Whitespace { text: pre_space.to_owned() }
                };
                b.text.push(gt);
            }
            b.text.push(slice_tok);

            off = tend;
            line = tendl;
        }
    }

    b.text.shrink_to_fit();
}


fn get_definition_context(cur: clang::Entity) -> Vec<String> {
    let mut next = Some(cur);
    let mut res = Vec::new();
    while let Some(cur) = next {
        next = cur.get_semantic_parent();
        if cur.is_definition() || cur.is_declaration() {
            let Some(dname) = cur.get_name() else {
                continue;
            };

            res.push(dname.into());
        }
    }

    res
}

fn slice_annotated<'tu>(
    annotated: &[Annotated<'tu>],
    start: Offset,
    end: Offset,
) -> Vec<Annotated<'tu>> {
    let start_token_index = match annotated.binary_search_by_key(&start, |Annotated { start, .. }| start.off) {
        Ok(i) => i,
        Err(i) => min(i, annotated.len() - 1),
    };

    let end_token_index = match annotated.binary_search_by_key(&end, |Annotated { end, .. }| end.off) {
        Ok(i) => i,
        Err(i) => i - 1,
    };

    annotated[start_token_index..=end_token_index].to_vec()
}


fn elide_nested(block: &mut Block) {
    let Some(mut nested_blocks) = block.context.nested.take() else { return; };
    if nested_blocks.is_empty() { return; }
    nested_blocks.sort_by_key(|(start, _, _)| start.off);

    let orig_text = std::mem::replace(&mut block.text, Vec::new());
    let filtered: Vec<GToken<ClangTokenContext>> = orig_text.into_iter().filter(|tok| {
        match nested_blocks.binary_search_by_key(&tok.offset, |(start, _, _)| start.off) {
            Ok(_) => false,
            Err(l) => nested_blocks.get(l.saturating_sub(1))
                .map(|(start, end, _)| !(start.off <= tok.offset && tok.offset < end.off))
                .unwrap_or(true),
        }
    }).collect();

    let mut elisions = nested_blocks.into_iter().map(|(start, end, id)| {
        GToken {
            offset: start.off,
            line: start.line,
            text: " … ".into(),
            type_: TokenKind::Identifier,
            context: ClangTokenContext::Elided { start_offset: start.off, end_offset: end.off, nested_block_key: id },
        }
    });
    block.text = vec![&mut filtered.into_iter() as &mut dyn Iterator<Item = _>, &mut elisions]
        .into_iter()
        .kmerge_by(|a, b| a.offset < b.offset)
        .collect();
}


/*
fn matches_debug_filter(debug_cfg: &DebugCfg, path: &AbsolutePath, at: &Annotated) -> bool {
    let mut location_matches_debug_filter = true;
    if let Some((dfile, dstart, dend)) = &debug_cfg.location_filter {
        if !(Some(dfile.as_str()) == path.file_name().map(|s| s.to_str().unwrap()) &&
             at.end.line >= *dstart && at.start.line <= *dend) {
            location_matches_debug_filter = false;
        }
    }
    location_matches_debug_filter
}

fn print_dbg(path: &AbsolutePath, at: &Annotated) {
    let debug_cfg = get_debug_cfg();

    if matches_debug_filter(debug_cfg, path, at) {
        if let (true, Some(cur)) = (debug_cfg.print_token_subtree, at.cur) {
            println!("    {} [{:?}]", at.tok.get_spelling(), at.tok.get_kind());
            dump_tree("        ", cur);
        } else if let (true, Some(cur)) = (debug_cfg.print_token_node, at.cur) {
            println!("    {} [{:?}]", at.tok.get_spelling(), at.tok.get_kind());
            dump_cur("        ", cur);
        } else if let (true, None) = (debug_cfg.print_token_node, at.cur) {
            println!("    {} [{:?}]", at.tok.get_spelling(), at.tok.get_kind());
        }
    }
}*/


/*
fn check_uncut<'tu>(
    path: &AbsolutePath,
    path_id: PathID,
    annotated: &[Annotated<'tu>],
    result: &Vec<Block<'tu>>,
) {
    let debug_cfg = get_debug_cfg();
    if !debug_cfg.print_uncut_tokens { return; }

    for at in annotated {
        if !matches_debug_filter(debug_cfg, path, at) { continue; }
        match at.tok.get_kind() {
            clang::token::TokenKind::Comment => { continue; }
            _ => {
                if at.tok.get_spelling() == ";" { continue; }
                if result.get(path_id, at.start.off).is_none() {
                    println!("token not covered by any node: {:?} {} at {}:{}", at.tok.get_kind(), at.tok.get_spelling(), at.start.line, at.start.col);
                }
            }
        }
    }
} */

fn cur_to_sem(repo_dir: &Path, cur: &clang::Entity, definition_context: &Vec<String>) -> Sem {
    use clang::EntityKind::*;
    let ref_ = if [Method, Constructor, Destructor, TemplateRef, TypeRef].contains(&cur.get_kind()) {
        cur.get_definition()
    } else {
        cur.get_reference()
    };
    let defn = ref_.and_then(|defn_cur| {
        let kind = defn_cur.get_kind();
        if ![VarDecl, MacroDefinition].contains(&kind) && !defn_cur.is_definition() || kind == Namespace
        {
            return None;
        }

        query_cur_location(repo_dir, &defn_cur)
    });

    let cur_usr = cur.get_usr();
    let ref_usr = cur.get_reference().and_then(|re| {
        re.get_usr()
    });
    let usr = cur_usr.or(ref_usr);

    let cur_start_loc = cur.get_range().map(|rng| rng.get_start().get_spelling_location());
    let cur_end_loc = cur.get_range().map(|rng| rng.get_end().get_spelling_location());

    Sem {
        transport_id: cur_hash(cur),
        is_declaration: cur.is_declaration(),
        is_definition: cur.is_definition(),
        is_function_like_macro: cur.is_function_like_macro(),
        usr: usr.map(|clang::Usr(u)| u.into()),
        name: cur.get_name().map(Into::into),
        kind: cur.get_kind().into(),
        linkage: cur.get_linkage().map(Into::into),
        local_defintion: defn,
        type_: cur.get_type().map(|t| t.get_display_name().into()),
        cur_start_offset: cur_start_loc.map(|loc| loc.offset),
        cur_end_offset: cur_end_loc.and_then(|loc| {
            if loc.file != cur_start_loc.and_then(|start_loc| start_loc.file) {
                return None;
            }

            return Some(loc.offset)
        }),
        definition_context: definition_context.clone(),
        display_name: cur.get_display_name().map(Into::into),
        curloc: curloc(repo_dir, cur),
    }
}


fn query_cur_location(
    repo_dir: &Path,
    cur: &clang::Entity,
) -> Option<LocalDefinitionLocation> {
    let path = source::cur_path(cur)?;

    let rel_path = path.to_relative(repo_dir);

    let loc = cur.get_location()?;
    let offset = loc.get_spelling_location().offset;

    Some(LocalDefinitionLocation { path: rel_path, offset, curloc: curloc(repo_dir, cur) })
}
