use cscanner::ast::ClangCurKind;
use territory_core::territory::index::token::Href;
use territory_core::territory::index::NodeIdWithOffsetHref;
use territory_core::{
    Node,
    TokenLocation,
    TokenKind,
    HyperlinkedNodeContext,
    HyperlinkedTokenContext, ReferencesLink, SymID,
};

use log::info;

use crate::intermediate_model::sqlite::{SqliteGSMReader, SqliteServices, SqliteUMQuery};
use crate::storage::start_from_args;
use crate::writer::{NodeWriter, IntermediateNodeFileReader};
use crate::args::Args;
use crate::intermediate_model::{
    sqlite,
    GlobalSymbolMapReader,
    SemFile,
    SemNode,
    SemTokenContext,
    UsesMapQuery,
};

use crate::timers::Timers;


struct OutputStage {
    node_writer: NodeWriter,
    references_enabled: bool,
}

impl OutputStage {
    pub fn new(node_writer: NodeWriter, references_enabled: bool) -> Self {
        Self { node_writer, references_enabled }
    }

    fn make_hyperlinked_node(
        &mut self,
        global_defs: &mut impl GlobalSymbolMapReader,
        uses: &impl UsesMapQuery,
        sem_node: SemNode,
    ) -> Node {
        let node_id = sem_node.id;
        sem_node
            .map_tokens(&mut |offset, _text, kind, ctx| {
                let sym_id = (*kind == TokenKind::Identifier).then_some(())
                .and_then(|_| ctx.sem.as_ref())
                .and_then(|sem| {
                    let usr = sem.usr.as_ref()?;
                    global_defs.get_sym_id(usr)
                });

                let href = if let Some(elided) = ctx.elided {
                    Some(Href::NodeIdRef(elided.node_id))
                } else if *kind == TokenKind::Identifier {
                    get_href(&ctx, sym_id)
                } else {
                    None
                };

                let token_location = TokenLocation { node_id, offset };
                let references = if *kind == TokenKind::Identifier {
                    if uses.has_uses(&token_location) {
                        ReferencesLink::TokenLocation(token_location)
                    } else {
                        ReferencesLink::None
                    }
                } else {
                    ReferencesLink::None
                };

                HyperlinkedTokenContext { href, sym_id, references }
            })
            .replace_context(HyperlinkedNodeContext {
                references: None,
            })
    }

    pub fn generate_hyperlinked_graph<'tu>(
        &mut self,
        global_defs: &mut impl GlobalSymbolMapReader,
        uses: &SqliteUMQuery,
        files: impl Iterator<Item=SemFile>,
    ) {
        for sem_file in files {
            let file = sem_file.map_nodes(&mut |sem_node: SemNode|
                self.make_hyperlinked_node(global_defs, uses, sem_node));
            self.node_writer.submit_blob(file);
        }
    }

    pub fn finalize(mut self, uses: impl UsesMapQuery) {
        if self.references_enabled {
            uses.write(&mut self.node_writer);
        }

        let writer_stats = self.node_writer.join();

        info!("translation unit done, wrote {} nodes", writer_stats.total_written);
        info!("total PB bytes written:    {}", writer_stats.pb_bytes_count);
        info!("total PB bytes reused:     {}", writer_stats.pb_bytes_reused);
    }
}


fn get_href(
    tok_ctx: &SemTokenContext,
    sym_id: Option<SymID>,
) -> Option<Href> {
    use ClangCurKind::*;

    let sem = tok_ctx.sem.as_ref()?;
    // TODO: prefer using SEM if available?
    if let Some(def_token_loc) = tok_ctx.local_definition {
        Some(Href::NodeIdWithOffsetRef(NodeIdWithOffsetHref { node_id: def_token_loc.node_id, offset: def_token_loc.offset }))
    } else if [DeclRefExpr, MemberRefExpr, TypeRef, Method, Constructor, Destructor].contains(&sem.kind) {
        sym_id.map(|SymID(id)| Href::SymIdRef(id))
    } else {
        None
    }
}


pub async fn output_stage(args: &Args) {
    let stores = sqlite::new_from_args(args);
    output_stage_with_stores(args, stores).await;
}

pub async fn output_stage_with_stores(args: &Args, stores: SqliteServices<SqliteGSMReader, SqliteUMQuery>) {
    let mut timers = Timers::new();

    let SqliteServices {
        global_symbol_map: mut global_defs,
        uses_map,
        output_map,
        ..
    } = stores;

    let (storage_done, storage_channel) = start_from_args(args).await;

    let node_writer = NodeWriter::start(&args, args.writer_concurrency, storage_channel, output_map);

    let mut output_stage_ = crate::output_stage::OutputStage::new(node_writer, !args.no_references);

    let mut node_file_reader = IntermediateNodeFileReader::new_with_slice(args, 1);

    timers.timed("hyperlinked graph generation", || {
        output_stage_.generate_hyperlinked_graph(&mut global_defs, &uses_map, &mut node_file_reader);
    });

    timers.timed("finalize", move || {
        output_stage_.finalize(uses_map);
    });

    timers.async_timed("storage.join", async {
        storage_done.await.unwrap();
    }).await;

    timers.dump();
}
