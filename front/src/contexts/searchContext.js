import {
    createContext,
    useContext,
} from 'react'
import { useQuery, useMutation } from '@tanstack/react-query'

import { GraphContext } from './graphContext'
import { getSearchIndex, getSearchQuery } from '../api/api'

export const SearchContext = createContext(undefined)

export const SearchContextProvider = ({
    children,
}) => {
    const { searchIndexConfig } = useContext(GraphContext)

    const indexQuery = useQuery(
        ['searchIndex'],
        () => getSearchIndex(searchIndexConfig)
    )

    const search = (query, limit = 10) => getSearchQuery(query, limit, searchIndexConfig)

    return (
        <SearchContext.Provider value={{
            isIndexLoading: indexQuery.isLoading,
            search,
        }}>
            {children}
        </SearchContext.Provider>
    )
}
