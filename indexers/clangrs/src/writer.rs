use std::collections::HashSet;
use std::fs::File;
use std::io::{stdout, Write};
use std::path::{PathBuf, Path};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::thread::{self, sleep};

use log::info;
use prost::Message;
use flate2::Compression;
use flate2::write::GzEncoder;
use prost::bytes::BytesMut;
use ring::digest::{Context, SHA256};

use territory_core::search::TrieIndex;
use territory_core::{legacy_refs_path, BlobID, HNBlob, Node, NodeID, Refs, TokenLocation};
use territory_core::territory::index::{self as pb, BlobSliceLoc, IndexItem, Build};
use crate::args::{get_debug_cfg, Args, CompressionMode};
use crate::intermediate_model::{SemFile, sqlite::OutputMap};
use crate::storage::StorageChannel;
use crate::testlib::repr_diff;


#[derive(Debug)]
pub struct ReferencesBlob(pub Vec<Refs>);

enum Work {
    Blob(HNBlob),
    References(Refs),
    ReferencesBlob(ReferencesBlob),
    Build(Build),
}

pub struct WriterStats {
    pub total_written: usize,
    pub pb_bytes_reused: usize,
    pub pb_bytes_count: usize,
}


pub struct NodeWriter {
    args: Args,
    sender: Option<crossbeam_channel::Sender<Work>>,
    receiver: crossbeam_channel::Receiver<Work>,
    total_submitted: usize,
    total_written: Arc<AtomicUsize>,
    join_handles: Vec<thread::JoinHandle<WriterStats>>,
    used_ref_hashes: HashSet<TokenLocation>,
}

impl NodeWriter {
    pub fn start(
        args: &Args,
        concurrency: usize,
        storage_channel: StorageChannel,
        // db_conn: Arc<Mutex<Connection>>,
        output_map: OutputMap,
    ) -> Self {
        let (s, r) = crossbeam_channel::unbounded();
        let write_counter = Arc::new(AtomicUsize::new(0));

        let join_handles = (0..concurrency).map(|_| {
            let tr = r.clone();
            let t_write_counter = Arc::clone(&write_counter);
            let mut local_write_counter = 0;
            let mut local_byte_counter = 0;
            let mut local_reuse_counter = 0;
            let t_args = args.clone();
            let t_storage_channel = storage_channel.clone();
            let t_output_map = output_map.clone();
            thread::spawn(move || {
                loop {
                    let result = tr.recv();
                    match result {
                        Ok(work) => {
                            match &work  {
                                Work::Blob(file) => {
                                    let (wrote, reused) = write_blob_pb(
                                        &t_args, &file, &t_storage_channel, &t_output_map);
                                    local_byte_counter += wrote;
                                    local_reuse_counter += reused;
                                }
                                Work::References(refs) => {
                                    local_byte_counter += write_references_pb(
                                        &t_args, refs, &t_storage_channel);
                                }
                                Work::ReferencesBlob(refs_file) => {
                                    let (wrote, reused) = write_references_file_pb(
                                        &t_args, refs_file, &t_storage_channel, &t_output_map);
                                    local_byte_counter += wrote;
                                    local_reuse_counter += reused;
                                }
                                Work::Build(build) => {
                                    local_byte_counter +=write_build_pb(
                                        &t_args, build, &t_storage_channel);
                                }
                            }

                            t_write_counter.fetch_add(
                                1,
                                std::sync::atomic::Ordering::SeqCst);
                            local_write_counter += 1;
                        }
                        Err(_) => { break; },
                    }
                }

                WriterStats {
                    total_written: local_write_counter,
                    pb_bytes_reused: local_reuse_counter,
                    pb_bytes_count: local_byte_counter,
                }
            })
        }).collect();

        Self {
            args: args.clone(),
            sender: Some(s),
            receiver: r,
            total_submitted: 0,
            total_written: write_counter,
            join_handles,
            used_ref_hashes: HashSet::new(),
        }
    }

    pub fn submit_blob(&mut self, file: HNBlob) {
        match &self.sender {
            Some(s) => { s.send(Work::Blob(file)).unwrap(); self.total_submitted += 1; },
            None => { panic!("submit to closed sender"); },
        }
    }

    pub fn submit_refs(&mut self, refs: Refs) {
        if get_debug_cfg().print_blob_writes {
            println!("{refs:#?}");
        }
        if self.used_ref_hashes.contains(&refs.token_location) {
            panic!("hash collission: {:?}", refs.token_location);
        }
        self.used_ref_hashes.insert(refs.token_location);

        match &self.sender {
            Some(s) => { s.send(Work::References(refs)).unwrap(); self.total_submitted += 1; },
            None => { panic!("submit to closed sender"); },
        }
    }

    pub fn submit_refs_file(&mut self, file_refs: ReferencesBlob) {
        if get_debug_cfg().print_blob_writes {
            println!("{file_refs:#?}");
        }
        match &self.sender {
            Some(s) => { s.send(Work::ReferencesBlob(file_refs)).unwrap(); self.total_submitted += 1; },
            None => { panic!("submit to closed sender"); },
        }
    }

    pub fn submit_build(&mut self, build: Build) {
        match &self.sender {
            Some(s) => { s.send(Work::Build(build)).unwrap(); self.total_submitted += 1; },
            None => { panic!("submit to closed sender"); },
        }
    }

    pub fn written_count(&self) -> usize {
        self.total_written.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn join(mut self: Self) -> WriterStats {
        self.sender = None;

        if !self.args.fastwait {
            while !self.receiver.is_empty() && !self.join_handles.iter().any(|jh| jh.is_finished()) {
                info!("{} of {} nodes written", self.written_count(), self.total_submitted);
                sleep(std::time::Duration::new(1, 0));
            }
        }

        let mut ws = WriterStats { total_written: 0, pb_bytes_reused: 0, pb_bytes_count: 0 };
        for jh in self.join_handles {
            let local_ws = jh.join().unwrap();
            ws.total_written += local_ws.total_written;
            ws.pb_bytes_reused += local_ws.pb_bytes_reused;
            ws.pb_bytes_count += local_ws.pb_bytes_count;
        }
        ws
    }
}


fn write_blob_pb(
    args: &Args,
    blob: &HNBlob,
    storage_channel: &StorageChannel,
    output_map: &OutputMap,
    // paths: &Paths,
) -> (usize, usize) {
    let mut total_output = Vec::new();

    let mut reused = 0;
    let mut blob_id = None;

    for node in &blob.nodes {
        assert!(node.id != NodeID::MAX);

        let pb_node: pb::Node = node.into();

        let rountripped_node = Node::from(&pb_node);
        assert!(
            rountripped_node == *node,
            "original node: {:#?}\nroundtripped node: {:#?}\ndiff: {}",
            node, rountripped_node, repr_diff(node, &rountripped_node)
        );


        if get_debug_cfg().print_blob_writes {
            let mut l = stdout().lock();
            territory_core::pretty_print::node(&mut l, &pb_node).unwrap();
        }

        reused += append_pb_node(&mut blob_id, &mut total_output, pb_node, output_map, args.compression);

    }

    if total_output.is_empty() { return (0, reused); }

    let blob_summary = submit_hashed_blob(&args.repo_id, blob_id.unwrap(), storage_channel, total_output);

    (blob_summary.length, reused)
}

pub fn append_pb_node(
    blob_id: &mut Option<BlobID>,
    total_output: &mut Vec<u8>,
    pb_node: pb::Node,
    output_map: &OutputMap,
    compression_mode: CompressionMode,
) -> usize {
    let mut output = Vec::new();
    pb_node.encode(&mut output).unwrap();

    let mut context = Context::new(&SHA256);
    context.update(&output);
    let hash = context.finish();

    if !output_map.refresh_node_location_if_exists(pb_node.id, hash) {
        let mut comp_output = apply_compression(compression_mode, output);

        prost::encode_length_delimiter(comp_output.len(), total_output).unwrap();

        let start_offset = total_output.len().try_into().expect("output too long to address");
        total_output.append(&mut comp_output);
        let end_offset = total_output.len().try_into().expect("output too long to address");

        // node_locs.push((pb_node.id, start_offset, end_offset, hash));
        let blob_id = blob_id.get_or_insert_with(|| output_map.new_blob_id());
        let loc = BlobSliceLoc { blob_id: blob_id.0, start_offset, end_offset };
        output_map.store_node_location(pb_node.id, &loc, hash);

        return 0;
    } else {
        return output.len();
    }
}


pub fn write_references_pb(
    args: &Args,
    references: &Refs,
    storage_channel: &StorageChannel,
) -> usize {
    let node_path = PathBuf::from("nodes")
        .join(&args.repo_id)
        .join(legacy_refs_path(&references.token_location));

    let pb_refs: pb::References = references.into();

    let mut output = Vec::new();
    pb_refs.encode(&mut output).unwrap();

    let output = apply_compression(args.compression, output);
    let len = output.len();

    storage_channel.submit_blob_blocking(node_path, output);

    len
}


pub fn write_references_file_pb(
    args: &Args,
    refs_file: &ReferencesBlob,
    storage_channel: &StorageChannel,
    output_map: &OutputMap,
) -> (usize, usize) {
    let mut total_output = Vec::new();
    let mut reused = 0;

    let mut refs_locs = Vec::new();

    let blob_id @ BlobID(blob_id_int) = output_map.new_blob_id();

    let ReferencesBlob(refs) = refs_file;
    for refs in refs {
        let mut output = Vec::new();
        let pb_node: pb::References = refs.into();

        pb_node.encode(&mut output).unwrap();

        let mut context = Context::new(&SHA256);
        context.update(&output);
        let hash = context.finish();

        let mut comp_output = apply_compression(args.compression, output);

        let start_offset: u64 = total_output.len().try_into().expect("output too long to address");
        let end_offset = start_offset.checked_add(
            comp_output.len().try_into().expect("output too long to address")
        ).expect("output too long to address");
        let slice_location = BlobSliceLoc { blob_id: blob_id_int, start_offset, end_offset };

        if let Some(previous_slice_location) = output_map.get_existing_slice_loc_or_insert(hash, slice_location) {
            refs_locs.push((refs.token_location, previous_slice_location));
            reused += total_output.len();
        } else {
            total_output.append(&mut comp_output);
            refs_locs.push((refs.token_location, slice_location));
        }
    }

    let len = if !total_output.is_empty() {
        let blob_summary = submit_hashed_blob(&args.repo_id, blob_id, storage_channel, total_output);
        blob_summary.length
    } else {
        0
    };

    refs_locs.sort();

    for (token_location, slice_location) in refs_locs {
        output_map.store_refs_location(token_location, &slice_location);
    }

    (len, reused)
}


fn write_build_pb(
    args: &Args,
    build: &Build,
    storage_channel: &StorageChannel,
) -> usize {
    let build_path = PathBuf::from("builds").join(&args.repo_id).join(&build.id.to_string());

    let mut output = Vec::new();
    build.encode(&mut output).unwrap();

    let output = apply_compression(args.compression, output);
    let len = output.len();

    storage_channel.submit_blob_blocking(build_path, output);

    len
}


pub struct BlobSummary {
    pub length: usize,
    pub blob_id: BlobID,
}


fn submit_hashed_blob(
    repo_id: &str,
    blob_id: BlobID,
    storage_channel: &StorageChannel,
    blob: Vec<u8>,
) -> BlobSummary {
    let length = blob.len();
    let path = PathBuf::from("nodes").join(repo_id).join("f").join(&blob_id.0.to_string());
    storage_channel.submit_blob_blocking(path.clone(), blob);
    BlobSummary {
        length,
        blob_id,
    }
}


pub struct InvertedIndexWriter {
    args: Args,
    sender: crossbeam_channel::Sender<pb::IndexItem>,
    join_handle: thread::JoinHandle<(Vec<u8>, Vec<IndexItem>)>,
    storage_channel: StorageChannel,
}

impl InvertedIndexWriter {
    pub fn start(args: &Args, storage_channel: StorageChannel) -> InvertedIndexWriter {


        let (sender, receiver) = crossbeam_channel::unbounded::<pb::IndexItem>();
        let join_handle = thread::spawn(move || {
            let mut buf = Vec::new();
            let mut inverted_index_items = Vec::new();

            loop {
                let result = receiver.recv();
                match result {
                    Ok(item) => {
                        inverted_index_items.push(item.clone());
                        item.encode_length_delimited(&mut buf).unwrap();
                    }
                    Err(_) => {
                        return (buf, inverted_index_items);
                    }
                }
            }
        });
        InvertedIndexWriter { args: args.clone(), sender, join_handle, storage_channel }
    }

    pub fn submit_item(&mut self, item: pb::IndexItem) {
        self.sender.send(item).unwrap();
    }

    pub async fn join(self) {
        if !self.args.fastwait {
            loop {
                if self.join_handle.is_finished() { break; }
                let l = self.sender.len();
                if l == 0 { break; }
                info!("writing {} inverted index items", l);
                sleep(std::time::Duration::new(1, 0));
            }
        }
        drop(self.sender);
        let (buf, mut items) = self.join_handle.join().unwrap();

        let trie = TrieIndex::from_index_items(&mut items);
        let mut trie_buf = BytesMut::new();
        trie.dump(&mut trie_buf).unwrap();
        let trie_buf = apply_compression(self.args.compression, trie_buf.to_vec());
        let trie_path = PathBuf::from("search").join(&self.args.repo_id).join(&self.args.build_id).join("trie");
        self.storage_channel.submit_blob(trie_path, trie_buf).await;

        let index_path = PathBuf::from("search").join(&self.args.repo_id).join(&self.args.build_id).join("all");
        let buf = apply_compression(self.args.compression, buf);
        self.storage_channel.submit_blob(index_path, buf).await;
    }
}


pub fn apply_compression(mode: CompressionMode, v: Vec<u8>) -> Vec<u8> {
    match mode {
        CompressionMode::None => v,
        CompressionMode::Gzip => {
            let mut zipw = GzEncoder::new(Vec::new(), Compression::default());
            zipw.write_all(&v).unwrap();
            zipw.finish().unwrap()
        }
    }
}


pub struct IntermediateNodeFileWriter {
    count: usize,
    writer: std::io::BufWriter<File>,
}

impl IntermediateNodeFileWriter {
    pub fn new_from_args(args: &Args) -> Self {
        Self::new(&semfile_path(args, args.slice))
    }

    pub fn new(temp_file_path: &Path) -> Self {
        info!("writing intermediate data to {:?}", temp_file_path);
        let file = File::create(temp_file_path).unwrap();
        let writer = std::io::BufWriter::new(file);
        Self {
            count: 0,
            writer,
        }
    }

    pub fn append(&mut self, semfile: &SemFile) {
        if get_debug_cfg().pretty_semfiles {
            serde_json::to_writer_pretty(&mut self.writer, semfile).unwrap();
        } else {
            serde_json::to_writer(&mut self.writer, semfile).unwrap();
        }
        self.count += 1;
    }
}

pub struct IntermediateNodeFileReader<'a> {
    file_path: PathBuf,
    deserializer: serde_json::de::StreamDeserializer<
        'a,
        serde_json::de::IoRead<std::io::BufReader<File>>,
        SemFile
    >
}

impl<'a> Iterator for &mut IntermediateNodeFileReader<'a> {
    type Item = SemFile;

    fn next(&mut self) -> Option<Self::Item> {
        self.deserializer.next().map(|re| re.unwrap())
    }
}

impl<'a> IntermediateNodeFileReader<'a> {
    pub fn new_with_slice(args: &Args, slice: usize) -> Self {
        assert_eq!(slice, 1);
        Self::new(semfile_path(args, slice))
    }

    pub fn new(file_path: PathBuf) -> Self {
        let read_file = File::open(&file_path).unwrap();
        let reader = std::io::BufReader::new(read_file);
        Self {
            file_path,
            deserializer: serde_json::de::Deserializer::from_reader(reader).into_iter(),
        }
    }

    pub fn restart(self) -> Self {
        drop(self.deserializer);
        Self::new(self.file_path)
    }
}


fn semfile_path(args: &Args, slice: usize) -> PathBuf {
    args.intermediate_path.join(format!("semfile.{}", slice))
}
