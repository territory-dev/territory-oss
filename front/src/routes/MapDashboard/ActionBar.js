import { useContext } from 'react'

import { GraphContext } from '../../contexts/graphContext'
import { SearchBar } from './SearchBar'
import { CopyMap } from './CopyMap'

export const ActionBar = () => {
    const { isOwner } = useContext(GraphContext)

    return isOwner ? <SearchBar /> : <CopyMap />
}