import React, { useEffect, useState, useMemo, useCallback } from 'react'
import { useParams, useLocation } from 'react-router-dom'

import { Layout } from '../../components/Layout'
import { Loader } from '../../components/Loader'
import { GraphContextProvider } from '../../contexts/graphContext'
import { SearchContextProvider } from '../../contexts/searchContext'
import { maps, getMap, unpackBuildRef, GraphStorage } from '../../api/api'
import { ShareButton } from './ShareButton'
import { MapInfo } from './MapInfo'

import { ActionBar } from './ActionBar'
import { ReferencesDialog } from './ReferencesDialog'
import { Graph } from './Graph'
import { WantNode } from './WantNode'

import styles from './Graph.module.css'
import { get } from 'lodash'

export const MapDashboard = () => {
    const { mapId } = useParams()
    const { state } = useLocation()
    const [repo, setRepo] = useState(null)
    const [build, setBuild] = useState(null)
    const [map, setMap] = useState(null)
    const [error, setError] = useState(null)
    const [graphData, setGraphData] = useState(null)

    let buildRef = map && unpackBuildRef(map.build.path)
    const graphStorage = useMemo(() => new GraphStorage(mapId), [mapId])

    useEffect(() => {
        let to = null;
        let cancel = false;

        const tryGetMap = (attempts = 5, dt = 1000) =>  {
            if (attempts == 0) return;

            let get = getMap(mapId)
                .then(data => {
                    if (data) {
                        setMap(data)
                    } else {
                        console.log('no map data')
                        setError(true)
                    }
                })
                .catch(err => {
                    if (cancel) return;
                    console.error('getMap', err)
                    setError(true)
                    to = setTimeout( () => tryGetMap(attempts - 1, dt * 2), dt )
                })
        }

        if (mapId) {
            tryGetMap()
        }

        return () => {
            cancel = true;
            if (to) clearTimeout(to);
        }
    }, [mapId])

    useEffect(() => {
        if (map) {

            maps.getRepo(buildRef.repo_id)
                .then(resp => {
                    if (resp) {
                        setRepo(resp)
                    } else {
                        setError(true)
                    }
                })
                .catch(() => setError(true))

            maps.getBuild(buildRef.repo_id, buildRef.branch, buildRef.build_id)
                .then(resp => {
                    if (resp) {
                        setBuild(resp)
                    } else {
                        setError(true)
                    }
                })
                .catch((err) => console.error('getBuild', err) || setError(true))

            graphStorage.getData().then((resp) => {
                if (resp) {
                    setGraphData(resp)
                } else {
                    console.error('no graph data')
                    setError(true)
                }
            })

        }
    }, [map, mapId, graphStorage])

    if (error) return (
        <Layout>
            <div className={styles.hint}>
                This map does not exist, or you have no access to it.
            </div>
        </Layout>
    )

    const isFetching = !map || !build || !graphData

    if (isFetching) return <Loader />

    const wantNodes = (state?.addNodes || []).map(nodeRef => <WantNode nodeRef={nodeRef} />)

    return (
        <GraphContextProvider key={mapId} map={map} build={build} graphData={graphData} graphStorage={graphStorage}>
            <SearchContextProvider>
                <Layout
                    searchBar={<ActionBar />}
                    shareButton={<ShareButton />}
                    routeInfo={<MapInfo
                        map={map}
                        repo={repo}
                        branch={buildRef.branch}
                        build={build}
                    />}
                >
                    {wantNodes}
                    <Graph />
                    <ReferencesDialog />
                </Layout>
            </SearchContextProvider>
        </GraphContextProvider>
    )
}
