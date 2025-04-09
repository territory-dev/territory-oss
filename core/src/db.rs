use rusqlite::{Connection, OptionalExtension};

use crate::pb::BlobSliceLoc;
use crate::{NodeID, SymID, TokenLocation};


// TODO: move to indexer?
pub fn init_db(conn: &Connection) {
    conn.execute("
        create table if not exists nodemap (
            node_id integer primary key,
            blob_id integer,
            start_offset integer,
            end_offset integer,
            sha256 blob,
            fresh bool
        )
    ", ()).unwrap();
    conn.execute("
        delete from nodemap where fresh=false;
    ", ()).unwrap();
    conn.execute("
        update nodemap set fresh=false;
    ", ()).unwrap();

    conn.execute("
        create table if not exists refmap (
            node_id integer,
            token_offset integer,
            blob_id integer,
            blob_start_offset integer,
            blob_end_offset integer,
            primary key (node_id, token_offset)
        )
    ", ()).unwrap();

    conn.execute("
        create table if not exists paths (
            path_id integer primary key,
            path string unique)
    ", ()).unwrap();

    conn.execute("
        create table if not exists path_nodes (
            path_id integer primary key,
            node_id integer
        )
    ", ()).unwrap();

    conn.execute("
        vacuum
    ", ()).unwrap();
}


pub fn get_node_location(conn: &Connection, node_id: NodeID) -> Option<BlobSliceLoc> {
    let mut get_by_usr_stmt = conn.prepare("
        select blob_id, start_offset, end_offset from nodemap where node_id=?1 and fresh=true
    ").unwrap();
    let mut res = get_by_usr_stmt.query_map([node_id], |row| {
        Ok(BlobSliceLoc {
            blob_id: row.get(0).unwrap(),
            start_offset: row.get(1).unwrap(),
            end_offset: row.get(2).unwrap(),
        })
    }).unwrap();

    res.next().map(|r| r.unwrap())
}


pub fn get_sym_location(conn: &Connection, sym_id: SymID) -> Option<BlobSliceLoc> {
    let mut get_by_usr_stmt = conn.prepare("
        select blob_id, start_offset, end_offset
        from nodemap, sym
        where nodemap.node_id=sym.node_id
            and sym.sym_id = ?1
    ").unwrap();
    let mut res = get_by_usr_stmt.query_map([sym_id.0], |row| {
        Ok(BlobSliceLoc {
            blob_id: row.get(0).unwrap(),
            start_offset: row.get(1).unwrap(),
            end_offset: row.get(2).unwrap(),
        })
    }).unwrap();

    res.next().map(|r| r.unwrap())
}


pub fn get_refs_location(conn: &Connection, token_location: &TokenLocation) -> Option<BlobSliceLoc> {
    let mut get_by_usr_stmt = conn.prepare("
        select blob_id, blob_start_offset, blob_end_offset
        from refmap
        where node_id=?1 and token_offset=?2
    ").unwrap();
    let mut res = get_by_usr_stmt.query_map([token_location.node_id, token_location.offset.into()], |row| {
        Ok(BlobSliceLoc {
            blob_id: row.get(0).unwrap(),
            start_offset: row.get(1).unwrap(),
            end_offset: row.get(2).unwrap(),
        })
    }).unwrap();

    res.next().map(|r| r.unwrap())
}


pub fn get_node_for_path(conn: &Connection, path: &String) -> Option<NodeID> {
    let mut stmt = conn.prepare("
        select node_id
        from path_nodes
            join paths on path_nodes.path_id = paths.path_id
        where paths.path = ?1
    ").unwrap();
    stmt.query_row(
            (path.as_str(),),
            |row| Ok(row.get(0).unwrap()))
        .optional()
        .unwrap()
}
