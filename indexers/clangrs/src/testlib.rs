use std::process::Command;
use std::sync::Once;
use std::env::current_dir;
use std::fmt::Debug;
use std::fs::{File, create_dir_all};
use std::io::{Read, Write};
use std::path::{PathBuf, Path};
use std::env::var;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use rusqlite::Connection;
use prost::Message;
use testdir::testdir;
use similar::{ChangeTag, TextDiff};
use lazy_static::lazy_static;
use rand::random;

use territory_core::pblib::decode_many;
use territory_core::resolver::{BasicResolver, ConcreteLocation, NeedData, ResolutionFailure, Resolver, TrieResolver};
use territory_core::territory::index::{Node, References, Build, IndexItem, IndexItemKind};
use territory_core::{pb_node_tokens, GenHref, IntoGenHref, ReferencesLink, Token};
use crate::args::{Args, CompressionMode};
use crate::intermediate_model::sqlite;

lazy_static! {
    pub static ref TERRITORY_ROOT: PathBuf = {
        if let Ok(ps) = var("TT_REPO_ROOT") {
            return PathBuf::from(ps);
        }

        let mut p = current_dir().unwrap();
        while !p.join(".git").exists() {
            if !p.pop() {
                panic!("no .git found, tests not executed from within the repo tree?");
            }
        }
        p
    };
}


pub struct InspectResult {
    pub conn: Arc<Mutex<Connection>>,
}

pub fn run_scanner_thread(repo_path: PathBuf, socket_path: PathBuf) -> JoinHandle<()> {
    std::thread::spawn(|| {
        let scanner_args = cscanner::Args {
            compile_commands_dir: repo_path.clone(),
            repo_path,
            sock: socket_path,
            chroot: None,
            setuid: None,
            setgid: None,
            socket_timeout: 30,
            dump_ccs: false,
        };
        let conn = cscanner::connect(&scanner_args).unwrap();
        cscanner::scanner_loop(&conn, &scanner_args).unwrap();
    })
}


pub fn init_logging() {
    let _ = simplelog::TestLogger::init(
        simplelog::LevelFilter::max(),
        simplelog::Config::default());

}

pub fn inspect_repo(args: &Args) -> InspectResult {
    init_logging();

    let mut stores_with_writer = sqlite::new_from_args(args);
    stores_with_writer.create_tables();

    let ta = args.clone();
    let scanner_thread = run_scanner_thread(ta.repo, ta.scanner_socket_path);

    crate::parse_stage::parse_stage_with_stores(args, &mut stores_with_writer);

    scanner_thread.join().unwrap();

    let mut stores_with_reader = sqlite::new_with_conn(stores_with_writer.conn);
    crate::uses_stage::uses_stage_with_store(args, &mut stores_with_reader);

    let conn = Arc::clone(&stores_with_reader.conn);
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let stores_with_uses = sqlite::new_with_conn(stores_with_reader.conn);
        crate::output_stage::output_stage_with_stores(args, stores_with_uses).await;

        let stores_with_uses = sqlite::new_with_conn(Arc::clone(&conn));
        crate::serial_stage::serial_stage_with_stores(args, stores_with_uses).await;
    });

    InspectResult { conn }
}



fn get_build(index_path: &PathBuf, build_id: &str) -> Build {
    let full_path = index_path.join("builds").join(build_id);

    let mut buf = Vec::<u8>::new();
    File::open(&full_path)
        .expect("error opening build file")
        .read_to_end(&mut buf)
        .unwrap();
    Build::decode(&buf[..]).unwrap()
}

pub struct GraphWalker {
    index_path: PathBuf,
    current_node: Node,
    history: Vec<Node>,
    pub resolver: TrieResolver<BasicResolver>,
}

impl<'a> GraphWalker {
    pub fn new(index_path: PathBuf) -> GraphWalker {
        Self::new_with_build(index_path, "test_repo", "test_build")
    }
    pub fn new_with_build(index_path: PathBuf, repo_id: &str, build_id: &str) -> GraphWalker {
        let build: Build = get_build(&index_path, &format!("{repo_id}/{build_id}"));

        let trie_cache = territory_core::slicemap_trie::SharedCache::new(1024);
        let nodemap = territory_core::slicemap_trie::SlicemapReader::new(
            build.nodemap_trie_root.unwrap(),
            territory_core::slicemap_trie::SharedCache::new_handle(&trie_cache, "test_repo/nodes"));
        let symmap = territory_core::slicemap_trie::SlicemapReader::new(
            build.symmap_trie_root.unwrap(),
            territory_core::slicemap_trie::SharedCache::new_handle(&trie_cache, "test_repo/syms"));
        let refmap = territory_core::slicemap_trie::SlicemapReader::new(
            build.references_trie_root.unwrap(),
            territory_core::slicemap_trie::SharedCache::new_handle(&trie_cache, "test_repo/refs"));
        let trie_resolver = TrieResolver::new(
            BasicResolver,
            nodemap,
            symmap,
            refmap,
            build.repo_root_node_id,
        );

        let mut gw = GraphWalker { index_path, current_node: Node::default(), history: vec![], resolver: trie_resolver };
        gw.go_to_node(gw.root_ref());
        gw
    }

    pub fn index_path(&'a self) -> &'a Path {
        &self.index_path
    }

    pub fn reset(&mut self) {
        self.history.clear();
        self.go_to_node(self.root_ref());
    }

    pub fn root_ref(&self) -> GenHref {
        GenHref::Path("".into())
    }

    pub fn follow_token(&mut self, text: &str) {
        self.follow_token_by(&mut |tok| tok.text.trim() == text).expect(&format!("failed to follow {text}"))
    }

    pub fn follow_nth_token(&mut self, text: &str, nth: usize) {
        let mut i = 0;
        self.follow_token_by(&mut |tok| {
            if tok.text.trim() == text {
                i+=1;
                if i == nth { return true; }
            }
            false
        }).expect(&format!("follow nth token ({:?}, {:?})", text, nth));
    }

    pub fn follow_token_by(&mut self, f: &mut dyn FnMut(&Token) -> bool) -> Result<(), Box<dyn std::error::Error>> {
        let tok = self.find_token_by(f).ok_or("token not found")?;
        let next_node = tok.context.href.ok_or("found token matching query, but no link")?;
        self.go_to_node(next_node);
        Ok(())
    }

    pub fn find_token(&mut self, text: &str) -> Option<Token> {
        self.find_token_by(&mut |tok| tok.text.trim() == text)
    }

    pub fn find_token_by(&mut self, f: &mut dyn FnMut(&Token) -> bool) -> Option<Token> {
        for tok in pb_node_tokens(&self.current_node) {
            if f(&tok) {
                return Some(tok);
            }
        }
        None
    }

    pub fn back(&mut self) -> Result<(), ()> {
        self.current_node = self.history.pop().ok_or(())?;
        Ok(())
    }

    pub fn token_references(&mut self, text: &str) -> References {
        for tok in pb_node_tokens(&self.current_node) {
            if tok.text == text {
                return self.resolve_token_references(tok.context.references);
            }
        }

        panic!("token not found: {:?}", text);
    }

    pub fn resolve_token_references(&mut self, refs: ReferencesLink) -> References {
        match refs {
            ReferencesLink::TokenLocation(token_location) => {
                return self.load(GenHref::RefsId(token_location));
            }
            ReferencesLink::LegacyID(_) => {
                panic!("found legacy token id as references link");
            }
            ReferencesLink::None => {
                panic!("found token matching query, but no references");
            }
        }
    }

    pub fn node(&'a self) -> &'a Node {
        &self.current_node
    }

    pub fn print_node(&self) {
        println!("dump: {:#?}", self.current_node);
        println!("tokens: {:#?}", pb_node_tokens(&self.current_node));
        println!("text: {}", self.current_node.text);
    }

    pub fn go_to_node(&mut self, href: impl IntoGenHref) {
        let mut n = self.load_node(href);
        std::mem::swap(&mut n, &mut self.current_node);
        self.history.push(n);
    }

    pub fn dump_nodes(&self) {
        todo!();

        /*
        for (node_id, node_loc) in self.resolver.nodes() {
            println!("\nnode #{} in {:?}", node_id, node_loc);
            let node: Node = self.load_from_loc(&node_loc);
            let mut text = node.text.clone();
            text.truncate(100);
            println!(
                "node.id: {}\nnode.kind: {:?}\nnode.path: {:?}\nnode.text: {}",
                node.id, node.kind(), node.path, text);
            for t in &node.tokens {
                if let Some(h) = t.href {
                    println!("token at {} -> {:?}", t.offset, h);
                }
            }
        }

        println!("\n\n## PATHS");
        for (path, node_id) in self.resolver.paths() {
            println!("{} -> {}", path, node_id);
        }

        println!("\n\n## SYM");
        for (sym_id, usr, node_id) in self.resolver.sym() {
            println!("{:?} {} -> {}", sym_id, usr, node_id);
        }
            */
    }

    fn load_node(&mut self, href: impl IntoGenHref) -> Node {
        self.load(href)
    }

    fn load<T>(&mut self, href: impl IntoGenHref) -> T where T: Message + Default {
        let loc = self.resolve_href(href);
        self.load_from_loc(&loc)
    }

    pub fn resolve_href(&mut self, href: impl IntoGenHref) -> ConcreteLocation {
        let href = href.into_gen_href();
        for _ in 0..10 {
            let resolve_res = self.resolver.resolve_href(&href);
            match resolve_res {
                Ok(loc) => {
                    return loc;
                },
                Err(ResolutionFailure::NeedData(NeedData(loc, cont))) => {
                    let bytes = self.load_bytes(&loc);
                    cont(&bytes).unwrap();
                },
                Err(e) => {
                    panic!("load failed for href {href:?}: {e:?}");
                },
            }
        }
        panic!("failed to resolve in 10 attempts: {:?}", href);
    }

    fn load_from_loc<T>(&self, loc: &ConcreteLocation) -> T where T: Message + Default {
        let bytes = self.load_bytes(loc);
        T::decode(&bytes[..]).unwrap()
    }

    fn load_bytes(&self, loc: &ConcreteLocation) -> Vec<u8> {
        let full_path = self.index_path.join("nodes/test_repo").join(&loc.path);

        let mut buf = Vec::<u8>::new();
        File::open(&full_path)
            .expect(&format!("error opening file: {:?} (resolved as {:?})", full_path, loc))
            .read_to_end(&mut buf)
            .unwrap();

        let bytes = match loc.blob_bytes {
            Some((l, r)) => &buf[l as usize..r as usize],
            None => &buf,
        };

        bytes.to_owned()
    }
}


pub fn defaut_args() -> Args {
    let resource_dir = var("CLANG_RESOURCE_DIR").unwrap_or("/usr/lib/llvm-18/lib/clang/18".to_string());
    let temp_dir_ = testdir!();
    Args {
        repo: TERRITORY_ROOT.join("repos/example"),
        storage_mode: crate::args::StorageMode::File,
        outdir: temp_dir_.join("output"),
        compression: CompressionMode::None,
        writer_concurrency: 1,
        store_concurrency: 4,
        single_file: vec![],
        fastwait: true,
        remove_path_prefix: None,
        repo_id: "test_repo".to_string(),
        build_id: "test_build".to_string(),
        bucket: "territory-index-scrap".to_string(),
        status_fd: None,
        intermediate_path: temp_dir_.join("model"),
        db_path: temp_dir_.join("model").join("sem.db"),
        par: 1,
        slice: 1,
        stage: None,
        no_references: false,
        clang_extra_args: Some(vec![format!("-resource-dir={resource_dir}")]),
        fatal_missing_spans: false,
        scanner_socket_path: Path::new("/tmp").join(format!("clangrs-scanner-{}.sock", random::<u64>())),
        scanner_socket_timeout: 5,
        log_dir: None,
        index_system: true,
        uim_input: None,
        max_node_len: 100_000,
    }
}


pub fn index_example() -> GraphWalker {
    index_with_changed_args(|_| {})
}


pub fn index_with_changed_args(f: impl FnOnce(&mut Args)) -> GraphWalker {
    let mut args = defaut_args();
    f(&mut args);
    inspect_repo(&args);

    GraphWalker::new_with_build(args.outdir, &args.repo_id, &args.build_id)
}


pub struct RepoWriter {
    repo_dir: PathBuf,
    compile_commands: Vec<String>,
}

impl RepoWriter {
    pub fn new(test_dir: &Path) -> Self {
        Self {
            repo_dir: test_dir.join("repo"),
            compile_commands: Vec::new(),
        }
    }

    pub fn repo_dir(&self) -> &PathBuf { &self.repo_dir }

    pub fn add_c_unit(&mut self, path: &str, code: &str) -> std::io::Result<()> {
        self.add_unit("clang", path, code)
    }

    pub fn add_cpp_unit(&mut self, path: &str, code: &str) -> std::io::Result<()> {
        self.add_unit("clang++", path, code)
    }

    fn add_unit(&mut self, compiler: &str, path: &str, code: &str) -> std::io::Result<()>  {
        let full_path = self.repo_dir.join(path);
        self.write(&full_path, code)?;

        let mut out_path = full_path.to_string_lossy().to_string();
        out_path.push_str(".o");

        let compile_command = format!(
            "{{ \"command\": \"{} -c -o {} {}\", \"file\": \"{}\", \"directory\": \"{}\" }}",
            compiler,
            out_path,
            full_path.to_string_lossy(),
            full_path.to_string_lossy(),
            self.repo_dir.to_string_lossy());

        self.compile_commands.push(compile_command);

        Ok(())
    }

    pub fn add(&self, path: &str, code: &str) -> std::io::Result<()> {
        let full_path = self.repo_dir.join(path);
        self.write(&full_path, code)?;
        Ok(())
    }

    pub fn add_custom_compile_command<T: std::string::ToString>(&mut self, cmd: T) {
        self.compile_commands.push(cmd.to_string());
    }

    pub fn write_clang_compile_commands(&self) -> std::io::Result<()> {
        self.add(
            "compile_commands.json",
            &format!("[{}]", self.compile_commands.join(",")))?;
        Ok(())
    }

    fn write(&self, full_path: &PathBuf, code: &str) -> std::io::Result<()> {
        create_dir_all(&full_path.parent().unwrap())?;
        File::create(full_path)?.write_all(code.as_bytes())?;
        Ok(())
    }

    pub fn update(&self, path: &str, code: &str) -> std::io::Result<()> {
        let full_path = self.repo_dir.join(path);
        File::options().write(true).open(full_path)?.write_all(code.as_bytes())?;
        Ok(())
    }

    pub fn index_repo(&self) -> GraphWalker {
        index_with_changed_args(|args| { args.repo = self.repo_dir.clone() })
    }

    pub fn index_repo_with_args(&self, f: impl FnOnce(&mut Args)) -> GraphWalker {
        index_with_changed_args(|args| { args.repo = self.repo_dir.clone(); f(args) })
    }

    pub fn dir<'a>(&'a self) -> &'a PathBuf {
        &self.repo_dir
    }
}


pub struct GoRepoWriter {
    pub repo_dir: PathBuf,
    pub uim_dir: PathBuf,
}

impl GoRepoWriter {
    pub fn new() -> Self {
        let td = testdir!();
        Self {
            repo_dir: td.join("repo"),
            uim_dir: td.join("uim"),
        }
    }

    fn write(&self, full_path: &PathBuf, code: &str) -> std::io::Result<()> {
        create_dir_all(&full_path.parent().unwrap())?;
        File::create(full_path)?.write_all(code.as_bytes())?;
        Ok(())
    }

    pub fn add_go(&self, path: &str, code: &str) -> std::io::Result<()> {
        let full_path = self.repo_dir.join(path);
        self.write(&full_path, code)?;
        Ok(())
    }

    pub fn add_mod(&self, m: &str) -> std::io::Result<()> {
        let mod_dir = self.repo_dir.join(m);
        create_dir_all(&mod_dir)?;
        let mod_file_path = mod_dir.join("go.mod");
        let content = format!(r#"module example.territory.dev/mod{}

go 1.22.7
"#, m);
        self.write(&mod_file_path, &content)?;
        Ok(())
    }

    pub fn index_repo(&self) -> GraphWalker {
        goscan(&[
            self.repo_dir.to_str().unwrap(),
            self.uim_dir.to_str().unwrap(),
        ]);

        index_uim(&self.repo_dir, &self.uim_dir)
    }

}

pub fn write_single_file_repo(test_dir: &Path, main_code: &str) -> PathBuf {
    let mut repo_writer = RepoWriter::new(test_dir);
    repo_writer.add_c_unit("main.c", main_code).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();
    repo_writer.repo_dir().clone()
}


pub fn str_diff(x: &str, y: &str) -> String {
    TextDiff::from_lines(x, y)
    .iter_all_changes()
    .map(|change|{
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal  => " ",
        };
        format!("{}{}", sign, change)
    }).collect::<String>()
}


pub fn repr_diff<T>(x: &T, y: &T) -> String
    where T: Debug
{
    str_diff(
        &format!("{:#?}", x),
        &format!("{:#?}", y))
}


pub fn read_search_index(repo_dir: &Path) -> Vec<IndexItem> {
    let mut buf = Vec::new();
    let path = repo_dir.join("search/test_repo/test_build/all");
    File::open(&path)
        .expect(&format!("opening search index file: {path:?}"))
        .read_to_end(&mut buf)
        .expect(&format!("reading search index file: {path:?}"));
    decode_many::<IndexItem>(&buf[..]).unwrap()
}


static GOSCAN_BINARY: Once = Once::new();


pub fn build_scanner() {
    GOSCAN_BINARY.call_once(|| {
        let go_indexer_path = TERRITORY_ROOT.join("indexers/go");
        let cmdout = Command::new("go")
            .current_dir(go_indexer_path)
            .env("CGO_ENABLED", "1")
            .env("CGO_CFLAGS", "-O2")
            .args([ "build", "-o", "goscan", "./main" ])
            .output()
            .unwrap();
        print!(
            "returned: {}\nstdout:\n{}\nstderr:\n{}\n",
            cmdout.status,
            String::from_utf8_lossy(&cmdout.stdout),
            String::from_utf8_lossy(&cmdout.stderr));
        assert!(cmdout.status.success());
    });
}


pub fn goscan(args: &[&str]) {
    build_scanner();

    let cmdout = Command::new(TERRITORY_ROOT.join("indexers/go/goscan"))
        .args(args)
        .output()
        .unwrap();
    print!(
        "returned: {}\nstdout:\n{}\nstderr:\n{}\n",
        cmdout.status,
        String::from_utf8_lossy(&cmdout.stdout),
        String::from_utf8_lossy(&cmdout.stderr));
    assert!(cmdout.status.success());
}


pub fn index_uim(repo: &Path, uim_dir: &Path) -> GraphWalker {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let args = crate::args::Args {
        uim_input: Some(uim_dir.into()),
        repo: repo.into(),
       ..defaut_args()
    };
    rt.block_on(async {
        crate::uim::index_uim(args.clone(), &uim_dir).await;
    });
    GraphWalker::new_with_build(args.outdir, &args.repo_id, &args.build_id)
}

