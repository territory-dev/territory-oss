use std::fs::{File, canonicalize};
use std::io::{Read, Write};
use std::path::Path;
use clangrs::args::Args;
use itertools::Itertools;
use testdir::testdir;

use territory_core::territory::index::{IndexItem, IndexItemKind};
use clangrs::testlib::{
    TERRITORY_ROOT,
    GraphWalker,
    RepoWriter,
    defaut_args,
    write_single_file_repo,
    inspect_repo,
    read_search_index,
};


#[test]
fn search_index_generated() {
    let args = defaut_args();
    inspect_repo(&args);

    let mut items = read_search_index(&args.outdir);
    items.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(items.iter().map(|ii| ii.key.clone()).collect::<Vec<String>>(), vec![
        "/",
        "DEFA",
        "bar",
        "baz",
        "dir/",
        "dir/mod2.c",
        "foo",
        "mod1.c",
        "shared.h",
    ].into_iter().map(|s| s.to_string()).collect::<Vec<String>>());
}


#[test]
fn search_index_generated_with_relative_paths() {
    let temp_dir_ = testdir!();
    let canon_test_dir = canonicalize(temp_dir_).unwrap();

    let main_code = r#"#include "incl.h"
int main() {}"#;
    let repo_dir = write_single_file_repo(&canon_test_dir, main_code);

    let mut include_file = repo_dir.clone();
    include_file.push("incl.h");
    File::create(include_file).unwrap().write_all(br#"#define FOO"#).unwrap();
    let mut args = defaut_args();

    args.repo = repo_dir;
    inspect_repo(&args);

    let mut items = read_search_index(&args.outdir);
    items.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(items.iter().map(|ii| ii.key.clone()).collect::<Vec<String>>(), vec![
        "/", "FOO", "incl.h", "main", "main.c"
    ].into_iter().map(|s| s.to_string()).collect::<Vec<String>>());
}


#[test]
fn index_function() {
    let mut repo_writer = RepoWriter::new(&testdir!());

    repo_writer.add_c_unit("main.c", r#"
int f(int argc, char **args);

int f(int argc, char **args) { }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let mut items = read_search_index(&args.outdir);
    items.retain(|ii| ii.kind() == IndexItemKind::IiSymbol);
    assert_eq!(items.len(), 1);
    let IndexItem { key, href, kind: _, path, r#type } = &items[0];
    assert_eq!(key, "f");
    assert_eq!(path, &Some("main.c".to_string()));
    assert_eq!(r#type, &Some("int (int, char **)".to_string()));

    let mut walker = GraphWalker::new(args.outdir);
    walker.go_to_node(href.clone());
    assert_eq!(walker.node().text, "int f(int argc, char **args) { }")
}


#[test]
fn index_global_var() {
    let mut repo_writer = RepoWriter::new(&testdir!());

    repo_writer.add_c_unit("main.c", r#"
int x = 1234;
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let mut items = read_search_index(&args.outdir);
    items.retain(|ii| ii.kind() == IndexItemKind::IiSymbol);
    assert_eq!(items.len(), 1);
    let IndexItem { key, href, kind: _, path, r#type } = &items[0];
    assert_eq!(key, "x");
    assert_eq!(path, &Some("main.c".to_string()));
    assert_eq!(r#type, &Some("int".to_string()));

    let mut walker = GraphWalker::new(args.outdir);
    walker.go_to_node(href.clone());
    assert!(walker.node().text.starts_with("int x ="));
}


#[test]
fn index_struct() {
    let mut repo_writer = RepoWriter::new(&testdir!());

    repo_writer.add_c_unit("main.c", r#"
struct s {
    int x, y;
}
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let mut items = read_search_index(&args.outdir);
    let mut walker = GraphWalker::new(args.outdir);

    items.retain(|ii| ii.kind() == IndexItemKind::IiSymbol);
    items.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(items.len(), 1);

    let IndexItem { key, href, kind: _, path, r#type } = &items[0];
    assert_eq!(key, "s");
    assert_eq!(path, &Some("main.c".to_string()));
    assert_eq!(r#type, &Some("struct s".to_string()));
    walker.go_to_node(href.clone());
    assert!(walker.node().text.starts_with("struct s {"));

    // let IndexItem { key, node_id, kind: _, path, r#type } = &items[1];
    // assert_eq!(key, "x");
    // assert_eq!(path, &Some("main.c".to_string()));
    // assert_eq!(r#type, &Some("int".to_string()));
    // walker.go_to_node(*node_id);
    // assert!(walker.node().text.starts_with("struct s {"));

    // let IndexItem { key, node_id, kind: _, path, r#type } = &items[2];
    // assert_eq!(key, "y");
    // assert_eq!(path, &Some("main.c".to_string()));
    // assert_eq!(r#type, &Some("int".to_string()));
    // walker.go_to_node(*node_id);
    // assert!(walker.node().text.starts_with("struct s {"));
}


#[test]
fn index_enum() {
    let mut repo_writer = RepoWriter::new(&testdir!());

    repo_writer.add_c_unit("main.c", r#"
enum E { E_ONE, E_TWO, };
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let mut items = read_search_index(&args.outdir);
    let mut walker = GraphWalker::new(args.outdir);

    items.retain(|ii| ii.kind() == IndexItemKind::IiSymbol);
    items.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(items.len(), 3);

    let IndexItem { key, href, kind: _, path, r#type } = &items[0];
    assert_eq!(key, "E");
    assert_eq!(path, &Some("main.c".to_string()));
    assert_eq!(r#type, &Some("enum E".to_string()));
    walker.go_to_node(href.clone());
    assert!(walker.node().text.starts_with("enum E {"));


    let IndexItem { key, href, kind: _, path, r#type } = &items[1];
    assert_eq!(key, "E_ONE");
    assert_eq!(path, &Some("main.c".to_string()));
    assert_eq!(r#type, &Some("enum E".to_string()));
    walker.go_to_node(href.clone());
    assert!(walker.node().text.starts_with("enum E {"));

    let IndexItem { key, href, kind: _, path, r#type } = &items[2];
    assert_eq!(key, "E_TWO");
    assert_eq!(path, &Some("main.c".to_string()));
    assert_eq!(r#type, &Some("enum E".to_string()));
    walker.go_to_node(href.clone());
    assert!(walker.node().text.starts_with("enum E {"));
}


#[test]
fn index_anon_enum() {
    let mut repo_writer = RepoWriter::new(&testdir!());

    repo_writer.add_c_unit("main.c", r#"
enum { E_ONE, E_TWO, };
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let mut items = read_search_index(&args.outdir);
    println!("{:#?}", items);
    // let mut walker = GraphWalker::new(args.outdir, res.conn);

    items.retain(|ii| ii.kind() == IndexItemKind::IiSymbol);
    items.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(items.len(), 2);

    let IndexItem { key, href: _, kind: _, path, r#type } = &items[0];
    assert_eq!(key, "E_ONE");
    assert_eq!(path, &Some("main.c".to_string()));
    assert_eq!(r#type, &None);

    let IndexItem { key, href: _, kind: _, path, r#type } = &items[1];
    assert_eq!(key, "E_TWO");
    assert_eq!(path, &Some("main.c".to_string()));
    assert_eq!(r#type, &None);
}



#[test]
fn index_macros() {
    let mut repo_writer = RepoWriter::new(&testdir!());

    repo_writer.add_c_unit("main.c", r#"
#define X 100
#define F(x, y) (x + y)
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let mut items = read_search_index(&args.outdir);
    let mut walker = GraphWalker::new(args.outdir);

    items.retain(|ii| ii.kind() == IndexItemKind::IiMacro);
    items.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(items.len(), 2);

    let IndexItem { key, href, kind: _, path, r#type } = &items[0];
    assert_eq!(key, "F");
    assert_eq!(path, &Some("main.c".to_string()));
    assert_eq!(r#type, &None);
    walker.go_to_node(href.clone());
    assert!(walker.node().text.starts_with("#define F(x, y)"));

    let IndexItem { key, href, kind: _, path, r#type } = &items[1];
    assert_eq!(key, "X");
    assert_eq!(path, &Some("main.c".to_string()));
    assert_eq!(r#type, &None);
    walker.go_to_node(href.clone());
    assert!(walker.node().text.starts_with("#define X"));
}


#[test]
fn index_enum_with_macro_name() {
    let mut repo_writer = RepoWriter::new(&testdir!());

    repo_writer.add_c_unit("main.c", r#"
#define M1
#define M2 NAME
enum M1 M2 { X, };
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let mut items = read_search_index(&args.outdir);
    let mut walker = GraphWalker::new(args.outdir);

    items.retain(|ii| ii.kind() == IndexItemKind::IiSymbol);
    items.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(items.len(), 2);

    let IndexItem { key, href, kind: _, path, r#type } = &items[0];
    assert_eq!(key, "M1 M2");
    assert_eq!(path, &Some("main.c".to_string()));
    assert_eq!(r#type, &Some("enum NAME".to_string()));
    walker.go_to_node(href.clone());
    assert!(walker.node().text.starts_with("enum M1 M2 {"));
}


#[test]
fn index_global_var_multiple() {
    let mut repo_writer = RepoWriter::new(&testdir!());

    repo_writer.add_c_unit("main.c", r#"
int x = 1234, y, *z;
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let mut items = read_search_index(&args.outdir);
    items.retain(|ii| ii.kind() == IndexItemKind::IiSymbol);
    items.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(items.len(), 3);

    let IndexItem { key, href: _, kind: _, path, r#type } = &items[0];
    assert_eq!(key, "x");
    assert_eq!(path, &Some("main.c".to_string()));
    assert_eq!(r#type, &Some("int".to_string()));

    let IndexItem { key, href: _, kind: _, path, r#type } = &items[1];
    assert_eq!(key, "y");
    assert_eq!(path, &Some("main.c".to_string()));
    assert_eq!(r#type, &Some("int".to_string()));

    let IndexItem { key, href: _, kind: _, path, r#type } = &items[2];
    assert_eq!(key, "z");
    assert_eq!(path, &Some("main.c".to_string()));
    assert_eq!(r#type, &Some("int *".to_string()));
}



#[test]
fn asm_files_are_searchable() {
    let mut args = defaut_args();
    args.repo = TERRITORY_ROOT.join("repos/example-asm");
    inspect_repo(&args);

    let mut items = read_search_index(&args.outdir);
    items.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(items.iter().map(|ii| ii.key.clone()).collect::<Vec<String>>(), vec![
        "/", "SYM_FUNC_START_NOALIGN", "inc.h", "pmjump.S"
    ].into_iter().map(|s| s.to_string()).collect::<Vec<String>>());

    let mut walker = GraphWalker::new(args.outdir);

    let IndexItem { key: _, href, kind, path, r#type } = &items.iter()
        .find(|it| it.key == "pmjump.S")
        .expect("file entry missing");
    assert_eq!(path, &None);
    assert_eq!(r#type, &None);
    assert_eq!(*kind, Into::<i32>::into(IndexItemKind::IiFile));
    walker.go_to_node(href.clone());
    assert!(walker.node().text.contains("movl	%edx, %esi"));
}


#[test]
fn unparsed_files_are_searchable() {
    let repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("TERRITORY_FILE_LISTING", "./README.md\n").unwrap();
    repo_writer.add("README.md", "readme text").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let mut items = read_search_index(&args.outdir);
    items.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(
        items.iter().map(|ii| ii.key.clone()).collect::<Vec<String>>(),
        vec!["/", "README.md"]
            .into_iter()
            .map(|s| s.to_string()).collect::<Vec<String>>());
}


#[test]
fn namespace_qualified_names() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r"
namespace foo {
    class A {
        void f();
    };
}

using namespace foo;

void A::f() { }

").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let items = read_search_index(&args.outdir);
    dbg!(&items);
    assert!(items.iter().map(|it| &it.key).contains(&"foo::A::f".to_string()));
}


#[test]
fn constructors() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r"
namespace foo {
    class A {
        A();
    };
}

using namespace foo;

A::A() { }

").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let items = read_search_index(&args.outdir);
    dbg!(&items);
    assert!(items.iter().map(|it| &it.key).contains(&"foo::A::A".to_string()));
}


#[test]
fn destructors() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r"
namespace foo {
    class A {
        ~A();
    };
}

using namespace foo;

A::~A() { }

").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let items = read_search_index(&args.outdir);
    dbg!(&items);
    assert!(items.iter().map(|it| &it.key).contains(&"foo::A::~A".to_string()));
}


#[test]
fn using() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r"
namespace a {
    using YourMom = long long;
}
").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let items = read_search_index(&args.outdir);
    dbg!(&items);
    assert!(items.iter().map(|it| &it.key).contains(&"a::YourMom".to_string()));
}


#[test]
fn function_template() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r"
namespace a {
  template <typename... Ts>
  bool RequireLiteralType(unsigned DiagID, const Ts &...Args) {
    return false;
  }

}
").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let items = read_search_index(&args.outdir);
    dbg!(&items);
    assert!(items.iter().map(|it| &it.key).contains(&"a::RequireLiteralType".to_string()));
}


#[test]
fn enum_class() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r"
namespace a {
    class Log final {
        using MaskType = long;
    };

    enum class Foo : Log::MaskType {
        FOO = 1,
        BAR = 2,
    };
}
").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let args = Args { repo: repo_writer.dir().clone(), ..defaut_args() };
    inspect_repo(&args);

    let items = read_search_index(&args.outdir);
    dbg!(&items);
    assert!(items.iter().map(|it| &it.key).contains(&"a::Foo".to_string()));
    assert!(items.iter().map(|it| &it.key).contains(&"FOO".to_string()));
    assert!(items.iter().map(|it| &it.key).contains(&"BAR".to_string()));
}
