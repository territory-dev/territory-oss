import React from 'react'
import moment from 'moment'
import { useState, useEffect, useContext, useMemo, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { useKey } from '../../hooks/useKey'
import Select from '@mui/material/Select'
import MenuItem from '@mui/material/MenuItem'
import Button from '@mui/material/Button'
import Dialog from '@mui/material/Dialog'
import DialogActions from '@mui/material/DialogActions'
import DialogContent from '@mui/material/DialogContent'
import DialogTitle from '@mui/material/DialogTitle'
import TextField from '@mui/material/TextField'
import Slide from '@mui/material/Slide'
import FormGroup from '@mui/material/FormGroup'
import InputLabel from '@mui/material/InputLabel'
import FormControlLabel from '@mui/material/FormControlLabel'
import Checkbox from '@mui/material/Checkbox'

import { createMap, getBranches, getBuilds, maps } from '../../api/api'
import { UserContext } from '../../contexts/userContext'

import styles from './Dashboard.module.css'

const Transition = React.forwardRef(function Transition(props, ref) {
    return <Slide direction="down" ref={ref} {...props} />;
  })

export const CreateMap = ({ closeModal, mapsList, open, selectedRepo, setSelectedRepo }) => {
    const navigate = useNavigate()
    const {
        user,
    } = useContext(UserContext)

    const [mapName, setMapName] = useState(`Map # ${mapsList?.length + 1}`)
    const [isPublic, setIsPublic] = useState(false)
    const [repos, setRepos] = useState(null)
    const [branches, setBranches] = useState(null)
    const [builds, setBuilds] = useState(null)
    const [selectedBranch, setSelectedBranch] = useState(null)
    const [selectedBuild, setSelectedBuild] = useState(null)

    const reposList = useMemo(() => {
        if (!repos) return null

        return repos.map((doc) => ({
            id: doc.id,
            name: doc.name,
            doc: doc,
        }))
    }, [repos])

    const branchesList = useMemo(() => {
        if (!branches) return null

        return branches.map(({ id, ...rest }) => id)

    }, [branches])

    const buildsList = useMemo(() => {
        if (!builds) return null

        return builds.map((doc) => ({
            id: doc.id,
            ...doc
        }))
        .filter(({ ready }) => ready)
        .sort((a, b) => (a?.ended.seconds > b?.ended.seconds) ? -1 : 1)
    }, [builds])

    const createNewMap = useCallback(() => {
        if (!selectedRepo || !selectedBranch || !selectedBuild) return;

        const selectedRepoData = reposList.find(({ id }) => id === selectedRepo)

        createMap({
            repoId: selectedRepo,
            branchId: selectedBranch,
            buildId: selectedBuild,
            display_name: mapName,
            public: isPublic,
        }).then((newMapId) => navigate(`/maps/${newMapId}`))
    }, [mapName, isPublic, reposList, buildsList, selectedRepo, selectedBuild, navigate])

    useEffect(() => {
        maps.getReposWithBuilds(user.uid).then((resp) => setRepos(resp))
    }, [])

    useEffect(() => {
        if (selectedRepo) return;
        if (reposList?.length) setSelectedRepo(reposList[0].id)
        else setSelectedRepo(null)
    }, [reposList])

    useEffect(() => {
        if (selectedRepo) getBranches(selectedRepo)
            .then((resp) => setBranches(resp))
    }, [selectedRepo])

    useEffect(() => {
        if (branchesList?.length) setSelectedBranch(branchesList[0])
        else setSelectedBranch(null)
    }, [branchesList])

    useEffect(() => {
        if (selectedBranch) getBuilds(selectedBranch, selectedRepo)
            .then((resp) => setBuilds(resp))
    }, [selectedBranch, selectedRepo])

    useEffect(() => {
        if (buildsList?.length) setSelectedBuild(buildsList[0].id)
        else setSelectedBuild(null)
    }, [buildsList])

    useKey('Enter', createNewMap)

    const isCreateButtonDisabled = !mapName || !selectedRepo || !selectedBranch || !selectedBuild

    return (
        <Dialog
            open={open}
            onClose={closeModal}
            TransitionComponent={Transition}
        >
            <DialogTitle>
                Create new map
            </DialogTitle>
            <DialogContent>
                <FormGroup
                    sx={{
                        marginTop: '16px',
                        display: 'flex',
                        flexDirection: 'column',
                        height: '500px',
                        width: '360px',
                        justifyContent: 'space-between',
                    }}
                >
                    <InputLabel id="repository-name-input">
                        Map name
                    </InputLabel>
                    <TextField
                        className='tMapName'
                        onChange={(e) => setMapName(e.target.value)}
                        value={mapName}
                        sx={{ maxWidth: '360px' }}
                    />
                    <InputLabel id="repository-select-label">
                        Repository
                    </InputLabel>
                    <Select
                        className='tRepoName'
                        labelId="repository-select-label"
                        value={selectedRepo || ''}
                        disabled={!selectedRepo}
                        onChange={(e) => setSelectedRepo(e.target.value)}
                        sx={{ maxWidth: '360px' }}
                    >
                        {reposList?.map(({ id, name }) => (
                            <MenuItem key={id} value={id}>{name}</MenuItem>
                        ))}
                    </Select>
                    <InputLabel id="repository-select-branch">
                        Branch
                    </InputLabel>
                    <Select
                        className='tBranch'
                        labelId="repository-select-branch"
                        value={selectedBranch || ''}
                        disabled={!selectedBranch}
                        onChange={(e) => setSelectedBranch(e.target.value)}
                        sx={{ maxWidth: '360px' }}
                    >
                        {branchesList?.map((id) => (
                            <MenuItem key={id} value={id}>{id}</MenuItem>
                        ))}
                    </Select>
                    <InputLabel id="repository-select-commit">
                        Commit
                    </InputLabel>
                    <Select
                        className='tBuild'
                        labelId="repository-select-commit"
                        value={selectedBuild || ''}
                        disabled={!selectedBuild}
                        onChange={(e) => setSelectedBuild(e.target.value)}
                        sx={{ maxWidth: '360px' }}
                    >
                        {buildsList?.map(({ id, commit, commit_message, ended }) => (
                            <MenuItem key={id} value={id}>
                                <span style={{ marginRight: '8px'}}>({commit.substring(0, 7)})</span>
                                {(commit_message.length > 50)
                                    ? commit_message.substring(0, 49).padEnd(51, '..')
                                    : commit_message
                                }
                                {ended && (<span className={styles.commitTime}>{moment(ended).fromNow()}</span>)}
                            </MenuItem>
                        ))}
                    </Select>
                    <FormControlLabel
                        className='tIsPublic'
                        control={
                            <Checkbox
                                checked={isPublic}
                                onChange={(e) => setIsPublic(e.target.checked)}
                            />
                        }
                        label={
                            <>
                                <div>Make public?</div>
                                <div className={styles.subLabel}>(other people will be able to see your map, but not edit it)</div>
                            </>
                        }
                        sx={{ maxWidth: '360px' }}
                    />
                </FormGroup>
            </DialogContent>
            <DialogActions>
                <Button className='tCreate' disabled={isCreateButtonDisabled} onClick={createNewMap}>
                    Create map
                </Button>
                <Button className='tCancel' onClick={closeModal}>
                    Cancel
                </Button>
            </DialogActions>
        </Dialog>
    )
}
