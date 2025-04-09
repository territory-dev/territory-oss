
import React from 'react'
import Button from '@mui/material/Button'
import Dialog from '@mui/material/Dialog'
import DialogActions from '@mui/material/DialogActions'
import DialogTitle from '@mui/material/DialogTitle'
import Slide from '@mui/material/Slide'

import { deleteMap } from "../../api/api"
import { useKey } from '../../hooks/useKey'

const Transition = React.forwardRef(function Transition(props, ref) {
    return <Slide direction="up" ref={ref} {...props} />;
  })


export const RemoveMap = ({
    close,
    mapToRemove,
    reloadMaps,
}) => {
    const doDelete = () => {
        deleteMap(mapToRemove.id)
            .then(() => reloadMaps())
            .then(() => close())
    }

    useKey('Enter', doDelete)

    return (
        <Dialog
            open={!!mapToRemove}
            TransitionComponent={Transition}
            onClose={close}
        >
            <DialogTitle>
                Are you sure you want to remove {mapToRemove.display_name}?
            </DialogTitle>
            <DialogActions>
                <Button onClick={doDelete}>
                    Yes
                </Button>
                <Button
                    onClick={close}
                >
                    No
                </Button>
            </DialogActions>
        </Dialog>
    )
}
