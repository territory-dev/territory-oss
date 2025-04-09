export const positionNewNode = (nodes, initialPosition) => {
    let i
    let positionUpdated = true

    const nodesArr = Object.values(nodes)

    while (positionUpdated) {
        initialPosition.top |= 0
        initialPosition.left |= 0
        positionUpdated = false
        for (i = 0; i < nodesArr.length; ++i) {
            let other = nodesArr[i]
            if (distanceSq(initialPosition, other) < 4) {
                if (other.height !== undefined) {
                    initialPosition.top += other.height + 100;
                } else {
                    initialPosition.top += other.top + 100;
                }
                positionUpdated = true;
            }
        }
    }
    return initialPosition
}

const distanceSq = (a, b) => {
    const dx = a.left - b.left, dy = a.top - b.top;
    return dx*dx + dy*dy
}
