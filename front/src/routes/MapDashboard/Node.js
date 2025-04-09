import { useContext, useState, useCallback, useMemo, useEffect, useRef } from 'react'
import classnames from 'classnames'
import { useQuery, useMutation } from '@tanstack/react-query'
import Draggable from 'react-draggable'
import { useControls } from 'react-zoom-pan-pinch'
import Tooltip from '@mui/material/Tooltip';

import { getNodeAbsolutePosition } from './utils'
import { isElementInViewport } from '../../utils/isElementInViewport'
import { GraphContext } from '../../contexts/graphContext'
import { FilesList } from './FilesList'
import { Sourcefile } from './Sourcefile'
import { Code } from './Code'
import { Loader } from '../../components/Loader'
import{ ReactComponent as ParentFolderIcon } from './resources/ParentFolder.svg'

import styles from './Node.module.css'

const mapKindToComponent = (node, addNode, showLines, onLineNumMouseOver, onLineNumMouseOut) => {
    switch (node.kind) {
        case 'directory':
            return <FilesList {...node} addNode={addNode} />
        case 'sourcefile':
            return <Sourcefile {...node} addNode={addNode}/>
        default:
            return <Code
                {...node}
                addNode={addNode}
                showLines={showLines}
                onLineNumMouseOver={onLineNumMouseOver}
                onLineNumMouseOut={onLineNumMouseOut}
            />
    }
}

export const Node = ({ id }) => {
    const {
        codeStorageConfig,
        removeNode,
        nodes,
        setNode,
        isDragging,
        setIsDragging,
        scale,
        activeNode,
        setActiveNode,
        addNode,
        saveNodePosition,
        isOwner,
        zoomState,
        map,
        relations,
        getNode,
    } = useContext(GraphContext)

    const [nodeFetchError, setNodeFetchError] = useState(false)
    const [dragCounter, setDragCounter] = useState(0)
    const [shouldZoom, setShouldZoom] = useState(false)
    const [blockZoom, setBlockZoom] = useState(false)
    const [showLines, setShowLines] = useState(false)
    const { zoomToElement } = useControls()

    useEffect(() => {
        // if got changed to newest node, set should zoom
        if (activeNode === id && !shouldZoom) {
            setShouldZoom(true)
        }
    }, [activeNode, setShouldZoom])

    const addNodeQuery = useMutation({
        mutationFn: addNode,
        mutationKey: ['addNode', id],
    })

    const {
        isLoading,
        data,
    } = useQuery(
        ['node', id],
        () => getNode(codeStorageConfig, id),
        {
            enabled: !nodeFetchError,
            onError: () => setNodeFetchError(true),
        }
    )

    const innerDomNodeRef = useRef(null)
    const refCallback = useCallback(innerDomNode => {
        innerDomNodeRef.current = innerDomNode
        if (innerDomNode) {
            // save position
            if (shouldZoom && data) {
                // zoom is flag set to should zoom
                if (!isElementInViewport(innerDomNode) && !blockZoom) {
                    // hack
                    setTimeout(() => {
                        zoomToElement(innerDomNode, scale)
                    }, 0)
                }
                setShouldZoom(false)
            } else {
                const absolutePosition = getNodeAbsolutePosition(
                    innerDomNode,
                    zoomState.scale,
                    map.id,
                )
                setNode(id, absolutePosition)
            }
        }
    },  [shouldZoom, dragCounter, data, map.id, scale, blockZoom])



    // take default position
    const defaultPosition = useMemo(() => {
        const { width, height, ...noSize } = nodes[id]
        return noSize
    }, [])


    const showLoader = isLoading || addNodeQuery.isPending

    const handleClick = useCallback(
        (ev) => {
            ev.stopPropagation()
            if (data?.container) addNodeQuery.mutate({
                href: data.container,
                opener: id,
                relation: 'parent',
            })
        },
        [id, data, addNode, activeNode]
    )

    const handleDrag = useCallback(
        ({ movementX, movementY }) => {
            if (movementX > 0 || movementX < 0 || movementY > 0 || movementY < 0) {
                setIsDragging(true)
                setDragCounter(ctr => ctr + 1)
            }
        },
        [setIsDragging, setDragCounter]
    )

    const isReferencingActiveNode = useMemo(() => {
        if (activeNode === id) return false

        const relationsWithActiveNode = Object.values(relations).filter(
            (relation) => (relation.id === activeNode || relation.caller === activeNode)
                && (relation.id === id|| relation.caller === id)
        )

        return !!relationsWithActiveNode.length

    }, [relations, id, activeNode])

    const [pathCopiedOpen, setPathCopiedOpen] = useState(false);
    const copyPath = () => {
        if (data && data.path) {
            navigator.clipboard.writeText(data.path)
            setPathCopiedOpen(true)
            setTimeout(() => { setPathCopiedOpen(false) }, 1000)
        }
    }
    const handleTooltipClose = () => {
        setPathCopiedOpen(false)
    }

    const title = <div
        className={styles.title}
        onClick={copyPath}
    >
        <Tooltip
            PopperProps={{
                disablePortal: true,
            }}
            onClose={handleTooltipClose}
            open={pathCopiedOpen}
            disableFocusListener
            disableHoverListener
            disableTouchListener
            title="Path copied"
            placement="top"
        >
            <div>{data?.path}</div>
        </Tooltip>
        <div>{data?.member_of}</div>
    </div>

    const onLineNumMouseOver = () => { setShowLines(true) }
    const onLineNumMouseOut = () => { setShowLines(false) }

    return (
        <Draggable
            nodeRef={innerDomNodeRef}
            onDrag={handleDrag}
            onMouseUp={(ev) => {
                ev.stopPropagation()
                setBlockZoom(false)
            }}
            onMouseDown={(ev) => {
                ev.stopPropagation()
                setActiveNode(id)
                setBlockZoom(true)
            }}
            onStop={(ev) => {
                ev.stopPropagation()
                setIsDragging(false)
                saveNodePosition(id)
            }}
            scale={scale}
            disabled={!isOwner}
        >
            <div
                className={classnames(
                    styles.node,
                    (activeNode === id) && styles.activeNode,
                    isReferencingActiveNode && styles.referencingActiveNode,
                    isDragging && styles.dragging,
                )}
                ref={refCallback}
                style={defaultPosition}
                onMouseLeave={onLineNumMouseOut}
                onClick={(ev) => {
                    ev.stopPropagation()
                }}
            >

                {data && (
                    <>
                        <div
                            className={classnames(styles.header, 'tNodeHeader')}
                        >
                                {(data?.container && isOwner)
                                    ? (
                                        <ParentFolderIcon
                                            onClick={handleClick}
                                            className={styles.parentIcon}
                                        />
                                    )
                                    : (
                                        <div className={styles.noIconFiller} />
                                    )
                                }
                                {title}
                            {isOwner ? (
                                <div className={styles.buttons}>
                                    <button className="tCloseNode" onClick={() => removeNode(id)}>
                                        ⨉
                                    </button>
                                </div>
                            ) : (
                                <div className={styles.noIconFiller} />
                            )}
                        </div>
                        <div
                            className={styles.leftMarginHover}
                            onMouseOver={() => { setShowLines(true) }}
                        ></div>
                        {mapKindToComponent(data, addNodeQuery.mutate, showLines, onLineNumMouseOver, onLineNumMouseOut)}
                    </>
                )}
                {showLoader && (
                    <Loader />
                )}
            </div>
        </Draggable>
    )
}
