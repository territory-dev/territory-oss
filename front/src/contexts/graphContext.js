import {
    createContext,
    useContext,
    useState,
    useCallback,
    useEffect,
} from 'react'
import { useMutation } from '@tanstack/react-query'

import * as api from '../api/api'
import { positionNewNode } from '../utils/positionNewNode'
import { UserContext } from './userContext'

export const GraphContext = createContext(undefined)

export const GraphContextProvider = ({
    children,
    map,
    build,
    getNodeFunc = null,
    graphData: {
        ...graph
    },
    graphStorage,
}) => {
    const { user } = useContext(UserContext)
    const { build: {path} } = map
    const { code_root, code_path, code_bucket, storage_root_url, code_storage, search_index_path, db_path, trie } = build
    const [activeNode, setActiveNode] = useState(null)
    const [newestNode, setNewestNode] = useState(null)
    const [showRefs, setShowRefs] = useState(null)
    const [zoomState, setZoomState] = useState({ scale: 1, positionX: 0, positionY: 0 })
    const [nodes, setNodes] = useState(graph?.nodes || {})
    const [relations, setRelations] = useState(graph?.relations || {})
    const [isDragging, setIsDragging] = useState(false)
    const [isDisabled, setIsDisabled] = useState(false)
    const [zoomDone, setZoomDone] = useState(false);
    const [empty, setEmpty] = useState(true);
    const [notes, setNotes] = useState(graph?.notes || {})
    const [highlight, setHighlight] = useState(null)

    const getNode = getNodeFunc || api.getNode

    const isOwner = map.free || (user && (map.owner === user.uid))

    const { scale } = zoomState

    const buildRef = api.unpackBuildRef(path)
    const codeStorageConfig = {
        backend: code_storage,
        bucket: code_bucket,
        root_url: storage_root_url,
        path: code_path,
        db_path: db_path,
        ...buildRef
    }

    const searchIndexConfig = {
        backend: code_storage,
        bucket: code_bucket,
        root_url: storage_root_url,
        path: search_index_path,
        trie: trie,
        ...buildRef
    }

    const saveNodePosition = useCallback((nodeId) => {
        graphStorage.setGraph({
            [`nodes.${nodeId}`]: nodes[nodeId],
        })
    }, [graphStorage, nodes])

    const saveNewNode = useCallback((nodeId, rect) => {
        graphStorage.setGraph({
            [`nodes.${nodeId}`]: rect,
        })
    }, [graphStorage])

    const saveRemoveNode = useCallback((nodeId) => {
        graphStorage.deleteNode(nodeId)
    }, [graphStorage])

    const saveRelation = useCallback((key, relation) => {
        graphStorage.setGraph({
            [`relations.${key}`]: relation,
        })
    }, [graphStorage])

    const saveNoteState = useCallback((noteId, n) => {
        graphStorage.setGraph({
            [`notes.${noteId}`]: n,
        })
    }, [graphStorage])

    const removeNode = useCallback((id) => {
        setNodes((oldNodes) => {
            const newNodes = { ...oldNodes }
            delete newNodes[id]
            return newNodes
        })
        saveRemoveNode(id)
    }, [setNodes, saveRemoveNode])

    const setNode = useCallback((id, rect) =>
        setNodes((prevRects) => ({ ...prevRects, [id]: rect })),
        [setNodes],
    )

    const setRelation = useCallback(({ id, caller, type }) => setRelations((oldRelations) => {
        const key = `${id} ${caller} ${type}`

        if (id !== caller && !oldRelations[key]) {
            const relation = { id, caller, type }
            saveRelation(key, relation)
            return ({
                ...oldRelations,
                [key]: relation,
            })
        }

        return oldRelations
    }), [setRelations, saveRelation])

    const addNode = useCallback(({
        href,
        opener,
        relation,
    }) => getNode(codeStorageConfig, href).then(({ id }) => {
        setActiveNode(id)

        if (!nodes[id]) {
            let rect = {}
            if (opener && nodes[opener]) {
                const openerRect = nodes[opener]
                rect = {
                    top: openerRect?.top,
                    left: openerRect?.left + openerRect?.width + 100,
                }
            } else if (activeNode && nodes[activeNode]) {
                const activeRect = nodes[activeNode]
                rect = {
                    top: activeRect?.top,
                    left: activeRect?.leftactiveRect?.width + 100,
                }
            }
            rect = positionNewNode(nodes, rect)
            setNode(id, rect)
            saveNewNode(id, rect)
            setNewestNode(id)
        }

        if (relation === 'container') {
            setRelation({ id: opener, caller: id, type: relation })
        } else if (relation === 'reference') {
            setRelation({ id: id, caller: opener, type: 'reference' })
        } else if (relation === 'parent') {
            setRelation({ id, caller: opener, type: 'container' })
        }

    }), [nodes, setNode, setActiveNode, setNewestNode, setRelation, activeNode, getNode, saveNewNode])

    const addNodeQuery = useMutation({
        mutationFn: addNode,
        mutationKey: 'addNode',
    })

    const requestCenter = () => {
        setZoomDone(false);
    }

    const addNote = useCallback((n) => {
        const id = ((Date.now() + Math.random() * 1000) | 0).toString()
        const newNotes = {
            ...notes,
            [id]: n,
        }
        setNotes(newNotes)
        saveNoteState(id, n)
    }, [notes, setNotes])

    const updateNoteText = useCallback((noteId, text) => {
        const prev = notes[noteId]
        if (prev.text == text) return
        const note = { ...prev, text }
        setNotes({
            ...notes,
            [noteId]: note,
        })
        saveNoteState(noteId, note)
    }, [notes, setNotes])

    const updateNoteSize = useCallback((noteId, size) => {
        const prev = notes[noteId]
        if (prev.size == size) return
        const note = { ...prev, size }
        setNotes({
            ...notes,
            [noteId]: note,
        })
        saveNoteState(noteId, note)
    }, [notes, setNotes])

    const updateNotePosition = useCallback((noteId, top, left) => {
        const prev = notes[noteId]
        const note = { ...prev, top, left }
        setNotes({
            ...notes,
            [noteId]: note,
        })
        saveNoteState(noteId, note)
    }, [notes, setNotes, saveNoteState])

    const removeNote = useCallback((noteId) => {
        setNotes((oldNotes) => {
            const newNotes = { ...oldNotes }
            delete newNotes[noteId]
            return newNotes
        })
        saveNoteState(noteId, null)
    }, [notes, setNotes, saveNoteState])


    useEffect(() => {
        setEmpty(
            (Object.keys(nodes || {}).length === 0) &&
            (Object.keys(notes || {}).length === 0)
        )
    }, [notes, nodes])

    return (
        <GraphContext.Provider value={{
            rootId: code_root,
            codeStorageConfig,
            searchIndexConfig,
            bucketName: code_bucket,
            searchIndexPath: search_index_path,
            path: code_path,
            nodes,
            relations,
            addNode: isOwner ? addNode : () => {},
            removeNode: isOwner ? removeNode : () => {},
            setNode,
            isDragging,
            setIsDragging,
            showRefs,
            setShowRefs: isOwner ? setShowRefs : () => {},
            newestNode,
            scale,
            zoomState,
            setZoomState,
            activeNode,
            setActiveNode,
            isDisabled,
            setIsDisabled,
            saveNodePosition,
            addNodeQuery,
            isOwner,
            map,
            build,
            getNode,
            requestCenter,
            zoomDone,
            setZoomDone,
            addNote: isOwner ? addNote : () => {},
            updateNoteText: isOwner ? updateNoteText : () => {},
            updateNotePosition: isOwner ? updateNotePosition : () => {},
            updateNoteSize: isOwner ? updateNoteSize : () => {},
            removeNote: isOwner ? removeNote : () => {},
            notes,
            empty,
            highlight,
            setHighlight,
        }}>
            {children}
        </GraphContext.Provider>
    )
}
