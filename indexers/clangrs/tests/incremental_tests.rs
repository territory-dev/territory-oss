use std::collections::HashMap;

use testdir::testdir;
use clangrs::testlib::{RepoWriter, GraphWalker};

use territory_core::resolver::ConcreteLocation;


#[test]
fn sym_id_preserved_for_same_usr() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("mod1.c", r#"
void a() { b() }
"#).unwrap();
    repo_writer.add_c_unit("mod2.c", r#"
void b() { }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("mod1.c");
    walker.follow_token("a");
    let node_href = walker.find_token("b").unwrap().context.href;

    repo_writer.update("mod2.c", r#"
void b() { (void)1; }
"#).unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.build_id = "test_build_2".to_string(); });
    walker.follow_token("mod1.c");
    walker.follow_token("a");
    let new_node_href = walker.find_token("b").unwrap().context.href;

    assert_eq!(node_href, new_node_href);
}


#[test]
fn blob_location_preserved_for_unchanged_nodes() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("mod1.c", r#"
void a() { }
"#).unwrap();
    repo_writer.add_c_unit("mod2.c", r#"
void b() { }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();

    let save_href = |w: &mut GraphWalker, map: &mut HashMap<&str, ConcreteLocation>, key| {
        let href = w.find_token(key).unwrap().context.href.expect("found token but no link");
        let loc = w.resolve_href(href);
        map.insert(key, loc);
    };

    let mut unchanged_locs_before: HashMap<&str, ConcreteLocation> = HashMap::new();
    let mut changed_locs_before: HashMap<&str, ConcreteLocation> = HashMap::new();

    save_href(&mut walker, &mut unchanged_locs_before, "mod1.c");
    save_href(&mut walker, &mut unchanged_locs_before, "mod2.c");

    walker.follow_token("mod1.c");
    save_href(&mut walker, &mut unchanged_locs_before, "a");

    walker.reset();
    walker.follow_token("mod2.c");
    save_href(&mut walker, &mut changed_locs_before, "b");

    repo_writer.update("mod2.c", r#"
void b() { (void)1; }
"#).unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.build_id = "test_build_2".to_string(); });
    let mut unchanged_locs_after: HashMap<&str, ConcreteLocation> = HashMap::new();
    let mut changed_locs_after: HashMap<&str, ConcreteLocation> = HashMap::new();

    save_href(&mut walker, &mut unchanged_locs_after, "mod1.c");
    save_href(&mut walker, &mut unchanged_locs_after, "mod2.c");

    walker.follow_token("mod1.c");
    save_href(&mut walker, &mut unchanged_locs_after, "a");
    assert_eq!(unchanged_locs_before, unchanged_locs_after);

    walker.reset();
    walker.follow_token("mod2.c");
    save_href(&mut walker, &mut changed_locs_after, "b");
    assert_ne!(changed_locs_before, changed_locs_after);
}
