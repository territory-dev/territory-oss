import { useCallback, useContext, useEffect, useMemo, useRef, useState } from "react"
import Draggable from 'react-draggable'
import DeleteIcon from '@mui/icons-material/Delete';
import CheckIcon from '@mui/icons-material/Check';
import TextIncrease from '@mui/icons-material/TextIncrease';
import TextDecrease from '@mui/icons-material/TextDecrease';

import { GraphContext } from "../../contexts/graphContext"

import styles from './Note.module.css'
import { clientToAbsolutePosition } from "./utils"
import { useKey } from "../../hooks/useKey"
import classNames from "classnames"


const note_sizes = ['S', 'M', 'L', 'XL', 'XXL'];


export const Note = ({id}) => {
    const {
        map,
        isOwner,
        notes,
        updateNoteText,
        updateNotePosition,
        updateNoteSize,
        removeNote,
        scale,
        zoomState,
        setIsDragging,
        isDragging
    } = useContext(GraphContext)
    const [editing, setEditing] = useState(false)
    const [showControls, setShowControls] = useState(false)
    const inputRef = useRef(null)
    const noteRef = useRef(null)

    const defaul = useMemo(() => notes[id], [])
    let size = 'S';
    if (note_sizes.indexOf(notes[id].size) !== -1) size = notes[id].size;

    const drag = useCallback(
        ({ movementX, movementY }) => {
            if (movementX > 0 || movementX < 0 || movementY > 0 || movementY < 0) {
                setIsDragging(true)
            }
        },
        [setIsDragging]
    )

    const clicked = (ev) => {
        if (isDragging) return;
        if (!isOwner) return;
        ev.stopPropagation()
        setEditing(true)
    }
    useEffect(() => {
        if (editing && inputRef.current)
            inputRef.current.focus()
    }, [editing])

    const save = useCallback((ev) => {
        if (!editing || !inputRef.current) return
        setEditing(false)
        ev && ev.preventDefault()
        updateNoteText(id, inputRef.current.value)
    }, [editing, setEditing, inputRef])
    const enlarge = useCallback((ev) => {
        console.log('enlarge');
        ev.stopPropagation()
        const next_size = note_sizes[note_sizes.indexOf(size) + 1]
        console.log('next_size', next_size);
        if (!next_size) return;
        updateNoteSize(id, next_size);
    }, [editing, inputRef, updateNoteSize])
    const shrink = useCallback((ev) => {
        ev.stopPropagation()
        const next_size = note_sizes[note_sizes.indexOf(size) - 1]
        if (!next_size) return;
        updateNoteSize(id, next_size);
    }, [editing, inputRef, updateNoteSize])

    const dragStop = useCallback((ev) => {
        ev.stopPropagation()
        setIsDragging(false)
        const client = noteRef.current.getBoundingClientRect()
        const abs = clientToAbsolutePosition(client.top, client.left, zoomState.scale, map.id)
        updateNotePosition(id, abs.top, abs.left)
    }, [noteRef, updateNotePosition, scale])

    useKey('Escape', () => save())

    const delet = (ev) => {
        ev.stopPropagation()
        removeNote(id)
    }

    return <Draggable
        key={id}
        nodeRef={noteRef}
        onMouseUp={(ev) => {
            ev.stopPropagation()
        }}
        onMouseDown={(ev) => {
            ev.stopPropagation()
        }}
        onDrag={drag}
        onStop={dragStop}
        scale={scale}
        disabled={!isOwner}
    >
        <div
            key={id}
            className={classNames(styles.Note, styles[`size_${size}`], isDragging && styles.dragging)}
            style={{top: defaul.top, left: defaul.left}}
            onClick={clicked}
            ref={noteRef}
            onMouseOver={() => setShowControls(true)}
            onMouseLeave={() => setShowControls(false)}
        >
            <div className={styles.body}>
                {editing
                    ? <form onSubmit={save} onMouseDown={ev=>{ev.stopPropagation()}}>
                        <textarea ref={inputRef} defaultValue={notes[id].text} onBlur={save}></textarea>
                        <CheckIcon className={styles.saveIcon} onClick={save} />
                    </form>
                    : <div className={styles.text}>{notes[id].text}</div>}
            </div>
            { !editing && isOwner && showControls &&
                <div className={styles.hoverControls}>
                    <TextIncrease className={styles.saveIcon} fontSize='small' onClick={enlarge} />
                    <TextDecrease className={styles.saveIcon} fontSize='small' onClick={shrink} />
                    <DeleteIcon htmlColor="#A6A6A6" className={styles.deletIcon} fontSize='small' onClick={delet} />
                </div>}
        </div>
    </Draggable>

}
