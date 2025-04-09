import { useContext } from 'react'

import { GraphContext } from '../../contexts/graphContext'
import { Arrow } from './Arrow'

export const Arrows = () => {
    const { relations } = useContext(GraphContext)
    return Object
        .keys(relations)
        .map((key) => {
            const { type, id, caller } = relations[key]

            return (
                <Arrow key={key} fromId={caller} toId={id} />
            )
        })
}