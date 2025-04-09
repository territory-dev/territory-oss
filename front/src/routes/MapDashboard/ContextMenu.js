import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faComment, faFolder } from '@fortawesome/free-regular-svg-icons'
import { Children, useCallback, useContext } from 'react'

import styles from './ContextMenu.module.css'
import { GraphContext } from '../../contexts/graphContext'
import { clientToAbsolutePosition } from './utils'


export const ContextMenu = ({top, left, onClose}) => {
    const {addNote, addNode, scale, map, rootId} = useContext(GraphContext)
    const addNoteClick = useCallback(() => {
        const absCoords = clientToAbsolutePosition(top, left, scale, map.id)
        addNote({top: absCoords.top, left: absCoords.left, text: ""})
        onClose()
    }, [addNote, scale, map, top, left])

    const repoRootClick = useCallback(() => {
        addNode({ href: rootId })
        onClose()
    }, [addNote, scale, map, top, left])

    return <div className={styles.ContextMenu} style={{top, left}}>
        <ContextMenuItem onClick={addNoteClick}>
            <FontAwesomeIcon className={styles.icon} size="lg" icon={faComment} />
            Add a note
        </ContextMenuItem>

        <ContextMenuItem onClick={repoRootClick}>
            <FontAwesomeIcon className={styles.icon} size="lg" icon={faFolder} />
            Open repository root
        </ContextMenuItem>
    </div>
}

const ContextMenuItem = ({children, onClick}) =>
    <button className={styles.ContextMenuItem} onClick={onClick}>{children}</button>
