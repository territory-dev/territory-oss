use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};

use prost::bytes::BytesMut;
use prost::Message;
use pyo3::types::{PyBytes, PyDict, PyNone};
use pyo3::prelude::*;
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyValueError, PyIOError};
use pythonize::{pythonize, depythonize};

use territory_core::pblib::decode_many;
use territory_core::Node;
use territory_core::resolver::{NeedData, ResolutionFailure, Resolver, SingleBlobResolver, TrieResolver};
use territory_core::territory::index as pb;
use territory_core::search::{self, TrieIndex};
use territory_core::slicemap_trie::{SharedCache, SlicemapReader};


fn runtime_err_str<T>(e: T) -> PyErr where T: ToString {
    PyRuntimeError::new_err(e.to_string())
}


#[pyclass]
pub struct PyNeedData {
    nd: Option<NeedData>
}

#[pymethods]
impl PyNeedData {
    pub fn location(&self, py: Python) -> PyResult<PyObject> {
        let Some(nd) = &self.nd else {
            return Err(PyValueError::new_err("use of expired PyNeedData"));
        };
        Ok(pythonize(py, &nd.0)?)
        // Ok(PyConcreteLocation { loc: nd.0.clone() })
    }

    pub fn got_data(&mut self, data: &[u8]) -> PyResult<()> {
        let Some(nd) = self.nd.take() else {
            return Err(PyValueError::new_err("use of expired PyNeedData"));
        };
        nd.1(data).map_err(runtime_err_str)?;
        Ok(())
    }
}

#[pyclass]
pub struct PyResolver {
    resolver: Box<dyn Resolver + Send>,
}

#[pymethods]
impl PyResolver {
    pub fn resolve_url(&mut self, py: Python<'_>, url: &str) -> PyResult<PyObject> {
        let res = self.resolver.resolve_url(url);
        match res {
            Ok(loc) =>
                Ok(pythonize(py, &loc)?),
            Err(ResolutionFailure::NotFound) =>
                Err(PyKeyError::new_err("url not found")),
            Err(ResolutionFailure::BadUrl) =>
                Err(PyValueError::new_err("bad url")),
            Err(ResolutionFailure::NeedData(nd)) =>
                Ok(PyNeedData { nd: Some(nd) }.into_py(py)),
            Err(e) =>
                Err(PyRuntimeError::new_err(format!("could not resolve: {:?}", e))),
        }
    }
}

#[pyclass]
pub struct SharedResolverCache {
    cache: Arc<Mutex<SharedCache>>,
}

#[pymethods]
impl SharedResolverCache {
    #[new]
    pub fn new(max_size: usize) -> Self {
        Self { cache: SharedCache::new(max_size) }
    }

    pub fn count(&self) -> usize {
        SharedCache::count(&self.cache)
    }

    pub fn get_trie_resolver(
        &self,
        repo_id: &str,
        build_data: &[u8],
    ) -> PyResult<PyResolver> {
        let build = pb::Build::decode(build_data)
            .map_err(runtime_err_str)?;

        let backup_resolver = territory_core::resolver::BasicResolver;
        let nodemap = SlicemapReader::new(
            build.nodemap_trie_root
                .ok_or(PyValueError::new_err("missing nodemap_trie_root"))?,
            SharedCache::new_handle(
                &self.cache, &format!("{}/n", repo_id)));
        let symmap = SlicemapReader::new(
            build.symmap_trie_root
                .ok_or(PyValueError::new_err("missing symmap_trie_root"))?,
            SharedCache::new_handle(
                &self.cache, &format!("{}/s", repo_id)));
        let refmap = SlicemapReader::new(
            build.references_trie_root
                .ok_or(PyValueError::new_err("missing references_trie_root"))?,
            SharedCache::new_handle(
                &self.cache, &format!("{}/r", repo_id)));
        let resolver = TrieResolver::new(backup_resolver, nodemap, symmap, refmap, build.repo_root_node_id);
        Ok(PyResolver { resolver: Box::new(resolver) })
    }
}

#[pyclass]
pub struct PySearchIndex {
    search_index: Vec<pb::IndexItem>,
}

#[pymethods]
impl PySearchIndex {
    pub fn search(&self, py: Python, query: &str, opts: &PyAny) -> PyResult<PyObject> {
        let opts = depythonize(opts)?;

        let index_items = search::search(&self.search_index, query, &opts);
        Ok(pythonize(py, &index_items)?)
    }

    pub fn make_trie<'p>(&mut self) -> PyTrieIndex {
        let ti = TrieIndex::from_index_items(&mut self.search_index);
        PyTrieIndex(ti)
    }
}


#[pyclass]
pub struct PyTrieIndex(TrieIndex);

#[pymethods]
impl PyTrieIndex {
    #[new]
    pub fn new(data: &[u8]) -> PyResult<Self> {
        let ti = TrieIndex::load(data).map_err(runtime_err_str)?;
        Ok(Self(ti))
    }

    pub fn search<'p>(&self, py: Python<'p>, q: &str, opts: &PyAny) -> PyResult<PyObject> {
        let opts = depythonize(opts)?;
        let res: Vec<_> = self.0.search(q, &opts)
            .into_iter()
            .collect();
        Ok(pythonize(py, &res)?)
    }

    pub fn keys_data<'py>(&self, py: Python<'py>) -> &'py PyBytes {
        PyBytes::new(py, &self.0.keys_data)
    }

    pub fn paths_data<'py>(&self, py: Python<'py>) -> &'py PyBytes {
        PyBytes::new(py, &self.0.paths_data)
    }

    pub fn types_data<'py>(&self, py: Python<'py>) -> &'py PyBytes {
        PyBytes::new(py, &self.0.types_data)
    }

    pub fn normalized_entries<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        Ok(pythonize(py, &self.0.entries)?)
    }

    pub fn normalized_entries_proto<'py>(&self, py: Python<'py>) -> PyResult<&'py PyBytes> {
        let mut buf = BytesMut::new();
        self.0.normalized_entries_proto(&mut buf).map_err(runtime_err_str)?;
        Ok(PyBytes::new(py, &buf))
    }

    pub fn data<'py>(&self, py: Python<'py>) -> PyResult<&'py PyBytes> {
        let mut buf = BytesMut::new();
        self.0.dump(&mut buf).map_err(runtime_err_str)?;
        Ok(PyBytes::new(py, &buf))
    }

}


#[pyfunction]
pub fn trie_from_strings<'py>(py: Python<'py>, strings: &PyAny) -> PyResult<&'py PyBytes> {
    let mut strings: Vec<String> = depythonize(strings)?;
    strings.sort();

    let mut w = territory_core::strings_trie::TrieWriter::new();
    for ii in strings {
        w.push(&ii, 0);
    }

    let by = w.data();
    Ok(PyBytes::new(py, &by))
}

#[pyfunction]
pub fn bytes_to_node<'py>(py: Python<'py>, data: &[u8]) -> PyResult<PyObject> {
    let pb_node = pb::Node::decode(data).unwrap();
    let node = Node::from(&pb_node);

    Ok(pythonize(py, &node)?)
}


#[pyfunction]
pub fn serial_read_nodes<'py>(py: Python<'py>, data: &[u8]) -> PyResult<PyObject> {
    let pb_nodes: Vec<pb::Node> = decode_many(data).map_err(runtime_err_str)?;
    let nodes: Vec<Node> = pb_nodes.iter().map(Node::from).collect();

    Ok(pythonize(py, &nodes)?)
}


#[pyfunction]
pub fn single_blob_resolver(data: &[u8]) -> PyResult<PyResolver> {
    let resolver = SingleBlobResolver::read_blob(data).map_err(runtime_err_str)?;
    Ok(PyResolver { resolver: Box::new(resolver) })
}


#[pymodule]
fn tt(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<SharedResolverCache>()?;
    m.add_class::<PyResolver>()?;
    m.add_class::<PyNeedData>()?;

    m.add_class::<PyTrieIndex>()?;
    m.add_function(wrap_pyfunction!(trie_from_strings, m)?)?;

    m.add_function(wrap_pyfunction!(bytes_to_node, m)?)?;

    m.add_function(wrap_pyfunction!(serial_read_nodes, m)?)?;
    m.add_function(wrap_pyfunction!(single_blob_resolver, m)?)?;

    Ok(())
}
