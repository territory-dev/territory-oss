import init, { decode_trie_index } from 'territory_core'
import { auth } from '../firebase'

const {
    REACT_APP_BUILD_ENDPOINT: BUILD_ENDPOINT,
} = process.env

const core = init();

const searchIndexCache = {}

export const getIndexKey = cfg => `${cfg.backend}/${cfg.bucket||cfg.root_url}/${cfg.trie}`

const fetchIndex = async (cfg) => {
    const url = `${BUILD_ENDPOINT}/search-blob` +
        `?repo_id=${encodeURIComponent(cfg.repo_id)}`+
        `&branch=${encodeURIComponent(cfg.branch)}`+
        `&build_id=${encodeURIComponent(cfg.build_id)}`

    let headers = {}
    if (auth.currentUser) {
        const token = await auth.currentUser.getIdToken()
        headers['Authorization'] = `Bearer ${token}`
    }

    const resp = await fetch(url, { headers })

    if (resp.status >= 300) {
        const text = await resp.text()
        throw Error(`resolver request error ${ resp.status }: ${ text }`)
    }
    const buffer = await resp.arrayBuffer()
    return decode_trie_index(buffer)
}


const loadIndex = (cfg) => {
    if (cfg.trie) {
        const key = getIndexKey(cfg)
        if (!searchIndexCache[key]) {
            searchIndexCache[key] = core
                .then(() => fetchIndex(cfg))
                .then((index) => {
                    postMessage({ t: 'indexLoaded', key })
                    return index
                })
        }
        return searchIndexCache[key]
    } else {
        throw Error("non-trie index unsupported")
    }
}

let pendingQuery = null;

const query = (cfg, q, key, limit = 10) => {
    loadIndex(cfg)
    .then(idx => {
        const res = idx.search(q, {limit})
        postMessage({t: 'result', cfg, q, key, data: res})
    })
}


const scheduleQuery = (cfg, q, key, limit) => {
    pendingQuery = [cfg,q,key, limit]
    setTimeout(() => {
        if (pendingQuery) {
            let [cfg,q,key, limit] = pendingQuery;
            pendingQuery = null;
            query(cfg, q ,key, limit);
        }
    })
}


onmessage = (ev) => {
    let msg = ev.data;

    switch (msg.t) {
        case 'loadIndex':
            loadIndex(msg.cfg)
            return;

        case 'query':
            scheduleQuery(msg.cfg, msg.q, msg.key, msg.limit)
            return;

        default:
            console.error('unkown message', msg);
            return;
    }
};
