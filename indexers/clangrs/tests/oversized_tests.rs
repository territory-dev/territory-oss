use testdir::testdir;
use clangrs::testlib::RepoWriter;



#[test]
fn hueg() {
    let mut code = String::new();
    code.push_str("struct foo {\n");
    for i in 1..100_000 {
        code.push_str(&format!("int field_{i};\n"));
    }
    code.push_str("};");

    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c", &code).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.max_node_len = 100; });
    walker.follow_token("main.c");
    walker.follow_token("foo");
    let n = walker.node();
    assert!(n.text.contains("<TRUNCATED>"));
    assert_eq!(n.tokens.len(), 168);  // 100 + whitespace
}


#[test]
#[ignore]
fn hueger() {
    let mut code = String::new();
    for j in 1..10 {
        code.push_str(&format!("enum ENUM_{j} {{\n"));
        for i in 1..100_000 {
            code.push_str(&format!("VAL_{j}_{i},\n"));
        }
        code.push_str("};");
    }

    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c", &code).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.max_node_len = 100; });
    walker.follow_token("main.c");
    walker.follow_token("ENUM_1");
    let n = walker.node();
    assert!(n.text.contains("<TRUNCATED>"));
}


#[test]
fn hueg_string() {
    let mut code = String::new();
    code.push_str("const char * yo_mama = ");
    for _ in 1..10_000 {
        code.push_str("   \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n");
    }
    code.push_str(";\n");

    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c", &code).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.max_node_len = 100; });
    walker.follow_token("main.c");
    walker.follow_token("yo_mama");
    let n = walker.node();
    assert!(n.text.contains("<TRUNCATED>"));
}


#[test]
#[ignore]
fn many() {
    let mut code = String::new();
    for i in 1..10_000_000 {
        code.push_str(&format!("struct s{i} {{ }};\n"));
    }

    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("main.c", &code).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.max_node_len = 100; });
    walker.follow_token("main.c");
    let n = walker.node();
    assert_eq!(n.tokens.len(), 100);  // 100 + whitespace

    walker.follow_token("s1");
    let n = walker.node();
    assert_eq!(n.text, "struct s1 { };");
}
