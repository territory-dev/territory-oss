
import { useCallback } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useParams, useLocation } from 'react-router-dom'

import { Loader } from '../../components/Loader'
import { GraphContextProvider } from '../../contexts/graphContext'
import { Layout } from "../../components/Layout"
import { getPublicMap } from "../../api/api"
import { Graph } from '../MapDashboard/Graph'
import { AnonUserMenu } from '../../components/AnonUserMenu'

import styles from './PublicMap.module.css'


const PublicGraph = ({ data, mapId }) => {
    const { build, repo_id, branch, build_id, graph, code } = data
    const safeBranch = encodeURIComponent(branch)

    const getNode = useCallback((cfg, id) => Promise.resolve(code[id]), [code])

    return (
        <GraphContextProvider
            key={mapId}
            build={build}
            graphData={{ ref: null, ...graph }}
            map={{
                id: mapId,
                build: {
                    path: `repos/${repo_id}/branches/${safeBranch}/builds/${build_id}`,
                }
            }}
            getNodeFunc={getNode}
        >
            <Graph />
        </GraphContextProvider>
    )
}

export const PublicMap = () => {
    const { mapId } = useParams()
    const { pathname } = useLocation()
    const { fetchStatus,  data, error } = useQuery(
        ['publicMap', mapId],
        () => getPublicMap(mapId),
        {
            retry: 0,
        }
    )

    return (
        <Layout
            searchBar={
                <div className={styles.wrapper}>
                    <div className={styles.message}>
                        Log in to explore this map.
                    </div>
                </div>
            }
            userMenu={<AnonUserMenu loginRedirect={pathname} />}
        >
            {data && (
                <PublicGraph data={data} mapId={mapId} />
            )}
            {error && (
                <div className={styles.hint}>
                    This map does not exist, or you have no access to it.
                </div>
            )}
            {fetchStatus === 'fetching' && (
                <Loader />
            )}
        </Layout>
    )
}
