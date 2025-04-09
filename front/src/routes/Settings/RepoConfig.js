import { useState, useContext, useCallback } from 'react'
import { useParams } from 'react-router-dom'
import { useNavigate } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import pick from 'lodash/pick'

import { UserContext } from '../../contexts/userContext'
import { SettingsLayout } from '../../components/SettingsLayout'
import { RepoForm } from './RepoForm'
import { maps } from '../../api/api'


const REPO_DEFAULTS = {
    name: '',
    public: true,
    origin: '',
    tracked_branch: '',
    image: '',
    prepare_script: '#!/bin/bash\n\nbash .territory/prepare.sh',
    manual: true,
    lang: 'c',
}


export const RepoConfig = () => {
    const {
        user,
        setUser,
    } = useContext(UserContext)

    const { repoId } = useParams()

    const { data } = useQuery(
        ['repo', repoId],
        () => maps.getRepo(repoId),
        { retry: 0, }
    )

    if (!user.account.canCreateRepos) return unavailable

    return <SettingsLayout selectedRoute="buildConfig" selectedRepo={repoId}>
        <h1>Repository configuration</h1>

        { data && <RepoForm user={user} repoInitial={data} /> }
    </SettingsLayout>
}


export const NewRepo = () => {
    const {
        user,
        setUser,
    } = useContext(UserContext)
    const navigate = useNavigate()
    const [errors, setErrors] = useState({})
    const save = useCallback(async (repo) => {
        console.log('save', repo)
        try {
            const repo_ = repo.manual ? pick(repo, ['name', 'public', 'manual', 'lang']) : repo
            const newRepo = await maps.createRepo(repo_)
            // setSnackbar({ open: true, message: 'Repository created successfully', severity: 'success' })
            navigate(`/repos/${newRepo.id}/jobs`)
        } catch (err) {
            console.log('save err', err)
            let data
            if (err.response && err.response.status === 400 && (data = await err.response.json()) && data.errors) {
                setErrors(data.errors)
                // setSnackbar({ open: true, message: 'Please correct the errors in the form', severity: 'error' })
            } else {
                // setSnackbar({ open: true, message: 'Failed to create repository. Please try again.', severity: 'error' })
            }
        }
    })

    if (!user.account.canCreateRepos) return unavailable

    return <SettingsLayout selectedRoute="newBuild">
        <h1>Configure a new repository</h1>

        <RepoForm user={user} save={save} repoInitial={REPO_DEFAULTS} errors={errors} nameEditable />
    </SettingsLayout>
}



const unavailable = <SettingsLayout><div>Feature currently unavailable</div></SettingsLayout>
