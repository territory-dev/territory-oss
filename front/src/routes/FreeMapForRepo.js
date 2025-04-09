import { useEffect, useState } from "react"
import { Navigate, useParams } from "react-router-dom"

import { maps } from '../api/api'


export const FreeMapForRepo = () => {
    const { repoId } = useParams()
    const [created, setCreated] = useState()

    const redirectToLocalMap = async () => {
        const repo = await maps.getRepo(repoId)
        const branch = repo.defaultBranch || (await maps.getBranches(repoId))[0].id
        const builds = await maps.getBuilds(repoId, branch)
        const buildId = builds[0].id

        const encBranch = encodeURIComponent(branch)
        setCreated(`/maps/local/${repoId}/${encBranch}/${buildId}`)
    }
    useEffect(() => {
        redirectToLocalMap()
    }, [repoId])

    return created ? <Navigate to={created} replace={true} /> : <div>Creating a map...</div>
}
