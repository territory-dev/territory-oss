use std::array;
use std::path::PathBuf;

use log::info;
use prost::Message;
use ring::digest::{Context, SHA256};

use territory_core::{BlobID, NodeID, SymID, TokenLocation};
use territory_core::slicemap_trie::{encode_to_prefix, Branch, TrieNode};
use territory_core::territory::index::BlobSliceLoc;

use crate::args::{Args, CompressionMode};
use crate::intermediate_model::sqlite::OutputMap;
use crate::storage::StorageChannel;
use crate::writer::apply_compression;

const MAX_LEVELS: usize = 4;
const BITS_IN_LEVELS: [usize; MAX_LEVELS] = [14, 14, 14, 22];
const BIT_OFFSET_IN_LEVELS: [usize; MAX_LEVELS] = [0, 14, 28, 42];


struct SlicemapWriterState<'a> {
    compression_mode: CompressionMode,
    storage: StorageChannel,
    output_map: &'a OutputMap,
    blob_id: BlobID,
    buffer: Vec<u8>,
    levels: [Vec<Branch>; MAX_LEVELS],
}

fn prefix(key: u64, level: usize) -> u64 {
    encode_to_prefix(BITS_IN_LEVELS[level], BIT_OFFSET_IN_LEVELS[level], key)
}

fn flush_node(
    state: &mut SlicemapWriterState,
    top_level: usize
) -> BlobSliceLoc {
    let branches = state.levels[top_level].drain(..).collect();
    let node = TrieNode {
        bit_offset: BIT_OFFSET_IN_LEVELS[top_level].try_into().unwrap(),
        bits: BITS_IN_LEVELS[top_level].try_into().unwrap(),
        branches,
    };

    let mut tbuf = Vec::new();
    node.encode(&mut tbuf).unwrap();

    let mut context = Context::new(&SHA256);
    context.update(&tbuf);
    let hash = context.finish();

    let mut cbuf = apply_compression(state.compression_mode, tbuf);

    let start_offset = state.buffer.len().try_into().unwrap();
    let BlobID(blob_id) = state.blob_id;

    let new_loc = BlobSliceLoc {
        blob_id: blob_id.try_into().unwrap(),
        start_offset,
        end_offset: start_offset.checked_add(cbuf.len().try_into().unwrap()).unwrap(),
    };

    if let Some(previous_slice_loc) = state.output_map.get_existing_slice_loc_or_insert(hash, new_loc) {
        return previous_slice_loc;
    }

    state.buffer.append(&mut cbuf);
    new_loc
}

fn flush_subtree(
    state: &mut SlicemapWriterState,
    prev_key: u64,
    top_level: usize,
) -> Branch {
    if top_level > 0 {
        let child_level = top_level - 1;
        let branch = flush_subtree(state, prev_key, child_level);
        state.levels[top_level].push(branch);
    }

    let prefix_ = prefix(prev_key, top_level+1);
    let location = flush_node(state, top_level);
    Branch {
        is_inner_node: true,
        prefix: prefix_,
        token_offset: None,
        location: Some(location),
    }
}

pub trait Ke {
    fn key(&self) -> u64;
    fn leaf_branch(&self, b: &mut Branch);
}

impl Ke for (NodeID, BlobSliceLoc) {
    fn key(&self) -> u64 {
        self.0
    }

    fn leaf_branch(&self, b: &mut Branch) {
        b.location = Some(self.1);
    }
}

impl Ke for (SymID, BlobSliceLoc) {
    fn key(&self) -> u64 {
        let SymID(id) = self.0;
        id as u64
    }

    fn leaf_branch(&self, b: &mut Branch) {
        b.location = Some(self.1);
    }
}

impl Ke for (TokenLocation, BlobSliceLoc) {
    fn key(&self) -> u64 {
        self.0.node_id
    }

    fn leaf_branch(&self, b: &mut Branch) {
        b.location = Some(self.1);
        b.token_offset = Some(self.0.offset);
    }
}


pub async fn write_slicemap<K: Ke>(
    repo_id: &str,
    compression_mode: CompressionMode,
    items: impl Iterator<Item = K>,
    output_map: &OutputMap,
    storage: StorageChannel,
) -> BlobSliceLoc {
    let levels: [Vec<Branch>; MAX_LEVELS] = array::from_fn(|_| Vec::new());
    let mut state = SlicemapWriterState {
        compression_mode,
        blob_id: output_map.new_blob_id(),
        output_map,
        storage,
        buffer: Vec::new(),
        levels,
    };

    let mut prev_key = 0;

    let mut root_level: usize = 0;

    for ke in items {
        let key = ke.key();
        assert!(key >= prev_key, "items passed to write_index not in key order");
        let key_diff = prev_key ^ key;

        let mut level = 0;
        for l in (0..MAX_LEVELS).rev() {
            let level_diff = prefix(key_diff, l);
            if level_diff != 0 {
                level = l;
                break;
            }
        }

        if level > root_level {
            root_level = level;
        }

        if level > 0 {
            let branch = flush_subtree(&mut state, prev_key, level-1);
            state.levels[level].push(branch);
        }

        let mut leaf_branch = Branch {
            is_inner_node: false,
            prefix: prefix(key, 0),
            ..Branch::default()
        };
        ke.leaf_branch(&mut leaf_branch);
        state.levels[0].push(leaf_branch);

        prev_key = key;
    }

    if root_level > 0 {
        let branch = flush_subtree(&mut state, prev_key, root_level - 1);
        state.levels[root_level].push(branch);
    }
    let root_loc = flush_node(&mut state, root_level);

    info!("generated {} bytes of slice map trie", state.buffer.len());

    let path = PathBuf::from("nodes").join(&repo_id).join("f").join(&state.blob_id.0.to_string());
    state.storage.submit_blob(path, state.buffer).await;

    root_loc
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::str::FromStr;

    use proptest::prelude::*;

    use territory_core::territory::index::BlobSliceLoc;
    use territory_core::slicemap_trie::{SlicemapReader, QueryResult, SharedCache};
    use territory_core::TokenLocation;
    use crate::intermediate_model::sqlite::{SqliteServices, SqliteGSMWriter, SqliteUMWriter};
    use crate::testlib::defaut_args;


    use super::{write_slicemap, BITS_IN_LEVELS};

    fn get_by_number_loop(blobs: &HashMap<PathBuf, Vec<u8>>, reader: &mut SlicemapReader, key: u64) -> BlobSliceLoc {
        let mut res;
        let mut iterations = 0;
        loop {
            iterations += 1;
            assert!( iterations <= BITS_IN_LEVELS.len() + 1 );

            res = reader.get_by_number(key);
            match res {
                QueryResult::Found(found_loc) => {
                    return found_loc;
                }
                QueryResult::NeedNode(loc) => {
                    let path = PathBuf::from_str(&format!("nodes/test_repo/f/{}", loc.blob_id)).unwrap();
                    let bytes = blobs.get(&path)
                        .expect(&format!("path {:?} requested but not generated", path));
                    reader.node_data_available(loc, &bytes[loc.start_offset as usize..loc.end_offset as usize])
                        .unwrap();
                }
                _ => {
                    assert!(false, "unexpected get_by_number result: {:?}", res);
                }
            }
        }
    }

    #[tokio::test]
    async fn get_by_number() {
        let items = vec![
            (1u64, BlobSliceLoc { blob_id: 10, start_offset: 0, end_offset: 1 }),
            (2, BlobSliceLoc { blob_id: 10, start_offset: 1, end_offset: 2 }),
            (3, BlobSliceLoc { blob_id: 10, start_offset: 2, end_offset: 3 }),
            (1000123, BlobSliceLoc { blob_id: 10, start_offset: 3, end_offset: 4 }),
            (92834934, BlobSliceLoc { blob_id: 10, start_offset: 4, end_offset: 5 }),
        ];

        let db: SqliteServices<
            SqliteGSMWriter,
            SqliteUMWriter,
        > = crate::intermediate_model::sqlite::new_mem();
        let output_map = db.output_map;

        let (storage, storage_chan) = crate::storage::MemStorage::start();

        let root_loc = write_slicemap("test_repo", crate::args::CompressionMode::None, items.clone().into_iter(), &output_map, storage_chan).await;

        let mem = storage.get_mem();

        let cache = SharedCache::new(1024);
        let mut ni = SlicemapReader::new(root_loc, SharedCache::new_handle(&cache, "c"));

        for (key, loc) in items {
            let found_loc = get_by_number_loop(&mem, &mut ni, key);
            assert_eq!(found_loc, loc);
        }
    }

    #[tokio::test]
    async fn reuse() {
        let items = vec![
            (1u64, BlobSliceLoc { blob_id: 10, start_offset: 0, end_offset: 1 }),
        ];

        let db: SqliteServices<
            SqliteGSMWriter,
            SqliteUMWriter,
        > = crate::intermediate_model::sqlite::new_mem();
        let output_map = db.output_map;
        let (_storage, storage_chan) = crate::storage::MemStorage::start();

        let root_loc_1 = write_slicemap("test_repo", crate::args::CompressionMode::None, items.clone().into_iter(), &output_map, storage_chan.clone()).await;
        let root_loc_2 = write_slicemap("test_repo", crate::args::CompressionMode::None, items.clone().into_iter(), &output_map, storage_chan).await;
        assert_eq!(root_loc_1, root_loc_2);
    }

    prop_compose! {
        fn blob_slice_loc() (blob_id: u64, start_offset: u64, end_offset: u64) -> BlobSliceLoc {
            BlobSliceLoc { blob_id, start_offset, end_offset }
        }
    }

    prop_compose! {
        fn token_location() (node_id: u64, offset: u32) -> TokenLocation {
            TokenLocation { node_id, offset }
        }
    }

    fn scale_arr_size() -> usize {
        (option_env!("PROPTEST_SCALE").unwrap_or("0.1").parse::<f32>().unwrap() * 1000.0) as usize
    }

    proptest! {
        #[test]
        fn prop_get_by_number(
            items in prop::collection::vec((any::<u64>(), blob_slice_loc()), 0..scale_arr_size())
        ) {
            let db: SqliteServices<
                SqliteGSMWriter,
                SqliteUMWriter,
            > = crate::intermediate_model::sqlite::new_mem();
            let output_map = db.output_map;

            let (storage, storage_chan) = crate::storage::MemStorage::start();

            let mut items_to_add = items.clone();
            items_to_add.sort_by(|(a, _), (b, _)| a.cmp(b));

            let root_loc = {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(
                    write_slicemap("test_repo", crate::args::CompressionMode::None, items_to_add.into_iter(), &output_map, storage_chan)
                )
            };

            let mem = storage.get_mem();

            let cache = SharedCache::new(1024);
            let ni = SlicemapReader::new(root_loc, SharedCache::new_handle(&cache, "c"));

            for (key, loc) in items {
                let mut res;
                let mut iterations = 0;
                loop {
                    iterations += 1;
                    assert!( iterations <= BITS_IN_LEVELS.len() + 1 );

                    res = ni.get_by_number(key);
                    match res {
                        QueryResult::Found(found_loc) => {
                            assert_eq!(found_loc, loc);
                            break;
                        }
                        QueryResult::NeedNode(loc) => {
                            let path = PathBuf::from_str(&format!("nodes/test_repo/f/{}", loc.blob_id)).unwrap();
                            let bytes = mem.get(&path)
                                .expect(&format!("path {:?} requested but not generated", path));
                            ni.node_data_available(loc, &bytes[loc.start_offset as usize..loc.end_offset as usize])
                                .unwrap();
                        }
                        _ => {
                            assert!(false, "unexpected get_by_number result: {:?}", res);
                        }
                    }
                }
            }
        }

        #[test]
        fn prop_get_by_number_with_token_offset(
            items in prop::collection::vec((token_location(), blob_slice_loc()), 0..scale_arr_size())
        ) {
            let db: SqliteServices<
                SqliteGSMWriter,
                SqliteUMWriter,
            > = crate::intermediate_model::sqlite::new_mem();
            let output_map = db.output_map;

            let (storage, storage_chan) = crate::storage::MemStorage::start();

            let mut items_to_add = items.clone();
            items_to_add.sort_by(|(a, _), (b, _)| a.cmp(b));
            let root_loc = {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(
                    write_slicemap("test_repo", crate::args::CompressionMode::None, items_to_add.into_iter(), &output_map, storage_chan)
                )
            };

            let mem = storage.get_mem();

            let cache = SharedCache::new(1024);
            let ni = SlicemapReader::new(root_loc, SharedCache::new_handle(&cache, "c"));

            for (key, loc) in items {
                let mut res;
                let mut iterations = 0;
                loop {
                    iterations += 1;
                    assert!( iterations <= BITS_IN_LEVELS.len() + 1 );

                    res = ni.get_by_number_with_offset(key.node_id, Some(key.offset));
                    match res {
                        QueryResult::Found(found_loc) => {
                            assert_eq!(found_loc, loc);
                            break;
                        }
                        QueryResult::NeedNode(loc) => {
                            let path = PathBuf::from_str(&format!("nodes/test_repo/f/{}", loc.blob_id)).unwrap();
                            let bytes = mem.get(&path)
                                .expect(&format!("path {:?} requested but not generated", path));
                            ni.node_data_available(loc, &bytes[loc.start_offset as usize..loc.end_offset as usize])
                                .unwrap();
                        }
                        _ => {
                            assert!(false, "unexpected get_by_number result: {:?}", res);
                        }
                    }
                }
            }
        }
    }
}
