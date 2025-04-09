use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::{rc::Rc, hash::Hasher};
use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use territory_core::{
    GBlob, GNode, Location, NodeID, Offset, PathID, Ref, Refs, RelativePath, SymID, TokenLocation
};

use cscanner::ast::Sem;

use crate::writer::NodeWriter;


#[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
pub enum DirChildType {
    SourceFile,
    Directory,
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct DirChild {
    pub text: String,
    pub path_id: PathID,
    pub node_id: NodeID,
    pub type_: DirChildType,
}

macro_rules! unsafe_cenum_serializer {
    ($name:ident, $x:ty) => {

        mod $name {
            use serde::{Deserialize, Serialize};
            type T = $x;

            #[allow(dead_code)]
            pub fn serialize<S>(value: &T, serializer: S) -> Result<S::Ok, S::Error> where S: serde::ser::Serializer {
                to_int(value).serialize(serializer)
            }

            #[allow(dead_code)]
            pub fn deserialize<'de, D>(deserializer: D) -> Result<T, D::Error> where D: serde::de::Deserializer<'de>, T: Sized {
                let i = i32::deserialize(deserializer)?;
                Ok(from_int(i))
            }

            pub fn from_int(i: i32) -> T {
                unsafe { std::mem::transmute(i) }
            }

            pub fn to_int(value: &T) -> i32 {
                unsafe { std::mem::transmute(*value) }
            }
        }

    };
}

unsafe_cenum_serializer!(unsafe_entitykind_serializer, clang::EntityKind);
unsafe_cenum_serializer!(unsafe_linkage_serializer, clang::Linkage);


#[derive(Debug, Serialize, Deserialize)]
pub struct SemTokenContext {
    // token_kind: clang::token::TokenKind,
    #[serde(rename="l")]
    pub loc: Location,
    #[serde(rename="s")]
    pub sem: Option<Sem>,
    #[serde(rename="d")]
    pub local_definition: Option<TokenLocation>,
    #[serde(rename="e")]
    pub elided: Option<TokenLocation>,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct SemNodeContext {
    #[serde(rename="k")]
    pub kind: cscanner::ast::ClangCurKind,
    #[serde(rename="dn")]
    pub display_name: Option<String>,
    #[serde(rename="s")]
    pub sem: Option<Sem>,
    #[serde(rename="ap")]
    pub path: RelativePath,
    #[serde(rename="pi")]
    pub path_id: PathID,
    #[serde(rename="n")]
    pub nest_level: usize,
    #[serde(rename="E")]
    pub end_offset: Offset,
}


pub type SemNode = GNode<SemNodeContext, SemTokenContext>;
pub type SemFile = GBlob<SemNodeContext, SemTokenContext>;


pub trait GlobalSymbolMapWriter {
    fn insert(&mut self, usr: String, address: TokenLocation);
}
pub trait GlobalSymbolMapReader {
    fn get(&self, usr: &str) -> Option<TokenLocation>;
    fn get_sym_id(&self, usr: &str) -> Option<SymID>;
}


pub trait UsesMap {
    fn record_use(&mut self, loc: TokenLocation, use_: Ref);
}
pub trait UsesMapQuery {
    fn has_uses(&self, loc: &TokenLocation) -> bool;
    fn write(self, nw: &mut NodeWriter);
}


type UsesHashmap = HashMap<TokenLocation, Refs>;

impl UsesMap for UsesHashmap {
    fn record_use(&mut self, loc: TokenLocation, use_: Ref) {
        let refs = self.entry(loc).or_insert_with(|| Refs::new(loc));
        refs.refs.insert(use_);
    }
}


impl UsesMapQuery for UsesHashmap {
    fn has_uses(&self, loc: &TokenLocation) -> bool {
        self.contains_key(&loc)
    }

    fn write(self, nw: &mut NodeWriter) {
        for refs in self.into_values() {
            nw.submit_refs(refs);
        }
    }
}


#[derive(Default)]
pub struct NodeHashToSemMap {
    map: HashMap<u64, Rc<Sem>>,
}


pub struct LocalSpanIndex<'a> {
    nest_levels: Vec<Vec<(PathID, Offset, Offset, NodeID)>>,
    forward: Option<&'a sqlite::SpanStore>,
}

impl<'a> LocalSpanIndex<'a> {
    pub fn from(nest_levels: Vec<Vec<(PathID, Offset, Offset, NodeID)>>) -> Self {
        Self { nest_levels, forward: None }
    }

    pub fn preload(
        forward: &'a sqlite::SpanStore,
        source_set: &Vec<PathID>,
    ) -> Self {
        let mut nest_levels = Vec::new();
        for (path_id, nest_level, start, end, node_id) in forward.get_paths(source_set) {
            loop {
                let Some(level) = nest_levels.get_mut(nest_level) else {
                    nest_levels.push(Vec::new());
                    continue;
                };

                level.push((path_id, start, end, node_id));

                break;
            }
        }

        for nest_level in &mut nest_levels {
            nest_level.sort();
        }

        Self { nest_levels, forward: Some(forward) }
    }

    pub fn set_forward_to(&mut self, store: &'a sqlite::SpanStore) {
        self.forward = Some(store);
    }

    pub fn get(&self, path_id: PathID, offset: territory_core::Offset) -> Option<NodeID> {
        for nest_level in self.nest_levels.iter().rev() {
            let result = nest_level.binary_search_by_key(
                &(path_id, offset),
                |(path_id, start, _, _)| (*path_id, *start));

            let candidate = match result {
                Ok(i) => nest_level.get(i),
                Err(i) => nest_level.get(i.saturating_sub(1)),
            };

            if let Some((candidate_path_id, _, end, node_id)) = candidate {
                if *candidate_path_id == path_id
                   && *end > offset
                {
                    return Some(*node_id);
                }
            }
        }

        if let Some(forward) = self.forward {
            return forward.get(path_id, offset);
        }

        None
    }
}



pub mod sqlite {
    use std::{cell::RefCell, collections::HashMap, error::Error, path::{Path, PathBuf}, sync::{Arc, Mutex}, time::Duration};

    use itertools::Itertools;
    use ring::digest::Digest;
    use rusqlite::{Connection, OptionalExtension};

    use territory_core::{
        BlobID,
        BlobSliceLoc,
        Location,
        NodeID,
        NodeKind,
        Offset,
        PathID,
        Ref,
        Refs,
        RelativePath,
        SymID,
        db::init_db,
    };

    use crate::{args::{get_debug_cfg, Args}, writer::ReferencesBlob};
    use super::TokenLocation;

    pub trait SqliteGSM {
        fn new_with_connection(conn: Arc<Mutex<Connection>>) -> Self;
    }
    pub struct SqliteGSMWriter {
        conn: Arc<Mutex<Connection>>,
    }
    #[derive(Clone)]
    pub struct SqliteGSMReader {
        conn: Arc<Mutex<Connection>>,
    }
    impl SqliteGSM for SqliteGSMReader {
        fn new_with_connection(conn: Arc<Mutex<Connection>>) -> Self { Self { conn } }
    }
    impl SqliteGSM for SqliteGSMWriter {
        fn new_with_connection(conn: Arc<Mutex<Connection>>) -> Self { Self { conn } }
    }


    impl SqliteGSMWriter {
        fn create_table(conn: &Connection) {
            conn.execute("
                 create table if not exists sym (
                    sym_id integer primary key autoincrement,
                    usr string unique,
                    node_id integer,
                    offset integer
                )
            ", ()).unwrap();
        }
    }

    impl super::GlobalSymbolMapWriter for SqliteGSMWriter {
        fn insert(&mut self, usr: String, address: TokenLocation) {
            let conn = self.conn.lock().unwrap();
            // TODO: multiple instances instead of ignore?
            let mut stmt = conn.prepare_cached("
                insert into sym (usr, node_id, offset)
                values (?1, ?2, ?3)
                on conflict do update set node_id=?2, offset=?3
            ").unwrap();

            stmt.execute((usr.as_str(), address.node_id as i64, address.offset as i64)).unwrap();
        }
    }

    impl super::GlobalSymbolMapReader for SqliteGSMReader {
        fn get(&self, usr: &str) -> Option<TokenLocation> {
            let conn = self.conn.lock().unwrap();
            let mut get_by_usr_stmt = conn.prepare("select node_id, offset from sym where usr=?1").unwrap();
            let mut res = get_by_usr_stmt.query_map([usr], |row| {
                Ok(TokenLocation {
                    node_id: row.get::<_, i64>(0).unwrap() as u64,
                    offset: row.get(1).unwrap(),
                })
            }).unwrap();

            res.next().map(|r| r.unwrap())
        }

        fn get_sym_id(&self, usr: &str) -> Option<SymID> {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("select sym_id from sym where usr=?1").unwrap();
            stmt.query_row(
                    (usr,),
                    |row| Ok(row.get(0).unwrap()))
                .optional()
                .unwrap()
                .map(SymID)
        }

    }

    pub trait SqliteUM {
        fn new_with_connection(conn: Arc<Mutex<Connection>>) -> Self;
    }
    #[derive(Clone)]
    pub struct SqliteUMWriter {
        conn: Arc<Mutex<Connection>>,
    }
    pub struct SqliteUMQuery {
        conn: Arc<Mutex<Connection>>,
    }
    impl SqliteUM for SqliteUMWriter {
        fn new_with_connection(conn: Arc<Mutex<Connection>>) -> Self { Self { conn } }
    }
    impl SqliteUM for SqliteUMQuery {
        fn new_with_connection(conn: Arc<Mutex<Connection>>) -> Self { Self { conn } }
    }
    impl SqliteUMQuery {
        pub fn get_file_uses(&self, path_id: PathID) -> Vec<Refs> {
            let conn = self.conn.lock().unwrap();
            let mut get_by_usr_stmt = conn.prepare("
                select
                    spans.path_id
                    node_id,
                    offset,
                    href,
                    context,
                    use_location_line,
                    use_location_col,
                    use_location_off,
                    linked_via_sym,
                    path
                from use, spans, paths
                where spans.path_id = ?1
                    and use.node_id = spans.location_id
                    and paths.path_is = use.use_path_id
                order by node_id, offset
            ").unwrap();

            let mut file_refs = Vec::new();
            let mut refs = None;
            let _ = get_by_usr_stmt.query_map([path_id.0], |row| {
                let row_token_location = TokenLocation {
                    node_id: row.get::<_, i64>(0).unwrap() as u64,
                    offset: row.get(1).unwrap()
                };
                let new_ref = Ref {
                    href: row.get::<_, i64>(2).unwrap() as u64,
                    context: row.get::<_, String>(3).unwrap().into(),
                    use_location: Location {
                        line: row.get(4).unwrap(),
                        col: row.get(5).unwrap(),
                        off: row.get(6).unwrap(),
                    },
                    linked_via_sym: row.get(7).unwrap(),
                    use_path: row.get(8).unwrap(),
                };
                match &mut refs {
                    Some(Refs { token_location, refs, .. }) if *token_location == row_token_location => {
                        refs.insert(new_ref);
                    },
                    _ => {
                        let mut new_refs = Refs::new(row_token_location);
                        new_refs.refs.insert(new_ref);
                        let prev_refs = std::mem::replace(&mut refs, Some(new_refs));
                        if let Some(r) = prev_refs {
                            file_refs.push(r);
                        }
                    }
                }

                Ok(())
            }).unwrap();

            if let Some(r) = refs {
                file_refs.push(r);
            }

            file_refs
        }
    }

    impl SqliteUMWriter {
        fn create_table(conn: &Connection) {
            conn.execute("
                drop table if exists use
            ", ()).unwrap();
            conn.execute("
                 create table use (
                    node_id integer,
                    offset integer,
                    href integer,
                    context string,
                    use_location_line integer,
                    use_location_col integer,
                    use_location_off integer,
                    linked_via_sym bool,
                    use_path_id integer,
                    PRIMARY KEY (node_id, offset, href, use_location_off)
                ) without rowid
            ", ()).unwrap();
            let _ = conn.execute("
                alter table use add column use_path_id integer
            ", ());
        }
    }

    impl super::UsesMap for SqliteUMWriter {
        fn record_use(&mut self, loc: TokenLocation, use_: territory_core::Ref) {
            if get_debug_cfg().print_references {
                println!("reference {:?} --> {:?}", loc, use_);
            }

            let conn = self.conn.lock().unwrap();
            conn.execute("insert or ignore into use (
                node_id,
                offset,
                href,
                context,
                use_location_line,
                use_location_col,
                use_location_off,
                linked_via_sym,
                use_path_id
            ) values (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                (select path_id from paths where path = ?9)
            )", (
                loc.node_id as i64,
                loc.offset,
                use_.href as i64,
                use_.context.to_string(),
                use_.use_location.line,
                use_.use_location.col,
                use_.use_location.off,
                use_.linked_via_sym,
                use_.use_path.to_string(),
            )).unwrap();
        }
    }

    impl super::UsesMapQuery for SqliteUMQuery {
        fn has_uses(&self, token_location: &TokenLocation) -> bool {
            let conn = self.conn.lock().unwrap();
            let mut existence_query = conn.prepare("
                select 1 from use where node_id=?1 and offset=?2 limit 1
            ").unwrap();
            let res = existence_query.query_row((
                token_location.node_id as i64,
                token_location.offset,
            ), |_| Ok(()));
            res.is_ok()
        }

        fn write(self, nw: &mut crate::writer::NodeWriter) {
            let conn = self.conn.lock().unwrap();
            let mut get_by_usr_stmt = conn.prepare("
                select
                    spans.node_id,
                    offset,
                    href,
                    context,
                    use_location_line,
                    use_location_col,
                    use_location_off,
                    linked_via_sym,
                    spans.path_id,
                    path
                from use, spans, paths
                where use.node_id = spans.node_id and paths.path_id = use_path_id
                order by spans.path_id, spans.node_id, offset
            ").unwrap();

            let res = get_by_usr_stmt.query_map(
                    [],
                    |row| {
                        let row_token_location = TokenLocation {
                            node_id: row.get::<_, i64>(0).unwrap() as u64,
                            offset: row.get(1).unwrap()
                        };
                        let path_id: PathID = PathID(row.get(8).unwrap());
                        let new_ref = Ref {
                            href: row.get::<_, i64>(2).unwrap() as u64,
                            context: row.get::<_, String>(3).unwrap().into(),
                            use_location: Location {
                                line: row.get(4).unwrap(),
                                col: row.get(5).unwrap(),
                                off: row.get(6).unwrap(),
                            },
                            linked_via_sym: row.get(7).unwrap(),
                            use_path: row.get(9).unwrap(),
                        };

                        Ok((path_id, row_token_location, new_ref))
                    })
                .unwrap()
                .map(|res| res.unwrap())
                .group_by(|(path_id, _, _)| *path_id);

            let res = res.into_iter()
                .map(|(_path_id, group)| {
                    let file_references = group
                        .group_by(|(_, row_token_location, _)| *row_token_location)
                        .into_iter()
                        .map(|(token_location, group)| Refs {
                            token_location,
                            refs: group.into_iter().map(|(_, _, r)| r).collect(),
                        })
                        .collect();
                    ReferencesBlob(file_references)
                });

            for refs_file in res {
                nw.submit_refs_file(refs_file);
            }

        }
    }

    #[derive(Clone)]
    pub struct SpanStore {
        conn: Arc<Mutex<Connection>>,
    }

    // like SpanIndex<LocationID> but persistent
    impl SpanStore {
        fn create_table(conn: &Connection) {
            conn.execute("
                 create table if not exists spans (
                    node_id integer primary key autoincrement,
                    path_id integer,
                    node_kind integer,
                    start integer,
                    end integer,
                    fresh bool)
            ", ()).unwrap();
            let _ = conn.execute("
                alter table spans add column nest_level integer default 0
            ", ());
            conn.execute("
                create unique index if not exists spans_location
                on spans (path_id, node_kind, start)
            ", ()).unwrap();

            conn.execute("
                update spans set fresh=false
            ", ()).unwrap();
        }

        pub fn store_one<'a>(
            &mut self,
            path_id: PathID,
            node_kind: NodeKind,
            start: Offset,
            end: Offset,
            nest_level: usize,
        ) -> Result<NodeID, Box<dyn Error>> {
            let conn = self.conn.lock().unwrap();
            let PathID(pid) = path_id;

            let mut stmt = conn.prepare_cached("
                insert into spans (path_id, node_kind, start, end, nest_level, fresh)
                values (?1, ?2, ?3, ?4, ?5, true)
                on conflict
                do update set fresh=true, end=?4
                returning node_id
            ")?;

            let id = stmt.query_row(
                (pid, node_kind, start, end, nest_level),
                |r| Ok(r.get::<_, i64>(0)? as u64))?;

            Ok(id)
        }

        pub fn get<'a>(&'a self, path_id: PathID, offset: territory_core::Offset) -> Option<NodeID> {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare_cached("
                select node_id
                from spans
                where
                    path_id = ?1
                    and start <= ?2
                    and end > ?2
                    and fresh = true
                order by nest_level desc
                limit 1
            ").unwrap();

            let PathID(path_id_int) = path_id;
            stmt.query_row(
                    (path_id_int, offset),
                    |row| Ok(row.get::<_, i64>(0).unwrap() as u64))
                .optional()
                .unwrap()
        }

        pub fn get_at_nest_level<'a>(
            &'a self,
            path_id: PathID,
            offset: territory_core::Offset,
            nest_level: u32
        ) -> Option<NodeID> {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare_cached("
                select node_id
                from spans
                where
                    path_id = ?1
                    and start <= ?2
                    and end > ?2
                    and fresh = true
                    and nest_level  = ?3
                limit 1
            ").unwrap();

            let PathID(path_id_int) = path_id;
            stmt.query_row(
                    (path_id_int, offset, nest_level),
                    |row| Ok(row.get::<_, i64>(0).unwrap() as u64))
                .optional()
                .unwrap()
        }

        pub fn get_paths(&self, path_ids: &Vec<PathID>) -> Vec<(PathID, usize, Offset, Offset, NodeID)> {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare_cached("
                select path_id, nest_level, start, end, node_id
                from spans
                where
                    path_id = ?1
                    and fresh = true
            ").unwrap();

            let mut result = Vec::new();

            for path_id in path_ids {
                let PathID(path_id_int) = path_id;
                let path_result = stmt.query_map(
                    (path_id_int,),
                    |row| Ok((
                        PathID(row.get::<_, i32>(0)? as u32),
                        row.get::<_, usize>(1)?,
                        row.get::<_, i64>(2)? as u32,
                        row.get::<_, i64>(3)? as u32,
                        row.get::<_, i64>(4)? as u64,
                    ))).unwrap();

                result.extend(path_result.into_iter().map(|r| r.unwrap()));
            }

            result
        }
    }

    pub struct Paths {
        conn: Arc<Mutex<Connection>>,
        path_to_path_id_cache: RefCell<HashMap<RelativePath, PathID>>,
        node_for_path_cache: RefCell<HashMap<PathID, NodeID>>,
    }

    impl Paths {
        fn new(conn: &Arc<Mutex<Connection>>) -> Self {
            Self {
                conn: Arc::clone(conn),
                path_to_path_id_cache: RefCell::new(HashMap::new()),
                node_for_path_cache: RefCell::new(HashMap::new())
            }
        }

        fn create_table(_conn: &Connection) {
            // created by territory_core::db::init_db
        }

        pub fn path_to_id(&self, path: &RelativePath) -> PathID {
            let mut cache = self.path_to_path_id_cache.borrow_mut();
            if let Some(path_id) = cache.get(path) {
                return *path_id;
            }

            let conn = self.conn.lock().unwrap();
            let path_str = path.to_string();

            let mut select_query = conn.prepare_cached("select path_id from paths where path=?1").unwrap();

            let res = conn.execute("
                insert or ignore into paths ( path )
                values (?1)
            ", (&path_str,)).unwrap();

            if res == 0 {
                select_query.query_row(
                        (&path_str,),
                        |row| Ok(PathID(row.get(0).unwrap())))
                    .unwrap()
            } else {
                let path_id = PathID(conn.last_insert_rowid().try_into().expect("PathID out of range"));
                cache.insert(path.clone(), path_id);
                path_id
            }
        }

        pub fn get(&self, path: &RelativePath) -> Option<PathID> {
            let mut cache = self.path_to_path_id_cache.borrow_mut();
            if let Some(path_id) = cache.get(path) {
                return Some(*path_id);
            }

            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("
                select path_id
                from paths
                where path=?1
            ").unwrap();

            let path_id = stmt.query_row(
                    (path.to_string(),),
                    |row| Ok(PathID(row.get(0).unwrap())))
                .optional()
                .unwrap()?;

            cache.insert(path.clone(), path_id);
            Some(path_id)
        }

        pub fn get_by_id(&self, id: PathID) -> Option<RelativePath> {
            let PathID(path_id) = id;
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("
                select path
                from paths
                where path_id=?1
            ").unwrap();

            stmt.query_row(
                    (path_id,),
                    |row| Ok(row.get(0).unwrap()))
                .optional()
                .unwrap()
        }

        pub fn get_node_for_path(&self, id: PathID) -> Option<NodeID> {
            let mut cache = self.node_for_path_cache.borrow_mut();
            if let Some(node_id) = cache.get(&id) {
                return Some(*node_id);
            }

            let conn = self.conn.lock().unwrap();
            let PathID(pid) = id;
            let mut stmt = conn.prepare("
                select node_id
                from path_nodes
                where path_id = ?1
            ").unwrap();
            let node_id = stmt.query_row(
                    (pid,),
                    |row| Ok(row.get(0).unwrap()))
                .optional()
                .unwrap()?;

            cache.insert(id, node_id);
            Some(node_id)
        }

        pub fn ensure_node_for_path(&self, id: PathID, node_kind: NodeKind) -> NodeID{
            let PathID(path_id) = id;
            let node_id = self.get_node_for_path(id).unwrap_or_else(|| {
                let conn = self.conn.lock().unwrap();
                conn.execute("
                        insert into spans (path_id, node_kind, start, end, nest_level, fresh)
                        values (?1, ?2, 0, 0, 0, true)
                        on conflict
                        do update set fresh=true, end=0
                    ", (path_id, node_kind))
                    .unwrap();

                let node_id = conn.last_insert_rowid() as u64;

                conn.execute("
                    insert into path_nodes ( path_id, node_id )
                    values (?1, ?2)
                ", (path_id, node_id)).unwrap();

                node_id
            });

            let mut cache = self.node_for_path_cache.borrow_mut();
            cache.insert(id, node_id);
            node_id
        }
    }

    pub struct Queue {
        conn: Arc<Mutex<Connection>>,
    }

    impl Queue {
        fn create_table(conn: &Connection) {
            conn.execute("
                drop table if exists queue
            ", ()).unwrap();
            conn.execute("
                 create table queue (
                    path string,
                    args string,
                    i integer primary key
                ) without rowid
            ", ()).unwrap();
        }


        pub fn put(&mut self, idx: usize, file: PathBuf, args: &Vec<String>) {
            let conn = self.conn.lock().unwrap();
            conn
                .execute(
                    "insert into queue (i, path, args) values (?1, ?2, ?3)",
                    (
                        idx,
                        file.to_string_lossy(),
                        serde_json::to_string(args).unwrap()
                    ))
                .unwrap();
        }

        pub fn pop(&mut self) -> rusqlite::Result<Option<(usize, PathBuf, Vec<String>)>> {
            let mut conn = self.conn.lock().unwrap();

            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Exclusive)?;

            let item_opt = tx.query_row(
                    "select i, path, args from queue
                    order by i asc
                    limit 1",
                    (),
                    |row| Ok((
                        row.get::<_, usize>(0)?,
                        row.get::<_, String>(1)?,
                        serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                    )))
                .optional()?
                .map(|(i, s, a)| (i, PathBuf::from(s), a));


            if let Some(item) = &item_opt {
                let del_res = tx.execute("delete from queue where i=?1", (item.0,))?;
                assert_eq!(del_res, 1)
            }

            tx.commit()?;

            Ok(item_opt)
        }
    }


    #[derive(Clone)]
    pub struct OutputMap {
        conn: Arc<Mutex<Connection>>,
    }

    impl OutputMap {
        fn create_table(conn: &Connection) {
            conn.execute("
                 create table if not exists blobs (
                    blob_id integer primary key autoincrement
                )
            ", ()).unwrap();
            conn.execute("
                 create table if not exists slices_by_hash (
                    sha256 blob primary key,
                    blob_id integer,
                    start_offset integer,
                    end_offset integer
                )
            ", ()).unwrap();
        }

        pub fn get_existing_slice_loc_or_insert(
            &self,
            hash: Digest,
            loc: BlobSliceLoc,
        ) -> Option<BlobSliceLoc> {
            let mut conn = self.conn.lock().unwrap();
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Exclusive).unwrap();

            let existing = {
                let mut stmt = tx.prepare("
                    select blob_id, start_offset, end_offset from slices_by_hash where sha256=?1
                ").unwrap();

                stmt.query_row(
                    (hash.as_ref(),),
                    |row| {
                        Ok(BlobSliceLoc {
                            blob_id: row.get(0)?,
                            start_offset: row.get(1)?,
                            end_offset: row.get(2)?,
                        })
                    })
                    .optional()
                    .unwrap()
            };

            if existing.is_some() {
                tx.commit().unwrap();
                return existing;
            }
            tx.execute(
                    "insert into slices_by_hash (sha256, blob_id, start_offset, end_offset) values (?1, ?2, ?3, ?4)",
                    (hash.as_ref(), loc.blob_id, loc.start_offset, loc.end_offset))
                .unwrap();

            tx.commit().unwrap();

            None
        }

        pub fn refresh_node_location_if_exists(
            &self,
            node_id: NodeID,
            hash: Digest,
        ) -> bool {
            let conn = self.conn.lock().unwrap();
            let nrows = conn.execute("
                    update nodemap set fresh=true
                    where node_id = ?1
                        and sha256 = ?2
                ", (node_id, hash.as_ref()))
                .unwrap();

            return nrows == 1
        }

        pub fn store_node_location(&self, node_id: NodeID, loc: &BlobSliceLoc, hash: Digest) {
            let conn = self.conn.lock().unwrap();
            conn.execute("
                insert or replace into nodemap (node_id, blob_id, start_offset, end_offset, sha256, fresh)
                values (?1, ?2, ?3, ?4, ?5, true)
            ", (node_id, loc.blob_id, loc.start_offset, loc.end_offset, hash.as_ref())).unwrap();
        }

        pub fn store_refs_location(&self, token_location: TokenLocation, loc: &BlobSliceLoc) {
            let conn = self.conn.lock().unwrap();
            conn.execute("
                insert or replace into refmap (
                    node_id,
                    token_offset,
                    blob_id,
                    blob_start_offset,
                    blob_end_offset
                ) values (?1, ?2, ?3, ?4, ?5)
            ", (token_location.node_id, token_location.offset, loc.blob_id, loc.start_offset, loc.end_offset)).unwrap();
        }

        pub fn new_blob_id(&self) -> BlobID {
            let conn = self.conn.lock().unwrap();
            conn.execute("
                insert into blobs default values
            ", ()).unwrap();


            BlobID(conn.last_insert_rowid().try_into().expect("BlobID out of range"))
        }

        pub fn node_locations(&self) -> Vec<(NodeID, BlobSliceLoc)> {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("
                select node_id, blob_id, start_offset, end_offset from nodemap where fresh=true
            ").unwrap();
            stmt.query_map(
                (),
                |row| {
                    let node_id: NodeID = row.get(0)?;
                    Ok((node_id, BlobSliceLoc {
                        blob_id: row.get(1)?,
                        start_offset: row.get(2)?,
                        end_offset: row.get(3)?,
                    }))
                })
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        }

        pub fn sym_locations(&self) -> Vec<(SymID, BlobSliceLoc)> {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("
                select sym.sym_id, nodemap.blob_id, nodemap.start_offset, nodemap.end_offset
                from nodemap, sym
                where nodemap.node_id = sym.node_id
                    and nodemap.fresh=true
                order by 1
            ").unwrap();
            stmt.query_map(
                (),
                |row| {
                    let sym_id = SymID(row.get(0)?);
                    Ok((sym_id, BlobSliceLoc {
                        blob_id: row.get(1)?,
                        start_offset: row.get(2)?,
                        end_offset: row.get(3)?,
                    }))
                })
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        }

        pub fn refs_locations(&self) -> Vec<(TokenLocation, BlobSliceLoc)> {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("
                select
                    node_id,
                    token_offset,
                    blob_id,
                    blob_start_offset,
                    blob_end_offset
                from refmap
                order by 1, 2
            ").unwrap();
            stmt.query_map(
                (),
                |row| {
                    let tokl = TokenLocation {
                        node_id: row.get(0)?,
                        offset: row.get(1)?,
                    };
                    Ok((tokl, BlobSliceLoc {
                        blob_id: row.get(2)?,
                        start_offset: row.get(3)?,
                        end_offset: row.get(4)?,
                    }))
                })
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        }

    }


    pub struct SqliteServices<GSM: SqliteGSM, UM: SqliteUM> {
        pub global_symbol_map: GSM,
        pub uses_map: UM,
        pub span_store: SpanStore,
        pub paths: Paths,
        pub queue: Queue,
        pub output_map: OutputMap,
        pub conn: Arc<Mutex<Connection>>,
    }
    impl<GSM: SqliteGSM, UM: SqliteUM> SqliteServices<GSM, UM> {
        pub fn create_tables(&self) {
            let conn = self.conn.lock().unwrap();
            init_db(&conn);
            SqliteGSMWriter::create_table(&conn);
            SqliteUMWriter::create_table(&conn);
            SpanStore::create_table(&conn);
            Paths::create_table(&conn);
            Queue::create_table(&conn);
            OutputMap::create_table(&conn);
        }

        pub fn delete_expired(&self) {
        }
    }

    pub fn new_from_args<GSM: SqliteGSM, UM: SqliteUM>(args: &Args) -> SqliteServices<GSM, UM> {
        std::fs::create_dir_all(&args.intermediate_path).unwrap();
        new(&args.db_path)
    }
    pub fn new<GSM: SqliteGSM, UM: SqliteUM>(db_path: &Path) -> SqliteServices<GSM, UM> {
        let conn = Connection::open(db_path).unwrap();
        new_with_conn(Arc::new(Mutex::new(conn)))
    }
    pub fn new_mem<GSM: SqliteGSM, UM: SqliteUM>() -> SqliteServices<GSM, UM> {
        let conn = Connection::open_in_memory().unwrap();
        let srvs = new_with_conn(Arc::new(Mutex::new(conn)));
        srvs.create_tables();
        srvs
    }
    pub fn new_with_conn<GSM: SqliteGSM, UM: SqliteUM>(conn: Arc<Mutex<Connection>>) -> SqliteServices<GSM, UM> {
        {
            let mconn = conn.lock().unwrap();
            mconn.busy_timeout(Duration::from_secs(60 * 30)).expect("couldn't set sqlite busy timeout");
            mconn.pragma_update(None, "synchronous", "OFF").unwrap();
        }

        SqliteServices {
            global_symbol_map: GSM::new_with_connection(Arc::clone(&conn)),
            uses_map: UM::new_with_connection(Arc::clone(&conn)),
            span_store: SpanStore { conn: Arc::clone(&conn) },
            paths: Paths::new(&conn),
            queue: Queue { conn: Arc::clone(&conn) },
            output_map: OutputMap {  conn: Arc::clone(&conn) },
            conn,
        }
    }

}



impl NodeHashToSemMap {
    pub fn cache(
        &mut self,
        cur: &clang::Entity,
        mksem: &dyn Fn(&clang::Entity) -> Sem,
    ) -> Rc<Sem> {
        let mut hasher = DefaultHasher::new();
        cur.hash(&mut hasher);
        let h = hasher.finish();
        self.map.entry(h)
            .or_insert_with(|| Rc::new(mksem(cur)))
            .to_owned()
    }
}
