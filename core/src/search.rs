use std::collections::{BinaryHeap, HashMap};
use std::error::Error;

use prost::DecodeError;
use prost::bytes::Buf;
use serde::{Deserialize, Serialize};
use prost::{Message, EncodeError, bytes::BytesMut};

use crate::{IntoGenHref, GenHref};
use crate::strings_trie::{TrieReader, TrieSymbol, TrieWriter};
use crate::territory::index::{IndexItem, index_item, IndexItemKind};


#[derive(Deserialize, Default)]
pub enum Ranking {
    #[default]
    None,
    Length,
}

#[derive(Deserialize)]
pub struct Options {
    pub limit: Option<usize>,

    #[serde(default)]
    pub ranking: Ranking,
}


impl Default for Options {
    fn default() -> Self {
        Self {
            limit: None,
            ranking: Ranking::default(),
        }
    }
}


#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, ::prost::Oneof)]
pub enum Href {
    #[prost(uint64, tag = "3")]
    DirectNodeLink(u64),

    #[prost(uint64, tag = "4")]
    NodeId(u64),
}

impl IntoGenHref for Href {
    fn into_gen_href(&self) -> crate::GenHref {
        match self {
            Href::DirectNodeLink(id) => GenHref::DirectNodeLink(*id),
            Href::NodeId(id) => GenHref::NodeId(*id),
        }
    }
}

impl IntoGenHref for Option<Href> {
    fn into_gen_href(&self) -> crate::GenHref {
        match self {
            Some(href) => href.into_gen_href(),
            None => panic!("unexpected None in href"),
        }
    }
}

#[derive(Message, Serialize)]
pub struct NormalizedItemEntry {
    #[prost(uint32, tag = "1", optional)]
    pub type_id: Option<u32>,

    #[prost(uint32, tag = "2", optional)]
    pub path_id: Option<u32>,

    #[prost(oneof = "Href", tags = "3, 4")]
    #[serde(with = "crate::ser::gen_href")]
    pub href: ::core::option::Option<Href>,

    #[prost(enumeration = "IndexItemKind", tag = "5")]
    pub kind: i32,
}

#[derive(Message)]
pub struct TrieLocation {
    #[prost(uint64, tag = "1")]
    pub start: u64,

    #[prost(uint64, tag = "2")]
    pub end: u64,
}

#[derive(Message)]
pub struct TrieIndexHeader {
    #[prost(uint64, tag = "1")]
    pub normalized_item_entries_count: u64,

    #[prost(uint64, tag = "2")]
    pub keys_trie_len: u64,

    #[prost(uint64, tag = "3")]
    pub types_trie_len: u64,

    #[prost(uint64, tag = "4")]
    pub paths_trie_len: u64,
}


#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ExpandedItemEntry {
    pub key: String,
    #[serde(rename="type")]
    pub ty: Option<String>,
    pub path: Option<String>,
    pub kind: IndexItemKind,
    #[serde(with = "crate::ser::gen_href")]
    pub href: Href,
}


#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct SearchResult {
    pub score: i64,
    #[serde(flatten)]
    pub item: ExpandedItemEntry,
    pub positions: Vec<usize>,
}


pub struct TrieIndex {
    pub keys_data: Vec<u8>,
    pub types_data: Vec<u8>,
    pub paths_data: Vec<u8>,
    pub entries: Vec<NormalizedItemEntry>,
    pub types: Vec<String>,
    pub paths: Vec<String>,
}


impl TrieIndex {
    pub fn load(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut bm = BytesMut::from(buf);
        let header = TrieIndexHeader::decode_length_delimited(&mut bm)?;

        let mut entries = Vec::with_capacity(header.normalized_item_entries_count.try_into().unwrap());
        for _ in 0..header.normalized_item_entries_count {
            let item = NormalizedItemEntry::decode_length_delimited(&mut bm)?;
            entries.push(item);
        }

        let tl = header.keys_trie_len.try_into().unwrap();
        let keys_data = bm.chunk()[0..tl].to_owned();
        bm.advance(tl);

        let tl = header.types_trie_len.try_into().unwrap();
        let types_data = bm.chunk()[0..tl].to_owned();
        bm.advance(tl);

        let tl = header.paths_trie_len.try_into().unwrap();
        let paths_data = bm.chunk()[0..tl].to_owned();
        bm.advance(tl);

        let types = TrieReader::new(&types_data).items().map(|(ty, _)| ty).collect();
        let paths = TrieReader::new(&paths_data).items().map(|(path, _)| path).collect();
        Ok(TrieIndex {
            keys_data,
            types_data,
            paths_data,
            entries,
            types,
            paths,
        })
    }

    pub fn dump(&self, buf: &mut BytesMut)  -> Result<(), Box <dyn Error>> {
        let header = TrieIndexHeader {
            normalized_item_entries_count: self.entries.len().try_into()?,
            keys_trie_len: self.keys_data.len().try_into()?,
            types_trie_len: self.types_data.len().try_into()?,
            paths_trie_len: self.paths_data.len().try_into()?,
        };
        header.encode_length_delimited(buf).unwrap();

        self.normalized_entries_proto(buf)?;

        buf.extend_from_slice(&self.keys_data);
        buf.extend_from_slice(&self.types_data);
        buf.extend_from_slice(&self.paths_data);

        Ok(())
    }

    pub fn normalized_entries_proto<'py>(&self, buf: &mut BytesMut) -> Result<(), EncodeError>{
        for item in &self.entries {
            item.encode_length_delimited(buf)?;
        }
        Ok(())
    }

    pub fn from_index_items(items: &mut Vec<IndexItem>) -> Self {
        let mut keys_writer = TrieWriter::new();
        let mut paths_writer = TrieWriter::new();
        let mut types_writer = TrieWriter::new();

        let mut entries = Vec::new();

        let mut path_ids: HashMap<String, u32> = HashMap::new();
        let mut paths: Vec<String> = items.iter().filter_map(|it| Some(it.path.clone()?)).collect();
        paths.sort();
        paths.dedup();
        for (i, path) in paths.iter().enumerate() {
            path_ids.insert(path.clone(), i.try_into().expect("path number out of range"));
            paths_writer.push(&path, 0);
        }

        let mut type_ids: HashMap<String, u32> = HashMap::new();
        let mut types: Vec<String> = items.iter().filter_map(|it| Some(it.r#type.clone()?)).collect();
        types.sort();
        types.dedup();
        for (i, ty) in types.iter().enumerate() {
            type_ids.insert(ty.clone(), i.try_into().expect("type number out of range"));
            types_writer.push(&ty, 0);
        }

        items.sort_by(|l, r| l.key.cmp(&r.key));
        items.dedup();
        for item in items {
            let href = match item.href {
                Some(index_item::Href::NodeId(id)) => Some(Href::NodeId(id)),
                Some(index_item::Href::DirectNodeLink(id)) => Some(Href::DirectNodeLink(id)),
                _ => {
                    println!("Index item has {:?} as href. Only NodeId hrefs are supported by from_index_items", item.href);
                    continue;
                }
            };
            keys_writer.push(&item.key, 0);
            entries.push(NormalizedItemEntry{
                path_id: item.path.as_ref().and_then(|p| path_ids.get(p).copied()),
                type_id: item.r#type.as_ref().and_then(|t| type_ids.get(t).copied()),
                href,
                kind: IndexItemKind::IiSymbol.into()
            });
        }

        Self {
            keys_data: keys_writer.data(),
            paths_data: paths_writer.data(),
            types_data: types_writer.data(),
            entries,
            types,
            paths,
        }
    }

    pub fn search(&self, raw_query: &str, options: &Options) -> Vec<SearchResult> {
        let query: &[u8] = raw_query.as_ref();
        let mut r = TrieReader::new(&self.keys_data);
        let mut top_results = BinaryHeap::new();

        let mut key = String::new();
        let mut matches = Vec::new();
        let mut idx = 0;
        loop {
            match r.read_symbol() {
                Err(e) => { panic!("trie read error: {:?}", e); },
                Ok(TrieSymbol::EOF) => { break; },
                Ok(TrieSymbol::ASCIIChar(c)) => {
                    let c = c as char;
                    if (matches.len() < query.len()) &&
                        (c.to_ascii_lowercase() == (query[matches.len()] as char).to_ascii_lowercase())
                    {
                        matches.push(key.len());
                    }
                    key.push(c);
                }
                Ok(TrieSymbol::Backspace(shift)) => {
                    for _ in 0..shift {
                        key.pop();
                        if let Some(lp) = matches.last() {
                            if *lp == key.len() { matches.pop(); }
                        }
                    }
                }
                Ok(TrieSymbol::Leaf(_)) => {
                    if matches.len() == query.len() {
                        let mut positions = matches.clone();
                        let mut score = i64::MAX / key.len() as i64;

                        if let Some(cont_match) = key.find(raw_query) {
                            positions.clear();
                            positions.extend(cont_match..cont_match+query.len());
                        } else {
                            score /= 2;
                        }

                        let rev = (-score, key.clone(), idx, positions);
                        top_results.push(rev);

                        if let Some(limit) = options.limit {
                            if top_results.len() > limit { top_results.pop(); }
                        }
                    }
                    idx += 1;
                }
            }
        }

        top_results.into_sorted_vec().into_iter()
            .map(|(score, key, idx, positions)| {

                let norm = &self.entries[idx];
                let href= norm.href.to_owned().expect(&format!("missing required href on item: {}", key));
                SearchResult {
                    score: -score,
                    positions,
                    item: ExpandedItemEntry {
                        key,
                        ty: norm.type_id.map(|tyid| self.types[tyid as usize].clone()),
                        path: norm.path_id.map(|pathid| self.paths[pathid as usize].clone()),
                        kind: norm.kind(),
                        href,
                    }
                }
            })
            .collect()
}


}


pub fn search<'a>(index: &'a Vec<IndexItem>, query: &str, options: &Options) -> Vec<&'a IndexItem> {
    let query = query.to_lowercase();
    let mut results: Vec<_> = index
        .iter()
        .filter(|item| item.key.to_lowercase().contains(&query))
        .collect();

    match options.ranking {
        Ranking::None => {}
        Ranking::Length => {
            results.sort_by(|a, b| a.key.len().cmp(&b.key.len()));
        }
    }

    if let Some(limit) = options.limit {
        results.truncate(limit);
    }

    results
}


#[cfg(test)]
mod test {
    use serde_json::to_string_pretty;

    use crate::territory::index::index_item::Href;
    use crate::territory::index::{IndexItem, IndexItemKind::IiSymbol};
    use crate::search::{Options, Ranking, TrieIndex, Href as THref, search};

    fn ii_defaults() -> IndexItem {
        IndexItem {
            key: "".to_owned(),
            href: None,
            kind: IiSymbol.into(),
            path: None,
            r#type: None,
        }
    }

    #[test]
    fn trie_search() {
        let mut index = vec![
            IndexItem { key: "foo".to_owned(), href: Some(Href::NodeId(1)), ..ii_defaults() },
            IndexItem { key: "bar".to_owned(), href: Some(Href::NodeId(2)), ..ii_defaults() },
            IndexItem { key: "oof".to_owned(), href: Some(Href::DirectNodeLink(3)), ..ii_defaults() },
        ];

        let trie = TrieIndex::from_index_items(&mut index);
        let res = trie.search("oo", &Options::default());
        assert_eq!(res.len(), 2);

        assert_eq!(&res[0].item.key, "foo");
        assert_eq!(res[0].item.href, THref::NodeId(1));
        assert_eq!(res[0].positions, vec![1, 2]);

        assert_eq!(&res[1].item.key, "oof");
        assert_eq!(res[1].item.href, THref::DirectNodeLink(3));
        assert_eq!(res[1].positions, vec![0, 1]);
    }

    #[test]
    fn trie_search_case_insensitive() {
        let mut index = vec![
            IndexItem { key: "A".to_owned(), href: Some(Href::NodeId(1)), ..ii_defaults() },
            IndexItem { key: "AA".to_owned(), href: Some(Href::DirectNodeLink(2)), ..ii_defaults() },
            IndexItem { key: "AAA".to_owned(), href: Some(Href::DirectNodeLink(3)), ..ii_defaults() },
        ];

        let trie = TrieIndex::from_index_items(&mut index);
        let res = trie.search("a", &Options { limit: Some(2), ..Options::default() });
        assert_eq!(res.len(), 2);

        assert_eq!(&res[0].item.key, "A");
        assert_eq!(res[0].item.href, THref::NodeId(1));
        assert_eq!(res[0].positions, vec![0]);

        assert_eq!(&res[1].item.key, "AA");
        assert_eq!(res[1].item.href, THref::DirectNodeLink(2));
        assert_eq!(res[1].positions, vec![0]);
    }

    #[test]
    fn trie_search_limit() {
        let mut index = vec![
            IndexItem { key: "a".to_owned(), href: Some(Href::NodeId(1)), ..ii_defaults() },
            IndexItem { key: "aa".to_owned(), href: Some(Href::DirectNodeLink(2)), ..ii_defaults() },
            IndexItem { key: "aaa".to_owned(), href: Some(Href::DirectNodeLink(3)), ..ii_defaults() },
        ];

        let trie = TrieIndex::from_index_items(&mut index);
        let res = trie.search("a", &Options { limit: Some(2), ..Options::default() });
        assert_eq!(res.len(), 2);

        assert_eq!(&res[0].item.key, "a");
        assert_eq!(res[0].item.href, THref::NodeId(1));
        assert_eq!(res[0].positions, vec![0]);

        assert_eq!(&res[1].item.key, "aa");
        assert_eq!(res[1].item.href, THref::DirectNodeLink(2));
        assert_eq!(res[1].positions, vec![0]);
    }


    #[test]
    fn empty_query() {
        let index = vec![
            IndexItem { key: "foo".to_owned(), href: Some(Href::DirectNodeLink(1)), ..ii_defaults()},
            IndexItem { key: "bar".to_owned(), href: Some(Href::DirectNodeLink(2)), ..ii_defaults() },
        ];

        assert_eq!(
            search(&index, "", &Options::default()),
            vec![
                &IndexItem { key: "foo".to_owned(), href: Some(Href::DirectNodeLink(1)), ..ii_defaults() },
                &IndexItem { key: "bar".to_owned(), href: Some(Href::DirectNodeLink(2)), ..ii_defaults() },
            ]
        );
    }

    #[test]
    fn substring() {
        let index = vec![
            IndexItem { key: "foo".to_owned(), href: Some(Href::DirectNodeLink(1)), ..ii_defaults() },
            IndexItem { key: "bar".to_owned(), href: Some(Href::DirectNodeLink(2)), ..ii_defaults() },
            IndexItem { key: "oof".to_owned(), href: Some(Href::DirectNodeLink(3)), ..ii_defaults() },
        ];

        assert_eq!(
            search(&index, "oo", &Options::default()),
            vec![
                &IndexItem { key: "foo".to_owned(), href: Some(Href::DirectNodeLink(1)), ..ii_defaults() },
                &IndexItem { key: "oof".to_owned(), href: Some(Href::DirectNodeLink(3)), ..ii_defaults() },
            ]
        );
    }

    #[test]
    fn limit() {
        let index = vec![
            IndexItem { key: "a".to_owned(), href: Some(Href::DirectNodeLink(1)), ..ii_defaults() },
            IndexItem { key: "aa".to_owned(), href: Some(Href::DirectNodeLink(2)), ..ii_defaults() },
            IndexItem { key: "aaa".to_owned(), href: Some(Href::DirectNodeLink(3)), ..ii_defaults() },
        ];

        assert_eq!(
            search(&index, "a", &Options { limit: Some(2), ..Options::default() }),
            vec![
                &IndexItem { key: "a".to_owned(), href: Some(Href::DirectNodeLink(1)), ..ii_defaults() },
                &IndexItem { key: "aa".to_owned(), href: Some(Href::DirectNodeLink(2)), ..ii_defaults() },
            ]
        );
    }

    #[test]
    fn case_insensitive() {
        let index = vec![
            IndexItem { key: "abcd".to_owned(), href: Some(Href::DirectNodeLink(1)), ..ii_defaults() },
            IndexItem { key: "ABCD".to_owned(), href: Some(Href::DirectNodeLink(2)), ..ii_defaults() },
            IndexItem { key: "aBcD".to_owned(), href: Some(Href::DirectNodeLink(3)), ..ii_defaults() },
        ];

        assert_eq!(
            search(&index, "abcd", &Options::default()),
            vec![
                &IndexItem { key: "abcd".to_owned(), href: Some(Href::DirectNodeLink(1)), ..ii_defaults() },
                &IndexItem { key: "ABCD".to_owned(), href: Some(Href::DirectNodeLink(2)), ..ii_defaults() },
                &IndexItem { key: "aBcD".to_owned(), href: Some(Href::DirectNodeLink(3)), ..ii_defaults() },
            ]
        );

        assert_eq!(
            search(&index, "ABCD", &Options::default()),
            vec![
                &IndexItem { key: "abcd".to_owned(), href: Some(Href::DirectNodeLink(1)), ..ii_defaults() },
                &IndexItem { key: "ABCD".to_owned(), href: Some(Href::DirectNodeLink(2)), ..ii_defaults() },
                &IndexItem { key: "aBcD".to_owned(), href: Some(Href::DirectNodeLink(3)), ..ii_defaults() },
            ]
        );
    }

    #[test]
    fn length_ranking() {
        let index = vec![
            IndexItem { key: "aaaa".to_owned(), href: Some(Href::DirectNodeLink(1)), ..ii_defaults() },
            IndexItem { key: "aa".to_owned(), href: Some(Href::DirectNodeLink(2)), ..ii_defaults() },
            IndexItem { key: "aaa".to_owned(), href: Some(Href::DirectNodeLink(3)), ..ii_defaults() },
        ];

        assert_eq!(
            search(&index, "a", &Options { ranking: Ranking::Length, ..Options::default() }),
            vec![
                &IndexItem { key: "aa".to_owned(), href: Some(Href::DirectNodeLink(2)), ..ii_defaults() },
                &IndexItem { key: "aaa".to_owned(), href: Some(Href::DirectNodeLink(3)), ..ii_defaults() },
                &IndexItem { key: "aaaa".to_owned(), href: Some(Href::DirectNodeLink(1)), ..ii_defaults() },
            ]
        );
    }

    #[test]
    fn result_json() {
        let item = IndexItem {
            key: "aaaa".to_string(),
            href: Some(Href::DirectNodeLink(123456)),
            kind: IiSymbol.into(),
            path: Some("/foo/bar/baz.c".to_string()),
            r#type: Some("int".to_string()),
        };

        let repr = to_string_pretty(&item).unwrap();
        println!("{}", repr);
        assert_eq!(repr, r#"{
  "key": "aaaa",
  "kind": 0,
  "path": "/foo/bar/baz.c",
  "type": "int",
  "href": "id:123456"
}"#);
    }
}
