import React, { useState, useContext } from 'react';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Snackbar from '@mui/material/Snackbar';
import Alert from '@mui/material/Alert';

import { Explained } from '../../components/Explained';
import { setUserDisplayName } from '../../api/api'; // Import the new API functions
import { UserContext } from '../../contexts/userContext';


export const AccountDisplayName  = () => {
    const { user, setUser } = useContext(UserContext);
    const [newDisplayName, setNewDisplayName] = useState(user.email.split('@')[0]);
    const [snackbar, setSnackbar] = useState({ open: false, message: '', severity: 'success' });
    const [errors, setErrors] = useState({});

    const handleSetDisplayName = async () => {
        try {
            await setUserDisplayName(newDisplayName);
            setUser({ ...user, displayName: newDisplayName });
            setSnackbar({ open: true, message: 'Display name updated successfully', severity: 'success' });
            setNewDisplayName('');
        } catch (err) {
            let data
            if (err.response && err.response.status === 400 && (data = await err.response.json()) && data.errors) {
                console.log('errors', data.errors)
                setErrors(data.errors)
            } else {
                setSnackbar({ open: true, message: 'Failed to update display name', severity: 'error' });
            }
        }
    };


    return <div>
        <h1>Account settings</h1>
        <h2>Identity</h2>
        <Explained explanation={
            <>
                <p>
                    The name that we should display to other users.
                    Once your display name is set, it cannot be changed.
                </p>
                <p>
                    Must match regular expression: <code>{'/^[a-z0-9-_]{1,30}$/i'}</code>
                </p>
            </>
        }>
            {user.displayName
                ? <div className="tFixedDisplayName">{user.displayName}</div>
                : <Stack direction="row" spacing={2}>
                        <TextField
                            id="new-display-name"
                            label="Display name"
                            variant="filled"
                            value={newDisplayName}
                            onChange={(e) => setNewDisplayName(e.target.value)}
                            onFocus={e => { e.target.select() }}
                            disabled={user.displayName}
                            error={!!errors.displayName}
                            helperText={errors.displayName}
                        />
                        <Button
                            variant="contained"
                            onClick={handleSetDisplayName}
                            disabled={!newDisplayName || user.displayName}
                        >
                            Set
                        </Button>
                    </Stack>}
        </Explained>

        <Snackbar
            open={snackbar.open}
            autoHideDuration={6000}
            onClose={() => setSnackbar({ ...snackbar, open: false })}
        >
            <Alert severity={snackbar.severity} sx={{ width: '100%' }}>
                {snackbar.message}
            </Alert>
        </Snackbar>

    </div>
}
