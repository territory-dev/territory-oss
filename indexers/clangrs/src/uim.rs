use std::io::{Read, stdout};
use std::fs::File;
use std::path::{Path, PathBuf};

use territory_core::{pblib, AbsolutePath, NodeID, NodeKind, PathID, Ref, RelativePath, TokenLocation};
use territory_core::territory::index::{self as pb, UniHref};

use crate::args::{get_debug_cfg, Args};
use crate::filetree::FileTree;
use crate::intermediate_model::{UsesMap, UsesMapQuery};
use crate::{buildroot, storage};
use crate::intermediate_model::sqlite::{self, Paths, SpanStore};
use crate::writer::{append_pb_node, InvertedIndexWriter, NodeWriter};


fn to_relpath(args: &Args, mut p: &str) -> RelativePath {
    if let Some(prefix) = &args.remove_path_prefix {
        p = p.strip_prefix(prefix).unwrap_or(p);
    }
    let pb = PathBuf::from(p);
    if pb.is_absolute() {
        let abspath = AbsolutePath::from(pb);
        abspath.to_relative(&args.repo)
    } else {
        RelativePath::from(pb)
    }
}


pub async fn index_uim(args: Args, uim_dir: &Path) {
    if !args.repo.is_absolute() {
        panic!("absolute repo path required");
    }
    let nodes_uim_path = uim_dir.join("nodes.uim");
    let search_uim_path = uim_dir.join("search.uim");

    std::fs::create_dir_all(&args.intermediate_path).unwrap();
    std::fs::create_dir_all(&args.outdir).unwrap();
    std::fs::create_dir_all(&uim_dir).unwrap();

    let mut store = sqlite::new_from_args::<sqlite::SqliteGSMWriter, sqlite::SqliteUMWriter>(&args);
    store.create_tables();

    let mut buf = Vec::new();
    File::open(nodes_uim_path).unwrap().read_to_end(&mut buf).unwrap();

    let mut blob_id = None;
    let mut out = Vec::new();
    let mut file_tree = FileTree::new(&mut store.paths);
    let mut index_pass_out = Vec::new();

    let (storage_done, storage_channel) = storage::start_from_args(&args).await;
    let mut inverted_index_writer = InvertedIndexWriter::start(&args, storage_channel.clone());

    // preprocessing and IDs
    pblib::decode_loop(&buf, &mut |mut n: pb::Node, _, _| {
        // println!("n.path: {}", n.path);
        let relpath = to_relpath(&args, &n.path);
        n.path = relpath.to_string();
        let path_id = file_tree.paths.path_to_id(&relpath);
        n.path_id = path_id.into();
        let off = n.start.as_ref().unwrap().offset;
        let kind_ = n.kind().into();

        if n.container.is_none() {
            n.container = match kind_ {
                NodeKind::Class | NodeKind::Definition => {
                    let file_path_id = file_tree.paths.path_to_id(&relpath);
                    Some(file_tree.paths.ensure_node_for_path(file_path_id, NodeKind::SourceFile))
                },
                NodeKind::SourceFile | NodeKind::Directory => {
                    relpath.parent().map(|p| {
                        let dir_path_id = file_tree.paths.path_to_id(&p);
                        file_tree.paths.ensure_node_for_path(dir_path_id, NodeKind::Directory)
                    })
                },
                _ => None,
            };
        }

        if kind_ == NodeKind::Directory || kind_ == NodeKind::File || kind_ == NodeKind::SourceFile {
            n.id = file_tree.paths.ensure_node_for_path(path_id, kind_);

            let item = pb::IndexItem {
                key: n.path.clone(),
                href: Some(pb::index_item::Href::NodeId(n.id)),
                kind: pb::IndexItemKind::IiFile.into(),
                path: None,
                r#type: None,
            };
            inverted_index_writer.submit_item(item);
        } else {
            n.id = store.span_store.store_one(
                path_id,
                territory_core::NodeKind::Definition,
                off,
                (n.text.len() + off as usize) as u32,
                n.uim_nest_level.unwrap_or(1).try_into().unwrap(),
            ).unwrap();
        }
        file_tree.add_leaf_path(&relpath, kind_);

        index_pass_out.push(n);
    }).unwrap();

    // linking stage
    for n in &mut index_pass_out {
        let mut refctx = "".to_string();
        if let Some(uim_refctx) = &n.uim_reference_context {
            refctx = uim_refctx.to_owned();
        }

        let node_nest_level = n.uim_nest_level.unwrap_or(1);
        if node_nest_level > 1 {
            let start = n.start.as_ref().expect(&format!("missing start offset for node {:?}", n));
            n.container = store.span_store.get_at_nest_level(PathID(n.path_id), start.offset, node_nest_level-1);
        }

        for tok in &mut n.tokens.iter_mut() {
            // TODO: increment line, col
            if let Some(pb::token::Href::UniHref(href)) = &tok.href {
                let offset = href.offset;
                match resolve_uni_href(&args, &file_tree.paths, &store.span_store, &href) {
                    Ok(node_id) => {
                        tok.href = Some(pb::token::Href::NodeIdWithOffsetRef( pb::NodeIdWithOffsetHref { node_id, offset }));

                        if let Some(uim_location) = &tok.uim_location {
                            if !tok.uim_elided.unwrap_or(false) && node_id != n.id {
                                store.uses_map.record_use(
                                    TokenLocation { node_id, offset },
                                    Ref {
                                        href: n.id,
                                        context: refctx.to_owned(),
                                        use_location: uim_location.clone().into(),
                                        use_path: to_relpath(&args, &n.path),
                                        linked_via_sym: false,
                                    });
                            }
                        }
                    }
                    Err(e) => {
                        tok.href = None;
                        let node_line = if let Some(start_loc) = &n.start {
                            start_loc.line.to_string()
                        } else {
                            "???".to_string()
                        };
                        println!(
                            "failed to resolve href for token at {} in node {} at {}:{} {}",
                            tok.offset, n.id, n.path, node_line, e);
                    }
                }
            }
            tok.uim_location = None;
        }
        n.uim_reference_context = None;
        n.uim_nest_level = None;
    }

    {
        let mut node_writer = NodeWriter::start(
            &args,
            args.writer_concurrency,
            storage_channel.clone(),
            store.output_map.clone());

        crate::unparsed_listing::scan_file_listing(
            &mut file_tree, &mut node_writer, &mut inverted_index_writer, &args.repo);

        file_tree.write_dir_nodes(&mut node_writer, Some(&mut inverted_index_writer));

        node_writer.join();
    }

    drop(store);
    let store = sqlite::new_from_args::<sqlite::SqliteGSMWriter, sqlite::SqliteUMQuery>(&args);

    // uses stage
    let mut node_writer = NodeWriter::start(
        &args,
        args.writer_concurrency,
        storage_channel.clone(),
        store.output_map.clone());
    for n in &mut index_pass_out {
        for tok in &mut n.tokens.iter_mut() {
            let loc = TokenLocation { node_id: n.id, offset: tok.offset + n.start.as_ref().unwrap().offset };
            tok.has_references = store.uses_map.has_uses(&loc);
        }
    }
    store.uses_map.write(&mut node_writer);
    node_writer.join();

    // output stage
    for n in index_pass_out {
        if get_debug_cfg().print_blob_writes {
            let mut l = stdout().lock();
            territory_core::pretty_print::node(&mut l, &n).unwrap();
        }
        append_pb_node(&mut blob_id, &mut out, n, &store.output_map, args.compression);
    }


    let mut node_writer = NodeWriter::start(
        &args,
        args.writer_concurrency,
        storage_channel.clone(),
        store.output_map.clone());

    buildroot::write_slicemap_tries(
        &args.repo_id, &args.build_id, args.compression,
        &mut node_writer, &store.output_map, &store.paths,
        storage_channel.clone()
    ).await;
    match blob_id {
        Some(new_blob_id) => {
            let blob_path = PathBuf::from("nodes")
                .join(&args.repo_id)
                .join("f")
                .join(&new_blob_id.0.to_string());
            storage_channel.submit_blob(blob_path, out).await;
        }
        None => {
            println!("no new blobs written");
        }
    }

    process_uim_search_index(
        &args, &store.paths, &store.span_store, &search_uim_path, &mut inverted_index_writer).await;
    inverted_index_writer.join().await;

    drop(storage_channel);
    node_writer.join();
    storage_done.await.unwrap();

}


async fn process_uim_search_index(
    args: &Args,
    paths: &Paths,
    span_store: &SpanStore,
    search_uim_path: &Path,
    iiwriter: &mut InvertedIndexWriter
) {
    let mut buf = Vec::new();
    File::open(search_uim_path).unwrap().read_to_end(&mut buf).unwrap();

    pblib::decode_loop(&buf, &mut |mut ii: pb::IndexItem, _, _| {
        ii.path = ii.path.map(|p| to_relpath(args, &p).to_string());
        if let Some(pb::index_item::Href::UniHref(href)) = ii.href {
            match resolve_uni_href(args, paths, span_store, &href) {
                Ok(res) => {
                    ii.href = Some(pb::index_item::Href::NodeId(res));
                }
                Err(e) => {
                    println!(
                        "failed to resolve href for index key {}: {}",
                        ii.key, e);
                    return;
                }
            };
        }
        iiwriter.submit_item(ii);
    }).unwrap();
}


fn resolve_uni_href(
    args: &Args,
    paths: &Paths,
    span_store: &SpanStore,
    href: &UniHref
) -> Result<NodeID, String> {
    let UniHref { path, offset } = href;
    let relpath = to_relpath(args, path);

    let Some(path_id) = paths.get(&relpath) else {
        return Err(format!("missing path ID for {path}"));
    };
    let Some(node_id) = span_store.get(path_id, *offset) else {
        return Ok(paths.ensure_node_for_path(path_id, NodeKind::SourceFile));
    };
    return Ok(node_id);
}
