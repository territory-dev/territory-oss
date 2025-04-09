import { useContext } from 'react'
import { GraphContext } from '../../contexts/graphContext'

export const WantNode = ({nodeRef}) => {
    const { nodes, addNode } = useContext(GraphContext)

    addNode({href: nodeRef})

    return null;
}
