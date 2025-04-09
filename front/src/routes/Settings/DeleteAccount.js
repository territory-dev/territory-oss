import React, { useState, useContext } from 'react';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogContentText from '@mui/material/DialogContentText';
import DialogTitle from '@mui/material/DialogTitle';

import { Explained } from '../../components/Explained';
import { deleteAccount } from '../../api/api'; // Import the new API functions
import { UserContext } from '../../contexts/userContext';


export const DeleteAccount = () => {
    const { user, setUser } = useContext(UserContext)
    const [newDisplayName, setNewDisplayName] = useState(user.email.split('@')[0])
    const [snackbar, setSnackbar] = useState({ open: false, message: '', severity: 'success' })
    const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)

    const handleDeleteAccount = async () => {
        try {
            await deleteAccount()
            // Handle successful account deletion (e.g., logout and redirect)
            setSnackbar({ open: true, message: 'Account deleted successfully', severity: 'success' })
            // You might want to redirect the user or clear the session here
        } catch (error) {
            setSnackbar({ open: true, message: 'Failed to delete account', severity: 'error' })
        }
        setDeleteDialogOpen(false)
    }


    return <div>
        <h2>Delete account</h2>
        <Explained explanation={
            <>
                <p>
                    This action will delete your account and all associated data,
                    including public repositories and maps.
                    The deletion cannot be undone.
                </p>
            </>
        }>
            <Button
                variant="contained"
                color="warning"
                onClick={() => setDeleteDialogOpen(true)}
            >
                Delete now
            </Button>
        </Explained>

        <Dialog
            open={deleteDialogOpen}
            onClose={() => setDeleteDialogOpen(false)}
        >
            <DialogTitle>Confirm Account Deletion</DialogTitle>
            <DialogContent>
                <DialogContentText>
                    Are you sure you want to delete your account? This action cannot be undone.
                </DialogContentText>
            </DialogContent>
            <DialogActions>
                <Button onClick={() => setDeleteDialogOpen(false)}>Cancel</Button>
                <Button onClick={handleDeleteAccount} color="warning">
                    Delete Account
                </Button>
            </DialogActions>
        </Dialog>
    </div>
}
