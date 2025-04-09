use std::io::Result;

const PROTO_FILE: &str = "../proto/index.proto";


fn main() -> Result<()> {
    println!("cargo:rerun-if-changed={}", PROTO_FILE);
    println!("cargo:rerun-if-changed=../proto/uim.proto");

    let mut c = prost_build::Config::new();

    c.type_attribute("territory.index.NodeIdWithOffsetHref", "#[derive(Eq, Hash, PartialOrd, Ord, serde::Serialize)]");
    c.type_attribute("territory.index.UniHref", "#[derive(Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]");
    c.type_attribute("territory.index.BlobSliceLoc", "#[derive(Copy, Eq, Hash, PartialOrd, Ord, serde::Serialize)]");

    c.type_attribute("territory.index.Node", "#[derive(serde::Serialize)]");
    c.field_attribute("territory.index.Node.id", "#[serde(with = \"crate::ser::node_id\")]");

    c.type_attribute("territory.index.Token", "#[derive(serde::Serialize)]");
    c.field_attribute("territory.index.Token.references", "#[serde(with = \"crate::ser::opt_node_id\")]");
    c.type_attribute("territory.index.Token.href", "#[derive(Hash, serde::Serialize)]");

    c.type_attribute("territory.index.Location", "#[derive(serde::Serialize, serde::Deserialize)]");

    c.type_attribute("territory.index.IndexItem", "#[derive(serde::Serialize)]");
    c.field_attribute("territory.index.IndexItem.href", "#[serde(with = \"crate::ser::gen_href\")]");

    c.type_attribute("territory.index.Reference", "#[derive(serde::Serialize)]");
    c.field_attribute("territory.index.Reference.href", "#[serde(with = \"crate::ser::gen_href\")]");
    c.type_attribute("territory.index.Reference.href", "#[derive(Hash, Copy, Eq, serde::Serialize)]");

    c.type_attribute("territory.index.References", "#[derive(serde::Serialize)]");
    c.field_attribute("territory.index.References.node_id", "#[serde(with = \"crate::ser::node_id\")]");

    c.type_attribute("territory.index.IndexItemKind", "#[derive(serde::Serialize)]");

    c.compile_protos(&[PROTO_FILE], &["../proto/"])?;
    Ok(())
}
