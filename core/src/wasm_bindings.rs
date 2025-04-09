use std::collections::HashMap;
use std::sync::Mutex;

use wasm_bindgen::prelude::*;
use gloo_utils::format::JsValueSerdeExt;
use prost::Message;

use crate::resolver::{ConcreteLocation, NeedData, ResolutionFailure};
use crate::Node;
use crate::search::{Options, search, TrieIndex};
use crate::slicemap_trie::{SlicemapReader, SharedCache};
use crate::territory::index as pb;


#[wasm_bindgen]
pub fn decode_node(raw_value: JsValue) -> Result<JsValue, JsValue> {
    let raw: serde_bytes::ByteBuf = serde_wasm_bindgen::from_value(raw_value)?;
    let proto_node = match pb::Node::decode(&raw[..]) {
        Ok(pn) => pn,
        Err(e) => { return Err(format!("decode error: {:?}", e).into()); }
    };
    let internal_node = Node::from(&proto_node);

    Ok(JsValue::from_serde(&internal_node).unwrap())
}


#[wasm_bindgen]
pub fn decode_references(raw_value: JsValue) -> Result<JsValue, JsValue> {
    let raw: serde_bytes::ByteBuf = serde_wasm_bindgen::from_value(raw_value)?;
    let proto_refs = match pb::References::decode(&raw[..]) {
        Ok(pn) => pn,
        Err(e) => { return Err(format!("decode error: {:?}", e).into()); }
    };
    Ok(serde_wasm_bindgen::to_value(&proto_refs)?)
}

#[wasm_bindgen]
pub fn decode_build(raw_value: JsValue) -> Result<Build, JsValue> {
    let raw: serde_bytes::ByteBuf = serde_wasm_bindgen::from_value(raw_value)?;
    let build = match pb::Build::decode(&raw[..]) {
        Ok(pn) => pn,
        Err(e) => { return Err(format!("decode error: {:?}", e).into()); }
    };

    Ok(Build { data: build })
}

#[wasm_bindgen]
pub struct Build {
    data: pb::Build,
}

#[wasm_bindgen]
impl Build {
    pub fn resolver(&self, max_mem: usize) -> Resolver {
        let cache = SharedCache::new(max_mem);
        Resolver {
            pending_fetches: Mutex::new(HashMap::new()),
            inner: Box::new(crate::resolver::TrieResolver::new(
                crate::resolver::BasicResolver,
                SlicemapReader::new(self.data.nodemap_trie_root.unwrap(), SharedCache::new_handle(&cache, "nodemap")),
                SlicemapReader::new(self.data.symmap_trie_root.unwrap(), SharedCache::new_handle(&cache, "symmap")),
                SlicemapReader::new(self.data.references_trie_root.unwrap(), SharedCache::new_handle(&cache, "refmap")),
                self.data.repo_root_node_id,
            )),
        }
    }
}

enum SearchIndexImpl {
    ItemList(Vec<pb::IndexItem>),
    Trie(TrieIndex),
}

#[wasm_bindgen]
pub struct SearchIndex {
    im: SearchIndexImpl,
}

#[wasm_bindgen]
impl SearchIndex {
    pub fn search(&self, query: JsValue, options: JsValue) -> Result<JsValue, JsValue> {
        let query_str: String = serde_wasm_bindgen::from_value(query)?;
        let options: Options = serde_wasm_bindgen::from_value(options)?;
        match &self.im {
            SearchIndexImpl::ItemList(data) => {
                let results = search(data, &query_str, &options);
                return Ok(JsValue::from_serde(&results).unwrap());
            },
            SearchIndexImpl::Trie(t) => {
                let results = t.search(&query_str, &options);
                return Ok(JsValue::from_serde(&results).unwrap());
            }
        }
    }
}

#[wasm_bindgen]
pub struct SearchIndexLoader {
    raw: prost::bytes::BytesMut,
    items: Vec<pb::IndexItem>,
    total_size: usize,
}

#[wasm_bindgen]
impl SearchIndexLoader {
    pub fn process(&mut self, count: usize) -> Result<bool, JsValue> {
        if self.raw.is_empty() {
            Ok(true)
        } else {
            for _ in 0..count {
                if self.raw.is_empty() { return Ok(true); }
                let item = pb::IndexItem::decode_length_delimited(&mut self.raw).map_err(|e| format!("decode error: {:?}", e))?;
                self.items.push(item);
            }
            Ok(false)

        }
    }

    pub fn get_result(self) -> Result<SearchIndex, JsValue> {
        Ok(SearchIndex { im: SearchIndexImpl::ItemList(self.items) })
    }

    pub fn get_progress(&self) -> f32 {
        1f32 - (self.raw.len() as f32 / self.total_size as f32)
    }
}

#[wasm_bindgen]
pub fn decode_index(raw_value: JsValue) -> Result<SearchIndexLoader, JsValue> {
    let raw: serde_bytes::ByteBuf = serde_wasm_bindgen::from_value(raw_value)?;
    let prost_raw = prost::bytes::BytesMut::from(&raw[..]);
    let total_size = prost_raw.len();

    let loader = SearchIndexLoader {
        raw: prost_raw,
        items: Vec::new(),
        total_size,
    };

    Ok(loader)
}


#[wasm_bindgen]
pub fn decode_trie_index(raw_value: JsValue) -> Result<SearchIndex, JsValue> {
    let raw: serde_bytes::ByteBuf = serde_wasm_bindgen::from_value(raw_value)?;
    let ti = TrieIndex::load(&raw)
        .map_err(|e| format!("decode error: {:?}", e))?;
    let si = SearchIndex { im: SearchIndexImpl::Trie(ti) };
    Ok(si)
}


#[wasm_bindgen]
pub struct Resolver {
    inner: Box<dyn crate::resolver::Resolver>,
    pending_fetches: Mutex<HashMap<ConcreteLocation, (js_sys::Promise, NeedData)>>,
}


#[wasm_bindgen]
impl Resolver {
    pub async fn resolve_url(&self, url: &str, more: &js_sys::Function) -> Result<JsValue, JsValue> {
        loop {
            let result = self.inner.resolve_url(url);
            match result {
                Ok(loc) => {
                    return Ok(JsValue::from_serde(&loc).unwrap());
                },
                Err(ResolutionFailure::NeedData(nd)) => {
                    let loc = nd.0.clone();

                    let more_prom: js_sys::Promise  = {
                        let mut pf = self.pending_fetches.lock().unwrap();
                        if let Some((prom, _nd)) = pf.get(&loc) {
                            prom.clone()
                        } else {
                            let prom: js_sys::Promise = more.call1(&JsValue::null(), &JsValue::from_serde(&loc).unwrap())?.try_into().unwrap();
                            pf.insert(loc.clone(), (prom.clone(), nd));
                            prom
                        }
                    };

                    let more_js = wasm_bindgen_futures::JsFuture::from(more_prom).await?;
                    let more_bytes: serde_bytes::ByteBuf = serde_wasm_bindgen::from_value(more_js)?;
                    {
                        let mut pf = self.pending_fetches.lock().unwrap();
                        if let Some((_prom, NeedData(_loc, cont))) = pf.remove(&loc) {
                            cont(&more_bytes)
                                .map_err(|e| format!("index decode error: {:?}", e))?;
                        }
                    }
                },
                Err(e) => {
                    return Err(format!("resolve error: {:?}", e).into());
                }
            }
        }
    }
}


