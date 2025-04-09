import React, { useCallback, useState } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import Stack from '@mui/material/Stack'
import TextField from '@mui/material/TextField'
import Button from '@mui/material/Button'
import Radio from '@mui/material/Radio'
import RadioGroup from '@mui/material/RadioGroup'
import FormControlLabel from '@mui/material/FormControlLabel'
import FormControl from '@mui/material/FormControl'
import FormLabel from '@mui/material/FormLabel'
import Snackbar from '@mui/material/Snackbar'
import Alert from '@mui/material/Alert'
import CircularProgress from '@mui/material/CircularProgress'

import { Explained } from '../../components/Explained'
import styles  from './RepoForm.module.css'

export const RepoForm = ({ user, repoInitial, nameEditable, errors = {}, save }) => {
    const navigate = useNavigate()
    const [repo, setRepo] = useState(repoInitial)
    const [isLoading, setIsLoading] = useState(false)
    const [snackbar, setSnackbar] = useState({ open: false, message: '', severity: 'error' })

    const handleInputChange = (key) => (e) => {
        const { value } = e.target
        setRepo(prevData => ({ ...prevData, [key]: value }))
        // Clear the error for this field when the user starts typing
        // setErrors(prevErrors => ({ ...prevErrors, [id.replace('repo-', '')]: '' }))
    }

    const handlePublicChange = (e) => {
        setRepo(prevData => ({
            ...prevData,
            public: e.target.value === "true"
        }))
    }

    const handleManualChange = useCallback((e) => {
        setRepo(prevData => ({
            ...prevData,
            manual: e.target.value === "true"
        }))
    }, [repo.manual])

    const handleSave = async () => {
        console.log('save clicked')
        if (!save) return
        setIsLoading(true)
        try {
            await save(repo)
        } finally {
            setIsLoading(false)
        }
    }


    if (!user.displayName) {
        return <div>Set a user display name in <Link to="/account">account settings</Link> first.</div>
    }


    return (
        <div>
            <Explained explanation={nameEditable && <>
                <p>Name of the repository.</p>
                <p>Must match the regular expression: <code>{'/^[a-z0-9-_]{1,30}$/i'}</code></p>
            </>}>
                <Stack direction="row">
                    <div className={styles.repoNameUser}>{user.displayName} /</div>
                    {nameEditable
                        ?  <TextField
                            label="Repository name"
                            variant="filled"
                            defaultValue={repo.name}
                            onChange={handleInputChange('name')}
                            error={!!errors.name}
                            helperText={errors.name}
                        />
                        : repo.name}
                </Stack>
            </Explained>

            <Explained explanation={<>
                <p>Sets who can view the repository.</p>
                <p>
                    <strong>Public repositories</strong> can be viewed by any Territory.dev user.
                    Make sure you are allowed to publish the code before making a repository public.
                </p>
                <p>
                    <strong>Private repositories</strong> are accessible by you only.
                </p>
                <p>Private repositories are unavailable at this time.</p>
            </>}>
                <FormControl error={!!errors.public}>
                    <FormLabel>Access</FormLabel>
                    <RadioGroup
                        value={repo.public}
                        onChange={handlePublicChange}
                        name="isPublic"
                    >
                        <FormControlLabel value={true} control={<Radio />} label="Public"/>
                        <FormControlLabel value={false} control={<Radio />} label="Private" disabled />
                    </RadioGroup>
                    {errors.public}
                </FormControl>
            </Explained>

            <Explained explanation={<>
                <p>Language of the repository.  Determines what parser will be used for indexing.</p>
            </>}>
                <FormControl error={!!errors.public}>
                    <FormLabel>Language</FormLabel>
                    <RadioGroup
                        value={repo.lang}
                        onChange={handleInputChange('lang')}
                        name="lang"
                    >
                        <FormControlLabel value="c" control={<Radio />} label="C/C++ (clang)"/>
                        <FormControlLabel value="go" control={<Radio />} label="Go" />
                        <FormControlLabel value="python" control={<Radio />} label="Python" />
                    </RadioGroup>
                    {errors.public}
                </FormControl>
            </Explained>

            <Explained explanation={<>
                <p>Sets how code is ingested for indexing.</p>
                <p>
                    Choose <strong>Uploaded</strong> if you prepared sources outside of Territory
                    and want to upload them using our command line client.  See
                    "<a href="https://github.com/territory-dev/cli?tab=readme-ov-file#uploading-sources-with-the-territorydev-cli-client">
                    Uploading sources with the CLI client
                    </a>".
                </p>
                <p>
                    Choose <strong>Tracked</strong> is you want Territory to track a git repository
                    and build updates automatically.
                </p>
                <p>
                    Adding tracked repositiories is not currently available through the web interface.
                    You can add a build script in our <a href="https://github.com/territory-dev/builds">GitHub repo</a>.
                </p>
            </>}>
                <FormControl error={!!errors.manual}>
                    <FormLabel>Tracking</FormLabel>
                    <RadioGroup
                        value={repo.manual}
                        onChange={handleManualChange}
                        name="isManual"
                    >
                        <FormControlLabel className="tUploaded" value={true} control={<Radio />} label="Uploaded" />
                        <FormControlLabel className="tTracked" value={false} control={<Radio />} label="Tracked" disabled />
                    </RadioGroup>
                    {errors.manual}
                </FormControl>
            </Explained>

            {!repo.manual && <>

                <Explained explanation={<>
                    <p>Git repository to pull code from.  Only Github repositories are allowed at this time.</p>
                </>}>
                    <TextField
                        label="Git origin"
                        variant="filled"
                        value={repo.origin}
                        onChange={handleInputChange('origin')}
                        error={!!errors.origin}
                        helperText={errors.origin}
                    />
                </Explained>

                <Explained explanation={<>
                    <p>
                        Branch of the repository to index.
                        New changes from this branch will be fetched periodically.
                    </p>
                </>}>
                    <TextField
                        label="Tracked branch"
                        variant="filled"
                        value={repo.tracked_branch}
                        onChange={handleInputChange('tracked_branch')}
                        error={!!errors.tracked_branch}
                        helperText={errors.tracked_branch}
                    />
                </Explained>

                <Explained explanation={<>
                    <p>
                        The docker image provides an environment for the build.
                        See <a href="https://github.com/territory-dev/builds">build documentation</a> to
                        learn about requirements for the image and examples.
                    </p>
                </>}>
                    <TextField
                        label="Docker image"
                        variant="filled"
                        value={repo.image}
                        onChange={handleInputChange('image')}
                        error={!!errors.image}
                        helperText={errors.image}
                    />
                </Explained>

                <Explained explanation={<>
                    <p>
                        We need a <code>compile_commands.json</code> file to index your C/C++ codebase.
                        This script will be executed after cloning the repo and should generate it.
                        You can find examples and help on how to write the script in
                        the <a href="https://github.com/territory-dev/builds">build documentation</a>.
                    </p>
                </>}>
                    <TextField
                        label="Build script"
                        variant="filled"
                        multiline
                        minRows={3}
                        value={repo.prepare_script}
                        onChange={handleInputChange('prepare_script')}
                        error={!!errors.prepare_script}
                        helperText={errors.prepare_script}
                    />
                </Explained>
            </>}

            <Button
                variant="contained"
                className='tSaveRepo'
                onClick={handleSave}
                disabled={isLoading}
                startIcon={isLoading ? <CircularProgress size={20} /> : null}
            >
                {isLoading ? 'Saving...' : 'Save'}
            </Button>

            <Snackbar
                open={snackbar.open}
                autoHideDuration={6000}
                onClose={() => setSnackbar(prev => ({ ...prev, open: false }))}
            >
                <Alert onClose={() => setSnackbar(prev => ({ ...prev, open: false }))} severity={snackbar.severity} sx={{ width: '100%' }}>
                    {snackbar.message}
                </Alert>
            </Snackbar>
        </div>
    );
};
