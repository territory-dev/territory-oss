import React, { useState } from 'react'

import { useQuery } from '@tanstack/react-query'
import Box from '@mui/material/Box'
import List from '@mui/material/List'
import ListItem from '@mui/material/ListItem'
import IconButton from '@mui/material/IconButton'
import DeleteIcon from '@mui/icons-material/Delete'
import AddIcon from '@mui/icons-material/Add';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Divider from '@mui/material/Divider'
import ListItemText from '@mui/material/ListItemText'

import moment from 'moment'

import { SettingsLayout } from '../../components/SettingsLayout'
import { maps } from '../../api/api'

import styles from './UploadTokenDashboard.module.css'


const CreateToken = ({onCreate}) => {
    const [open, setOpen] = React.useState(false);
    const [gotToken, setGotToken] = React.useState(null);

    const handleClickOpen = () => {
        setOpen(true)
    };

    const handleClose = () => {
        setOpen(false)
        setGotToken(null)
    };

    return <React.Fragment>
        <Button
            variant="contained"
            onClick={() => handleClickOpen()}
        >
            <AddIcon />
            New upload token
        </Button>

        <Dialog
            open={open}
            onClose={handleClose}
            maxWidth="md"
            fullWidth={!!gotToken}
            PaperProps={{
                component: 'form',
                onSubmit: (event) => {
                    event.preventDefault()
                    const formData = new FormData(event.currentTarget)
                    const formJson = Object.fromEntries(formData.entries())
                    const display_name = formJson.name
                    maps.createUploadToken({display_name})
                    .then((resp) => {
                        setGotToken(resp.upload_token)
                        if (onCreate) onCreate()
                    });
                },
            }}
        >
            <DialogTitle>Create upload token</DialogTitle>
            { gotToken
                ? <>
                    <DialogContent>
                        <TextField
                            className="tUploadToken"
                            label="Upload Token"
                            margin="dense"
                            defaultValue={gotToken}
                            fullWidth
                            autoFocus
                            multiline
                            InputProps={{
                                readOnly: true,
                            }}
                        />
                    </DialogContent>
                    <DialogActions>
                        <Button onClick={handleClose}>Close</Button>
                    </DialogActions>
                </>
                : <>
                    <DialogContent>
                        <TextField
                            autoFocus
                            required
                            margin="dense"
                            id="name"
                            name="name"
                            label="Token name"
                            fullWidth
                            variant="standard"
                        />
                    </DialogContent>
                    <DialogActions>
                        <Button onClick={handleClose}>Cancel</Button>
                        <Button className="tCreateToken" type="submit">Create</Button>
                    </DialogActions>
                </>}
        </Dialog>
    </React.Fragment>
}


export const UploadTokensDashboard = () => {
    const [removingToken, setRemovingToken] = useState(null)

    const { data, refetch } = useQuery(
        ['uploadTokens'],
        () => maps.getUploadTokens())

    const initiateRemove = (tokenId) => {
        setRemovingToken(tokenId)
        maps.removeUploadToken(tokenId).then(res => {
            return refetch()
        }).then(() => {
            setRemovingToken(null)
        })
    }

    const onCreate = () => {
        return refetch()
    }

    const tokenList = (data && data.length)
        ? <Box sx={{ width: '100%', marginTop: '40px', marginBottom: '40px', bgcolor: 'background.paper' }}>
            <List className={styles.tokenList}>
                {data && data.map((tok, i) => <div key={tok.id}>
                    { i != 0 && <Divider key={'divider-' + tok.id} /> }
                    <ListItem
                        secondaryAction={
                            (removingToken !== tok.id) &&
                            <IconButton
                                edge="end"
                                aria-label="delete"
                                onClick={() => initiateRemove(tok.id)}
                            >
                                <DeleteIcon />
                            </IconButton>
                        }
                    >
                        <ListItemText>
                            <dl>
                                <dt>Host</dt>
                                <dd>{tok.display_name}</dd>

                                <dt>Token ID</dt>
                                <dd className='tTokenId'>{tok.userId}:{tok.id}</dd>

                                <dt>Created</dt>
                                <dd>{moment(tok.created_at).fromNow()}</dd>

                                <dt>Last used</dt>
                                <dd>{tok.last_used ? moment(tok.last_used).fromNow() : "Never"}</dd>
                            </dl>
                        </ListItemText>
                    </ListItem>
                </div>)}
            </List>
        </Box>
        : <div>No upload tokens have been created yet</div>

    return <SettingsLayout selectedRoute="upload-tokens">
        <h1>Upload tokens</h1>

        <CreateToken onCreate={() => onCreate()} />

        {tokenList}
    </SettingsLayout>
}

