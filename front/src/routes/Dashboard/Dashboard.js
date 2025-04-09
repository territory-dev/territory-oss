import React, { useContext, useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'

import { Loader } from '../../components/Loader'
import { Layout } from '../../components/Layout'
import { DashboardHeader } from '../../components/DashboardHeader'
import { MapsContext } from '../../contexts/mapsContext'
import { UserContext } from '../../contexts/userContext'

import { MapsList } from './MapsList'
import { CreateMap } from './CreateMap'
import { RemoveMap } from './RemoveMap'
import { CreateMapButton } from './CreateMapButton'
import { RepoCatalog } from './RepoCatalog'
import { QuickBuildForm } from './QuickBuildForm'
import { repoMapDefaults } from '../../utils/newMapUtils'

export const Dashboard = () => {
    const {
        user,
        authDisabled,
    } = useContext(UserContext)

    return (user)
        ? <UserDasboard />
        : <PublicDashboard authDisabled={authDisabled} />;
}


export const PublicDashboard = ({ authDisabled }) => {
    const navigate = useNavigate()

    const createMap = async (repoId, repo) => {
        const {repoId: _, branch, buildId} = await repoMapDefaults(repoId);

        const encBranch = encodeURIComponent(branch)
        navigate(`/maps/local/${repoId}/${encBranch}/${buildId}`)
    }

    return (
        <Layout scrollable legalFooter>
            {authDisabled || <QuickBuildForm />}
            <RepoCatalog createMap={createMap}/>
        </Layout>
    )
}


export const UserDasboard = () => {
    const {
        mapsList,
        mapsLoading,
        reloadMaps,
    } = useContext(MapsContext)

    const [isModalOpen, setIsModalOpen] = useState(false)
    const [mapToRemove, setMapToRemove] = useState(null)
    const [selectedRepo, setSelectedRepo] = useState(null)

    useEffect(() => { reloadMaps() }, [])

    const createMap = (repoId) => {
        setSelectedRepo(repoId)
        setIsModalOpen(true)
    }

    return (
        <Layout scrollable legalFooter>
            <QuickBuildForm createMap={createMap} />
            <DashboardHeader>My maps</DashboardHeader>
            <CreateMapButton setIsModalOpen={setIsModalOpen}></CreateMapButton>
            {mapsLoading
                ? <Loader />
                : (mapsList.length == 0)
                    ? null
                    : <MapsList maps={mapsList} setRemoveMap={setMapToRemove} />
            }
            <RepoCatalog createMap={createMap}/>
            {isModalOpen && (
                <CreateMap
                    closeModal={() => setIsModalOpen(false)}
                    mapsList={mapsList}
                    open={isModalOpen}
                    selectedRepo={selectedRepo}
                    setSelectedRepo={setSelectedRepo}
                />
            )}
            {mapToRemove ? (
                <RemoveMap
                    close={() => setMapToRemove(null)}
                    reloadMaps={reloadMaps}
                    mapToRemove={mapToRemove}
                />
            ) : null}
        </Layout>
    )
}
