use log::info;

use territory_core::territory::index::Build;
use territory_core::RelativePath;

use crate::args::CompressionMode;
use crate::intermediate_model::sqlite::{Paths, OutputMap};
use crate::slicemap_trie_writer;
use crate::storage::StorageChannel;
use crate::writer::NodeWriter;


pub async fn write_slicemap_tries(
    repo_id: &str,
    build_id: &str,
    compression_mode: CompressionMode,
    node_writer: &mut NodeWriter,
    output_map: &OutputMap,
    paths: &Paths,
    storage_channel: StorageChannel,
) {
    info!("writing nodemap trie");
    let node_locations = output_map.node_locations();
    let nodemap_trie_root = slicemap_trie_writer::write_slicemap(
        repo_id, compression_mode,
        node_locations.into_iter(), &output_map, storage_channel.clone()
    ).await;

    info!("writing sym map trie");
    let sym_locations = output_map.sym_locations().into_iter();
    let symmap_trie_root = slicemap_trie_writer::write_slicemap(
        repo_id, compression_mode,
        sym_locations, &output_map, storage_channel.clone()
    ).await;

    info!("writing refs trie");
    let refs_locations = output_map.refs_locations().into_iter();
    let references_trie_root = slicemap_trie_writer::write_slicemap(
        repo_id, compression_mode,
        refs_locations, &output_map, storage_channel.clone()
    ).await;

    let root_node_id = paths
        .get(&RelativePath::repo_root())
        .and_then(|p| paths.get_node_for_path(p))
        .expect("missing root node ID (no output generated?)");
    let build = Build {
        id: build_id.to_string(),
        nodemap_trie_root: Some(nodemap_trie_root),
        symmap_trie_root: Some(symmap_trie_root),
        references_trie_root: Some(references_trie_root),
        repo_root_node_id: root_node_id,
    };
    info!("created build: {build:?}");
    node_writer.submit_build(build);
}


