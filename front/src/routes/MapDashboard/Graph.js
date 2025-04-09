import { useContext, useCallback, useRef, useEffect, useState } from 'react'
import { TransformWrapper, TransformComponent } from 'react-zoom-pan-pinch'
import { GraphContext } from '../../contexts/graphContext'

import { NavOverlay } from './NavOverlay'
import { Nodes } from './Nodes'
import { Arrows } from './Arrows'
import styles from './Graph.module.css'
import { ContextMenu } from './ContextMenu'
import { Notes } from './Notes'


const getGraphCenter = (element, nodes) => {
    if (!nodes || !Object.keys(nodes).length) return { scale: 1, positionX: 0, positionY: 0 }
    const entries = Object.entries(nodes)
    const top = entries.reduce(
        (prev, [id, { top }]) => ((prev === null) || top < prev) ? top : prev,
        null
    )
    const left = entries.reduce(
        (prev, [id, { left }]) => ((prev === null) || left < prev) ? left : prev,
        null,
    )
    const bottom = entries.reduce(
        (prev, [id, { top, height}]) => ((prev === null) || (top + height) > prev) ?  (top + height) : prev,
        null
    )
    const right = entries.reduce(
        (prev, [id, { left, width }]) => ((prev === null) || (left + width) > prev) ? (left + width) : prev,
        null,
    )

    const width = Math.abs(right - left) * 1.1
    const height = Math.abs(bottom - top) * 1.1

    const H = element.clientHeight
    const W = element.clientWidth
    const scale = Math.min(
        H / height,
        W / width,
    )

    const scaled_w = scale * width;
    const scaled_h = scale * height;

    return ({
        y: 0.05 * H - top * scale + (H - scaled_h) / 2,
        x: 0.05 * W - left * scale + (W - scaled_w) / 2,
        scale,
    })
}


export const Graph = () => {
    const {
        nodes,
        isDisabled,
        isDragging,
        zoomState,
        setZoomState,
        map,
        zoomDone,
        setZoomDone,
        notes,
        empty,
    } = useContext(GraphContext)

    const transformRef = useRef(null)
    const containerRef = useRef(null)
    const [contextMenuPosition, setContextMenuPosition] = useState(null)

    const zoomToCenter = useCallback(() => {
        if (!containerRef) return;
        const center = getGraphCenter(containerRef.current, nodes)
        const { x, y, scale } = center
        transformRef.current.setTransform(x, y, scale)

        setZoomDone(true);
    }, [nodes, containerRef])

    useEffect(() => {
        if (zoomDone) return;

        if (transformRef?.current) {
            zoomToCenter(transformRef.current)
        }
    }, [zoomToCenter, zoomDone])

    const onContextMenu = useCallback((ev) => {
        ev.preventDefault()
        setContextMenuPosition({top: ev.clientY, left: ev.clientX })
    }, [contextMenuPosition, setContextMenuPosition])


    return <div
        id={`graph-wrapper-${map.id}`}
        className={styles.graphWrapper}
        ref={containerRef}
        onContextMenu={onContextMenu}
        onClick={() => setContextMenuPosition(null)}
    >
        <TransformWrapper
            initialScale={zoomState.scale}
            initialPositionX={zoomState.positionX * zoomState.scale}
            initialPositionY={zoomState.positionY * zoomState.scale}
            minScale={0.04}
            maxScale={1}
            disabled={isDragging}
            wrapperClass={styles.wrapper}
            limitToBounds={false}
            ref={transformRef}
            wheel={{
                disabled: isDisabled,
            }}
            onTransformed={(ref, state) => {
                setZoomState(state)
            }}
            panning={{
                velocityDisabled: true,
            }}
            zoomAnimation={{
                disabled: true,
            }}
        >
            <TransformComponent
                wrapperStyle={{
                    width: '100%',
                    height: '100%',
                    padding: 0,
                    margin: 0,
                    overflow: 'visible',
                }}
            >
                <div className={styles.graph} id={`nodes-wrapper-${map.id}`}>
                    <Nodes />
                    <Notes notes={notes} />
                </div>
                <Arrows />
            </TransformComponent>
        </TransformWrapper>

        {empty && <div className={styles.hint}>right click for more</div>}

        {(contextMenuPosition !== null) &&
            <ContextMenu
                onClose={() => setContextMenuPosition(null)}
                top={contextMenuPosition.top}
                left={contextMenuPosition.left}
            />}

        <NavOverlay />
    </div>
}
