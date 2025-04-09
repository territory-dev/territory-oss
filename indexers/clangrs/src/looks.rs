use std::collections::HashSet;

use itertools::{Itertools, PeekingNext};
use log::info;

use territory_core::{
    territory::index as pb,
    GToken,
    Token,
    TokenKind,
    Offset,
    NodeID,
    nice_location,
};
use territory_core::territory::index::{IndexItemKind, IndexItem};
use territory_core::token_writer::TokenWriter;
use cscanner::ast::{Sem, ClangCurKind};
use crate::intermediate_model::{SemNode, SemTokenContext};


pub fn write_file_entry_tokens<'a>(node: &SemNode, dest: &mut Vec<Token>, max_node_len: usize, offset: Offset, line: u32, href: NodeID) {
    let mut w = TokenWriter::new(dest, offset, line, max_node_len);
    let h = Some(href);
    write_elision_tokens(node, true, true, &mut |kind, text| {
        w.write(kind, text, None, h, None);
    });
}


pub fn write_elision_tokens(
    node: &SemNode,
    nl: bool,
    join_lines: bool,
    write_token: &mut impl FnMut(TokenKind, &str),
) {

    let tkinds: Vec<(TokenKind, &str)> = node.text.iter()
        .filter_map(|tok| match tok.type_ {
            TokenKind::WS => None,
            _ => Some((tok.type_, &tok.text as &str)),
        })
        .collect();
    use ClangCurKind::*;
    use TokenKind::{Punctuation, Identifier, WS, Keyword};
    let first_token = node.text.first().expect("empty node text");
    let first_token_kind: Option<ClangCurKind> = first_token.context.sem.as_ref().map(|sem| sem.kind);
    match (node.context.kind, &tkinds[..]) {
        (PreprocessingDirective, [(Punctuation, "#"), (Identifier, "define"), (Identifier, id), tail @ ..]) => {
            let mut it = node.text.iter();
            let mut is_function_like = false;
            for tok in &mut it {
                if let Some(sem) = &tok.context.sem {
                    if sem.kind == MacroDefinition {
                        is_function_like = sem.is_function_like_macro;
                        break;
                    }
                }
            }
            write_token(Keyword, "#");
            write_token(Keyword, "define");
            write_token(WS, " ");
            write_token(TokenKind::Identifier, id);
            if is_function_like {
                for tok in it {
                    write_token(tok.type_, &tok.text);
                    if tok.type_ == Punctuation && tok.text == ")" { break; }
                }
            }
            if !tail.is_empty() {
                write_token(WS, " …");
            }
            if nl { write_token(WS, "\n"); }
        }
        (FunctionDecl | EnumDecl | Method | Destructor | FunctionTemplate, _) => {
            for tok in &node.text {
                let txt = if join_lines { &tok.text.replace("\n", " ") } else { &tok.text };
                write_token(tok.type_, txt);
                if tok.type_ == Punctuation && tok.text == "{" {
                    write_token(WS, " … ");
                    write_token(Punctuation, "}");
                    break;
                }
            }
            if nl { write_token(WS, "\n"); }
        }
        (Constructor, _) => {
            for tok in &node.text {
                let txt = if join_lines { &tok.text.replace("\n", " ") } else { &tok.text };
                write_token(tok.type_, txt);
                if tok.type_ == Punctuation && tok.text == "{" {
                    write_token(WS, " … ");
                    write_token(Punctuation, "}");
                    break;
                }
                if tok.type_ == Punctuation && tok.text == ":" {
                    write_token(WS, " … ");
                    write_token(Punctuation, "{");
                    write_token(WS, " … ");
                    write_token(Punctuation, "}");
                    break;
                }
            }
            if nl { write_token(WS, "\n"); }
        }
        (VarDecl | StructDecl | ClassDecl | ClassTemplate, _) => {
            let mut add = true;
            let mut text = node.text.iter();
            'outer: while let Some(tok) = text.next() {
                let tok_cur_kind = tok.context.sem.as_ref().map(|sem| sem.kind);
                if tok_cur_kind == Some(VarDecl) {
                    add = true;
                }

                if add && tok_cur_kind == Some(FieldDecl) {
                    write_token(WS, " … ");
                    write_token(Punctuation, "}");
                    write_token(WS, " ");
                    add = false;
                }

                if add {
                    let txt = if join_lines { &tok.text.replace("\n", " ") } else { &tok.text };
                    write_token(tok.type_, &txt);
                }

                // skip over a block to variable declarations
                if add && tok.text == "{" {
                    write_token(WS, " … ");
                    write_token(Punctuation, "}");
                    // write_token(WS, " ");
                    let Some(skip_until) = tok.context.sem.as_ref().and_then(|sem| sem.cur_end_offset) else {
                        continue;
                    };
                    while text.peeking_next(|tok| tok.offset < skip_until).is_some() {}
                    continue;
                }

                if add & (tok.type_ == Punctuation && tok.text == "=") {
                    write_token(WS, " …");
                    add = false;
                }

                if add && (tok.type_ == Keyword && tok.text == "template") {
                    let mut open_angle_braces = 0;
                    while let Some(tok) = text.next() {
                        let txt = if join_lines { &tok.text.replace("\n", " ") } else { &tok.text };
                        write_token(tok.type_, &txt);

                        if tok.type_ == Punctuation && tok.text == "<" {
                            open_angle_braces += 1;
                        }
                        if tok.type_ == Punctuation && tok.text == ">" {
                            open_angle_braces -= 1;
                            if open_angle_braces == 0 {
                                continue 'outer;
                            }
                        }
                    }
                }
            }
            if nl { write_token(WS, "\n"); }
        }
        (_, [(Identifier, id), tail @ ..])
            if first_token_kind == Some(MacroExpansion)
            || node.context.kind == MacroExpansion
            || first_token_kind == Some(PreprocessingDirective) =>
        {
            write_token(Identifier, id);
            if !tail.is_empty() {
                // if node.context.sem.as_ref().map(|sem| sem.is_function_like_macro) == Some(true) {
                if first_token.context.sem.as_ref().map(|sem| sem.is_function_like_macro) == Some(true) {
                    write_token(Punctuation, "(");
                    write_token(WS, "…");
                    write_token(Punctuation, ")");
                }
                if let (Some(cur_end_off), Some(last_tok)) = (node.context.sem.as_ref().and_then(|sem| sem.cur_end_offset), node.text.last()) {
                    if (cur_end_off as usize) < last_tok.offset as usize + last_tok.text.len() {
                        write_token(WS, " …");
                    }
                }
            }
            if nl { write_token(WS, "\n"); }
        }
        (UsingDirective, toks) => {
            // copy
            for (kind, text) in toks { write_token(*kind, text); }
        }
        _ => {
            if let Some(display_name) = &node.context.display_name {
                write_token(TokenKind::Identifier, display_name);
                if nl { write_token(WS, "\n"); }
            } else {
                write_token(first_token.type_, &first_token.text);
                write_token(WS, " …");
                if nl { write_token(WS, "\n"); }
            };
        }
    }
}


pub fn inverted_index_entries(node: &SemNode) -> Vec<IndexItem> {
    let cur_kind = node.context.kind;
    let def_sem = if let Some(sem) = node.context.sem.as_ref() {
        sem
    } else {
        return vec![];
    };

    let mut items = Vec::new();

    let mut type_ = def_sem.type_.clone();
    if type_.as_ref().map(|t| t.starts_with("enum (unnamed at")) == Some(true) {
        type_ = None;
    }

    let make_item_with_type = |key: String, type_: Option<String>| {
        IndexItem {
            key: key.into(),
            href: Some(pb::index_item::Href::NodeId(node.id)),
            kind: match cur_kind {
                ClangCurKind::PreprocessingDirective => IndexItemKind::IiMacro.into(),
                _ => IndexItemKind::IiSymbol.into(),
            },
            path: Some(node.path.clone()),
            r#type: type_.map(Into::into),
        }
    };

    let make_item = |key: String| {
        make_item_with_type(key, type_.clone())
    };

    match (cur_kind, &def_sem.name) {
        (ClangCurKind::EnumDecl, _) => {
            assert!(node.text.len() > 0);
            if node.text[0].text.to_lowercase() != "enum" {
                // include/linux/cgroup-defs.h:43
                info!(
                    "expected enum, found {} at {}",
                    node.text[0].text,
                    nice_location(&node.path, &node.text[0].context.loc));
            }
            let mut res: Vec<String> = Vec::new();
            for tok in node.text[1..].iter() {
                if tok.text == "class" && tok.type_ == TokenKind::Keyword {
                    // enum class T : ... { ... }

                    let qual_name = def_sem.definition_context.iter().rev().join("::");
                    items.push(make_item(qual_name.to_string()));
                    res.clear();
                    break;
                }
                if tok.type_ == TokenKind::WS { continue; }

                if tok.text == "{" { break; }

                if let Some(sem) = &tok.context.sem {
                    if sem.kind == ClangCurKind::EnumConstantDecl { break; }
                }

                res.push(tok.text.clone());
            }
            inverted_index_enum_items(&mut items, &node.text, &make_item);
            if !res.is_empty() {
                items.push(make_item(res.join(" ")));
            }
        },
        (ClangCurKind::VarDecl, _) => {
            let mut uniq_items = HashSet::new();

            for tok in &node.text {
                if let Some(sem) = &tok.context.sem {
                    if let (Some(name), type_, ClangCurKind::VarDecl) = (&sem.name, &sem.type_, sem.kind) {
                        uniq_items.insert((name.clone(), type_.clone()));
                    }
                }
            }

            for (uiname, uitype) in uniq_items {
                items.push(make_item_with_type(uiname.to_string(), uitype));
            }
        }
        (_, Some(def_name)) if def_sem.is_definition => {
            let qual_name = def_sem.definition_context.iter().rev().join("::");
            items.push(make_item(qual_name.to_string()));
        }
        (ClangCurKind::PreprocessingDirective, _) => {
            let ts = node.text.iter()
                .filter(|t| t.type_ != TokenKind::WS)
                .take(3)
                .collect::<Vec<&GToken<SemTokenContext>>>();
            if ts.len() >= 3 && ts[1].text == "define" {
                items.push(make_item(ts[2].text.to_string()));
            } else {
                return vec![];
            }
        }
        _ => {
            info!(
                "no display name, cannot add node to inverted index: {}",
                nice_location(&node.path, &node.start));
            return vec![];
        }
    }

    items.retain(|item| item.key.len() < 200);

    items
}


fn inverted_index_enum_items(
    items: &mut Vec<IndexItem>,
    toks: &[GToken<SemTokenContext>],
    mut make_item: impl FnMut(String) -> IndexItem,

) {
    let mut hs = HashSet::new();
    for tok in toks {
        if let Some(sem) = &tok.context.sem {
            if let Sem { kind: ClangCurKind::EnumConstantDecl, name: Some(name), .. }  = &sem {
                let new = hs.insert(name);
                if ! new { continue; }
                items.push(make_item(name.to_string()));
            }
        }
    }
}
