pub mod node_id {
    use serde::{Serializer, Deserialize};

    type T = u64;

    pub fn serialize<S>(
        node_id: &u64,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self::to_str(*node_id);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<T, D::Error> where D: serde::de::Deserializer<'de>, T: Sized {
        let s = String::deserialize(deserializer)?;
        Ok(from_str(s))
    }

    pub fn from_str(s: String) -> T {
        assert!(s.starts_with("id:"));
        let digits = &s[3..];
        digits.parse().unwrap()
    }

    pub fn to_str(node_id: T) -> String {
        format!("id:{}", node_id)
    }
}

pub mod opt_node_id {
    use serde::{Serializer, Serialize, Deserialize};

    type T = Option<u64>;

    pub fn serialize<S>(
        node_id: &Option<u64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match node_id {
            Some(id) => {
                super::node_id::serialize(id, serializer)
            }
            _ => {
                None::<String>.serialize(serializer)
            }
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<T, D::Error> where D: serde::de::Deserializer<'de>, T: Sized {
        let opt = Option::<String>::deserialize(deserializer)?;
        Ok(opt.map(super::node_id::from_str))
    }
}

pub mod opt_href {
    use std::path::Path;

    use serde::{Serializer, Serialize};
    use crate::{pb::token::Href, territory::index::{NodeIdWithOffsetHref, UniHref}};
    use super::node_id;

    pub fn serialize<S>(
        href: &Option<Href>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &href {
            Some(Href::DirectNodeLink(id)) |
            Some(Href::NodeIdRef(id)) => {
                let s = node_id::to_str(*id);
                serializer.serialize_str(&s)
            }
            Some(Href::NodeIdWithOffsetRef(NodeIdWithOffsetHref { node_id, offset })) => {
                let s = node_id::to_str(*node_id);
                serializer.serialize_str(&s)
            }
            Some(Href::SymIdRef(id)) => {
                let s = format!("sym:{}", id);
                serializer.serialize_str(&s)
            }
            Some(Href::UniHref(UniHref { path, offset })) => {
                let s = format!("path:{}#token-{}", path, offset);
                serializer.serialize_str(&s)
            }
            None => {
                None::<String>.serialize(serializer)
            }
        }
    }
}

pub mod gen_href {
    use regex::Regex;
    use serde::Serializer;
    use crate::{GenHref, IntoGenHref, territory::index::BlobSliceLoc, NodeID, SymID, TokenLocation, refs_url};

    pub fn serialize<S>(
        href: &impl IntoGenHref,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self::to_str(href);
        serializer.serialize_str(&s)
    }

    pub fn to_str(href: &impl IntoGenHref) -> String {
        let ghref: GenHref = href.into_gen_href();
        match ghref {
            GenHref::DirectNodeLink(id) |
            GenHref::NodeId(id) => format!("id:{}", id),
            GenHref::SymId(SymID(id)) => format!("sym:{}", id),
            GenHref::RefsId(token_location) => refs_url(&token_location),
            GenHref::BLoc(loc) => {
                format!("slice:f/{}[{}:{}]", loc.blob_id, loc.start_offset, loc.end_offset)
            },
            GenHref::Path(path) => format!("path:{}", path),
            GenHref::UniHref(path, offset) => format!("path:{}#token-{}", path, offset),
        }
    }
    pub fn from_str(url: &str) -> Option<GenHref> {
        let file_slice_re = Regex::new(r"^slice:f/([0-9]+)\[([0-9]+):([0-9]+)\]$").unwrap();
        let refs_re = Regex::new(r"^refs:([0-9]+)/([0-9]+)$").unwrap();

        if url.starts_with("id:") {
            let id: NodeID = url[3..].parse().ok()?;
            Some(GenHref::NodeId(id))
        } else if url.starts_with("sym:") {
            let id: u64 = url[4..].parse().ok()?;
            Some(GenHref::SymId(SymID(id)))
        } else if url.starts_with("path:") {
            Some(GenHref::Path(url[5..].into()))
        } else if url.starts_with("cur/") {
            let id: u64 = url[4..].parse().ok()?;
            Some(GenHref::DirectNodeLink(id))
        } else if let Some(caps) = refs_re.captures(url) {
            let token_location = TokenLocation {
                node_id: caps[1].parse().ok()?,
                offset: caps[2].parse().ok()?,
            };
            Some(GenHref::RefsId(token_location))
        } else if let Some(caps) = file_slice_re.captures(url) {
            let floc = BlobSliceLoc {
                blob_id: caps[1].parse().ok()?,
                start_offset: caps[2].parse().ok()?,
                end_offset: caps[3].parse().ok()?,
            };
            Some(GenHref::BLoc(floc))
        } else {
            None
        }
    }

    #[cfg(test)]
    mod test {
        use crate::{GenHref, SymID, TokenLocation};
        use super::{from_str, to_str};

        #[test]
        fn node_id_roundtrip() {
            let id = GenHref::NodeId(99999999u64);
            assert_eq!(Some(id.clone()), from_str(&to_str(&id)));
        }

        #[test]
        fn sym_id_roundtrip() {
            let id = GenHref::SymId(SymID(99999999u64));
            assert_eq!(Some(id.clone()), from_str(&to_str(&id)));
        }

        #[test]
        fn floc_roundtrip() {
            let loc = GenHref::BLoc(crate::territory::index::BlobSliceLoc {
                blob_id: 123, start_offset: 10, end_offset: 20
            });
            assert_eq!(Some(loc.clone()), from_str(dbg!(&to_str(&loc))));
        }

        #[test]
        fn path_roundtrip() {
            let path = GenHref::Path("foo.c".into());
            assert_eq!(Some(path.clone()), from_str(dbg!(&to_str(&path))));
        }

        #[test]
        fn refs_roundtrip() {
            let refs_id = GenHref::RefsId(TokenLocation { node_id: 98765, offset: 1234 });
            assert_eq!(Some(refs_id.clone()), from_str(dbg!(&to_str(&refs_id))));
        }
    }
}

pub mod opt_refs {
    use serde::{Serializer, Serialize};

    use crate::{ReferencesLink, refs_url};

    pub fn serialize<S>(
        references_link: &ReferencesLink,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match references_link {
            ReferencesLink::TokenLocation(token_location) => {
                serializer.serialize_str(&refs_url(&token_location))
            }
            ReferencesLink::LegacyID(token_id) => {
                let s = format!("backrefs/cur/{}", token_id);
                serializer.serialize_str(&s)
            }
            ReferencesLink::None => {
                None::<String>.serialize(serializer)
            }
        }
    }
}

pub mod token_id {
    use serde::{Serializer, Deserialize};

    type T = Offset;

    use crate::Offset;

    pub fn serialize<S>(
        offset: &Offset,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("tok-{}", offset);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<T, D::Error> where D: serde::de::Deserializer<'de>, T: Sized {
        let s = String::deserialize(deserializer)?;
        Ok(from_str(s))
    }

    pub fn from_str(s: String) -> T {
        assert!(s.starts_with("tok-"));
        let digits = &s[4..];
        digits.parse().unwrap()
    }
}
