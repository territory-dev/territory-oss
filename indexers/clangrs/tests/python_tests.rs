use std::process::Command;
use testdir::testdir;

use clangrs::testlib::{init_logging, index_uim, TERRITORY_ROOT};


pub fn python_scan(repo_path: &str, uim_path: &str) {
    let cmdout = Command::new("python")
        .args(["-m", "territory_python_scanner", repo_path, uim_path])
        .output()
        .unwrap();
    print!(
        "returned: {}\nstdout:\n{}\nstderr:\n{}\n",
        cmdout.status,
        String::from_utf8_lossy(&cmdout.stdout),
        String::from_utf8_lossy(&cmdout.stderr));
    assert!(cmdout.status.success());
}


#[test]
fn reachability() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/py");

    python_scan(repo_path.to_str().unwrap(), tmp.to_str().unwrap());

    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("example.py");
    walker.follow_token("foo");

    let expected = r#"def foo(a: str = None):
    return text
"#;
    assert_eq!(walker.node().text, expected);
}


#[test]
fn relative_path() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/py");

    python_scan(repo_path.to_str().unwrap(), tmp.to_str().unwrap());

    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("example.py");
    walker.follow_token("foo");

    assert_eq!(walker.node().path, "example.py");
}


#[test]
fn method_container() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/py");

    python_scan(repo_path.to_str().unwrap(), tmp.to_str().unwrap());

    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("example.py");
    walker.follow_token("A");
    let class_node_id = walker.node().id;
    walker.follow_token("bar");

    assert_eq!(walker.node().container, Some(class_node_id));
}


#[test]
fn method_membership() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/py");

    python_scan(repo_path.to_str().unwrap(), tmp.to_str().unwrap());

    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("example.py");
    walker.follow_token("A");
    walker.follow_token("bar");

    assert_eq!(walker.node().member_of(), "A");
}


#[test]
fn references() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/py");

    python_scan(repo_path.to_str().unwrap(), tmp.to_str().unwrap());

    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("example.py");
    let refs = walker.token_references("text");
    assert_eq!(refs.refs.len(), 1);
    walker.go_to_node(refs.refs[0].href);
    assert!(walker.node().text.starts_with("def foo"));
}


#[test]
fn reference_context() {
    init_logging();
    let tmp = testdir!();
    let repo_path = TERRITORY_ROOT.join("repos/py");

    python_scan(repo_path.to_str().unwrap(), tmp.to_str().unwrap());

    let mut walker = index_uim(&repo_path, &tmp);

    walker.follow_token("example.py");

    let refs = walker.token_references("text");
    assert_eq!(refs.refs.len(), 1);
    assert_eq!(refs.refs[0].context, "foo");

    walker.follow_token("foo");
    let mut refs = walker.token_references("foo");
    println!("{:#?}", refs);
    assert_eq!(refs.refs.len(), 2);
    refs.refs.sort_by(|a, b| a.context.cmp(&b.context));
    assert_eq!(refs.refs[0].context, "A.bar");
    assert_eq!(refs.refs[1].context, "main");
}
