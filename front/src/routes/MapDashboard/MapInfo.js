import { useContext, useState, useCallback } from 'react'
import { useMutation } from '@tanstack/react-query'
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faPen, faXmark } from '@fortawesome/free-solid-svg-icons'
import TextField from '@mui/material/TextField'
import { ClickAwayListener } from '@mui/material'
import { useKey } from 'react-keyboard-hooks'

import { GraphContext } from '../../contexts/graphContext'
import { Loader } from '../../components/Loader'
import { BuildInfo } from './BuildInfo'
import { updateMapName } from '../../api/api'

import styles from './MapInfo.module.css'

export const MapInfo = ({
    map,
    repo,
    branch,
    build,
}) => {
    const [newMapName, setNewMapName] = useState(map.display_name)
    const [isEditState, setIsEditState] = useState(false)
    const query = useMutation(updateMapName)
    const { isOwner } = useContext(GraphContext)

    const cancel = useCallback(() => {
        if (query.isLoading) return
        setIsEditState(false)
        setNewMapName(map.display_name)
    }, [map, setIsEditState, setNewMapName, query])

    const saveNewName = useCallback(() => {
        if (!newMapName || newMapName === map.display_name) cancel()

        query.mutate(
            { mapId: map.id, name: newMapName },
            {
                onSuccess: () => setIsEditState(false),
                onError: cancel,
            }
        )
    }, [map, newMapName, cancel, query, setIsEditState])

    useKey('Escape', isEditState ? cancel : () => {})
    useKey('Enter',  isEditState ? saveNewName : () => {})

    return (
        <div className={styles.mapData}>
            {isEditState
                ? (
                    <div className={styles.mapNameWrapper}>
                        <ClickAwayListener onClickAway={saveNewName}>
                            <TextField
                                className={styles.textField}
                                value={newMapName}
                                onChange={(e) => setNewMapName(e.target.value)}
                                variant="standard"
                            />
                        </ClickAwayListener>
                        {!query.isLoading
                            ? (
                                <FontAwesomeIcon
                                    className={styles.icon}
                                    icon={faXmark}
                                    size="sm"
                                    onClick={cancel}
                                />
                            ) : (
                                <Loader size="xs" className={styles.loader} />
                            )
                        }
                    </div>
                )
                : (
                    <div className={styles.mapNameWrapper}>
                        <div className={styles.mapNameText} onDoubleClick={() => setIsEditState(true)}>
                            {newMapName}
                        </div>
                        {isOwner && (
                            <FontAwesomeIcon
                                className={styles.icon}
                                icon={faPen}
                                size="xs"
                                onClick={() => setIsEditState(true)}
                            />
                        )}
                    </div>
                )
            }
            <BuildInfo
                repo={repo}
                branch={branch}
                build={build}
            />
        </div>
    )
}
