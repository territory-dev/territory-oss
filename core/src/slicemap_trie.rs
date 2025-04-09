use std::{cell::Cell, collections::HashMap, error::Error, rc::Rc, sync::{Arc, Mutex, MutexGuard}};

use prost::Message;

use crate::{pb::BlobSliceLoc, Offset};


pub fn encode_to_prefix(bits: usize, offset: usize, number: u64) -> u64 {
    let mask = (1 << bits) - 1;
    (number >> offset) & mask
}

#[derive(Message)]
pub struct Branch {
    #[prost(uint64, tag = "1")]
    pub prefix: u64,

    #[prost(message, tag = "2")]
    pub location: Option<BlobSliceLoc>,

    #[prost(bool, tag = "3")]
    pub is_inner_node: bool,

    #[prost(uint32, optional, tag = "4")]
    pub token_offset: Option<Offset>,
}


#[derive(Message)]
pub struct TrieNode {
    #[prost(uint64, tag = "1")]
    pub bit_offset: u64,

    #[prost(uint64, tag = "2")]
    pub bits: u64,

    #[prost(message, repeated, tag = "3")]
    pub branches: Vec<Branch>,
}

impl TrieNode {
    pub fn find_key_mut<'a>(&'a mut self, number: u64) -> Option<&'a mut Branch> {
        let number_enc =  self.encode_to_nodes_prefix(number);
        let res = self.branches.binary_search_by_key(&number_enc, |k| k.prefix);
        res.ok().map(|i| &mut self.branches[i])
    }

    pub fn find_key<'a>(&'a self, number: u64) -> Option<&'a Branch> {
        let number_enc =  self.encode_to_nodes_prefix(number);
        let res = self.branches.binary_search_by_key(&number_enc, |k| k.prefix);
        res.ok().map(|i| &self.branches[i])
    }

    pub fn find_key_with_offset<'a>(&'a self, number: u64, token_offset: Option<Offset>) -> Option<&'a Branch> {
        let number_enc =  self.encode_to_nodes_prefix(number);
        let i = self.branches.binary_search_by_key(&number_enc, |k| k.prefix).ok()?;
        let found = &self.branches[i];
        if found.is_inner_node { return Some(found); }

        let res = self.branches.binary_search_by_key(
            &(number_enc, token_offset),
            |k| (k.prefix, k.token_offset));
        res.ok().map(|i| &self.branches[i])
    }

    fn encode_to_nodes_prefix(&self, number: u64) -> u64 {
        encode_to_prefix(self.bits as usize, self.bit_offset as usize, number)
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum QueryResult {
    Found(BlobSliceLoc),
    NotFound,
    NeedNode(BlobSliceLoc),
}

#[derive(PartialEq, Eq, Debug)]
pub enum InsertError {
    MissingNode(BlobSliceLoc),
}

/*
pub enum IndexType {
    Nodemap,
    Symmap,
    Refmap,
}

pub struct CacheGroupKey {
    repo_id: String,
    index_type: IndexType,
}*/

type CacheKey = (String, BlobSliceLoc);

type LRUListRef = Rc<LRUListItem>;

struct LRUListItem {
    more_recent: Cell<Option<LRUListRef>>,
    less_recent: Cell<Option<LRUListRef>>,
    key: CacheKey,
    value: TrieNode,
}

impl std::fmt::Debug for LRUListItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let lr = self.less_recent.take();
        let res = f.debug_struct("LRUListItem")
        .field("less_recent", &lr)
        .field("key", &self.key)
        .field("value", &self.value)
        .finish();

        self.less_recent.set(lr);
        res
    }
}


pub struct SharedCache {
    nodes: HashMap<CacheKey, LRUListRef>,
    max_size: usize,
    lru_list_ends: Option<( LRUListRef, LRUListRef )>,
}

#[derive(Clone)]
pub struct CacheHandle {
    cache: Arc<Mutex<SharedCache>>,
    key: String,
}

pub struct CacheAccess<'a> {
    lock: MutexGuard<'a, SharedCache>,
    key: &'a String,
}

impl SharedCache {
    pub fn new(max_size: usize) -> Arc<Mutex<Self>> {
        let s = Self {
            nodes: HashMap::new(),
            max_size,
            lru_list_ends: None,
        };
        Arc::new(Mutex::new(s))
    }

    pub fn new_handle(cache: &Arc<Mutex<Self>>, key: &str) -> CacheHandle {
        CacheHandle {
            cache: Arc::clone(cache),
            key: key.to_string(),
        }
    }

    pub fn count(cache: &Arc<Mutex<Self>>) -> usize {
        let locked = cache.lock().unwrap();
        locked.nodes.len()
    }
}
impl CacheHandle {
    pub fn lock<'a>(&'a self) -> CacheAccess {
        CacheAccess {
            lock: self.cache.lock().unwrap(),
            key: &self.key,
        }
    }
}

// all access to underlying data structures is protected by requiring a global lock on the cache
unsafe impl<'a> Sync for CacheAccess<'a> { }
unsafe impl<'a> Send for CacheAccess<'a> { }
unsafe impl Sync for CacheHandle { }
unsafe impl Send for CacheHandle { }
unsafe impl Sync for SharedCache { }
unsafe impl Send for SharedCache { }

impl<'a> CacheAccess<'a> {
    pub fn access<T>(&mut self, loc: BlobSliceLoc, f: &mut impl FnMut(&TrieNode) -> T) -> Option<T> {
        let cache = &mut self.lock;

        let lru_list_item = {
            let Some(i) = cache.nodes.get(&(self.key.clone(), loc)) else {
                return None;
            };
            Rc::clone(i)
        };

        let Some((freshest, stalest)) = cache.lru_list_ends.take() else {
            unreachable!("lru_list_ends are None but item is present in the hashmap");
        };

        {
            let ff = freshest.more_recent.take();
            assert!(ff.is_none());
            freshest.more_recent.set(ff);
        }

        let mut my_fresher = lru_list_item.more_recent.take();
        let mut my_staler  = lru_list_item.less_recent.take();

        if let Some(item) = &mut my_fresher {
            let prev = item.less_recent.take();
            assert_eq!(prev.unwrap().key, lru_list_item.key);
            item.less_recent.set(my_staler.as_ref().map(|i| Rc::clone(i)));
        }
        if let Some(item) = &mut my_staler {
            let prev = item.more_recent.take();
            assert_eq!(prev.unwrap().key , lru_list_item.key);
            item.more_recent.set(
                my_fresher.as_ref().map(|i| Rc::clone(i))
                .or(Some(Rc::clone(&lru_list_item))));
        }

        match (my_fresher, my_staler) {
            (Some(_), Some(_))  => {
                // taken from middle of the list, least recent end unchanged
                cache.lru_list_ends.replace((Rc::clone(&lru_list_item), stalest));
                lru_list_item.less_recent.set(Some(Rc::clone(&freshest)));
                freshest.more_recent.set(Some(Rc::clone(&lru_list_item)));
            },
            (None, Some(my_staler)) => {
                // taken from the beginning of the list
                cache.lru_list_ends.replace((Rc::clone(&lru_list_item), stalest));
                lru_list_item.less_recent.set(Some(my_staler));
            },
            (Some(my_fresher), None) => {
                // this was the least recent end
                cache.lru_list_ends.replace((Rc::clone(&lru_list_item), my_fresher));
                lru_list_item.less_recent.set(Some(Rc::clone(&freshest)));
                freshest.more_recent.set(Some(Rc::clone(&lru_list_item)));
            },
            (None, None) => {
                // only item
                cache.lru_list_ends.replace((Rc::clone(&lru_list_item), Rc::clone(&lru_list_item)));
                {
                    let l = lru_list_item.more_recent.take();
                    assert!(l.is_none(), "{:?}", l);
                    lru_list_item.more_recent.set(l);
                }
                {
                    let r = lru_list_item.less_recent.take();
                    assert!(r.is_none());
                    lru_list_item.less_recent.set(r);
                }
            }
        }

        assert!(cache.lru_list_ends.as_ref().unwrap().0.key == lru_list_item.key);

        return Some(f(&lru_list_item.value))
    }

    fn release_least_recent(cache: &mut SharedCache) {
        let Some((most_recent, old_least_recent)) = cache.lru_list_ends.take() else {
            return;
        };

        let removed = cache.nodes.remove(&old_least_recent.key);
        assert!(removed.is_some());

        assert!(most_recent.key != old_least_recent.key);

        // let mr_lock = least_recent.more_recent.lock().unwrap();
        match old_least_recent.more_recent.take() {
            Some(new_least_recent) => {
                cache.lru_list_ends = Some((most_recent, Rc::clone(&new_least_recent)));
                let prev = new_least_recent.less_recent.take();
                assert!(prev.unwrap().key == old_least_recent.key);
            },
            None => {
                assert!(most_recent.key == old_least_recent.key);
                cache.lru_list_ends = None;
            }
        }
    }

    pub fn insert(&mut self, loc: BlobSliceLoc, node: TrieNode) -> Result<(), Box<dyn Error>> {
        let cache = &mut self.lock;

        if cache.nodes.len() > cache.max_size {
            Self::release_least_recent(cache);
        }


        let key = (self.key.clone(), loc);
        if cache.nodes.contains_key(&key) {
            return Err(format!("already in the cache: {:?}", key).into());
        }

        let new_most_recent = Rc::new(LRUListItem {
            more_recent: Cell::new(None),
            less_recent: Cell::new(
                cache.lru_list_ends.as_ref().map(|(most_recent, _least_recent)| Rc::clone(most_recent))
            ),
            key: key.clone(),
            value: node,
        });

        cache.lru_list_ends = match cache.lru_list_ends.take() {
            Some((old_most_recent, least_recent)) => {
                let prev = old_most_recent.more_recent.replace(Some(Rc::clone(&new_most_recent)));
                assert!(prev.is_none());

                Some((Rc::clone(&new_most_recent), least_recent))
            },
            None => {
                Some((Rc::clone(&new_most_recent), Rc::clone(&new_most_recent)))
            },
        };

        cache.nodes.insert(key, new_most_recent);

        Ok(())
    }
}

pub struct SlicemapReader {
    root_loc: BlobSliceLoc,
    cache: CacheHandle,
}

impl SlicemapReader {
    pub fn new(root_loc: BlobSliceLoc, cache: CacheHandle) -> Self {
        Self {
            root_loc,
            cache,
        }
    }

    pub fn get_by_number_with_offset(
        &self,
        number: u64,
        token_offset: Option<Offset>,
    ) -> QueryResult {
        let mut node_loc = self.root_loc;

        loop {
            let mut cache = self.cache.lock();


            let found = cache.access(node_loc, &mut |node| {
                if  let Some(k) = node.find_key_with_offset(number, token_offset) {
                    node_loc = k.location.expect("location missing from NumberedKey");
                    if k.is_inner_node {
                        return None
                    } else {
                        return Some(QueryResult::Found(node_loc));
                    }
                }
                return Some(QueryResult::NotFound);
            });
            match found {
                Some(Some(result)) => { return result; }
                Some(None)         => { continue; }
                None               => { return QueryResult::NeedNode(node_loc); }
            };
        }
    }

    pub fn get_by_number(
        &self,
        number: u64,
    ) -> QueryResult {
        self.get_by_number_with_offset(number, None)
    }

    pub fn node_available(&self, loc: BlobSliceLoc, node: TrieNode) -> Result<(), Box<dyn Error>> {
        let mut c = self.cache.lock();
        c.insert(loc, node)
    }

    pub fn node_data_available(&self, loc: BlobSliceLoc, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let node = TrieNode::decode(data)?;
        self.cache.lock().insert(loc, node)?;
        Ok(())
    }
}
