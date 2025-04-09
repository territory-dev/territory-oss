use crate::{Token, Offset, TokenKind, NodeID, HyperlinkedTokenContext, TokenLocation, ReferencesLink, SymID};
use crate::territory::index::{token::Href, NodeIdWithOffsetHref};


pub struct TokenWriter<'a> {
    vec: &'a mut Vec<Token>,
    offset: Offset,
    line: u32,
    max_node_len: usize,
    wrote_trunc: bool,
}

impl<'a> TokenWriter<'a> {
    pub fn new(
        vec: &'a mut Vec<Token>,
        initial_offset: Offset,
        initial_line: u32,
        max_node_len: usize,
    ) -> TokenWriter {
        TokenWriter { vec, offset: initial_offset, line: initial_line, max_node_len, wrote_trunc: false }
    }

    pub fn write(
        &mut self,
        kind: TokenKind,
        text: &str,
        sym_id: Option<SymID>,
        href: Option<NodeID>,
        references: Option<TokenLocation>,
    ) {
        if self.vec.len() >= self.max_node_len {
            if ! self.wrote_trunc {
                self.wrote_trunc = true;
            }

            return;
        }

        let off = self.offset;
        let tlen: Offset = text.len().try_into().expect("text too long");
        let reflink = match references {
            Some(token_location) => ReferencesLink::TokenLocation(token_location),
            None => ReferencesLink::None,
        };
        let t = Token {
            offset: off,
            line: self.line,
            text: text.into(),
            type_: kind,
            context: HyperlinkedTokenContext {
                href: href.map(|h| Href::NodeIdRef(h)),
                sym_id,
                references: reflink,
            },
        };

        self.vec.push(t);
        self.offset += tlen;
        self.line += text.chars().filter(|c| *c == '\n').count() as u32;
    }
}

