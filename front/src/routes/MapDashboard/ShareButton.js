import { useState, useContext, useCallback } from 'react'
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faShare, faGlobe, faCopy, faCheck } from '@fortawesome/free-solid-svg-icons'
import { useMutation } from '@tanstack/react-query'
import { TextField, Button, ClickAwayListener } from '@mui/material'

import { GraphContext } from '../../contexts/graphContext'
import { updateMapPublic } from '../../api/api'
import { copyTextToClipboard } from '../../utils/copyToClipboard'
import { Loader } from '../../components/Loader'

import styles from './ShareButton.module.css'

const baseUrl = process.env.REACT_APP_SELF_BASE_URL || 'https://app.territory.dev'

export const ShareButton = () => {
    const [isOpen, setIsOpen] = useState(false)
    const [copied, setCopied] = useState(false)
    const { map, isOwner } = useContext(GraphContext)
    const [isPublic, setIsPublic] = useState(map?.public)
    const query = useMutation(updateMapPublic)
    const url = `${baseUrl}/maps/${map.id}`

    const changeIcon = useCallback(() => {
        setCopied(true)
        setTimeout(() => {
            setCopied(false)
        }, 5000)
    }, [copied, setCopied])

    const clickHandler = useCallback(() => {
        if (isPublic) {
            setIsOpen(true)
        } else {
            query.mutate(
                { mapId: map.id, isPublic: true},
                { onSuccess: () => {
                    setIsOpen(true)
                    setIsPublic(true)
                }},
            )
        }
    }, [map, isOpen, setIsOpen])

    const makePrivate = useCallback(() => {
        query.mutate(
            { mapId: map.id, isPublic: false},
            { onSuccess: () => {
                setIsOpen(false)
                setIsPublic(false)
            }})
    }, [map, isOpen, setIsOpen])

    if (!isOwner) return null

    return (
        <div className={styles.wrapper}>
            <Button className={styles.button} disabled={query.isLoading} onClick={clickHandler}>
                {query.isLoading
                ? <Loader size="xs" />
                : <FontAwesomeIcon className={styles.buttonIcon} icon={isPublic ? faGlobe : faShare}/>
                }
                {isPublic ? "Sharing" : "Share"}
            </Button>
            {isOpen && (
                <ClickAwayListener onClickAway={() => setIsOpen(false)}>
                    <div className={styles.dropdown}>
                        <div className={styles.message}>
                            Everyone can now view this map using the link below:
                        </div>
                        <div className={styles.copyLink}>
                            <TextField className={styles.textField + ' tShareUrl'} value={url} readOnly />
                            <Button onClick={() => copyTextToClipboard(url, changeIcon)}>
                                <FontAwesomeIcon className={styles.icon} icon={copied ? faCheck : faCopy} size="xl"/>
                            </Button>
                        </div>
                        <Button onClick={makePrivate}>
                            Stop sharing
                        </Button>
                    </div>
                </ClickAwayListener>
            )}
        </div>
    )
}
