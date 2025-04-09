export const getNodeAbsolutePosition = (node, scale, mapId) => {
    const { width, height, top, left, right, bottom } = node
        ?.getBoundingClientRect()

    const nodesWrapperRect = document
        ?.getElementById(`nodes-wrapper-${mapId}`)
        ?.getBoundingClientRect()

    return ({
        top: (top - nodesWrapperRect.top) / scale,
        bottom: (bottom - nodesWrapperRect.bottom) / scale,
        left: (left - nodesWrapperRect.left) / scale,
        right: (right - nodesWrapperRect.right) / scale,
        width: width / scale,
        height: height / scale,
    })
}

export const clientToAbsolutePosition = (top, left, scale, mapId) =>{
    const nodesWrapperRect = document
        ?.getElementById(`nodes-wrapper-${mapId}`)
        ?.getBoundingClientRect()

    return {
        top: (top - nodesWrapperRect.top) / scale,
        left: (left - nodesWrapperRect.left) / scale,
    }
}

export const getNodeRelativePosition = (node, scale, mapId) => {
    if (!mapId || !node) return null
    const { left, top, bottom, right, width, height } = node

    const nodesWrapperRect = document
        ?.getElementById(`nodes-wrapper-${mapId}`)
        ?.getBoundingClientRect() || {
            left: 0,
            top: 0,
        }

    return {
        top: nodesWrapperRect.top + (top * scale),
        left: nodesWrapperRect.left + (left * scale),
        width: width * scale,
        height: height * scale,
    }
}
