use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use log::{error, info};

use territory_core::territory::index::{IndexItemKind, IndexItem, index_item};
use territory_core::{
    GToken, HNBlob, HyperlinkedNodeContext, HyperlinkedTokenContext, Location, Node, NodeID, NodeKind, RelativePath, TokenKind
};

use crate::{filetree::FileTree, writer::{InvertedIndexWriter, NodeWriter}};

pub fn scan_file_listing(
    file_tree: &mut FileTree,
    node_writer: &mut NodeWriter,
    inverted_index_writer: &mut InvertedIndexWriter,
    repo_dir: &Path
) {
    let listing_path = repo_dir.join("TERRITORY_FILE_LISTING");

    if !listing_path.exists() {
        info!("no file listing");
        return;
    }

    let Ok(mut f) = File::open(listing_path) else {
        error!("failed to open TERRITORY_FILE_LISTING");
        return;
    };

    let mut buf = String::new();
    let Ok(_) = f.read_to_string(&mut buf) else {
        error!("failed to read TERRITORY_FILE_LISTING");
        return;
    };

    for l in buf.lines() {
        let l = l.trim_start_matches("./");
        let plain_path = PathBuf::from(&l);
        let p = RelativePath::from(plain_path.clone());

        let new = file_tree.add_leaf_path(&p, NodeKind::SourceFile);
        if new {
            info!("writing unparsed file: {p}");
            write_unparsed(file_tree, node_writer, inverted_index_writer, repo_dir, &p, &plain_path);
        }
    }

}

fn write_unparsed(
    file_tree: &mut FileTree,
    node_writer: &mut NodeWriter,
    inverted_index_writer: &mut InvertedIndexWriter,
    repo_dir: &Path,
    path: &RelativePath,
    plain_path: &Path
) {
    let mut text = String::new();
    let p = repo_dir.join(plain_path);

    let Ok(is_binary) = is_binary(&p) else {
        error!("could not read listed file: {path}");
        return;
    };

    if is_binary {
        text.push_str("<BINARY>");
    } else {
        let Ok(mut f) = File::open(p) else {
            error!("could not open listed file: {path}");
            return;
        };
        let Ok(_) = f.read_to_string(&mut text) else {
            error!("could not read listed file: {path}");
            return;
        };
    }

    let mut blob = HNBlob::new();

    let paths = &file_tree.paths;

    let path_id = file_tree.paths.path_to_id(path);

    let dir_path_id = paths.path_to_id(&path.parent().unwrap());
    let container_id = paths.ensure_node_for_path(dir_path_id, NodeKind::Directory);

    let node_id = paths.ensure_node_for_path(path_id, NodeKind::SourceFile);
    let node = Node {
        container: Some(container_id),
        context: HyperlinkedNodeContext {
            references: None,
        },
        id: node_id,
        kind: NodeKind::SourceFile,
        member_of: None,
        path: path.to_string(),
        path_id,
        start: Location::zero(),
        text: vec![
            GToken {
                context: HyperlinkedTokenContext {
                    href: None,
                    sym_id: None,
                    references: territory_core::ReferencesLink::None,
                },
                line: 0,
                offset: 0,
                type_: if is_binary { TokenKind::Comment } else { TokenKind::WS },
                text,
            }
        ],
    };
    blob.nodes.push(node);
    node_writer.submit_blob(blob);

    let item = IndexItem {
        key: path.to_string(),
        href: Some(index_item::Href::NodeId(node_id)),
        kind: IndexItemKind::IiFile.into(),
        path: None,
        r#type: None,
    };
    inverted_index_writer.submit_item(item);
}

fn is_binary(p: &Path) -> Result<bool, std::io::Error> {
    let Ok(result) = std::panic::catch_unwind(|| {
        binaryornot::is_binary(&p)
    }) else {
        error!("panic in is_binary when checking {p:?} (assuming binary)");
        return Ok(false);
    };

    result
}
