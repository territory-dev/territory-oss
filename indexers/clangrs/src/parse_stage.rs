use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, File};

use itertools::Itertools;
use log::{warn, debug};

use territory_core::{
    GToken, Location, NodeID, NodeKind, RelativePath, TokenKind, TokenLocation
};
use cscanner::ast::{Block, ClangCurKind, ClangTokenContext, LocalDefinitionLocation, Sem, TransportID};

use crate::intermediate_model::sqlite::{
    SpanStore,
    SqliteServices,
    SqliteGSMWriter,
    SqliteUMWriter,
    Paths,
};
use crate::looks::write_elision_tokens;
use crate::scanner_driver::driver_loop;
use crate::writer::IntermediateNodeFileWriter;
use crate::args::{Args, get_debug_cfg};
use crate::intermediate_model::{
    sqlite, GlobalSymbolMapWriter, LocalSpanIndex, SemFile, SemNode, SemNodeContext, SemTokenContext
};
use crate::timers::Timers;


struct SliceState {
    sem_nodes: Vec<SemNode>,
    blocks: HashMap<TransportID, Block>,
}


struct Indexer<'a> {
    args: Args,
    pub paths: &'a Paths,
    span_store: &'a mut SpanStore,
    slice_states: Vec<SliceState>
}

impl<'a> Indexer<'a> {
    pub fn new(
        args: &Args,
        paths: &'a Paths,
        span_store: &'a mut SpanStore,
    ) -> Self {
        debug!("writing output to {:?}", args.outdir);

        Self {
            args: args.clone(),
            paths,
            span_store,
            slice_states: (0..args.par)
                .map(|_i| SliceState { sem_nodes: vec![], blocks: HashMap::new() })
                .collect_vec(),

        }
    }

    fn blocks<'b>(&'b mut self, slice: usize) -> &'b mut HashMap<TransportID, Block> {
        let state = self.slice_states
            .get_mut(slice-1)
            .expect(&format!("missing SliceState for slice {}", slice));
        &mut state.blocks
    }

    // returns if topmost node has been appended to the end of results
    fn write_sem_node(
        &mut self,
        slice: usize,
        mut block: Block,
        // blocks: &mut HashMap<TransportID, Block>,
        results: &mut Vec<SemNode>,
        container_id: Option<NodeID>,
    ) {
        let path_id = self.paths.path_to_id(&block.context.relative_path);

        let id = match  block.kind {
            NodeKind::SourceFile => self.paths.ensure_node_for_path(path_id, NodeKind::SourceFile),
            NodeKind::Definition | NodeKind::Class => self.span_store.store_one(
                path_id,
                block.kind,
                block.start.off,
                block.end.off,
                block.context.nest_level
            ).unwrap(),
            _ => {
                panic!(
                    "unexpected node kind {:?} for block at {}:{}",
                    block.kind, block.context.abs_path, block.start.line);
            },
        };

        let path = block.context.relative_path.clone();
        let nest_level = block.context.nest_level;

        let n = SemNode {
            id,
            container: container_id,
            kind: block.kind,
            member_of: block.member_of,
            path: block.context.relative_path.to_string(),
            path_id,
            start: block.start,
            context: SemNodeContext {
                path_id,
                kind: block.context.root.kind,
                display_name: block.context.root.display_name.clone(),
                path: path.clone(),
                sem: Some(block.context.root.clone()),
                nest_level,
                end_offset: block.end.off,
            },
            text: self.write_sem_toks(slice, id, results, &mut block.sems, block.text),
        };

        results.push(n);
    }

    fn write_sem_toks(
        &mut self,
        slice: usize,
        node_id: NodeID,
        results: &mut Vec<SemNode>,
        sems: &mut HashMap<TransportID, Sem>,
        text: Vec<GToken<ClangTokenContext>>
    ) -> Vec<GToken<SemTokenContext>> {
        let mut sem_toks = Vec::new();

        for cl_tok in text {
            let offset = cl_tok.offset;
            let line = cl_tok.line;
            let text = cl_tok.text;
            let type_ = cl_tok.type_;

            match cl_tok.context {
                ClangTokenContext::Token { sem, start, .. } => {
                    let tc = SemTokenContext {
                        // token_kind: tok.get_kind(),
                        loc: start,
                        sem: sem.map(|sem_key| sems
                            .get(&sem_key)
                            .expect("references Sem not in block sems")
                            .clone()),
                        local_definition: None,  // set later
                        elided: None,
                    };
                    let sem_tok = GToken { context: tc, offset, line, text, type_ };
                    sem_toks.push(sem_tok);
                    },
                    ClangTokenContext::Whitespace { .. } => {
                        let tc = SemTokenContext {
                            loc: Location::zero(), // TODO
                            sem: None,
                            local_definition: None,
                            elided: None,
                        };
                        let sem_tok = GToken { context: tc, offset, line, text, type_ };
                        sem_toks.push(sem_tok);
                    },
                    ClangTokenContext::Elided { nested_block_key, .. } => {
                        let mut nested_block = self.blocks(slice)
                            .remove(&nested_block_key)
                            .expect(&format!("referenced nested block missing ({}:{})", line, text));

                        let offset = nested_block.start.off;

                        if let Some(reason) = self.should_skip_node(&nested_block) {
                            if get_debug_cfg().print_node_skips {
                                debug!("skipping block {}:{}: {}", nested_block.context.abs_path, nested_block.start.line, reason);
                            }
                            let mut st = self.write_sem_toks(slice, node_id, results, &mut nested_block.sems, nested_block.text);
                            sem_toks.append(&mut st);
                            continue;
                        } else {
                            self.write_sem_node(slice, nested_block, results, Some(node_id));
                        };

                        let nested_node = &results.last().expect("write_sem_node returned true, expected results to be populated");
                        write_elision_tokens(nested_node, false, false, &mut |type_, text| {
                            let tc = SemTokenContext {
                                loc: Location::zero(), // TODO
                                sem: None,
                                local_definition: None,
                                elided: Some(TokenLocation {
                                    node_id: nested_node.id,
                                    offset,
                                }),
                            };
                            let sem_tok = GToken { context: tc, offset, line, text: text.into(), type_ };
                            sem_toks.push(sem_tok);
                        });
                    },
                }
            }

            sem_toks
    }


    fn resolve_local_definitions(&self, nodes: &mut Vec<SemNode>, source_set: &HashSet<RelativePath>) {
        let mut lsi = LocalSpanIndex::preload(
            &self.span_store,
            &source_set.iter().map(|rp| self.paths.path_to_id(rp)).collect_vec());
        lsi.set_forward_to(self.span_store);

        for node in nodes {
            for tok in &mut node.text {
                tok.context.local_definition = tok.context.sem.as_mut()
                    .and_then(|sem| sem.local_defintion.as_mut()
                    .and_then(|ldl| {
                        match self.ldl_to_tl(&lsi, &ldl) {
                            Ok(x) => x,
                            Err(e) => {
                                warn!(
                                    "failed to resolve local definition location for {} pointed to by {} (cur kind {:?}): {}",
                                    ldl.curloc,
                                    sem.curloc,
                                    sem.kind,
                                    e);

                                if self.args.fatal_missing_spans {
                                    panic!("missing spans are fatal");
                                }

                                None
                            }
                        }
                    }));
            }
        }
    }

    fn ldl_to_tl(
        &self,
        local_index: &LocalSpanIndex,
        ldl: &LocalDefinitionLocation
    ) -> Result<Option<TokenLocation>, String> {
        if !self.args.index_system && !ldl.path.is_in_repo() {
            return Ok(None)
        }

        let Some(path_id) = self.paths.get(&ldl.path) else {
            return Err(format!("missing path_id for {:?}", ldl.path));
        };

        let Some(node_id) = local_index.get(path_id, ldl.offset) else {
        // let Some(node_id) = self.span_store.get(path_id, ldl.offset) else {
            let file_node_id = self.paths.ensure_node_for_path(path_id, NodeKind::SourceFile);
            return Ok(Some(TokenLocation { node_id: file_node_id, offset: ldl.offset }));
        };
        Ok(Some(TokenLocation { node_id, offset: ldl.offset }))
    }

    pub fn should_skip_node(&self, block: &Block) -> Option<&'static str> {
        if block.kind == NodeKind::Definition && !block.text.iter().any(|tok| match tok.type_ {
            TokenKind::Literal | TokenKind::Identifier | TokenKind::Keyword => true,
            TokenKind::Comment | TokenKind::Punctuation | TokenKind::WS => false,
        }) { return Some("not code"); }

        if !self.args.index_system && !block.context.relative_path.is_in_repo() {
            return Some("not in repo path and system indexing is disabled");
        }

        let tkinds: Vec<(TokenKind, String)> = block.text.iter()
            .filter_map(|slice_tok| match slice_tok.context {
                ClangTokenContext::Token { .. } => Some((slice_tok.type_, slice_tok.text.clone())),
                ClangTokenContext::Whitespace { .. } => None,
                ClangTokenContext::Elided { .. } => None,
            })
            .take(3)
            .collect();
        let root_kind = block.context.root.kind;
        let tkinds_slice = tkinds.iter().map(|(k, s)| (k, s as &str)).collect::<Vec<_>>();
        use TokenKind::{Punctuation, Identifier};
        use ClangCurKind::*;
        match (root_kind, &tkinds_slice[..]) {
            (FunctionDecl, _) => {
                if block.context.is_forward_decl { return Some("forward decl"); }
            }
            (InclusionDirective, _) => { return Some("InclusionDirective"); }
            (PreprocessingDirective,
                [(Punctuation, "#"), (Identifier, "define"), (Identifier, _id), ..]) => {}
            (PreprocessingDirective,
                [(Punctuation, "#"), (Identifier, "undef"), (Identifier, _id), ..]) => {
                    return Some("#undef"); }
            (PreprocessingDirective, _) => { return Some("PreprocessingDirective"); }
            _ => {}
        }

        None
    }

}


// returns Some(reason) if we should skip

pub fn store_external_definition_locations(
    sem_nodes: &Vec<SemNode>,
    global_defs: &mut impl GlobalSymbolMapWriter,
) {
    for node in sem_nodes {
        for tok in &node.text {
            if let Some(sem) = &tok.context.sem {
                if let (
                    true,
                    Some(usr),
                    Some(loc_def)
                ) = (
                    sem.is_definition,
                    &sem.usr,
                    tok.context.local_definition
                ) {
                    global_defs.insert(usr.clone(), loc_def);
                }
            }
        }
    }
}


fn dump_sems(sems: &Vec<SemNode>) {
    for n in sems {
        debug!("{}:{} {:?} #{}", n.path, n.start.line, n.kind, n.id);
        for tok in &n.text {
            debug!("    {:?} {}", tok.type_, tok.text);
            if let Some(sem) = tok.context.sem.as_ref() {
                debug!("        {:?}", sem);
            }
        }
    }
}


pub fn parse_stage(args: &Args) {
    let mut stores = sqlite::new_from_args(args);
    parse_stage_with_stores(args, &mut stores)
}

pub fn parse_stage_with_stores(args: &Args, stores: &mut SqliteServices<SqliteGSMWriter, SqliteUMWriter>) {
    std::fs::create_dir_all(&args.intermediate_path).unwrap();

    let SqliteServices {
        global_symbol_map: global_defs,
        uses_map: _uses,
        span_store,
        paths,
        ..
    } = stores;

    let indexer = crate::parse_stage::Indexer::new(&args, paths, span_store);

    let mut node_file_writer = IntermediateNodeFileWriter::new_from_args(args);

    let mut scan_log_file = args.log_dir.as_ref().map(|log_dir| {
        create_dir_all(log_dir).unwrap();
        File::options().create(true).append(true).write(true).open(log_dir.join("scan")).unwrap()
    });

    let mut timers = Timers::new();

    driver_loop(args, indexer, scan_log_file.as_mut(),
    &mut |indexer: &mut Indexer, block: Block, slice: usize| {
        if block.context.nest_level > 0 {
            indexer.blocks(slice).insert(block.transport_key, block);
            return Ok(());
        }

        let container_id = match block.kind {
            NodeKind::Class | NodeKind::Definition => {
                let file_path_id = paths.path_to_id(&block.context.relative_path);
                paths.ensure_node_for_path(file_path_id, NodeKind::SourceFile)
            },
            NodeKind::SourceFile => {
                let dir_path_id = paths.path_to_id(&block.context.relative_path.parent().unwrap());
                paths.ensure_node_for_path(dir_path_id, NodeKind::Directory)
            },
            _ => {
                panic!(
                    "<{}> unexpected node kind {:?} for block at {}:{}",
                    slice, block.kind, block.context.abs_path, block.start.line);
            },

        };

        if let Some(reason) = indexer.should_skip_node(&block) {
            if get_debug_cfg().print_node_skips {
                debug!("skipping block {}:{}: {}", block.context.abs_path, block.start.line, reason);
            }
        } else {
            let mut results = Vec::new();
            indexer.write_sem_node(slice, block, &mut results, Some(container_id));
            indexer.slice_states[slice-1].sem_nodes.extend(results);
        }

        Ok(())
    },
    &mut |indexer: &mut Indexer, slice: usize, source_set: &HashSet<RelativePath>| {
        let mut sem_nodes = std::mem::replace(&mut indexer.slice_states[slice-1].sem_nodes, Vec::new());

        timers.timed("resolve local definitions", || {
            indexer.resolve_local_definitions(&mut sem_nodes, source_set);
        });

        if get_debug_cfg().print_sem_nodes {
            dump_sems(&sem_nodes);
        }

        timers.timed("external definition locations", || {
            store_external_definition_locations(&sem_nodes, global_defs)
        });

        timers.timed("semfile write", || {
            for (_path_id, nodes) in &sem_nodes.into_iter().group_by(|n| n.context.path_id) {
                let sf = SemFile { nodes: nodes.collect() };
                node_file_writer.append(&sf);
            }
        });

        Ok(())
    });

    if get_debug_cfg().print_global_defs {
        // println!("global desfs: {:#?}", global_defs);
        todo!();
    }

    timers.dump();
}
