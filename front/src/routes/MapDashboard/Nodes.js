import { useContext } from 'react'

import { GraphContext } from '../../contexts/graphContext'
import { Node } from './Node'

export const Nodes = () => {
    const { nodes } = useContext(GraphContext)

    return Object.keys(nodes).map((id) => (
        <Node
            id={id}
            key={id}
            opener={nodes[id].opener}
        />
    ))
}
