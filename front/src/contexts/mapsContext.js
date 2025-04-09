import { createContext, useState, useContext, useEffect, useMemo, useCallback } from 'react'

import { getMaps } from '../api/api'
import { Loader } from '../components/Loader'

import { UserContext } from './userContext'

export const MapsContext = createContext(undefined)

export const MapsContextProvider = ({
    children,
}) => {
    const {
        user,
    } = useContext(UserContext)

    const [maps, setMaps] = useState([])
    const [mapsLoading, setMapsLoading] = useState(true)

    const mapsList = useMemo(() => {
        if (!maps) return null

        return Object.keys(maps).map(id => ({
            id,
            ...maps[id]
        }))
    }, [maps])

    const reloadMaps = useCallback(
        () => {
            setMaps([])
            setMapsLoading(true)
            if (user) {
                return getMaps(user.uid)
                    .then(resp => {
                        setMaps(resp)
                        setMapsLoading(false)
                    })
            }
        }
    , [maps, setMaps, user])

    return (
        <MapsContext.Provider value={{
            maps,
            mapsList,
            mapsLoading,
            reloadMaps,
        }}>
            {children}
        </MapsContext.Provider>
    )
}
