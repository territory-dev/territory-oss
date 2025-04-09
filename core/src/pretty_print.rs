use std::fmt::{Write as FWrite};
use std::io::Write;

use crate::territory::index::Node;
use crate::ser::gen_href;
use crate::GNode;

pub fn node(out: &mut dyn Write, n: &Node) -> Result<(), std::io::Error> {

    writeln!(out, "###############################################################################")?;
    writeln!(out, "NODE {:?}", n.id)?;
    writeln!(out, "kind: {:?}", n.kind())?;
    writeln!(out, "path: {:?}", n.path)?;
    writeln!(out, "container: {:?}", n.container)?;
    writeln!(out, "start: {:?}", n.start)?;
    writeln!(out, "###############################################################################")?;
    writeln!(out, "")?;

    for (i, t) in n.tokens.iter().enumerate() {
        let e = if i == n.tokens.len() - 1 {
            n.text.len()
        } else {
            n.tokens[i+1].offset as usize
        };
        let txt = &n.text[t.offset as usize..e];

        let mut marker = String::new();

        if let Some(h) = &t.href {
            write!(&mut marker, "|{}", gen_href::to_str(h)).unwrap();
        }
        // if let Some(sym_id) = &t.sym_id {
        //     write!(&mut marker, "$sym:{}", sym_id).unwrap();
        // }
        if t.has_references { marker.push('#'); }

        if marker.is_empty() {
            write!(out, "{}", txt)?;
        } else {
            write!(out, "[{}{}]", txt, marker)?;
        }
    }

    writeln!(out, "")?;
    writeln!(out, "")?;

    Ok(())
}


pub fn gnode<T, U>(out: &mut dyn Write, n: &GNode<T, U>) -> Result<(), std::io::Error> {

    writeln!(out, "###############################################################################")?;
    writeln!(out, "GNODE {:?}", n.id)?;
    writeln!(out, "kind: {:?}", n.kind)?;
    writeln!(out, "path: {:?}", n.path)?;
    writeln!(out, "container: {:?}", n.container)?;
    writeln!(out, "start: {:?}", n.start)?;
    writeln!(out, "###############################################################################")?;
    writeln!(out, "")?;

    for t in &n.text {
        write!(out, "{}", t.text)?;
    }

    writeln!(out, "")?;
    writeln!(out, "")?;

    Ok(())
}
