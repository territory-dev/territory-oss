use std::fs::read_to_string;

use testdir::testdir;
use serde_json::json;

use clangrs::testlib::RepoWriter;


#[test]
fn logs() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_c_unit("mod1.c", r#"
#define D
#include "def.h"
T x;
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let repo_dir = repo_writer.repo_dir();
    let log_dir = testdir!().join("logs");

    repo_writer.index_repo_with_args(|args| {
        args.log_dir = Some(log_dir.clone());
        args.clang_extra_args = None;
    });

    let log_text = read_to_string(log_dir.join("scan")).expect("can't read scan log file");
    let expected_logs = [
        format!(
            r#"<1> [1/1] {}/mod1.c ["clang", "-c", "-o", "{}/mod1.c.o"]
"#,
            repo_dir.to_string_lossy(),
            repo_dir.to_string_lossy()),
        format!(
            r#"<1> expected #include to point to a file: {}/mod1.c:3 "def.h"
"#,
            repo_dir.to_string_lossy()),
        format!(
            r#"<1> {}/mod1.c:3:10: fatal error: 'def.h' file not found
"#,
            repo_dir.to_string_lossy()),
    ];
    for expected_log in expected_logs {
        assert!(log_text.contains(&expected_log), "expected {} to contain {}", log_text, expected_log);
    }
}


#[test]
fn log_parse_error() {
    let dir = testdir!();
    let repo_writer = RepoWriter::new(&dir);
    let ccs = json!([
        {
            "command": "clang -c /does/not/exist.c",
            "file": "/does/not/exist.c",
            "directory": dir,
        }
    ]);
    repo_writer.add(
        "compile_commands.json",
        &serde_json::to_string_pretty(&ccs).expect("failed to serialize CCs ")
    ).unwrap();

    let log_dir = testdir!().join("logs");

    let _ = std::panic::catch_unwind(|| {
        repo_writer.index_repo_with_args(|args| {
            args.log_dir = Some(log_dir.clone());
        });
    });

    let log_text = read_to_string(log_dir.join("scan")).expect("can't read scan log file");
    assert!(log_text.contains("<1> /does/not/exist.c: source file does not exist\n"));
}
