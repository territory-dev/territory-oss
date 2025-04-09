use std::cmp::Ordering;
use std::collections::{HashSet, HashMap};

use log::info;

use territory_core::territory::index::{IndexItemKind, IndexItem, index_item};
use crate::writer::NodeWriter;
use crate::{intermediate_model::{DirChild, DirChildType, sqlite::Paths}, writer::InvertedIndexWriter};
use territory_core::territory::index::token::Href;
use territory_core::{
    Node,
    Token,
    TokenKind,
    NodeKind,
    Location,
    Offset,
    HyperlinkedNodeContext,
    HyperlinkedTokenContext, RelativePath, HNBlob,
};


pub struct FileTree<'a> {
    pub items: HashMap<RelativePath, HashSet<DirChild>>,
    pub paths: &'a mut Paths,
}


impl<'a> FileTree<'a> {
    pub fn new(paths: &'a mut Paths) -> Self {
        Self {items: HashMap::new(), paths}
    }

    pub fn add_leaf_path(&mut self, path: &RelativePath, leaf_node_kind: NodeKind) -> bool {
        let mut path_id = self.paths.path_to_id(&path);
        let mut text = path.file_name().unwrap().to_string_lossy().to_string();
        let mut type_ = DirChildType::SourceFile;
        let mut node_kind = leaf_node_kind;
        let mut new = false;

        let stripped_path = path.clone();
        for e in stripped_path.parent().unwrap().ancestors() {
            let node_id = self.paths.ensure_node_for_path(path_id, node_kind);

            let slash = if type_ == DirChildType::Directory { "/" } else { "" };
            let dc = DirChild {
                path_id,
                node_id,
                text: format!("{}{}\n", text, slash).into(),
                type_,
            };
            let hs = self.items.entry(e.clone()).or_insert_with(HashSet::new);
            let did_insert = hs.insert(dc);
            new |= (type_ == DirChildType::SourceFile) && did_insert;

            path_id = self.paths.path_to_id(&e);
            text = e.file_name().map(|p| p.to_string_lossy().to_string()).unwrap_or("".to_string());
            type_ = DirChildType::Directory;
            node_kind = NodeKind::Directory;
        }
        self.paths.ensure_node_for_path(path_id, node_kind);

        new
    }

    pub fn write_dir_nodes(
        self,
        node_writer: &mut NodeWriter,
        mut inverted_index_writer: Option<&mut InvertedIndexWriter>,
    ) {
        let dir_nodes: Vec<_> = self.items.into_iter().map(|(rel_path, children)| {
            // write dir nodes
            let dir_id = self.paths.path_to_id(&rel_path);
            let dir_path = format!("{}/", rel_path);
            info!("dir node: {:?} {:?}", dir_id, dir_path);

            let mut children: Vec<DirChild> = children.into_iter().collect();
            children.sort_by(|t1, t2| {
                use DirChildType::Directory;
                match (t1.type_, t2.type_) {
                    (t, u) if t == u => t1.text.cmp(&t2.text),
                    (Directory, _)   => Ordering::Less,
                    (_, Directory)   => Ordering::Greater,
                    (_, _)           => t1.text.cmp(&t2.text),
                }
            });

            const NK: NodeKind = NodeKind::Directory;
            let dir_node = Node {
                id: self.paths.ensure_node_for_path(dir_id, NK),
                container: None,
                kind: NK,
                path: dir_path.clone(),
                path_id: dir_id,
                member_of: None,
                start: Location::zero(),
                text: vec![],
                context: HyperlinkedNodeContext {
                    references: None,
                },
            };

            (rel_path, dir_node, children)
        })
        .collect();

        let mut blob = HNBlob::new();
        for (rel_path, mut dir_node, children) in dir_nodes {
            if let Some(ref mut iiw) = inverted_index_writer  {
                iiw.submit_item(IndexItem {
                    key: dir_node.path.clone(),
                    href: Some(index_item::Href::NodeId(dir_node.id)),
                    kind: IndexItemKind::IiDirectory.into(),
                    path: None,
                    r#type: None,
                });
            }

            let mut off: Offset = 0;
            let mut line = 0;
            let tokens: Vec<Token> = children.iter().map(|c| {
                let child_href = self.paths.get_node_for_path(c.path_id)
                    .expect("missing path_id->node_id association for a file/dir node");
                let t = Token {
                    offset: off,
                    line,
                    text: c.text.clone(),
                    type_: TokenKind::Identifier,
                    context: HyperlinkedTokenContext {
                        href: Some(Href::NodeIdRef(child_href)),
                        sym_id: None,
                        references: territory_core::ReferencesLink::None,
                    },
                };
                off += c.text.len() as u32;
                line += 1;
                t
            })
            .collect();
            dir_node.text = tokens;

            let container_id = rel_path.parent()
                .as_ref().and_then(|p| {
                    let pid = self.paths.path_to_id(p);
                    self.paths.get_node_for_path(pid)
                });
            dir_node.container = container_id;

            blob.nodes.push(dir_node)
        }
        node_writer.submit_blob(blob);
    }
}


#[cfg(test)]
mod test {
    use std::path::PathBuf;
    use territory_core::{AbsolutePath, NodeKind, RelativePath};

    use crate::intermediate_model::sqlite::{SqliteServices, self};

    use super::FileTree;

    #[test]
    fn add_file() {
        let repo_path = PathBuf::from("/src/repo");
        let file_abs_path: AbsolutePath = repo_path.join("a/b/d.c").into();
        let file_rel_path: RelativePath = file_abs_path.to_relative(&repo_path);

        let SqliteServices {
            mut paths, ..
        } = sqlite::new_mem::<sqlite::SqliteGSMReader, sqlite::SqliteUMQuery>();
        let mut ft = FileTree::new(&mut paths);

        ft.add_leaf_path(&file_rel_path, NodeKind::File);

        let mut items = ft.items
            .iter()
            .map(|(k, v)| (k.to_string(), v.iter().map(|dirc| dirc.text.clone()).collect()))
            .collect::<Vec<_>>();
        items.sort();
        assert_eq!(items.as_ref(), vec![
            ("".to_string(), vec!["a/\n".into()]),
            ("a".to_string(), vec!["b/\n".into()]),
            ("a/b".to_string(), vec!["d.c\n".into()]),
        ]);
    }
}
