import { useContext, useMemo } from 'react'
import classNames from 'classnames'

import { GraphContext } from '../../contexts/graphContext'
import { getSvgData } from './arrowCalc'
import styles from './Arrow.module.css'

const ARROW_HEAD_SIZE = 7

const ARROW_COLOR = '#A6A6A6'

export const Arrow = ({
    fromId,
    toId,
}) => {
    const {
        nodes,
        activeNode,
        zoomState,
    } = useContext(GraphContext)

    const { scale } = zoomState

    const fromRect = nodes[fromId]
    const toRect = nodes[toId]

    const svgData = useMemo(
        () => {
            return getSvgData(toRect, fromRect, toId, fromId, activeNode, scale)
        },
        [scale, toRect, fromRect],
    )

    const isActive = useMemo(() => [toId, fromId].includes(activeNode), [activeNode])

    if (
        !fromRect
        || !toRect
        || !svgData
    ) {
        return null
    }

    const {
        fromPoint,
        styles: { top, left, width, height, path },
        arrowStyle,
    } = svgData

    return (
        <div
            id={`${fromId}-${toId}`}
            className={styles.wrapper}
        >
            <svg
                width={(width > 3) ? width : 3}
                height={(height > 3) ? height : 3}
                className={classNames(
                    isActive && styles.active,
                    styles.path
                )}
                style={{
                    top,
                    left,
                    position: 'absolute',
                }}
            >
                <path d={path} strokeWidth={1 / scale} stroke={ARROW_COLOR} fill="none" />
            </svg>
            <div
                className={classNames(
                    styles.arrowhead,
                    isActive && styles.active,
                    arrowStyle,
                )}
                style={{
                    top: fromPoint.top,
                    left: fromPoint.left,
                    borderWidth: (ARROW_HEAD_SIZE / Math.max(scale, 0.45)),
                }}
            />
        </div>
    )
}
