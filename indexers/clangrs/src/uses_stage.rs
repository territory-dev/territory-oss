use itertools::Itertools;
use log::info;

use cscanner::ast::ClangCurKind;
use territory_core::{
    Ref,
    TokenLocation,
    TokenKind,
    GToken, nice_location,
};

use crate::intermediate_model::sqlite::{SqliteServices, SqliteGSMReader, SqliteUMWriter};
use crate::writer::IntermediateNodeFileReader;
use crate::args::Args;
use crate::intermediate_model::{
    sqlite,
    GlobalSymbolMapReader,
    SemNode,
    SemTokenContext,
    UsesMap,
};


pub fn make_reference_to_location(
    node: &SemNode,
    tok: &GToken<SemTokenContext>,
    via_usr: bool,
) -> Ref {
    let use_node_id = node.id;

    Ref {
        href: use_node_id,
        context: ref_context(tok),
        use_location: tok.context.loc,
        linked_via_sym: via_usr,
        use_path: node.context.path.clone(),
    }
}

fn ref_context(
    tok: &GToken<SemTokenContext>,
) -> String {
    let Some(sem) = &tok.context.sem else {
        return "".to_string();
    };

    if sem.definition_context.is_empty() {
        return "".to_string();
    }

    sem.definition_context.iter().rev().join("::")
}


fn make_reference(
    node: &SemNode,
    tok: &GToken<SemTokenContext>,
) -> Option<(TokenLocation, Ref)> {
    if let Some(defn_token_location) = tok.context.local_definition {
        if defn_token_location.node_id != node.id {
            let ref_ = make_reference_to_location(node, tok, false);
            Some((defn_token_location, ref_))
        } else {
            None
        }
    } else {
        None
    }
}


fn collect_intra_tu_uses(uses: &mut impl UsesMap, node: &SemNode) {
    for tok in &node.text {
        if tok.type_ != TokenKind::Identifier { continue; }
        if let Some((defn_id, ref_)) = make_reference(node, tok) {
            uses.record_use(defn_id, ref_)
        }
    }
}


pub fn collect_cross_tu_uses(
    global_defs: &mut impl GlobalSymbolMapReader,
    uses: &mut impl UsesMap,
    b: &SemNode,
) {
    for tok in &b.text {
        if let Some(sem) = &tok.context.sem {

            if let (ClangCurKind::DeclRefExpr, None, Some(name)) = (sem.kind, tok.context.local_definition, &sem.name) {
                if let Some(usr) = &sem.usr {
                    if let Some(
                        ext_location @ TokenLocation{ node_id: ext_node_id, offset: _ext_offset }
                    ) = global_defs.get(usr) {
                        if ext_node_id != b.id {
                            let ref_ = make_reference_to_location(b, tok, true);
                            uses.record_use(ext_location, ref_);
                        }
                    } else {
                        info!("missing symbol definition: {} at {}", usr, nice_location(&b.path, &tok.context.loc));
                    }
                } else {
                    info!("missing declaration: {} at {}", name, nice_location(&b.path, &tok.context.loc));
                }
            }
        }
    }
}


pub fn uses_stage(args: &Args) {
    let mut stores = sqlite::new_from_args(args);
    uses_stage_with_store(args, &mut stores)
}

pub fn uses_stage_with_store(args: &Args, stores: &mut SqliteServices<SqliteGSMReader, SqliteUMWriter>) {
    let SqliteServices {
        global_symbol_map: ref mut global_defs,
        ref mut uses_map,
        ..
    } = stores;

    if args.no_references { return; }

    let mut node_file_reader = IntermediateNodeFileReader::new_with_slice(args, 1);

    let (sender, recv) = crossbeam_channel::bounded(args.par);
    let mut threads = Vec::new();
    for _i in 1..=args.par {
        let ch = recv.clone();
        let mut uses_map = uses_map.clone();
        let mut global_defs = global_defs.clone();

        let t  = std::thread::spawn(move || {
            while let Ok(n) = ch.recv() {
                collect_intra_tu_uses(&mut uses_map, &n);
                collect_cross_tu_uses(&mut global_defs, &mut uses_map, &n);
            }
        });
        threads.push(t);
    }
    drop(recv);

    for n in node_file_reader.flat_map(move |semfile| semfile.nodes.into_iter().collect::<Vec<SemNode>>()) {
        sender.send(n).unwrap();
    }
    drop(sender);
    for t in threads {
        t.join().unwrap();
    }
}
