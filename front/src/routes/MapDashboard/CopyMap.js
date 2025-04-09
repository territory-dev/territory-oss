import { useContext } from 'react'
import Button from '@mui/material/Button'
import { useMutation } from '@tanstack/react-query'
import { useNavigate } from 'react-router-dom'

import { GraphContext } from '../../contexts/graphContext'
import { UserContext } from '../../contexts/userContext'
import { createMap, unpackBuildRef } from '../../api/api'
import { Loader } from '../../components/Loader'

import styles from './CopyMap.module.css'

export const CopyMap = () => {
    const navigate = useNavigate()
    const { user } = useContext(UserContext)
    const { nodes, relations, map } = useContext(GraphContext)

    const { repo_id, branch, build_id } = unpackBuildRef(map.build.path)

    const copyQuery = useMutation({
        mutationKey: 'copyMap',
        mutationFn: () => createMap({
            buildId: build_id,
            branchId: branch,
            repoId: repo_id,
            display_name: map.display_name + ' (copy)',
            last_changed: new Date(),
            public: true,
            graph: { nodes, relations }
        }),
        onSuccess: (newMapId) => navigate(`/maps/${newMapId}`),
    })

    return (
        <div className={styles.wrapper}>
            <div className={styles.message}>
                You are not the owner of this map and can only view it. Copy it to make changes.
            </div>
            {copyQuery.isLoading ? (
                <Loader size="sm" className={styles.loader} />
            ) : (
                <Button onClick={copyQuery.mutate}>Copy</Button>
            )}
        </div>
    )
}
