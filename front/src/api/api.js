import EventEmitter from 'eventemitter3'
import init, { decode_node, decode_references } from 'territory_core'
import { getIndexKey } from './searchWorker'
import { auth } from '../firebase'

export const {
    REACT_APP_RESOLVER_ENDPOINT: RESOLVER_ENDPOINT,
    REACT_APP_PUBLIC_MAP_ENDPOINT: PUBLIC_MAP_ENDPOINT,
    REACT_APP_MAPS_ENDPOINT: MAPS_ENDPOINT,
    REACT_APP_TRACK_ENDPOINT: TRACK_ENDPOINT,
    REACT_APP_BUILDS_ENDPOINT: BUILDS_ENDPOINT,
    REACT_APP_USE_FIRESTORE_EMULATOR: USE_FIRESTORE_EMULATOR,
} = process.env

const core = init()

if (window._searchWorker)
    window._searchWorker.terminate()
const searchWorker = window._searchWorker = new Worker(new URL('./searchWorker.js', import.meta.url))

export const ApiEvents = new EventEmitter()
let runningReqeuests = 0
const requestStarted = () => {
    if (runningReqeuests === 0)
        ApiEvents.emit('activityStarted')
    runningReqeuests++
}
const requestEnded = () => {
    runningReqeuests--
    if (runningReqeuests === 0)
        ApiEvents.emit('activityCeased')
}
export const isActive = () => (runningReqeuests !== 0)

export const unpackBuildRef = (ref) => {
    const match = ref.match(/repos\/([^/]+)\/branches\/([^/]+)\/builds\/([^/]+)/)
    return {
        repo_id: match[1],
        branch: decodeURIComponent(match[2]),
        build_id: match[3],
    }
}

export const maps = {
    forceRefresh: false,

    async request(method, resource, body) {
        requestStarted()

        let headers = {};
        if (auth.currentUser) {
            const token = await auth.currentUser.getIdToken(this.forceRefresh)
            headers['Authorization'] = `Bearer ${token}`
            this.forceRefresh = false;
        }

        let req = { method, headers }
        if (body) {
            req.body = JSON.stringify(body)
            req.headers['Content-Type'] = 'application/json'
        }
        let response
        try {
            response = await fetch(
                `${MAPS_ENDPOINT}/${resource}`,
                req)
        } finally {
            requestEnded()
        }
        if (!response.ok) {
            const err = new Error(`reqeuest to maps API failed: ${response.status}`)
            err.response = response
            throw err
        }

        if (method !== 'DELETE')
            return await response.json()
    },

    async getMaps() {
        return await this.request('GET', 'maps')
    },
    async getRepos() {
        return await this.request('GET', 'repos')
    },
    async getRepo(repoId) {
        return await this.request('GET', `repos/${repoId}`)
    },
    async getOwnedRepos() {
        return await this.request('GET', 'repos?f=owned')
    },
    async getReposWithBuilds() {
        return await this.request('GET', 'repos?f=hasBuilds')
    },
    async getBranches(repoId) {
        return await this.request('GET', `repos/${repoId}/branches`)
    },
    async getBuilds(repoId, branch) {
        branch = encodeURIComponent(branch)
        return await this.request('GET', `repos/${repoId}/branches/${branch}/builds`)
    },
    async getBuild(repoId, branch, buildId) {
        branch = encodeURIComponent(branch)
        return await this.request('GET', `repos/${repoId}/branches/${branch}/builds/${buildId}`)
    },
    async getMap(mapId) {
        return await this.request('GET', `maps/${mapId}`)
    },
    async createMap(mapData) {
        return await this.request('POST', 'maps', mapData)
    },
    async deleteMap(mapId) {
        return await this.request('DELETE', `maps/${mapId}`)
    },
    async updateMapPublic(mapId, data) {
        return await this.request('PUT', `maps/${mapId}/public`, data)
    },
    async updateDisplayName(mapId, data) {
        return await this.request('PUT', `maps/${mapId}/display_name`, data)
    },
    async setUserDisplayName(data) {
        const result = await this.request('POST', 'account/display-name', data)
        this.forceRefresh = true;
        return result
    },
    async getAccount() {
        return await this.request('GET', 'account')
    },
    async deleteAccount() {
        return await this.request('DELETE', 'account')
    },
    async createRepo(data) {
        return await this.request('POST', 'repos', data)
    },
    async getUploadTokens() {
        return await this.request('GET', 'upload-tokens')
    },
    async createUploadToken(data) {
        return await this.request('POST', 'upload-tokens', data)
    },
    async removeUploadToken(id) {
        return await this.request('DELETE', `upload-tokens/${id}`)
    },
    async requestImmediateBuild(data) {
        return await this.request('POST', 'build-request-immediate', data)
    },
}


export const getMap = async (mapId) => {
    return await maps.getMap(mapId)
}

export const getMaps = async (userId) => {
    return await maps.getMaps()
}

export const getBranches = async (repoId) => {
    return await maps.getBranches(repoId)
}

export const getBuilds = async(branchId, repoId) => {
    return await maps.getBuilds(repoId, branchId)
}


export const getBuild = async (repoId, branch, buildId) => {
    return maps.getBuild(repoId, branch, buildId)
 }


export const deleteMap = async (mapId) => {
    await maps.deleteMap(mapId)
}


export class GraphStorage {
    constructor(mapId) {
        this.mapId = mapId
    }

    getGraphKey() {
        return this.mapId
    }

    async getData() {
    }

    async setGraph(data) {
        await maps.request('PATCH', `maps/${this.mapId}/graph`, data)
    }

    async deleteNode(nodeId) {
        nodeId = encodeURIComponent(nodeId)
        await maps.request('DELETE', `maps/${this.mapId}/graph/nodes/${nodeId}`)
    }

}

export const updateMapPublic = async ({ mapId, isPublic }) => {
    await maps.updateMapPublic(mapId, { public: isPublic })
}

export const updateMapName = async ({ mapId, name }) => {
    await maps.updateDisplayName(mapId, { display_name: name });
}


export const createMap = async (mapData) => {
    let result = await maps.createMap(mapData)
    return result.id
}

export const setUserDisplayName = async (displayName) => {
    return await maps.setUserDisplayName({displayName})
}

export const deleteAccount = async () => {
    return await maps.deleteAccount()
}

export const getRepos = async (userId) => {
    return await maps.getRepos()
}

const getProto = (url)  => {
    requestStarted();

    let response;
    const headers = {
        'Accept': 'application/x-protobuf',
    }
    if (auth.currentUser) {
        response = auth.currentUser.getIdToken().then(idToken => {
            headers['Authorization'] = `Bearer ${idToken}`
            return fetch(url, { headers })
        })
    } else {
        response = fetch(url, { headers })
    }


    return response
        .finally(requestEnded)
        .then(resp => {
            if (resp.status >= 300) {
                return resp.text()
                .then(text => {
                    throw Error(`resolver request error ${ resp.status }: ${ text }`)
                })
            } else {
                return resp.arrayBuffer()
            }
        })
}


export const getReferences = (codeStorageConfig, r) => core.then(() => {
    return getProto(
            `${RESOLVER_ENDPOINT}?action=relay&url=${r}`+
            `&repo_id=${encodeURIComponent(codeStorageConfig.repo_id)}`+
            `&branch=${encodeURIComponent(codeStorageConfig.branch)}`+
            `&build_id=${encodeURIComponent(codeStorageConfig.build_id)}`)
        .then(proto_data => decode_references(proto_data))
})


const resolverUrl = (storageConfig, id) =>
    `${RESOLVER_ENDPOINT}?action=relay&url=${id}`+
    `&repo_id=${encodeURIComponent(storageConfig.repo_id)}`+
    `&branch=${encodeURIComponent(storageConfig.branch)}`+
    `&build_id=${encodeURIComponent(storageConfig.build_id)}`


const doGetNode = (storageConfig, url) => {
    if (storageConfig.backend !== 'firebase+relay') {
        throw new Error("unsupported storage backend: " + storageConfig.backend)
    }
    return getProto(url)
        .then(proto_data => core.then(() => decode_node(proto_data)))
}


const nodesCache = {}
export const getNode = (codeStorageConfig, id) => {
    const url = resolverUrl(codeStorageConfig, id)

    if (!nodesCache[url]) {
        nodesCache[url] = doGetNode(codeStorageConfig, url);
    }

    return nodesCache[url]
}

export const getSearchIndex = (cfg) => {
    const key = getIndexKey(cfg)
    let indexLoaded
    const listener = (ev) => {
        if (ev.data.t === 'indexLoaded' && ev.data.key === key) {
            indexLoaded({ data: 'true' })
            searchWorker.removeEventListener('message', listener)
        }
    }
    const promise = new Promise((resolve, reject) => {
        indexLoaded = resolve
    })
    searchWorker.addEventListener('message', listener)
    searchWorker.postMessage({t: 'loadIndex', cfg: cfg})

    return promise
}

export const getSearchQuery = (query, limit, cfg) => {
    let queryResolved
    const listener = (ev) => {
        if (ev.data.t === 'result' && ev.data.q === query) {
            queryResolved(ev.data.data)
            searchWorker.removeEventListener('message', listener)
        }
    }
    const promise = new Promise((resolve, reject) => {
        queryResolved = resolve
    })

    searchWorker.addEventListener('message', listener)
    searchWorker.postMessage({ t: 'query', cfg, q: query, key: getIndexKey(cfg), limit })

    return promise
}

export const getPublicMap = (id) => fetch(`${PUBLIC_MAP_ENDPOINT}?mapId=${id}`, {
    headers: {
        accept: 'application/json'
    }
}).then(resp => resp.json())


export const getBuildJobs = (repo_id) =>
    auth.currentUser.getIdToken()
    .then(idToken => fetch(
        `${BUILDS_ENDPOINT}/builds/?repo_id=${repo_id}`,
        {
            headers: { 'Authorization': `Bearer ${idToken}` },
        }))
    .then(resp => resp.json())


export const getBuildJobLog = (log_url) =>
    auth.currentUser.getIdToken()
    .then(idToken => fetch(
        `${BUILDS_ENDPOINT}${log_url}`,
        {
            headers: { 'Authorization': `Bearer ${idToken}` },
        }))
    .then(resp => resp.json())
