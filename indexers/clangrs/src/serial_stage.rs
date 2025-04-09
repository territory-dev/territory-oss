use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use log::info;

use territory_core::{
    GToken, HNBlob, HyperlinkedNodeContext, HyperlinkedTokenContext, Location, Node, NodeID, NodeKind, RelativePath, TokenKind
};
use territory_core::territory::index::{IndexItemKind, IndexItem, index_item};

use crate::intermediate_model::sqlite::{Paths, self, SqliteServices, SqliteGSMReader, SqliteUMQuery};
use crate::storage::start_from_args;
use crate::writer::{NodeWriter, InvertedIndexWriter, IntermediateNodeFileReader};
use crate::args::Args;
use crate::intermediate_model::SemNode;
use crate::looks::{
    write_file_entry_tokens,
    inverted_index_entries,
};
use crate::filetree::FileTree;
use crate::buildroot::write_slicemap_tries;
use crate::unparsed_listing::scan_file_listing;


struct SerialStage<'a> {
    args: Args,
    file_nodes: HashMap<String, Node>,
    file_tree: FileTree<'a>,
    node_writer: NodeWriter,
    inverted_index_writer: InvertedIndexWriter,
}


impl<'a> SerialStage<'a> {
    pub fn new(
        args: &Args,
        node_writer: NodeWriter,
        inverted_index_writer: InvertedIndexWriter,
        paths: &'a mut Paths,
    ) -> Self {
        Self {
            args: args.clone(),
            file_nodes: HashMap::new(),
            file_tree: FileTree::new(paths),
            node_writer,
            inverted_index_writer,
        }
    }

    fn update_file_tree(&mut self, node: &SemNode, path: &RelativePath, node_id: NodeID) {
        if node.context.nest_level > 0 { return; }

        if node.kind == NodeKind::SourceFile {
            self.file_tree.add_leaf_path(&path, NodeKind::SourceFile);

            let item = IndexItem {
                key: node.path.clone(),
                href: Some(index_item::Href::NodeId(node.id)),
                kind: IndexItemKind::IiFile.into(),
                path: None,
                r#type: None,
            };
            self.inverted_index_writer.submit_item(item);

            return;
        }

        // update directory tree
        self.file_tree.add_leaf_path(&path, NodeKind::File);

        // add to file's content
        let file_node = self.file_nodes.entry(node.path.clone()).or_insert_with(|| {
            let path_id = self.file_tree.paths.path_to_id(&path);

            let container = path
                .parent()
                .as_ref().map(|p| {
                    let pid = self.file_tree.paths.path_to_id(p);
                    self.file_tree.paths.get_node_for_path(pid)
                        .expect(&format!("missing path_id->node_id map for container of file node {:?}", p))
                });

            const NK: NodeKind = NodeKind::File;
            let n = Node {
                id: self.file_tree.paths.ensure_node_for_path(path_id, NK),
                kind: NK,
                path: node.path.clone(),
                path_id,
                member_of: None,
                container,
                start: Location::zero(),
                text: Vec::new(),
                context: HyperlinkedNodeContext {
                    references: None,
                }
            };

            let item = IndexItem {
                key: node.path.clone(),
                href: Some(index_item::Href::NodeId(n.id)),
                kind: IndexItemKind::IiFile.into(),
                path: None,
                r#type: None,
            };
            self.inverted_index_writer.submit_item(item);

            n
        });
        let file_node_offset = file_node.text.last().map(|last_tok| last_tok.text.len() as u32 + last_tok.offset).unwrap_or(file_node.start.off);
        let file_node_line = file_node.text.last().map(|last_tok| last_tok.line + 1).unwrap_or(file_node.start.line);
        write_file_entry_tokens(node, &mut file_node.text, self.args.max_node_len, file_node_offset, file_node_line, node_id);
    }

    fn add_to_inverted_index(&mut self, node: &SemNode) {
        let entries = inverted_index_entries(node);
        for e in entries {
            self.inverted_index_writer.submit_item(e);
        }
    }

    pub async fn finalize(mut self) {
        self.file_tree.write_dir_nodes(
            &mut self.node_writer,
            Some(&mut self.inverted_index_writer));

        for node in self.file_nodes.into_values() {
            info!("file node: {} {:?}", node.id, node.path);
            let mut blob = HNBlob::new();
            blob.nodes.push(node);
            self.node_writer.submit_blob(blob);
        }

        self.inverted_index_writer.join().await;

        let writer_stats = self.node_writer.join();

        info!("translation unit done, wrote {} nodes", writer_stats.total_written);
        info!("total PB bytes written:    {}", writer_stats.pb_bytes_count);
    }
}

pub async fn serial_stage(args: &Args) {
    let stores = sqlite::new_from_args::<SqliteGSMReader, SqliteUMQuery>(args);
    serial_stage_with_stores(args, stores).await;
}


pub async fn serial_stage_with_stores(args: &Args, mut stores: SqliteServices<SqliteGSMReader, SqliteUMQuery>) {
    let (storage_done, storage_channel) = start_from_args(args).await;
    let node_writer = NodeWriter::start(
        &args,
        args.writer_concurrency,
        storage_channel.clone(),
        stores.output_map.clone());
    let inverted_index_writer = InvertedIndexWriter::start(&args, storage_channel.clone());
    let mut stage = SerialStage::new(args, node_writer, inverted_index_writer, &mut stores.paths);

    let mut node_file_reader = IntermediateNodeFileReader::new_with_slice(args, 1);

    for sem_node in &mut node_file_reader.flat_map(|f| f.nodes.into_iter()) {
        stage.update_file_tree(&sem_node, &sem_node.context.path, sem_node.id);
        stage.add_to_inverted_index(&sem_node);
    }

    scan_file_listing(&mut stage.file_tree, &mut stage.node_writer, &mut stage.inverted_index_writer, &args.repo);

    stage.finalize().await;

    // need to wait for the original InvertedIndexWriter to fill the output map
    let mut node_writer = NodeWriter::start(
        &args,
        args.writer_concurrency,
        storage_channel.clone(),
        stores.output_map.clone());
    write_slicemap_tries(
        &args.repo_id, &args.build_id, args.compression,
        &mut node_writer, &stores.output_map, &stores.paths, storage_channel
    ).await;
    node_writer.join();

    info!("delete expired rows");
    stores.delete_expired();

    storage_done.await.unwrap();
}
