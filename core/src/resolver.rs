use std::{collections::BTreeMap, error::Error, fmt::Debug, sync::{Arc, Mutex}};

use prost::DecodeError;
use serde::Serialize;

use crate::{legacy_refs_path, pblib::{decode_loop, decode_many}, ser::{self, node_id}, slicemap_trie::{QueryResult, SlicemapReader}, territory::index::{self as pb, BlobSliceLoc}, GenHref, Node, NodeID, Offset, SymID, TokenLocation};


#[derive(Default, PartialEq, Eq, Debug, Serialize, Clone, Hash)]
pub struct ConcreteLocation {
    pub path: String,
    pub blob_bytes: Option<(u64, u64)>,
    pub token_offset: Option<Offset>,
}

impl From<&BlobSliceLoc> for ConcreteLocation {
    fn from(floc: &BlobSliceLoc) -> Self {
        ConcreteLocation {
            path: format!("f/{}", floc.blob_id),
            blob_bytes: Some((floc.start_offset, floc.end_offset)),
            token_offset: None }
    }
}

pub struct NeedData(pub ConcreteLocation, pub Box<dyn (FnOnce(&[u8]) -> Result<(), Box<dyn Error>>) + Send>);

#[derive(Debug)]
pub enum ResolutionFailure {
    NotFound,
    BadUrl,
    UnsupportedUrl,
    NeedData(NeedData),
    Error(Box<dyn Error>),
}

impl From<Box<dyn Error>> for ResolutionFailure {
    fn from(value: Box<dyn Error>) -> Self {
        Self::Error(value)
    }
}

impl Debug for NeedData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("NeedData({:?}, ...)", self.0))?;
        Ok(())
    }
}


pub type ResolutionResult = Result<ConcreteLocation, ResolutionFailure>;


pub trait Resolver {
    fn resolve_href(&self, href: &GenHref) -> ResolutionResult;

    fn resolve_url(&self, url: &str) -> ResolutionResult {
        let href = ser::gen_href::from_str(url).ok_or(ResolutionFailure::BadUrl)?;
        self.resolve_href(&href)
    }
}

pub struct BasicResolver;

impl Resolver for BasicResolver {
    fn resolve_href(&self, href: &GenHref) -> ResolutionResult {
        match href {
            GenHref::DirectNodeLink(id) => Ok(direct(*id)),
            GenHref::BLoc(floc) => Ok(floc.into()),
            _ => Err(ResolutionFailure::UnsupportedUrl),
        }
    }
}


pub struct SingleBlobResolver {
    node_id_index: BTreeMap<NodeID, BlobSliceLoc>
}

impl SingleBlobResolver {
    pub fn read_blob(data: &[u8]) -> Result<Self, DecodeError> {
        let mut node_id_index = BTreeMap::new();

        decode_loop(data, &mut |item: pb::Node, off, size| {
            node_id_index.insert(item.id, BlobSliceLoc {
                blob_id: 0,
                start_offset: off as u64,
                end_offset: (off + size) as u64,
            });
        })?;

        Ok(SingleBlobResolver { node_id_index })
    }
}

impl Resolver for SingleBlobResolver {
    fn resolve_href(&self, href: &GenHref) -> ResolutionResult {
        match href {
            GenHref::NodeId(id) => {
                let Some(sloc) = self.node_id_index.get(id) else {
                    return Err(ResolutionFailure::NotFound)
                };
                Ok(ConcreteLocation {
                    path: format!("f/{}", sloc.blob_id),
                    blob_bytes: Some((sloc.start_offset, sloc.end_offset)),
                    token_offset: None,
                })
            },
            _ => Err(ResolutionFailure::UnsupportedUrl)
        }
    }
}


#[cfg(feature = "db")]
pub struct DBResolver {
    db_conn: Arc<Mutex<rusqlite::Connection>>,
}

#[cfg(feature = "db")]
impl DBResolver {
    pub fn new(db_conn: Arc<Mutex<rusqlite::Connection>>) -> DBResolver {
        Self { db_conn }
    }
}

#[cfg(feature = "db")]
impl Resolver for DBResolver {
    fn resolve_href(&self, href: &GenHref) -> ResolutionResult {
        match href {
            GenHref::NodeId(id) => {
                let conn = self.db_conn.lock().unwrap();
                if let Some(result) = crate::db::get_node_location(&conn, *id) {
                    Ok((&result).into())
                } else {
                    Ok(direct(*id))
                }
            }
            GenHref::SymId(id) => {
                let conn = self.db_conn.lock().unwrap();
                let result = crate::db::get_sym_location(&conn, *id).ok_or(ResolutionFailure::NotFound)?;
                Ok((&result).into())
            }
            GenHref::DirectNodeLink(id) => Ok(direct(*id)),
            GenHref::BLoc(floc) => Ok(floc.into()),
            GenHref::Path(path) => {
                let conn = self.db_conn.lock().unwrap();
                let id = crate::db::get_node_for_path(&conn, path).ok_or(ResolutionFailure::NotFound)?;
                if let Some(result) = crate::db::get_node_location(&conn, id) {
                    Ok((&result).into())
                } else {
                    Ok(direct(id))
                }
            },
            GenHref::RefsId(token_location) => {
                let conn = self.db_conn.lock().unwrap();
                if let Some(result) = crate::db::get_refs_location(&conn, token_location) {
                    Ok((&result).into())
                } else {
                    Ok(ConcreteLocation { path: legacy_refs_path(token_location), blob_bytes: None, token_offset: None, })
                }
            },
            GenHref::UniHref(_, _) => Err(ResolutionFailure::UnsupportedUrl)
        }
    }
}

#[cfg(feature = "db")]
impl DBResolver {
    pub fn nodes(&self) -> Vec<(NodeID, ConcreteLocation)> {
        let conn = self.db_conn.lock().unwrap();
        let mut stmt = conn.prepare("select node_id from nodemap").unwrap();
        stmt.query_map(
            (),
            |row| {
                let node_id: NodeID = row.get(0)?;
                Ok((node_id, ConcreteLocation::from(&crate::db::get_node_location(&conn, node_id).unwrap())))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    pub fn paths(&self) -> Vec<(String, NodeID)> {
        let conn = self.db_conn.lock().unwrap();
        let mut stmt = conn.prepare("select paths.path, path_nodes.node_id from paths, path_nodes where paths.path_id = path_nodes.path_id").unwrap();
        stmt.query_map(
            (),
            |row| {
                let path: String = row.get(0)?;
                let node_id: NodeID = row.get(1)?;
                Ok((path, node_id))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    pub fn sym(&self) -> Vec<(SymID, String, NodeID)> {
        let conn = self.db_conn.lock().unwrap();
        let mut stmt = conn.prepare("select sym_id, usr, node_id from sym").unwrap();
        stmt.query_map(
            (),
            |row| {
                let sym_id: SymID = SymID(row.get(0)?);
                let usr: String = row.get(1)?;
                let node_id: NodeID = row.get(2)?;
                Ok((sym_id, usr, node_id))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }
}


pub struct TrieResolver<BR: Resolver> {
    backup_resolver: BR,
    nodemap: Arc<SlicemapReader>,
    symmap: Arc<SlicemapReader>,
    refmap: Arc<SlicemapReader>,
    repo_root_node_id: NodeID,
}

impl<BR: Resolver> TrieResolver<BR> {
    pub fn new(
        backup_resolver: BR,
        nodemap: SlicemapReader,
        symmap: SlicemapReader,
        refmap: SlicemapReader,
        repo_root_node_id: NodeID,
    ) -> Self {
        Self {
            backup_resolver,
            nodemap: Arc::new(nodemap),
            symmap: Arc::new(symmap),
            refmap: Arc::new(refmap),
            repo_root_node_id,
        }
    }

    fn query_slicemap(slicemap: Arc<SlicemapReader>, key: u64, token_offset: Option<Offset>) -> ResolutionResult {
        for _ in 0..10 {
            let res = slicemap.get_by_number_with_offset(key, token_offset);
            match res {
                QueryResult::Found(loc) => {
                    return Ok(ConcreteLocation::from(&loc));
                }
                QueryResult::NotFound => {
                    return Err(ResolutionFailure::NotFound);
                }
                QueryResult::NeedNode(loc) => {
                    let cont = Box::new(move |data: &[u8]| {
                        slicemap.node_data_available(loc, data)?;
                        Ok(())
                    });
                    return Err(ResolutionFailure::NeedData(NeedData((&loc).into(), cont)));
                }
            }

        }
        Err(ResolutionFailure::Error(format!("failed to resolve key {key:?} in 10 iterations").into()))
    }
}

impl<BR: Resolver> Resolver for TrieResolver<BR> {
    fn resolve_href(&self, href: &GenHref) -> ResolutionResult {
        match href {
            GenHref::NodeId(id) => {
                Self::query_slicemap(Arc::clone(&self.nodemap), *id, None)
            }
            GenHref::SymId(SymID(id)) => {
                Self::query_slicemap(Arc::clone(&self.symmap), *id as u64, None)
            }
            GenHref::RefsId(TokenLocation { node_id, offset }) => {
                Self::query_slicemap(Arc::clone(&self.refmap), *node_id, Some(*offset))
            }
            GenHref::Path(p) => {
                if p == "" {
                    Self::query_slicemap(Arc::clone(&self.nodemap), self.repo_root_node_id, None)
                } else {
                    self.backup_resolver.resolve_href(href)
                }
            }
            _ => {
                self.backup_resolver.resolve_href(href)
            }
        }
    }
}


fn direct(id: u64) -> ConcreteLocation {
    ConcreteLocation { path: ser::node_id::to_str(id), ..Default::default() }
}


#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{ConcreteLocation, Resolver};

    #[cfg(feature = "db")]
    #[test]
    fn db_resolver_node_id() {
        let db_conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_db(&db_conn);
        db_conn.execute("insert into nodemap (node_id, blob_id, start_offset, end_offset, fresh) values (5,6,7,8,true)", ()).unwrap();

        let r = super::DBResolver{db_conn: Arc::new(Mutex::new(db_conn))};
        let res = r.resolve_url("id:5").unwrap();
        assert_eq!(res, ConcreteLocation { path: "f/6".to_string(), blob_bytes: Some((7, 8)), token_offset: None });
    }

    #[cfg(feature = "db")]
    #[test]
    fn db_resolver_sym_id() {
        let db_conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_db(&db_conn);
        db_conn.execute("insert into nodemap (node_id, blob_id, start_offset, end_offset, fresh) values (5,6,7,8,true)", ()).unwrap();
        db_conn.execute("create table if not exists sym (sym_id integer primary key, node_id integer, offset integer)", ()).unwrap();
        db_conn.execute("insert into sym (sym_id, node_id, offset) values (9, 5, 10)", ()).unwrap();

        let r = super::DBResolver{db_conn: Arc::new(Mutex::new(db_conn))};
        let res = r.resolve_url("sym:9").unwrap();
        assert_eq!(res, ConcreteLocation { path: "f/6".to_string(), blob_bytes: Some((7, 8)), token_offset: None });
    }

    #[cfg(feature = "db")]
    #[test]
    fn db_resolver_path() {
        let db_conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_db(&db_conn);
        db_conn.execute("insert into paths (path, path_id) values (\"foo.c\", 123)", ()).unwrap();
        db_conn.execute("insert into path_nodes (path_id, node_id) values (123, 10)", ()).unwrap();
        db_conn.execute("insert into nodemap (node_id, blob_id, start_offset, end_offset, fresh) values (10,6,7,8,true)", ()).unwrap();
        db_conn.execute("insert into paths (path, path_id) values (\"bar.c\", 234)", ()).unwrap();
        db_conn.execute("insert into path_nodes (path_id, node_id) values (234, 20)", ()).unwrap();
        db_conn.execute("insert into nodemap (node_id, blob_id, start_offset, end_offset, fresh) values (20,6,9,10,true)", ()).unwrap();

        let r = super::DBResolver{db_conn: Arc::new(Mutex::new(db_conn))};
        let res = r.resolve_url("path:foo.c").unwrap();
        assert_eq!(res, ConcreteLocation { path: "f/6".to_string(), blob_bytes: Some((7,8)), token_offset: None });
    }
}
