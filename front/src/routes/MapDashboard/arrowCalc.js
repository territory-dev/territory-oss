import styles from './Arrow.module.css'

const leftS = ({ left, top, height }) => ({
    left,
    top: top + (height/2)
})

const rightS = ({ left, top, height, width }) => ({
    left: left + width,
    top: top + (height/2)
})

const topS = ({ left, top, width }) => ({
    left: left + (width / 2),
    top,
})

const bottomS = ({ left, top, width, height }) => ({
    left: left + (width / 2),
    top: top + height,
})

export const getPoints = (toRect, fromRect, isFromOnTop) => {

    let fromPoint, toPoint, type, arrowStyle

    if (fromRect.left > (toRect.left + toRect.width)) {
        fromPoint = leftS(fromRect)
        toPoint = rightS(toRect)
        type = 'side'
        arrowStyle = styles.arrowRight
    } else if ((fromRect.left + fromRect.width) < toRect.left) {
        fromPoint = rightS(fromRect)
        toPoint = leftS(toRect)
        type = 'side'
        arrowStyle = styles.arrowLeft
    } else if ((fromRect.top + fromRect.height) < toRect.top) {
        fromPoint = bottomS(fromRect)
        toPoint = topS(toRect)
        type = 'top'
        arrowStyle = styles.arrowTop
    } else if ((toRect.top + toRect.height) < fromRect.top) {
        fromPoint = topS(fromRect)
        toPoint = bottomS(toRect)
        type = 'top'
        arrowStyle = styles.arrowBottom
    } else if (isFromOnTop) { // should check order
        type = 'overlap'
        // from on top
        if (fromRect.left > toRect.left) {
            toPoint = leftS(toRect)
            fromPoint = leftS(fromRect)
            arrowStyle = styles.arrowRight
        } else {
            toPoint = rightS(toRect)
            fromPoint = rightS(fromRect)
            arrowStyle = styles.arrowLeft
        }
    } else {
        type = 'overlap'
        // to on to
        if (toRect.left > fromRect.left) {
            toPoint = leftS(toRect)
            fromPoint = leftS(fromRect)
            arrowStyle = styles.arrowRight
        } else {
            toPoint = rightS(toRect)
            fromPoint = rightS(fromRect)
            arrowStyle = styles.arrowLeft
        }
    }

    return [fromPoint, toPoint, type, arrowStyle]
}
const x = 100

const getPoint = ([x,y]) => x !== null ? `${x},${y}` : ''

export const getSvgData = (toRect, fromRect, toId, fromId, activeNode, scale) => {
    if (!fromRect || !toRect) return null
    const rectWithDefaults = r => ({height: 0, width: 0, ...r})
    fromRect = rectWithDefaults(fromRect)
    toRect = rectWithDefaults(toRect)

    const [fromPoint, toPoint, type, arrowStyle] = getPoints(toRect, fromRect, fromId === activeNode)

    const data = {
        width: Math.abs(fromPoint.left - toPoint.left),
        height: Math.abs(fromPoint.top - toPoint.top),
        top: Math.min(fromPoint.top, toPoint.top),
        left: Math.min(fromPoint.left, toPoint.left),
    }

    const { height, width } = data
    const start = []
    const end = []
    let cp1, cp2

    const X = width + x

    if (fromPoint.left < toPoint.left) {
        start[0] = 0
        end[0] = width
    } else {
        start[0] = width
        end[0] = 0
    }
    if (fromPoint.top > toPoint.top) {
        end[1] = 0
        start[1] = height
    } else {
        end[1] = height
        start[1] = 0
    }

    if (type === 'side') {
        cp1 = [end[0], start[1]]
        cp2 = [start[0], end[1]]
    } else if (type === 'top') {
        cp2= [end[0], start[1]]
        cp1 = [start[0], end[1]]
    } else {
        if (toRect.left > fromRect.left && (fromId === activeNode)
        || toRect.left < fromRect.left && (toId === activeNode)) {
            cp1 = [start[0] + X, start[1]]
            cp2 = [start[0] + X, end[1]]
        } else {
            cp1 = [start[0] - X, start[1]]
            cp2 = [start[0] - X, end[1]]
        }
    }

    data.path = `M${getPoint(start)} C${getPoint(cp1)} ${getPoint(cp2)} ${getPoint(end)}`

    return {
        styles: data,
        toPoint,
        fromPoint,
        arrowStyle,
    }
}
