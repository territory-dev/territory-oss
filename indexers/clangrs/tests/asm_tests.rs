use clangrs::testlib::{index_with_changed_args, repr_diff, TERRITORY_ROOT};
use territory_core::{pb_node_tokens, TokenKind};

#[test]
fn test_assembly_file_node() {
    let mut walker = index_with_changed_args(|args| {
        args.repo = TERRITORY_ROOT.join("repos/example-asm");
    });

    walker.follow_token("pmjump.S");
    let expected = r#"#include "inc.h"

	.text
	.code16

SYM_FUNC_START_NOALIGN(protected_mode_jump)
	movl	%edx, %esi		# Pointer to boot_params table
	addl	%ebx, 2f"#
        .to_string();

    let got = &walker.node().text;
    assert_eq!(got, &expected, "{}", repr_diff(&expected, got));

    use TokenKind::{Identifier, Punctuation, WS, Literal};
    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    let expected = vec![
        (Punctuation, "#"),
        (Identifier, "include"),
        (WS, " "),
        (Literal, "\"inc.h\""),
        (WS, "\n\n\t"),
        (Punctuation, "."),
        (Identifier, "text"),
        (WS, "\n\t"),
        (Punctuation, "."),
        (Identifier, "code16"),
        (WS, "\n\n"),
        (Identifier, "SYM_FUNC_START_NOALIGN"),
        (Punctuation, "("),
        (Identifier, "protected_mode_jump"),
        (Punctuation, ")"),
        (WS, "\n\t"),
        (Identifier, "movl"),
        (WS, "\t"),
        (Punctuation, "%"),
        (Identifier, "edx"),
        (Punctuation, ","),
        (WS, " "),
        (Punctuation, "%"),
        (Identifier, "esi"),
        (WS, "\t\t"),
        (Punctuation, "#"), (WS, " "), (Identifier, "Pointer"), (WS, " "), (Identifier, "to"), (WS, " "), (Identifier, "boot_params"), (WS, " "), (Identifier, "table"),
        (WS, "\n\t"),
        (Identifier, "addl"),
        (WS, "\t"),
        (Punctuation, "%"),
        (Identifier, "ebx"),
        (Punctuation, ","),
        (WS, " "),
        (Literal, "2f"),
    ];
    assert_eq!(
        got,
        expected,
        "token references don't match: {}",
        repr_diff(&expected, &got)
    );
}


#[test]
fn macro_link() {
    let mut walker = index_with_changed_args(|args| {
        args.repo = TERRITORY_ROOT.join("repos/example-asm");
    });

    walker.follow_token("pmjump.S");
    walker.follow_token("SYM_FUNC_START_NOALIGN");

    assert_eq!(walker.node().text, r#"#define SYM_FUNC_START_NOALIGN(l) \
	l:"#);

}


#[test]
fn assembly_file_container_is_dir() {
    let mut walker = index_with_changed_args(|args| {
        args.repo = TERRITORY_ROOT.join("repos/example-asm");
    });

    let dir_node_id = walker.node().id;
    walker.follow_token("pmjump.S");
    assert_eq!(walker.node().container, Some(dir_node_id));
}
