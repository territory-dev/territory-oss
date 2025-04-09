use std::collections::HashSet;
use std::fs::{canonicalize, read_to_string, File};
use std::io::Write;

use testdir::testdir;

use territory_core::territory::index::{reference, NodeKind, Location};
use territory_core::{TokenKind, ReferencesLink, pb_node_tokens};

use clangrs::testlib::{
    index_example, index_with_changed_args, repr_diff, run_scanner_thread, write_single_file_repo, RepoWriter, TERRITORY_ROOT
};


#[test]
fn exec() {
    use std::process::Command;

    let temp_dir_ = testdir!();
    let outdir = temp_dir_.join("output");

    let clangrs_path = env!("CARGO_BIN_EXE_clangrs");

    let repo_path = TERRITORY_ROOT.join("repos/example");
    let scanner_sock_path = temp_dir_.join("scanner.sock");
    let par = 1;
    let mut scanners = Vec::new();
    for _ in 1..=par {
        let s = run_scanner_thread(repo_path.clone(), scanner_sock_path.clone());
        scanners.push(s);
    }

    let cmdout = Command::new(clangrs_path)
        .args([
            "-r", repo_path.to_str().expect("bad repo path"),
            "-m", "file",
            "-o", outdir.to_str().expect("bad output path"),
            "--repo-id", "example",
            "--build-id", "build1",
            "--par", &par.to_string(),
            "--intermediate-path", temp_dir_.join("model").to_str().expect("bad model path"),
            "--db-path", temp_dir_.join("model").join("sem.db").to_str().unwrap(),
            "--fastwait",
            "--scanner-socket-path", scanner_sock_path.to_str().unwrap(),
            "--log-dir", temp_dir_.join("logs").to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        cmdout.status.success(),
        "clangrs failed\nstdout:\n{}\n\nstderr:\n{}",
        String::from_utf8(cmdout.stdout).unwrap(),
        String::from_utf8(cmdout.stderr).unwrap(),
    );
    assert!(dbg!(outdir.join("nodes/example")).exists());
    assert!(outdir.join("search/example/build1/all").exists());

    let log_text = read_to_string(temp_dir_.join("logs/index")).expect("can't read scan log file");
    assert!(log_text.contains("clangrs indexer starting"));

    for s in scanners { s.join().unwrap(); }
}


#[test]
fn root_has_files() {
    let walker = index_example();

    let index_node = walker.node();
    assert_eq!(index_node.path, "/");
    assert_eq!(index_node.kind(), NodeKind::Directory);
    assert_eq!(pb_node_tokens(index_node).iter().map(|tok| (tok.type_, tok.text.as_ref())).collect::<Vec<(TokenKind, &str)>>(), vec![
        (TokenKind::Identifier, "dir/\n"),
        (TokenKind::Identifier, "mod1.c\n"),
        (TokenKind::Identifier, "shared.h\n"),
    ]);

}


#[test]
fn build_without_references() {
    let mut walker = index_with_changed_args(|args| { args.no_references = true; args.fastwait = false; });

    walker.follow_token("mod1.c");
    walker.follow_token("foo");
    let tok = walker.find_token("foo").unwrap();
    assert_eq!(tok.context.references, ReferencesLink::None);
}


#[test]
fn definition_reachable() {
    let mut walker = index_example();

    dbg!(&walker.node().text);
    walker.follow_token("mod1.c");

    let file_node = walker.node();
    // dbg!(&file_node);
    assert_eq!(file_node.path, "mod1.c");
    assert_eq!(file_node.kind(), NodeKind::SourceFile);
    dbg!(&file_node.text);

    walker.follow_token("foo");

    let source_node = walker.node();
    assert_eq!(source_node.path, "mod1.c");
    assert_eq!(source_node.kind(), NodeKind::Definition);

    use TokenKind::{Keyword, WS, Punctuation, Identifier};
    assert_eq!(
        pb_node_tokens(source_node)
            .iter()
            .map(|tok| (tok.type_, tok.text.as_ref()))
            .collect::<Vec<(TokenKind, &str)>>(),
        vec![
            (Keyword, "int"),
            (WS, " "),
            (Identifier, "foo"),
            (Punctuation, "("),
            (Punctuation, ")"),
            (WS, " "),
            (Punctuation, "{"),
            (WS, "\n    "),
            (Keyword, "return"),
            (WS, " "),
            (Identifier, "bar"),
            (Punctuation, "("),
            (Identifier, "DEFA"),
            (Punctuation, ")"),
            (Punctuation, ";"),
            (WS, "\n"),
            (Punctuation, "}"),
        ]);
}

#[test]
fn references() {
    let mut walker = index_example();
    walker.follow_token("mod1.c");
    walker.follow_token("foo");
    let refs = walker.token_references("foo");

    assert_eq!(refs.refs.len(), 1);
    let ref_ = &refs.refs[0];
    assert_eq!(ref_.use_path, "mod1.c");
    assert_eq!(ref_.use_location, Some(Location {
        line: 9,
        column: 5,
        offset: 76,
    }));
    assert_eq!(ref_.context, "baz");

    walker.go_to_node(refs.refs[0].href);

    assert!(walker.node().text.starts_with("void baz() {"));
}

#[test]
fn references_with_vars() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("mod.c", r#"
int x;

void f() {
    int y = 123;
    int v = 100, z = x;
}"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("mod.c");
    walker.follow_token("x");
    let refs = walker.token_references("x");

    assert_eq!(refs.refs.len(), 1);
    let ref_ = &refs.refs[0];
    assert_eq!(ref_.context, "f::z");

    walker.go_to_node(refs.refs[0].href);

    assert!(walker.node().text.starts_with("void f() {"));
}

#[test]
fn external_references() {
    let mut walker = index_example();
    walker.follow_token("dir/");
    walker.follow_token("mod2.c");
    walker.follow_token("bar");
    let refs = walker.token_references("bar");

    let mut refs_vec = refs.refs.clone();
    refs_vec.sort_by(|a, b| a.context.cmp(&b.context));
    assert_eq!(refs.refs.len(), 1);
    let ref_ = &refs.refs[0];
    assert_eq!(ref_.use_path, "mod1.c");
    assert_eq!(ref_.use_location, Some(Location {
        line: 5,
        column: 12,
        offset: 45,
    }));
    assert_eq!(ref_.context, "foo");


    walker.go_to_node(refs_vec[0].href);
    assert!(walker.node().text.starts_with("int foo() {"));

    // walker.go_to_node(refs_vec[1].href);
    // assert!(walker.node().text.starts_with("int bar(int x)"));
}

#[test]
fn which_function_tokens_have_references() {
    let mut walker = index_example();
    walker.follow_token("dir/");
    walker.follow_token("mod2.c");
    walker.follow_token("bar");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.text.as_ref(), tok.context.references.is_set()))
        .collect::<Vec<(&str, bool)>>();
    let expected = vec![
        ("int", false),
        (" ", false),
        ("bar", true),
        ("(", false),
        ("int", false),
        (" ", false),
        ("x", false),  // local var
        (")", false),
        (" ", false),
        ("{", false),
        ("\n    ", false),
        ("return", false),
        (" ", false),
        ("x", false),
        (" ", false),
        ("+", false),
        (" ", false),
        ("1", false),
        (";", false),
        ("\n", false),
        ("}", false),
    ];
    assert_eq!(got, expected, "token references don't match: {}", repr_diff(&expected, &got));
}

#[test]
fn inlined_include() {
    let mut walker = index_with_changed_args(|args| {
        args.repo = TERRITORY_ROOT.join("repos/example-inlined-include");
    });

    walker.follow_token("constants.c");
    walker.follow_token("additional");
    assert_eq!(walker.node().text, r#"static const struct error_info additional[] =
{
#define SENSE_CODE(c, s) {c, sizeof(s)},
#include "sense_codes.h"
#undef SENSE_CODE
}"#);
    walker.back().unwrap();

    walker.follow_token("additional_text");
    assert_eq!(walker.node().text, r#"static const char *additional_text =
#define SENSE_CODE(c, s) s "\0"
#include "sense_codes.h"
#undef SENSE_CODE
	;"#);
}


#[test]
fn macro_definition() {
    let mut walker = index_example();

    walker.follow_token("shared.h");
    walker.follow_token("DEFA");
    use TokenKind::{Punctuation, WS, Literal, Identifier};
    assert_eq!(
        pb_node_tokens(walker.node())
            .iter()
            .map(|tok| (tok.type_, tok.text.as_ref()))
            .collect::<Vec<(TokenKind, &str)>>(),
        vec![
            (Punctuation, "#"),
            (Identifier, "define"),
            (WS, " "),
            (Identifier, "DEFA"),
            (WS, " "),
            (Literal, "1234")
        ]);
}


#[test]
fn which_macro_definition_tokens_have_references() {
    let mut walker = index_example();
    walker.follow_token("shared.h");
    walker.follow_token("DEFA");

    let tokens = pb_node_tokens(walker.node());
    assert_eq!(
        tokens
            .iter()
            .map(|tok| (tok.text.as_ref(), tok.context.references.is_set()))
            .collect::<Vec<(&str, bool)>>(),
        vec![
            ("#", false),
            ("define", false),
            (" ", false),
            ("DEFA", true),
            (" ", false),
            ("1234", false)
        ]);
}


#[test]
fn macro_href() {
    let mut walker = index_example();
    walker.follow_token("mod1.c");
    walker.follow_token("foo");
    walker.follow_token("DEFA");
    assert_eq!(walker.node().text, "#define DEFA 1234");
}


#[test]
fn macro_references() {
    let mut walker = index_example();

    walker.follow_token("shared.h");
    walker.follow_token("DEFA");

    let refs = walker.token_references("DEFA");
    assert_eq!(refs.refs.len(), 1);
    let ref_ = &refs.refs[0];
    assert_eq!(ref_.use_path, "mod1.c");
    assert_eq!(ref_.use_location, Some(Location {
        line: 5,
        column: 16,
        offset: 49,
    }));
    assert_eq!(ref_.context, "foo");


    walker.go_to_node(refs.refs[0].href);

    assert!(walker.node().text.starts_with("int foo() {"));
}


#[test]
fn local_function_href() {
    let mut walker = index_example();
    walker.follow_token("mod1.c");
    walker.follow_token("baz");
    walker.follow_token("foo");
    assert!(walker.node().text.starts_with("int foo() {"));
}


#[test]
fn external_function_href() {
    let mut walker = index_example();
    walker.follow_token("mod1.c");
    walker.follow_token("foo");
    walker.follow_token("bar");

    assert_eq!(walker.node().path, "dir/mod2.c");

    let node_text = &walker.node().text;
    assert!(
        node_text.starts_with("int bar(int x) {"),
        "unexpected node text: {}", node_text);
}


#[test]
fn repo_relative_paths() {
    let temp_dir_ = testdir!();
    let canon_test_dir = canonicalize(temp_dir_).unwrap();

    let main_code = r#"#include "incl.h"
int main() {}"#;
    let repo_dir = write_single_file_repo(&canon_test_dir, main_code);

    let mut include_file = repo_dir.clone();
    include_file.push("incl.h");
    File::create(include_file).unwrap().write_all(br#"#define FOO"#).unwrap();

    let mut walker = index_with_changed_args(|args| {
        args.repo = repo_dir.clone();
    });

    assert_eq!(walker.node().text, "incl.h\nmain.c\n");
    assert_eq!(walker.node().path, "/");

    walker.follow_token("main.c");
    assert_eq!(walker.node().path, "main.c");

    walker.follow_token("main");
    assert_eq!(walker.node().path, "main.c");

    walker.back().unwrap();
    walker.back().unwrap();
    walker.follow_token("incl.h");
    assert_eq!(walker.node().path, "incl.h");

    walker.follow_token("FOO");
    assert_eq!(walker.node().path, "incl.h");
}


#[test]
fn comment_cutting() {
    let mut walker = index_with_changed_args(|args| {
        args.repo = TERRITORY_ROOT.join("repos/example-comments");
    });

    walker.follow_token("mod1.c");
    walker.follow_token("foo");
    assert_eq!(walker.node().text, r#"int foo() {
    return 0;
}"#);

    /* TODO:
    assert_eq!(walker.node().text, r#"/*
 * I am a comment block adjacent to the function
 */
int foo() {
    return 0;
}"#);
    */
}


#[test]
fn containers() {
    let mut walker = index_example();
    assert_eq!(walker.node().container, None);

    let root_id = walker.node().id;
    walker.follow_token("dir/");
    assert_eq!(walker.node().container, Some(root_id));

    let dir_id = walker.node().id;
    walker.follow_token("mod2.c");
    assert_eq!(walker.node().container, Some(dir_id));

    let file_id = walker.node().id;
    walker.follow_token("bar");
    assert_eq!(walker.node().container, Some(file_id));
}


#[test]
fn ifdefs() {
    let repo_path = write_single_file_repo(&testdir!(),
r#"#define DF1

#ifdef DF1

int x;

#else

int y;

#endif


#define DF2 2

#if DF2 == 1

int z;

#endif

"#);
    let mut walker = index_with_changed_args(|args| {
        args.repo = repo_path;
    });
    walker.follow_token("main.c");

    use TokenKind::{Keyword, WS, Identifier, Punctuation, Literal};
    let tokens = pb_node_tokens(walker.node());
    let got = tokens.iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    let expected = vec![
        (Keyword, "#"), (Keyword, "define"), (WS, " "), (Identifier, "DF1"), (WS, "\n\n"),

        (Punctuation, "#"), (Identifier, "ifdef"), (WS, " "), (Identifier, "DF1"), (WS, "\n\n"),
        (Keyword, "int"), (WS, " "), (Identifier, "x"), (Punctuation, ";"), (WS, "\n\n"),
        (Punctuation, "#"), (Keyword, "else"), (WS, "\n\n"),
        (Keyword, "int"), (WS, " "), (Identifier, "y"), (Punctuation, ";"), (WS, "\n\n"),
        (Punctuation, "#"), (Identifier, "endif"),

        (WS, "\n\n\n"),

        (Keyword, "#"), (Keyword, "define"), (WS, " "), (Identifier, "DF2"), (WS, " …"), (WS, "\n\n"),

        (Punctuation, "#"), (Keyword, "if"), (WS, " "), (Identifier, "DF2"),
            (WS, " "), (Punctuation, "=="), (WS, " "), (Literal, "1"), (WS, "\n\n"),

        (Keyword, "int"), (WS, " "), (Identifier, "z"), (Punctuation, ";"), (WS, "\n\n"),

        (Punctuation, "#"), (Identifier, "endif"),
    ];
    assert_eq!(got, expected, "token references don't match: {}", repr_diff(&expected, &got));
}

#[test]
fn macro_used_in_multiple_units() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("shared.h", "#define FOO 1234\n").unwrap();
    repo_writer.add_c_unit("mod1.c", r#"
#include "shared.h"

int x = FOO;
"#).unwrap();
    repo_writer.add_c_unit("mod2.c", r#"
#include "shared.h"

int y = FOO;
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("shared.h");
    walker.follow_token("FOO");
    let define_node_id = walker.node().id;

    let mut refs: HashSet<_> = walker
        .token_references("FOO")
        .refs
        .iter().map(|r| r.href).collect();
    dbg!(&refs);

    walker.reset();
    walker.follow_token("mod1.c");
    walker.follow_token("x");
    assert!(refs.remove(&Some(reference::Href::NodeId(walker.node().id))));
    walker.follow_token("FOO");
    assert_eq!(walker.node().id, define_node_id);

    walker.reset();
    walker.follow_token("mod2.c");
    walker.follow_token("y");
    assert!(refs.remove(&Some(reference::Href::NodeId(walker.node().id))));
    walker.follow_token("FOO");
    assert_eq!(walker.node().id, define_node_id);

    assert!(refs.is_empty(), "unaccounted references to FOO found");
}


#[test]
fn ext_declaration_used_in_multiple_units() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("shared.h", r#"
int shared_func(int x);
"#).unwrap();
    repo_writer.add_c_unit("use1.c", r#"
#include "shared.h"

void use1(void) { shared_func(1); }
"#).unwrap();
    repo_writer.add_c_unit("use2.c", r#"
#include "shared.h"

void use2(void) { shared_func(2); }
"#).unwrap();
    repo_writer.add_c_unit("def.c", r#"
#include "shared.h"

int shared_func(int x) { return 0; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("def.c");
    walker.follow_token("shared_func");
    let define_node_id = walker.node().id;

    walker.reset();
    walker.follow_token("use1.c");
    walker.follow_token("use1");
    walker.follow_token("shared_func");
    assert_eq!(walker.node().id, define_node_id);

    walker.reset();
    walker.follow_token("use2.c");
    walker.follow_token("use2");
    walker.follow_token("shared_func");
    assert_eq!(walker.node().id, define_node_id);
}


#[test]
fn struct_field_href() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c", r#"
struct S {
    int x, y;
    char *a, *b;
};

void use(void) { struct S s; s.x = 10; }
void use_ptr(struct S *s) { s->y; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.c");
    walker.follow_token("use");
    walker.follow_token("x");
    assert!(walker.node().text.starts_with("struct S {"));

    walker.back().unwrap();
    walker.back().unwrap();
    walker.follow_token("use_ptr");
    walker.follow_token("y");
    assert!(walker.node().text.starts_with("struct S {"));
}


#[test]
fn struct_field_references() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c", r#"
struct S {
    int x, y;
    char *a, *b;
};

void use(void) { struct S s; s.x = 10; }
void use_ptr(struct S *s) { s->y; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.c");
    walker.follow_token("S");

    let x_refs = walker.token_references("x");
    assert_eq!(x_refs.refs.len(), 1);
    walker.go_to_node(x_refs.refs[0].href);
    assert!(walker.node().text.starts_with("void use(void) {"));

    walker.back().unwrap();

    let x_refs = walker.token_references("y");
    assert_eq!(x_refs.refs.len(), 1);
    walker.go_to_node(x_refs.refs[0].href);
    assert!(walker.node().text.starts_with("void use_ptr(struct S *s) {"));
}


#[test]
fn which_struct_tokens_have_references() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c", r#"
struct S {
    int x, y;
    char *a, *b;
};

void use(void) { struct S s; s.x = 10; }
void use_ptr(struct S *s) { s->y; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.c");
    walker.follow_token("S");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.text.as_ref(), tok.context.references.is_set()))
        .collect::<Vec<(&str, bool)>>();
    let expected = vec![
        ("struct", false),
        (" ", false),
        ("S", true),
        (" ", false),
        ("{", false),
        ("\n    ", false),
        ("int", false),
        (" ", false),
        ("x", true),
        (",", false),
        (" ", false),
        ("y", true),
        (";", false),
        ("\n    ", false),
        ("char", false),
        (" ", false),
        ("*", false),
        ("a", false),
        (",", false),
        (" ", false),
        ("*", false),
        ("b", false),
        (";", false),
        ("\n", false),
        ("}", false),
    ];
    assert_eq!(got, expected, "token references don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn enum_href() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c", r#"
enum E { E_ONE, E_TWO, } ge;
void use(void) { ge = E_ONE; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.c");
    walker.follow_token("use");
    walker.follow_token("E_ONE");
    assert!(walker.node().text.starts_with("enum E {"));
}


#[test]
fn enum_variable_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c",
r#"enum E { E_ONE, E_TWO, } ge;
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| tok.text.as_ref())
        .collect::<Vec<&str>>();
    let expected = vec![
        "enum", " ", "E", " ", "{", " … ", "}", " ", "ge",  ";",
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn enum_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c", r#"enum E { E_ONE, E_TWO, };"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| tok.text.as_ref())
        .collect::<Vec<&str>>();
    let expected = vec![
        "enum", " ", "E", " ", "{", " … ", "}", ";",
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn enum_references() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c", r#"
enum E { E_ONE, E_TWO, } ge;
void use(void) { ge = E_ONE; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.c");
    walker.follow_token("E");
    let refs = walker.token_references("E_ONE");
    assert_eq!(refs.refs.len(), 1);
    walker.go_to_node(refs.refs[0].href);
    assert!(walker.node().text.starts_with("void use(void) {"));
}


#[test]
fn late_definition() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c",
r#"int f1(int);

int f2(void) { f1(100); }

int f1(int x) { return 200; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.context.href.is_some(), tok.text.as_ref()))
        .collect::<Vec<(bool, &str)>>();
    let expected = vec![
        (false, "int"),
        (false, " "),
        (false, "f1"),
        (false, "("),
        (false, "int"),
        (false, ")"),
        (false, ";"),
        (false, "\n\n"),
        (true,  "int"),
        (true,  " "),
        (true,  "f2"),
        (true,  "("),
        (true,  "void"),
        (true,  ")"),
        (true,  " "),
        (true,  "{"),
        (true,  " … "),
        (true,  "}"),
        (false, "\n\n"),
        (true,  "int"),
        (true,  " "),
        (true,  "f1"),
        (true,  "("),
        (true,  "int"),
        (true,  " "),
        (true,  "x"),
        (true,  ")"),
        (true,  " "),
        (true,  "{"),
        (true,  " … "),
        (true,  "}"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));

    walker.follow_token("f2");
    walker.follow_token("f1");
    assert!(walker.node().text.starts_with("int f1(int x) {"));
}


#[test]
fn late_static_definition() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c",
r#"static int f1(int);

int f2(void) { f1(100); }

static int f1(int x) { return 200; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.context.href.is_some(), tok.text.as_ref()))
        .collect::<Vec<(bool, &str)>>();
    let expected = vec![
        (false, "static"),
        (false, " "),
        (false, "int"),
        (false, " "),
        (false, "f1"),
        (false, "("),
        (false, "int"),
        (false, ")"),
        (false, ";"),
        (false, "\n\n"),
        (true,  "int"),
        (true,  " "),
        (true,  "f2"),
        (true,  "("),
        (true,  "void"),
        (true,  ")"),
        (true,  " "),
        (true,  "{"),
        (true,  " … "),
        (true,  "}"),
        (false, "\n\n"),
        (true, "static"),
        (true, " "),
        (true,  "int"),
        (true,  " "),
        (true,  "f1"),
        (true,  "("),
        (true,  "int"),
        (true,  " "),
        (true,  "x"),
        (true,  ")"),
        (true,  " "),
        (true,  "{"),
        (true,  " … "),
        (true,  "}"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));

    walker.follow_token("f2");
    walker.follow_token("f1");
    assert!(walker.node().text.starts_with("static int f1(int x) {"));
}


#[test]
fn directory_sorting() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("a/f.c", "int a = 0;").unwrap();
    repo_writer.add_c_unit("d/f.c", "int b = 0;").unwrap();
    repo_writer.add_c_unit("b.c", "int a = 0;").unwrap();
    repo_writer.add_c_unit("c.c", "int b = 0;").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let walker = repo_writer.index_repo();
    let index_node = walker.node();
    assert_eq!(pb_node_tokens(index_node).iter().map(|tok| (tok.type_, tok.text.as_ref())).collect::<Vec<(TokenKind, &str)>>(), vec![
        (TokenKind::Identifier, "a/\n"),
        (TokenKind::Identifier, "d/\n"),
        (TokenKind::Identifier, "b.c\n"),
        (TokenKind::Identifier, "c.c\n"),
    ]);

}


#[test]
fn function_definition_file_entry() {
    let mut walker = index_example();

    walker.follow_token("dir/");
    walker.follow_token("mod2.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, Keyword, WS, Literal};
    let expected = vec![
        ( Punctuation, "#",),
        ( Identifier, "include",),
        ( WS, " ",),
        ( Literal, "\"../shared.h\"",),
        ( WS, "\n\n\n",),
        (Keyword, "int"),
        (WS, " "),
        (Identifier, "bar"),
        (Punctuation, "("),
        (Keyword, "int"),
        (WS, " "),
        (Identifier, "x"),
        (Punctuation, ")"),
        (WS, " "),
        (Punctuation, "{"),
        (WS, " … "),
        (Punctuation, "}"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn macro_definition_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c",
r#"#define M_PLAIN
#define M_SHORT 1234
#define M_LINES "some multiline " \
                "text"
#define M_PAR (1234)
#define M_FUN(x, y) { x = y; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();

    walker.follow_token("main.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, Keyword, WS};
    let expected = vec![
        (Keyword, "#"), (Keyword, "define"), (WS, " "), (Identifier, "M_PLAIN"), (WS, "\n"),

        (Keyword, "#"), (Keyword, "define"), (WS, " "), (Identifier, "M_SHORT"), (WS, " …"), (WS, "\n"),

        (Keyword, "#"), (Keyword, "define"), (WS, " "), (Identifier, "M_LINES"), (WS, " …"), (WS, "\n"),

        (Keyword, "#"), (Keyword, "define"), (WS, " "), (Identifier, "M_PAR"), (WS, " …"), (WS, "\n"),

        (Keyword, "#"), (Keyword, "define"), (WS, " "),
        (Identifier, "M_FUN"), (Punctuation, "("),
            (Identifier, "x"), (Punctuation, ","), (WS, " "), (Identifier, "y"),
        (Punctuation, ")"),
        (WS, " …"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn global_var_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c",
r#"static int x = 100;
static int *ptr;
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();

    walker.follow_token("main.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, Keyword, WS};
    let expected = vec![
        (Keyword, "static"), (WS, " "), (Keyword, "int"), (WS, " "), (Identifier, "x"),
        (WS, " "), (Punctuation, "="), (WS, " …"), (Punctuation, ";"), (WS, "\n"),

        (Keyword, "static"), (WS, " "), (Keyword, "int"), (WS, " "),
        (Punctuation, "*"), (Identifier, "ptr"), (Punctuation, ";"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn macro_use_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("def.h", r"
#define M_DEF(name)  int name;
").unwrap();
    repo_writer.add_c_unit("main.c",
r#"#include "def.h"
M_DEF(v)
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();

    walker.follow_token("main.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, WS, Literal};
    let expected = vec![
        (Punctuation, "#"), (Identifier, "include"), (WS, " "), (Literal, "\"def.h\""),
        (WS, "\n"),
        (Identifier, "M_DEF"), (Punctuation, "("), (Identifier, "v"), (Punctuation, ")")
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn file_entry_for_function_declared_with_a_macro() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("def.h",
r"#define M_FUN(name)  int name(int argc, char **argv)
#define M_COMPLEX  int x; void f()
").unwrap();
    repo_writer.add_c_unit("main.c",
r#"#include "def.h"
M_FUN(main)
{
    return 0;
}

M_COMPLEX { return; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();

    walker.follow_token("main.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, WS, Literal};
    let expected = vec![
        ( Punctuation, "#",), ( Identifier, "include",), ( WS, " ",), ( Literal, "\"def.h\"",), ( WS, "\n",),

        (Identifier, "M_FUN"), (Punctuation, "("), (Identifier, "main"), (Punctuation, ")"),
        (WS, "\n"), (Punctuation, "{"), (WS, " … "), (Punctuation, "}"),
        (WS, "\n\n"),

        // TODO
        (Identifier, "M_COMPLEX"), (WS, " "),
        (Punctuation, "{"), (WS, " …"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn file_entry_for_function_with_macro_return_type() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("def.h",
r"#define S static
#define bool int
").unwrap();
    repo_writer.add_c_unit("main.c",
r#"#include "def.h"
S bool fn() { return 0; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();

    walker.follow_token("main.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, WS, Literal};
    let expected = vec![
        ( Punctuation, "#",), ( Identifier, "include",), ( WS, " ",), ( Literal, "\"def.h\"",), ( WS, "\n",),

        (Identifier, "S"), (WS, " "), (Identifier, "bool"), (WS, " "), (Identifier, "fn"),
        (Punctuation, "("), (Punctuation, ")"), (WS, " "),
        (Punctuation, "{"), (WS, " … "), (Punctuation, "}"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn file_entry_for_function_with_typedef_return_type() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("def.h",
r"typedef unsigned int B;
").unwrap();
    repo_writer.add_c_unit("main.c",
r#"#include "def.h"
B fn() { return 0; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();

    walker.follow_token("main.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, WS, Literal};
    let expected = vec![
        ( Punctuation, "#",), ( Identifier, "include",), ( WS, " ",), ( Literal, "\"def.h\"",), ( WS, "\n",),

        (Identifier, "B"), (WS, " "), (Identifier, "fn"),
        (Punctuation, "("), (Punctuation, ")"), (WS, " "),
        (Punctuation, "{"), (WS, " … "), (Punctuation, "}"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn compound_variable_declaration_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c",
r#"int x = 1234, y = 2048, *z;
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();

    walker.follow_token("main.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, Keyword, WS};
    let expected = vec![
        (Keyword, "int"), (WS, " "), (Identifier, "x"),
        (WS, " "), (Punctuation, "="), (WS, " …"),
        (Punctuation, ","), (WS, " "),
        (Identifier, "y"), (WS, " "), (Punctuation, "="), (WS, " …"),
        (Punctuation, ","), (WS, " "),
        (Punctuation, "*"), (Identifier, "z"),
        (Punctuation, ";"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn struct_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c",
r#"struct S { int x; };
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();

    walker.follow_token("main.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, Keyword, WS};
    let expected = vec![
        (Keyword, "struct"),
        (WS, " "),
        (Identifier, "S"),
        (WS, " "),
        (Punctuation, "{"),
        (WS, " … "),
        (Punctuation, "}"),
        (Punctuation, ";"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}



#[test]
fn struct_with_values_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c",
r#"struct { int x; } s = { .x = 100, };
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();

    walker.follow_token("main.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, Keyword, WS};
    let expected = vec![
        (Keyword, "struct"),
        (WS, " "),
        (Punctuation, "{"),
        (WS, " … "),
        (Punctuation, "}"),
        (WS, " "),
        (Identifier, "s"),
        (WS, " "),
        (Punctuation, "="),
        (WS, " …"),
        (Punctuation, ";"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn struct_with_comment_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c",
r#"struct sock_common {
};

struct sock {
	/*
	 * Now struct inet_timewait_sock also uses sock_common, so please just
	 * don't add nothing before this first member (__sk_common) --acme
	 */
	struct sock_common	__sk_common;
};
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();

    walker.follow_token("main.c");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| tok.text.as_ref())
        .collect::<Vec<&str>>();
    let expected = vec![
        "struct", " ", "sock_common", " ", "{", " … ", "}", ";", "\n\n",
        "struct", " ", "sock", " ", "{", " … ", "}", ";",
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}

#[test]
fn nested_struct_file_entry() {
}


#[test]
fn include_path_expansion() {
    let mut walker = index_with_changed_args(|args| {
        args.repo = TERRITORY_ROOT.join("repos/example-include-path");
    });

    assert_eq!(walker.node().text, "include/\nsrc/\n");
    walker.follow_token("include/");
    assert_eq!(walker.node().text, "y.h\n");
    walker.back().unwrap();

    walker.follow_token("src/");
    assert_eq!(walker.node().text, "main.c\n");
}


#[test]
fn module_struct_pointer() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("structdef.h", r"
struct S { int a; };
").unwrap();
    repo_writer.add_c_unit("mod.c", r#"
#include "structdef.h"
static struct S *s;
void f() { s->a = 1; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("mod.c");
    walker.follow_token("f");
    walker.follow_token("s");
    assert_eq!(walker.node().path, "mod.c");
    assert_eq!(walker.node().text, "static struct S *s");
}


#[test]
fn angle_include() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("mod.c", r#"
#include <stdio.h>

void f() {
    printf("\n");
}"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("mod.c");
    walker.follow_token("f");
    assert!(walker.node().text.starts_with("void f() {"));
}


#[test]
fn tokens_have_line_numbers() {
    let mut walker = index_example();

    walker.follow_token("mod1.c");

    let file_node = walker.node();
    assert_eq!(file_node.path, "mod1.c");
    assert_eq!(file_node.kind(), NodeKind::SourceFile);

    walker.follow_token("foo");

    let source_node = walker.node();
    assert_eq!(source_node.path, "mod1.c");
    assert_eq!(source_node.kind(), NodeKind::Definition);

    assert_eq!(
        pb_node_tokens(source_node)
            .iter()
            .map(|tok| (tok.line, tok.text.as_ref()))
            .collect::<Vec<(u32, &str)>>(),
        vec![
            (4, "int"),
            (4, " "),
            (4, "foo"),
            (4, "("),
            (4, ")"),
            (4, " "),
            (4, "{"),
            (4, "\n    "),
            (5, "return"),
            (5, " "),
            (5, "bar"),
            (5, "("),
            (5, "DEFA"),
            (5, ")"),
            (5, ";"),
            (5, "\n"),
            (6, "}"),
        ]);
}


#[test]
fn typedef() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("mod.c", r#"
typedef unsigned int __u32;
__u32 x;
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("mod.c");
    walker.follow_token("x");
    walker.follow_token("__u32");
    assert_eq!(walker.node().text, "typedef unsigned int __u32");
}


#[ignore]
#[test]
fn differently_included() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("def.h", r"
#ifdef D
    typedef unsigned int T;
#else
    typedef float T;
#endif
").unwrap();
    repo_writer.add_c_unit("mod1.c", r#"
#define D
#include "def.h"
T x;
"#).unwrap();
    repo_writer.add_c_unit("mod2.c", r#"
#include "def.h"
T y;
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("mod1.c");
    walker.follow_token("x");
    walker.follow_token("T");
    assert_eq!(walker.node().text, "typedef unsigned int T");
    walker.reset();

    walker.follow_token("mod2.c");
    walker.follow_token("y");
    walker.follow_token("T");
    assert_eq!(walker.node().text, "typedef float T");
}


#[test]
fn include_cycle() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("def1.h", r#"
#ifndef D1
#define D1

#include "def2.h"

#endif
"#).unwrap();
    repo_writer.add("def2.h", r#"
#ifndef D2
#define D2

#include "def1.h"

#endif
"#).unwrap();
    repo_writer.add_c_unit("mod.c", r#"
#include "def1.h"
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let walker = repo_writer.index_repo();
    assert_eq!(walker.node().text, "def1.h\ndef2.h\nmod.c\n");
}


#[test]
fn unparsed_content() {
    let repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("TERRITORY_FILE_LISTING", "./README.md\n").unwrap();
    repo_writer.add("README.md", "readme text").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("README.md");
    assert_eq!(walker.node().text, "readme text");
}


#[test]
fn unparsed_binary_content() {
    let repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("TERRITORY_FILE_LISTING", "./bin\n").unwrap();
    repo_writer.add("bin", "\0\0\0\0").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("bin");
    assert_eq!(walker.node().text, "<BINARY>");
}


#[test]
fn no_system() {
    let td = testdir!();
    let mut repo_writer = RepoWriter::new(&td.join("repo"));
    repo_writer.add_c_unit("mod.cpp", r#"
#include <iostream>

void f() {
    std::cout << "";
}
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.index_system = false; });
    walker.follow_token("mod.cpp");
    walker.follow_token("f");
    let tok = walker.find_token("cout").expect("token not found");
    assert_eq!(tok.context.href, None);
}


#[test]
fn relative_file_path_in_cc() {
    let td = testdir!();
    let mut repo_writer = RepoWriter::new(&td.join("repo"));
    repo_writer.add("foo.h", r"int x;").unwrap();
    repo_writer.add("dir/mod.cpp", r#"
#include "../foo.h"
int main() { return x; }
"#).unwrap();
    repo_writer.add_custom_compile_command(serde_json::json!({
        "file": "./mod.cpp",
        "directory": repo_writer.repo_dir().join("dir"),
        "command": "cc -c ./mod.cpp",
    }));
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.index_system = false; });
    walker.follow_token("dir/");
    walker.follow_token("mod.cpp");
    walker.follow_token("main");
    walker.follow_token("x");
    assert_eq!(walker.node().text, "int x");
}


#[test]
fn relative_file_path_in_cc_with_relative_include() {
    let td = testdir!();
    let mut repo_writer = RepoWriter::new(&td.join("repo"));
    repo_writer.add("foo.h", r"int x;").unwrap();
    repo_writer.add("dir/mod.cpp", r#"
#include "foo.h"
int main() { return x; }
"#).unwrap();
    repo_writer.add_custom_compile_command(serde_json::json!({
        "file": "./mod.cpp",
        "directory": repo_writer.repo_dir().join("dir"),
        "command": "cc -I.. -c ./mod.cpp",
    }));
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.index_system = false; });
    walker.follow_token("dir/");
    walker.follow_token("mod.cpp");
    walker.follow_token("main");
    walker.follow_token("x");
    assert_eq!(walker.node().text, "int x");
}
