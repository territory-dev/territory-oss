import React, { useEffect, useState, useMemo, useContext } from 'react'
import { useParams, useLocation, Navigate, useSearchParams } from 'react-router-dom'

import { Layout } from '../../components/Layout'
import { Loader } from '../../components/Loader'
import { UserMenu } from '../../components/UserMenu'
import { GraphContextProvider } from '../../contexts/graphContext'
import { SearchContextProvider } from '../../contexts/searchContext'
import { UserContext } from '../../contexts/userContext'
import { maps } from '../../api/api'
import { GraphStorage } from '../../local/local'

import { ReferencesDialog } from '../MapDashboard/ReferencesDialog'
import { Graph } from '../MapDashboard/Graph'
import { WantNode } from '../MapDashboard/WantNode'
import styles from '../MapDashboard/Graph.module.css'
import { SearchBar } from '../MapDashboard/SearchBar'

import { LoginModal } from './LoginModal'
import { UnsavedMapInfo } from './UnsavedMapInfo'

export const FreeMap = () => {
    const [searchParams, doSetSearchParams] = useSearchParams()
    const [isLoginModalOpened, doSetIsLoginModalOpened] = useState(searchParams.get('showLogin') === '1')
    const setIsLoginModalOpened = (u) => {
        doSetSearchParams( u ? {showLogin:'1'} : {}, {replace: true} );
        doSetIsLoginModalOpened(u);
    }

    const { user, authDisabled } = useContext(UserContext)

    const { repoId: repo_id, branch, buildId: build_id } = useParams()
    const graphStorage = useMemo(
        () => new GraphStorage(repo_id, branch, build_id),
        [repo_id, branch, build_id])

    const { state } = useLocation()
    const [repo, setRepo] = useState(null)
    const [build, setBuild] = useState(null)
    const [error, setError] = useState(null)
    const [graphData, setGraphData] = useState(null)

    const { pathname } = useLocation()


    useEffect(() => {
        maps.getRepo(repo_id)
            .then(resp => {
                if (resp) {
                    setRepo(resp)
                } else {
                    setError(true)
                }
            })
            .catch(() => setError(true))

        maps.getBuild(repo_id, branch, build_id)
            .then(resp => {
                if (resp) {
                    setBuild(resp)
                } else {
                    setError(true)
                }
            })
            .catch(() => setError(true))

        graphStorage.getGraph().then(g => setGraphData(g))

    }, [repo_id, branch, build_id, graphStorage])

    const want_ref = searchParams.get('ref');
    if (want_ref) {
        graphStorage.setGraph({
            nodes: {
                [want_ref]: { top: 0, left: 0 },
            },
            relations: [],
        });
        // remove ?ref=...
        doSetSearchParams( isLoginModalOpened ? {showLogin:'1'} : {}, {replace: true} );
    }
    if (user) return <Navigate to={`/public/maps/new?repo_id=${repo_id}&branch=${encodeURIComponent(branch)}&build_id=${build_id}`} />

    if (error) return (
        <Layout
            searchBar={
                <div className={styles.wrapper}>
                    <div className={styles.message}>
                        Log in to explore this map.
                    </div>
                </div>
            }
            routeInfo={<div />}
            userMenu={<UserMenu loginRedirect={pathname} />}
        >
            <div className={styles.hint}>
                This map does not exist, or you have no access to it.
            </div>
        </Layout>
    )

    if (!build) return <Loader />

    const wantNodes = (state?.addNodes || []).map(nodeRef => <WantNode nodeRef={nodeRef} />)

    const buildPath = `repos/${repo_id}/branches/${encodeURIComponent(branch)}/builds/${build_id}`
    const map = {
        free: true,
        build: {
            path: buildPath,
            ...build
        },
        repo_name: repo?.name,
    }
    return (
        <GraphContextProvider key={map.build.path} map={map} build={build} graphData={graphData} graphStorage={graphStorage}>
            <SearchContextProvider>
                <Layout
                    searchBar={<SearchBar />}
                    routeInfo={
                        <UnsavedMapInfo
                            repo={repo}
                            branch={branch}
                            build={build}
                            onSavePropmptClick={() => setIsLoginModalOpened(true)}
                        />
                    }
                    userMenu={
                        <UserMenu
                            loginRedirect={pathname}
                            onLogin={() => setIsLoginModalOpened(true)}
                        />
                    }
                >
                    {wantNodes}
                    <Graph />
                    <ReferencesDialog />
                    {authDisabled || <LoginModal
                        isOpened={isLoginModalOpened}
                        setIsOpened={setIsLoginModalOpened}
                        repoId={repo_id}
                        branch={branch}
                        buildId={build_id}
                    />}
                </Layout>
            </SearchContextProvider>
        </GraphContextProvider>
    )
}
