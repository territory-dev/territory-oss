mod wasm_bindings;
mod ser;
pub mod search;
pub mod token_writer;
pub mod pblib;
pub mod resolver;
pub mod strings_trie;
pub mod slicemap_trie;
pub mod pretty_print;
pub mod node_diff;

#[cfg(feature = "db")]
pub mod db;

pub mod territory {
    pub mod index {
        include!(concat!(env!("OUT_DIR"), "/territory.index.rs"));
    }
}

use regex::Regex;
use territory::index::{self as pb, NodeIdWithOffsetHref, UniHref};
pub use territory::index::BlobSliceLoc;

#[cfg(feature = "db")]
use rusqlite::types::FromSqlError;

use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Serialize, Deserialize};
use serde::ser::SerializeStruct;

pub type NodeID = u64;
pub type Offset = u32;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct PathID(pub u32);

impl Into<u32> for PathID {
    fn into(self) -> u32 {
        let PathID(id) = self;
        id
    }
}
impl Into<u64> for PathID {
    fn into(self) -> u64 {
        let PathID(id) = self;
        id as u64
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct BlobID(pub u64);

#[derive(PartialEq, PartialOrd, Debug, Hash, Copy, Clone)]
pub struct SymID(pub u64);

impl Serialize for SymID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        let SymID(id) = self;
        serializer.serialize_str(&format!("sym:{}", id))
    }
}

impl<'de> Deserialize<'de> for SymID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        let re = Regex::new(r"^sym:([0-9]+)$").unwrap();

        let s = dbg!(String::deserialize(deserializer))?;
        let caps = dbg!(re.captures(&s))
            .ok_or(serde::de::Error::custom(&format!("malformed symbol ID: {}", s)))?;
        let id = caps[1].parse().unwrap();
        Ok(SymID(id))
    }
}


#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AbsolutePath(PathBuf);

impl AbsolutePath {
    pub fn to_relative(&self, repo_path: &Path) -> RelativePath {
        let AbsolutePath(p) = self;
        RelativePath(p.strip_prefix(repo_path).unwrap_or(p).to_path_buf())
    }
    pub fn file_name(&self) -> Option<&std::ffi::OsStr> {
        let Self(path) = self;
        path.file_name()
    }
}

impl From<PathBuf> for AbsolutePath {
    fn from(value: PathBuf) -> Self {
        assert!(value.is_absolute(), "expected path to be absolute: {:?}", value);
        AbsolutePath(value)
    }
}

impl Into<PathBuf> for AbsolutePath {
    fn into(self) -> PathBuf {
        let AbsolutePath(p) = self;
        p
    }
}

impl AsRef<Path> for AbsolutePath {
    fn as_ref(&self) -> &Path {
        let AbsolutePath(p) = self;
        p
    }
}

impl std::fmt::Display for AbsolutePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let AbsolutePath(p) = self;
        write!(f, "{}", p.to_string_lossy())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct RelativePath(PathBuf);

impl RelativePath {
    pub fn repo_root() -> Self {
        Self(PathBuf::new())
    }
    pub fn parent(&self) -> Option<Self> {
        let Self(p) = self;
        p.parent().map(Into::into).map(Self)
    }
    pub fn ancestors(&self) -> impl Iterator<Item=RelativePath> + '_ {
        let Self(p) = self;
        p.ancestors().map(Into::into).map(RelativePath)
    }
    pub fn file_name(&self) -> Option<&std::ffi::OsStr> {
        let Self(path) = self;
        path.file_name()
    }
    pub fn is_in_repo(&self) -> bool {
        let Self(p) = self;
        !p.starts_with("/") && !p.starts_with("..")
    }
}

impl std::fmt::Display for RelativePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let RelativePath(p) = self;
        write!(f, "{}", p.to_string_lossy())?;
        Ok(())
    }
}

impl From<PathBuf> for RelativePath {
    fn from(value: PathBuf) -> Self {
        assert!(!value.is_absolute(), "expected path to be relative: {:?}", value);
        Self(value)
    }
}

#[cfg(feature = "db")]
impl rusqlite::types::FromSql for RelativePath {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value {
            rusqlite::types::ValueRef::Text(bytes) => {
                let text = String::from_utf8_lossy(&bytes[..]);
                Ok(Self(PathBuf::from_str(&text).map_err(|e| FromSqlError::Other(Box::new(e)))?))
            }
            _ => Err(rusqlite::types::FromSqlError::InvalidType)
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Hash, Debug)]
pub enum NodeKind {
    #[serde(rename="definition")]
    Definition,

    #[serde(rename="sourcefile")]
    File,

    #[serde(rename="directory")]
    Directory,

    #[serde(rename="structure")]
    Structure,

    #[serde(rename="fullsourcefile")]
    SourceFile,

    #[serde(rename="class")]
    Class,
}


impl Into<pb::NodeKind> for NodeKind {
    fn into(self: Self) -> pb::NodeKind {
        match self {
            NodeKind::Definition  => pb::NodeKind::Definition,
            NodeKind::File  => pb::NodeKind::File,
            NodeKind::Directory  => pb::NodeKind::Directory,
            NodeKind::Structure  => pb::NodeKind::Structure,
            NodeKind::SourceFile  => pb::NodeKind::SourceFile,
            NodeKind::Class  => pb::NodeKind::Class,
        }
    }
}


impl From<pb::NodeKind> for NodeKind {
    fn from(nk: pb::NodeKind) -> Self {
        match nk {
            pb::NodeKind::Definition => NodeKind::Definition,
            pb::NodeKind::File => NodeKind::File,
            pb::NodeKind::Directory => NodeKind::Directory,
            pb::NodeKind::Structure => NodeKind::Structure,
            pb::NodeKind::SourceFile => NodeKind::SourceFile,
            pb::NodeKind::Class => NodeKind::Class,
        }
    }
}


#[cfg(feature = "db")]
impl rusqlite::types::ToSql for NodeKind {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        use NodeKind::*;
        let kind_int = match self {
            Definition => 0,
            Directory => 1,
            File => 2,
            Structure => 3,
            SourceFile => 4,
            Class => 5,
        };
        Ok(kind_int.into())
    }
}


#[derive(Copy, Clone, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub enum TokenKind {
    WS = 0,
    Keyword = 1,
    Identifier = 2,
    Punctuation = 3,
    Comment = 4,
    Literal = 5,
}

impl From<TokenKind> for pb::TokenType {
    fn from(t: TokenKind) -> pb::TokenType {
        match t {
            TokenKind::WS => pb::TokenType::Ws,
            TokenKind::Comment => pb::TokenType::Comment,
            TokenKind::Identifier => pb::TokenType::Identifier,
            TokenKind::Keyword => pb::TokenType::Keyword,
            TokenKind::Literal => pb::TokenType::Literal,
            TokenKind::Punctuation => pb::TokenType::Punctuation,
        }
    }
}

impl Into<TokenKind> for pb::TokenType {
    fn into(self: pb::TokenType) -> TokenKind {
        match self {
            pb::TokenType::Ws => TokenKind::WS,
            pb::TokenType::Comment => TokenKind::Comment,
            pb::TokenType::Identifier => TokenKind::Identifier,
            pb::TokenType::Keyword => TokenKind::Keyword,
            pb::TokenType::Literal => TokenKind::Literal,
            pb::TokenType::Punctuation => TokenKind::Punctuation,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Debug)]
pub struct Location {
    pub line: u32,
    pub col: u32,
    pub off: Offset,
}

impl Location {
    pub fn zero() -> Location { Self { line: 0, col: 0, off: 0 } }
}

impl From<pb::Location> for Location {
    fn from(l: pb::Location) -> Location {
        Location { line: l.line, col: l.column, off: l.offset }
    }
}

impl Into<pb::Location> for Location {
    fn into(self) -> pb::Location{
        pb::Location {
            line: self.line,
            column: self.col,
            offset: self.off,
        }
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> Ordering {
        self.off.cmp(&other.off)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone)]
pub struct TokenLocation {
    pub node_id: NodeID,
    pub offset: Offset,
}


#[derive(Serialize, Deserialize, PartialEq, Hash, Debug)]
pub struct GNode<C, T> {
    #[serde(with="crate::ser::node_id")]
    pub id: NodeID,

    #[serde(default)]
    #[serde(with="crate::ser::opt_node_id")]
    pub container: Option<NodeID>,

    pub kind: NodeKind,
    pub path: String,
    pub path_id: PathID,
    pub member_of: Option<String>,
    pub start: Location,

    pub text: Vec<GToken<T>>,

    #[serde(flatten)]
    pub context: C,
}

impl<C, T> GNode<C, T> {
    pub fn replace_context<U>(self, context: U) -> GNode<U, T> {
        GNode {
            id: self.id,
            container: self.container,
            kind: self.kind,
            path: self.path,
            path_id: self.path_id,
            member_of: self.member_of,
            start: self.start,
            text: self.text,
            context,
        }
    }

    pub fn replace<U, K>(self, context: U, text: Vec<GToken<K>>) -> GNode<U, K> {
        GNode {
            id: self.id,
            container: self.container,
            kind: self.kind,
            path: self.path,
            path_id: self.path_id,
            member_of: self.member_of,
            start: self.start,
            text,
            context,
        }
    }

    pub fn replace_tokens_with<K>(self, f: &mut impl FnMut(&C, Vec<GToken<T>>) -> Vec<GToken<K>>) -> GNode<C, K> {
        let text = f(&self.context, self.text);
        GNode {
            id: self.id,
            container: self.container,
            kind: self.kind,
            path: self.path,
            path_id: self.path_id,
            member_of: self.member_of,
            start: self.start,
            text,
            context: self.context,
        }
    }

    pub fn map_tokens<K>(self, f: &mut impl FnMut(Offset, &String, &TokenKind, T) -> K) -> GNode<C, K> {
        GNode {
            id: self.id,
            container: self.container,
            kind: self.kind,
            path: self.path,
            path_id: self.path_id,
            member_of: self.member_of,
            start: self.start,
            text: self.text.into_iter().map(|tok| tok.map(f)).collect(),
            context: self.context,
        }
    }
}

#[derive(Serialize, PartialEq, Hash, Debug)]
pub struct HyperlinkedNodeContext {
    pub references: Option<String>,
}

pub type Node = GNode<HyperlinkedNodeContext, HyperlinkedTokenContext>;


impl<'a> Into<pb::Node> for &'a Node {
    fn into(self: &'a Node) -> pb::Node {
        let mut full_text = String::new();
        let mut off = 0;
        let mut real_offset = self.start.off;
        let mut real_line = self.start.line;
        let mut tokens = Vec::new();
        for tok in &self.text {
            full_text.push_str(&tok.text);

            let mut pbtok = pb::Token::default();
            pbtok.offset = off;
            pbtok.sym_id = tok.context.sym_id.map(|SymID(id)| id);
            pbtok.set_type(tok.type_.into());
            pbtok.href = tok.context.href.clone();
            pbtok.has_references = tok.context.references.is_set();
            if tok.offset != real_offset {
                pbtok.real_offset = Some(tok.offset);
                real_offset = tok.offset;
            }
            if tok.line != real_line {
                pbtok.real_line = Some(tok.line);
                real_line = tok.line;
            }
            tokens.push(pbtok);

            let delta_off = tok.text.len() as u32;
            off += delta_off;
            real_offset += delta_off;

            real_line += tok.text.chars().filter(|c| *c == '\n').count() as u32;
        }

        let mut n = pb::Node {
            id: self.id,
            kind: 0,
            path: self.path.to_owned(),
            path_id: self.path_id.0,
            member_of: self.member_of.as_ref().map(|s| s.to_string()),
            container: self.container,
            start: Some(self.start.into()),
            text: full_text,
            tokens,
            uim_reference_context: None,  // TODO
            uim_nest_level: None,
        };
        n.set_kind(self.kind.into());
        n
    }
}


pub fn pb_node_tokens(n: &pb::Node) -> Vec<Token> {
    let mut real_offset = n.start.as_ref().map_or(0, |loc| loc.offset);
    let mut real_line = n.start.as_ref().map_or(1, |loc| loc.line);

    n.tokens.iter()
    .enumerate()
    .map(|(i, pbtok)| {
        let end_offset = if i == n.tokens.len() - 1 {
            n.text.len()
        } else {
            n.tokens[i+1].offset as usize
        };
        if let Some(tre) = pbtok.real_offset { real_offset = tre; }
        if let Some(trl) = pbtok.real_line { real_line = trl; }
        let t = Token {
            offset: real_offset,
            line: real_line,
            text: n.text[pbtok.offset as usize..end_offset].into(),
            type_: pbtok.r#type().into(),
            // href: pbtok.href.map(|href| format!("{}#tok-{}", cursor_node_id_from_hash(href), "")),  // TODO
            context: HyperlinkedTokenContext {
                href: pbtok.href.clone(),
                sym_id: pbtok.sym_id.map(SymID),
                references: if let Some(legacy_tok_id) = pbtok.references {
                    ReferencesLink::LegacyID(legacy_tok_id)
                } else if pbtok.has_references {
                    ReferencesLink::TokenLocation(TokenLocation { node_id: n.id, offset: real_offset })
                } else {
                    ReferencesLink::None
                },
            },
        };
        real_offset += t.text.len() as u32;
        real_line += t.text.chars().filter(|c| *c == '\n').count() as u32;
        t
    })
    .collect()
}


impl<'a> From<&pb::Node> for Node {
    fn from(dn: &pb::Node) -> Node {
        Node {
            id: dn.id,
            container: dn.container,
            kind: dn.kind().into(),
            path: dn.path.to_owned(),
            path_id: PathID(dn.path_id),
            member_of: dn.member_of.as_ref().map(|s| s.into()),
            start: dn.start.clone().map(|pbloc| pbloc.into()).unwrap_or_else(Location::zero),
            text: pb_node_tokens(dn),
            context: HyperlinkedNodeContext {
                references: None,
            }
        }
    }
}


#[derive(Serialize, Deserialize, PartialEq, Hash, Debug)]
pub struct GToken<C> {
    #[serde(rename="id")]
    #[serde(with="crate::ser::token_id")]
    pub offset: Offset,

    #[serde(rename="t")]
    pub text: String, //

    #[serde(rename="T")]
    pub type_: TokenKind,

    #[serde(rename="N")]
    pub line: u32,

    #[serde(flatten)]
    pub context: C,
}

impl<C> GToken<C> {
    pub fn map<T>(self, f: &mut impl FnMut(Offset, &String, &TokenKind, C) -> T) -> GToken<T> {
        let ctx = f(self.offset, &self.text, &self.type_, self.context);
        GToken {
            offset: self.offset,
            text: self.text,
            type_: self.type_,
            line: self.line,
            context: ctx,
        }
    }
}


#[derive(PartialEq, Hash, Debug)]
pub enum ReferencesLink {
    None,
    TokenLocation(TokenLocation),
    LegacyID(u64),
}

impl serde::Serialize for ReferencesLink {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        crate::ser::opt_refs::serialize(self, serializer)
    }
}

impl ReferencesLink {
    pub fn is_set(&self) -> bool {
        match self {
            Self::None => false,
            _ => true,
        }
    }
}

#[derive(PartialEq, Hash, Debug)]
pub struct HyperlinkedTokenContext {
    pub href: Option<pb::token::Href>,
    pub sym_id: Option<SymID>,
    pub references: ReferencesLink,
}

impl Serialize for HyperlinkedTokenContext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        let mut state = serializer.serialize_struct("TokenContext", 4)?;
        let h = self.href.as_ref().map(|h| ser::gen_href::to_str(h));
        state.serialize_field("h", &h)?;
        let ht = match self.href {
            Some(pb::token::Href::NodeIdWithOffsetRef(NodeIdWithOffsetHref {offset, ..})) => Some(format!("tok-{}", offset)),
            _ => None
        };
        state.serialize_field("ht", &ht)?;
        state.serialize_field("s", &self.sym_id)?;
        state.serialize_field("r", &self.references)?;
        state.end()
    }
}

pub type Token = GToken<HyperlinkedTokenContext>;

#[derive(Serialize, Debug)]
pub struct Refs {
    pub token_location: TokenLocation,
    pub refs: HashSet<Ref>,
}


pub fn legacy_refs_path(token_location: &TokenLocation) -> String {
    format!("backrefs/cur/{}/{}", token_location.node_id, token_location.offset)
}

pub fn refs_url(token_location: &TokenLocation) -> String {
    format!("refs:{}/{}", token_location.node_id, token_location.offset)
}

impl Refs {
    pub fn new(token_location: TokenLocation) -> Refs {
        Refs {token_location, refs: HashSet::new()}
    }
}


impl Into<pb::References> for &Refs {
    fn into(self) -> pb::References {
        pb::References {
            node_id: self.token_location.node_id,
            offset: self.token_location.offset,
            refs: self.refs.iter().map(|r| r.into()).collect(),
        }
    }
}



#[derive(Serialize, Hash, PartialEq, Eq, Clone, Debug)]
pub struct Ref {
    pub href: NodeID,
    pub context: String,
    pub use_location: Location,
    pub use_path: RelativePath,
    pub linked_via_sym: bool
}


impl Into<pb::Reference> for &Ref {
    fn into(self) -> pb::Reference {
        pb::Reference {
            context: self.context.to_string(),
            href: Some(pb::reference::Href::NodeId(self.href)),
            use_location: Some(self.use_location.into()),
            linked_via_sym: self.linked_via_sym,
            use_path: self.use_path.to_string(),
        }
    }
}


#[derive(Serialize, Deserialize, PartialEq, Hash, Debug)]
pub struct GBlob<C, T> {
    // pub path: RelativePath,
    // pub file_id: u32,  // uniquely identifies path+content
    // pub path_id: PathID,  // stable across file changes
    pub nodes: Vec<GNode<C, T>>
}


impl<C, T> GBlob<C, T> {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn map_nodes<U, V>(self, f: &mut impl FnMut(GNode<C, T>) -> GNode<U, V>) -> GBlob<U, V>{
        GBlob {
            // path: self.path,
            // file_id: self.file_id,
            // path_id: self.path_id,
            nodes: self.nodes.into_iter().map(f).collect::<Vec<GNode<U, V>>>(),
        }
    }
}


pub type HNBlob = GBlob<HyperlinkedNodeContext, HyperlinkedTokenContext>;


#[derive(Clone, Debug, PartialEq)]
pub enum GenHref {
    DirectNodeLink(NodeID),
    NodeId(NodeID),
    BLoc(BlobSliceLoc),
    SymId(SymID),
    RefsId(TokenLocation),
    Path(String),
    UniHref(String, Offset),
}
pub trait IntoGenHref {
    fn into_gen_href(&self) -> GenHref;
}
impl IntoGenHref for GenHref {
    fn into_gen_href(&self) -> GenHref {
        self.clone()
    }
}
impl IntoGenHref for pb::token::Href {
    fn into_gen_href(&self) -> GenHref {
        match self {
            pb::token::Href::DirectNodeLink(id) => GenHref::DirectNodeLink(*id),
            pb::token::Href::NodeIdRef(id) => GenHref::NodeId(*id),
            pb::token::Href::NodeIdWithOffsetRef(NodeIdWithOffsetHref { node_id, offset }) => GenHref::NodeId(*node_id),
            pb::token::Href::SymIdRef(id) => GenHref::SymId(SymID(*id)),
            pb::token::Href::UniHref(UniHref { path, offset }) => GenHref::UniHref(path.clone(), *offset),
        }
    }
}
impl IntoGenHref for Option<pb::reference::Href> {
    fn into_gen_href(&self) -> GenHref {
        match self {
            Some(pb::reference::Href::DirectNodeLink(id)) => GenHref::DirectNodeLink(*id),
            Some(pb::reference::Href::NodeId(id)) => GenHref::NodeId(*id),
            None => panic!("unexpected None in Reference.href"),
        }
    }
}
impl IntoGenHref for Option<pb::index_item::Href> {
    fn into_gen_href(&self) -> GenHref {
        match self {
            Some(pb::index_item::Href::DirectNodeLink(id)) => GenHref::DirectNodeLink(*id),
            Some(pb::index_item::Href::NodeId(id)) => GenHref::NodeId(*id),
            Some(pb::index_item::Href::Floc(loc)) => GenHref::BLoc(*loc),
            Some(pb::index_item::Href::UniHref(_)) => unimplemented!(),
            None => panic!("unexpected None in InvertedIndexItem.href"),
        }
    }
}

pub fn nice_location(p: &str, loc: &Location) -> String {
    format!("{}:{}:{}", p, loc.line, loc.col)
}


#[cfg(test)]
mod test {
    use std::vec;

    use serde_json::to_string_pretty;

    use crate::{territory::index::token::Href, PathID, SymID};

    use super::{
        Node,
        NodeKind,
        HyperlinkedNodeContext,
        HyperlinkedTokenContext,
        Token,
        TokenKind,
        TokenLocation,
        Location,
        ReferencesLink,
    };

    #[test]
    fn node_json() {
        let node = Node {
            id: 9999999999999u64,
            container: Some(123412341234u64),
            kind: NodeKind::Directory,
            path: "/foo/bar.c".to_string(),
            path_id: PathID(69),
            member_of: None,
            start: Location { line: 1, col: 1, off: 0 },
            context: HyperlinkedNodeContext { references: None, },
            text: vec![
                Token {
                    offset: 0,
                    line: 1,
                    type_: TokenKind::Identifier,
                    text: "hello".into(),
                    context: HyperlinkedTokenContext {
                        href: Some(Href::DirectNodeLink(8888888888888u64)),
                        sym_id: Some(SymID(12345)),
                        references: ReferencesLink::TokenLocation(TokenLocation{ node_id: 76575756765765765u64, offset: 123 }),
                    },
                },
            ],
        };

        let repr = to_string_pretty(&node).unwrap();
        println!("{}", repr);
        assert_eq!(repr, r#"{
  "id": "id:9999999999999",
  "container": "id:123412341234",
  "kind": "directory",
  "path": "/foo/bar.c",
  "path_id": 69,
  "member_of": null,
  "start": {
    "line": 1,
    "col": 1,
    "off": 0
  },
  "text": [
    {
      "id": "tok-0",
      "t": "hello",
      "T": "Identifier",
      "N": 1,
      "h": "id:8888888888888",
      "ht": null,
      "s": "sym:12345",
      "r": "refs:76575756765765765/123"
    }
  ],
  "references": null
}"#);
    }

    #[test]
    fn node_json_with_legacy_reference_link() {
        let node = Node {
            id: 9999999999999u64,
            container: Some(123412341234u64),
            kind: NodeKind::Directory,
            path: "/foo/bar.c".to_string(),
            path_id: PathID(69),
            member_of: None,
            start: Location { line: 1, col: 1, off: 0 },
            context: HyperlinkedNodeContext { references: None, },
            text: vec![
                Token {
                    offset: 0,
                    line: 1,
                    type_: TokenKind::Identifier,
                    text: "hello".into(),
                    context: HyperlinkedTokenContext {
                        href: Some(Href::DirectNodeLink(8888888888888u64)),
                        sym_id: None,
                        references: ReferencesLink::LegacyID(76575756765765765u64),
                    },
                },
            ],
        };

        let repr = to_string_pretty(&node).unwrap();
        println!("{}", repr);
        assert_eq!(repr, r#"{
  "id": "id:9999999999999",
  "container": "id:123412341234",
  "kind": "directory",
  "path": "/foo/bar.c",
  "path_id": 69,
  "member_of": null,
  "start": {
    "line": 1,
    "col": 1,
    "off": 0
  },
  "text": [
    {
      "id": "tok-0",
      "t": "hello",
      "T": "Identifier",
      "N": 1,
      "h": "id:8888888888888",
      "ht": null,
      "s": null,
      "r": "backrefs/cur/76575756765765765"
    }
  ],
  "references": null
}"#);
    }

    #[test]
    fn node_json_with_offset_link() {
        let node = Node {
            id: 9999999999999u64,
            container: Some(123412341234u64),
            kind: NodeKind::Directory,
            path: "/foo/bar.c".to_string(),
            path_id: PathID(69),
            member_of: None,
            start: Location { line: 1, col: 1, off: 0 },
            context: HyperlinkedNodeContext { references: None, },
            text: vec![
                Token {
                    offset: 0,
                    line: 1,
                    type_: TokenKind::Identifier,
                    text: "hello".into(),
                    context: HyperlinkedTokenContext {
                        href: Some(Href::NodeIdWithOffsetRef(crate::territory::index::NodeIdWithOffsetHref { node_id: 8888888888888u64, offset: 9999 })),
                        sym_id: Some(SymID(12345)),
                        references: ReferencesLink::TokenLocation(TokenLocation{ node_id: 76575756765765765u64, offset: 123 }),
                    },
                },
            ],
        };

        let repr = to_string_pretty(&node).unwrap();
        println!("{}", repr);
        assert_eq!(repr, r#"{
  "id": "id:9999999999999",
  "container": "id:123412341234",
  "kind": "directory",
  "path": "/foo/bar.c",
  "path_id": 69,
  "member_of": null,
  "start": {
    "line": 1,
    "col": 1,
    "off": 0
  },
  "text": [
    {
      "id": "tok-0",
      "t": "hello",
      "T": "Identifier",
      "N": 1,
      "h": "id:8888888888888",
      "ht": "tok-9999",
      "s": "sym:12345",
      "r": "refs:76575756765765765/123"
    }
  ],
  "references": null
}"#);
    }

    #[test]
    fn node_json_with_member_of() {
        let node = Node {
            id: 9999999999999u64,
            container: Some(123412341234u64),
            kind: NodeKind::Directory,
            path: "/foo/bar.c".to_string(),
            path_id: PathID(69),
            member_of: Some("AbstractFactoryBaseClass".into()),
            start: Location { line: 1, col: 1, off: 0 },
            context: HyperlinkedNodeContext { references: None, },
            text: vec![
                Token {
                    offset: 0,
                    line: 1,
                    type_: TokenKind::Identifier,
                    text: "hello".into(),
                    context: HyperlinkedTokenContext {
                        href: Some(Href::DirectNodeLink(8888888888888u64)),
                        sym_id: Some(SymID(12345)),
                        references: ReferencesLink::TokenLocation(TokenLocation{ node_id: 76575756765765765u64, offset: 123 }),
                    },
                },
            ],
        };

        let repr = to_string_pretty(&node).unwrap();
        println!("{}", repr);
        assert_eq!(repr, r#"{
  "id": "id:9999999999999",
  "container": "id:123412341234",
  "kind": "directory",
  "path": "/foo/bar.c",
  "path_id": 69,
  "member_of": "AbstractFactoryBaseClass",
  "start": {
    "line": 1,
    "col": 1,
    "off": 0
  },
  "text": [
    {
      "id": "tok-0",
      "t": "hello",
      "T": "Identifier",
      "N": 1,
      "h": "id:8888888888888",
      "ht": null,
      "s": "sym:12345",
      "r": "refs:76575756765765765/123"
    }
  ],
  "references": null
}"#);
    }
}
