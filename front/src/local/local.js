export class GraphStorage {

    constructor(repo_id, branch, build_id) {
        this.key = `repos/${repo_id}/branches/${encodeURIComponent(branch)}/builds/${build_id}/graph`
        const raw = localStorage.getItem(this.key)
        if (raw)
            this.data = JSON.parse(raw)
        else
            this.data = {
                nodes: {},
                relations: {},
            }
    }

    getGraphKey() {
        return this.key
    }

    async getGraph() {
        return this.data
    }

    async setGraph(data) {
        Object.entries(data).forEach(([k, v]) => {
            walkKey(this.data, k, (obj, tk) => {
                if (v === null)
                    delete obj[v]
                else
                    obj[tk] = v
            })
        })
        this._saveData()
    }

    async deleteNode(nodeId) {
        delete this.data.nodes[nodeId]
        this._saveData()
    }

    _saveData() {
        localStorage.setItem(this.key, JSON.stringify(this.data))
    }

}


const walkKey = (data, key, onTerminal) => {
    let i, kp, o = data
    const keyParts = key.split('.')
    for (i = 0; i < keyParts.length - 1; ++i) {
        kp = keyParts[i]
        if (!o[kp]) o[kp] = {}
        o = o[kp]
    }
    onTerminal(o, keyParts[keyParts.length-1])
}
