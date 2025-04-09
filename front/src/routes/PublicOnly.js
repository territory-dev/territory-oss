import { useContext } from 'react'
import { Navigate, useLocation } from 'react-router-dom'

import { UserContext } from '../contexts/userContext'


export const PublicOnly = ({ children }) => {
    const {
        user,
    } = useContext(UserContext)

    let redirectUrl = '/'

    const location = useLocation()

    if (location.pathname.startsWith('/public/maps/')) {
        const [a,b,c,mapId] = location.pathname.split('/')
        redirectUrl = `/maps/${mapId}`
    }

    return user
        ? <Navigate to={redirectUrl} />
        : children

}