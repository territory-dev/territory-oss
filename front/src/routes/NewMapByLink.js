import { useNavigate, useSearchParams } from "react-router-dom";
import { useState } from "react";

import { Layout } from '../components/Layout'
import { createMap, GraphStorage } from '../api/api'
import { GraphStorage as LocalGraphStorage } from "../local/local";



export const NewMapByLink = () => {
    const [searchParams] = useSearchParams()
    const [createError, setCreateError] = useState(null);
    const navigate = useNavigate()

    const repoId = searchParams.get('repo_id')
    const branch = searchParams.get('branch')
    const buildId = searchParams.get('build_id')
    const ref = searchParams.get('ref')
    const graph = searchParams.get('graph')

    createMap({
        repoId,
        branchId: branch,
        buildId,
        public: false,
        display_name: 'New map',
    })
    .then((newMapId) => {
        return new LocalGraphStorage(repoId, branch, buildId).getGraph().then(storedGraphData => {
            const gs = new GraphStorage(newMapId)
            return gs.setGraph(storedGraphData)
                .then(() => newMapId)
        })
    })
    .then((newMapId) => {
        const state = {
            addNodes: ref ? [ ref ] : [],
        }
        navigate(`/maps/${newMapId}`, { state })
    })
    .catch((e) => { setCreateError(true) })

    return (
        <Layout scrollable>
            {createError
                ? <div>Map creation failed, please try again later.</div>
                : <div>Creating map...</div>}
        </Layout>
    )
}
