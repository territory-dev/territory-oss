use std::process::Command;
use territory_core::{pb_node_tokens, territory::index::{self as pb, IndexItemKind}, TokenKind};
use testdir::testdir;

use clangrs::testlib::{self, init_logging, read_search_index, str_diff, build_scanner, goscan, index_uim, TERRITORY_ROOT};


#[test]
fn main_reachable() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/go");

    goscan(&[
        repo_path.to_str().unwrap(),
        tmp.to_str().unwrap(),
    ]);

    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("main.go");
    walker.follow_token("main");

    let expected = r#"func main() {
	var x int64
	f(&x)
	fmt.Printf("%d\n", x)
}"#;
    assert_eq!(walker.node().text, expected);

    use TokenKind::{Keyword, Punctuation, Identifier, Literal, WS};
    assert_eq!(
        pb_node_tokens(walker.node())
            .iter()
            .map(|tok| (tok.type_, tok.text.as_ref()))
            .collect::<Vec<(TokenKind, &str)>>(),
        vec![
            (Keyword, "func"), (WS, " "), (Identifier, "main"), (Punctuation, "("), (Punctuation, ")"), (WS, " "), (Punctuation, "{"),
            (WS, "\n\t"),
            (Keyword, "var"), (WS, " "), (Identifier, "x"), (WS, " "), (Identifier, "int64"),
            (Punctuation, "\n\t"),
            (Identifier, "f"), (Punctuation, "("), (Punctuation, "&"), (Identifier, "x"), (Punctuation, ")"),
            (Punctuation, "\n\t"),
            (Identifier, "fmt"), (Punctuation, "."), (Identifier, "Printf"),
                (Punctuation, "("), (Literal, "\"%d\\n\""), (Punctuation, ","), (WS, " "), (Identifier, "x"), (Punctuation, ")"),
            (Punctuation, "\n"),
            (Punctuation, "}"),
        ]);
}


#[test]
fn internal_hyperlinks() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/go");

    goscan(&[
        repo_path.to_str().unwrap(),
        tmp.to_str().unwrap(),
    ]);


    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("main.go");
    walker.follow_token("main");
    walker.follow_token("f");

    let expected = r#"func f(x *int64) {
	*x = 10
}"#;
    assert_eq!(walker.node().text, expected);
}


#[test]
fn multifile_definitions() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/go-multifile");

    goscan(&[
        repo_path.to_str().unwrap(),
        tmp.to_str().unwrap(),
    ]);


    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("secondary.go");
    walker.follow_token("f");

    let expected = r#"func f(x *int64) {
	*x = 10
}"#;
    assert_eq!(walker.node().text, expected);
}


#[test]
fn cross_package_links() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/go-multipkg");

    goscan(&[
        repo_path.to_str().unwrap(),
        tmp.to_str().unwrap(),
    ]);


    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("foo/");
    walker.follow_token("pkg2/");
    walker.follow_token("pkg2.go");
    walker.follow_token("G");
    walker.follow_token("F");

    let expected = r#"func F() {
	fmt.Println("pkg1.F called")
}"#;
    assert_eq!(walker.node().text, expected);
}


#[test]
fn cross_module_links() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/go-multimod");

    goscan(&[
        repo_path.to_str().unwrap(),
        tmp.to_str().unwrap(),
    ]);


    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("mod2/");
    walker.follow_token("pkg2/");
    walker.follow_token("g.go");
    walker.follow_token("G");
    walker.follow_token("F");

    let expected = r#"func F() {
}"#;
    assert_eq!(walker.node().text, expected);
}


#[test]
fn function_search() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/go");

    goscan(&[
        repo_path.to_str().unwrap(),
        tmp.to_str().unwrap(),
    ]);

    let mut walker = index_uim(&repo_path, &tmp);

    let mut items = read_search_index(walker.index_path());

    items.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(items.len(), 4);

    assert_eq!(items[0].key, "/".to_string());
    assert_eq!(items[0].kind(), pb::IndexItemKind::IiDirectory);
    assert_eq!(items[0].path, None);
    walker.go_to_node(items[0].href.clone());
    assert_eq!(walker.node().text, "main.go\n");

    assert_eq!(items[1].key, "f".to_string());
    assert_eq!(items[1].kind(), pb::IndexItemKind::IiSymbol);
    assert_eq!(items[1].path, Some("main.go".to_string()));
    walker.go_to_node(items[1].href.clone());
    assert_eq!(walker.node().text, r#"func f(x *int64) {
	*x = 10
}"#);

    assert_eq!(items[2].key, "main".to_string());
    assert_eq!(items[2].kind(), pb::IndexItemKind::IiSymbol);
    assert_eq!(items[2].path, Some("main.go".to_string()));
    walker.go_to_node(items[2].href.clone());
    assert_eq!(walker.node().text, r#"func main() {
	var x int64
	f(&x)
	fmt.Printf("%d\n", x)
}"#);

    assert_eq!(items[3].key, "main.go".to_string());
    assert_eq!(items[3].kind(), pb::IndexItemKind::IiFile);
    assert_eq!(items[3].path, None);
    walker.go_to_node(items[3].href.clone());
    assert_eq!(walker.node().text, "func main() { … }\n\nfunc f(x *int64) { … }\n\n");
}


#[test]
fn dir_search() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/go-multipkg");

    goscan(&[
        repo_path.to_str().unwrap(),
        tmp.to_str().unwrap(),
    ]);

    let walker = index_uim(&repo_path, &tmp);

    let mut items = read_search_index(walker.index_path());

    items.sort_by(|a, b| a.key.cmp(&b.key));
    let keys: Vec<_> = items
        .into_iter()
        .filter(|ii| ii.kind() == IndexItemKind::IiDirectory)
        .map(|ii| ii.key)
        .collect();
    assert_eq!(keys, vec![
        "/".to_string(),
        "bar/".to_string(),
        "bar/pkg1/".to_string(),
        "foo/".to_string(),
        "foo/pkg1/".to_string(),
        "foo/pkg2/".to_string(),
    ]);
}



#[test]
fn package_name_collision() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/go-multipkg");

    build_scanner();

    let cmdout = Command::new(TERRITORY_ROOT.join("indexers/go/goscan"))
        .args(&[
            repo_path.to_str().unwrap(),
            tmp.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let out = String::from_utf8_lossy(&cmdout.stderr);
    println!("stderr: {}", out);
    assert!(!out.contains("f redeclared"));
    assert!(cmdout.status.success());
}


#[test]
fn top_level_vars() {
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/go-toplevel-vars");

    goscan(&[
        repo_path.to_str().unwrap(),
        tmp.to_str().unwrap(),
    ]);

    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("main.go");
    let expected_file_node_text = r#"type T1 …

type T2 …

type T3 …

type J …

type I …

type T4 …

const X …

const Y …

const Z …

var chrootDir …

var OtherFlag …

func main() { … }

"#;
    let file_node_text = &walker.node().text;
    assert_eq!(
        file_node_text,
        expected_file_node_text,
        "file node not as expected: {}",
        str_diff(&expected_file_node_text, &file_node_text));
    walker.follow_token("chrootDir");
    assert_eq!(walker.node().text, r#"var chrootDir = flag.String("chroot", "", "chroot before scanning")"#);

}



#[test]
fn typedef_links() {
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/go-toplevel-vars");

    goscan(&[
        repo_path.to_str().unwrap(),
        tmp.to_str().unwrap(),
    ]);

    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("main.go");
    walker.follow_token("main");
    walker.follow_token("T3");
    assert_eq!(walker.node().text, r#"type T3 struct {
	field   int32
	t4field *T4
}"#);
    walker.follow_token("T4");
    // TODO: more granular nodes
    assert_eq!(walker.node().text, r#"type (
	I interface {
		f()
	}

	T4 struct {
		field int32
		i     J
	}
)"#);
    walker.follow_token("J");
    assert_eq!(walker.node().text, "type J interface{}");

}


#[test]
fn references() {
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/go-toplevel-vars");

    goscan(&[
        repo_path.to_str().unwrap(),
        tmp.to_str().unwrap(),
    ]);

    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("main.go");
    walker.follow_token("T3");
    let refs = walker.token_references("T3");

    dbg!(&refs);
    assert_eq!(refs.refs.len(), 1);
    let ref_ = &refs.refs[0];
    assert_eq!(ref_.use_path, "main.go");
    assert_eq!(ref_.use_location, Some(pb::Location {
        line: 44,
        column: 8,  // Tabs miscalc?
        offset: 421,
    }));
    assert_eq!(ref_.context, "main");

    walker.go_to_node(refs.refs[0].href);

    assert!(walker.node().text.starts_with("func main() {"));
}

#[test]
fn container() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/go-multipkg");

    goscan(&[
        repo_path.to_str().unwrap(),
        tmp.to_str().unwrap(),
    ]);


    let mut walker = index_uim(&repo_path, &tmp);

    let mut parent = walker.node().id;
    walker.follow_token("foo/");
    assert_eq!(walker.node().container, Some(parent));

    parent = walker.node().id;
    walker.follow_token("pkg2/");
    assert_eq!(walker.node().container, Some(parent));

    parent = walker.node().id;
    walker.follow_token("pkg2.go");
    assert_eq!(walker.node().container, Some(parent));

    parent = walker.node().id;
    walker.follow_token("G");
    assert_eq!(walker.node().container, Some(parent));
}


#[test]
fn struct_container() {
    let rw = testlib::GoRepoWriter::new();
    rw.add_go("main.go", r#"
package main

type S struct { }
"#).unwrap();
    rw.add_mod("").unwrap();

    let mut walker = rw.index_repo();
    walker.follow_token("main.go");
    let main_node = walker.node().id;
    walker.follow_token("S");
    assert_eq!(walker.node().container, Some(main_node));
}


#[test]
fn definition_comments() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/go-comments");

    goscan(&[
        repo_path.to_str().unwrap(),
        tmp.to_str().unwrap(),
    ]);

    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("main.go");

    walker.follow_token("F");
    assert!(
        walker.node().text.starts_with("// the F function\nfunc F"),
        "got {}", walker.node().text);

    walker.back().unwrap();
    walker.follow_token("A");
    assert!(
        walker.node().text.starts_with(
            "// A is the answer to the ultimate question of life, the universe, and everything.\nconst A"),
        "got {}", walker.node().text);

    walker.back().unwrap();
    walker.follow_token("B");
    assert!(
        walker.node().text.starts_with("// a group of consts\nconst ("),
        "got {}", walker.node().text);

    walker.back().unwrap();
    walker.follow_token("X");
    assert!(
        walker.node().text.starts_with("// X is a number\ntype X"),
        "got {}", walker.node().text);
}


#[test]
fn free_modules() {
    let rw = testlib::GoRepoWriter::new();
    rw.add_go("foo/main.go", r#"
package main

func main() { }
"#).unwrap();
    rw.add_mod("foo").unwrap();
    rw.add_go("bar/main.go", r#"
package main

func main() { }
"#).unwrap();
    rw.add_mod("bar").unwrap();

    let mut walker = rw.index_repo();
    walker.follow_token("foo/");
    walker.follow_token("main.go");
    walker.follow_token("main");
    walker.reset();
    walker.follow_token("bar/");
    walker.follow_token("main.go");
    walker.follow_token("main");
}


#[test]
fn interfaces() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/go");

    goscan(&[
        repo_path.to_str().unwrap(),
        tmp.to_str().unwrap(),
    ]);
    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("main.go");
    walker.follow_token("main");
    let _tok = walker.find_token("f");

    // TODO
}


#[test]
fn embedded_struct() {
    let rw = testlib::GoRepoWriter::new();
    rw.add_go("main.go", r#"
package main

type T struct {
    i uint32
}

type U struct {
    T
}

func main() {
    var u U
    u.T.i = 1
}
"#).unwrap();
    rw.add_mod("").unwrap();

    let mut walker = rw.index_repo();
    walker.follow_token("main.go");
    walker.follow_token("U");

    let refs = walker.token_references("T");
    assert_eq!(refs.refs.len(), 1);
    assert_eq!(refs.refs[0].context, "main");

    walker.follow_token("T");
    assert!(walker.node().text.starts_with("type T struct {"), "found {}", walker.node().text);
}
