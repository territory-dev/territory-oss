import { useContext } from 'react'
import { Navigate, useLocation } from 'react-router-dom'

import { UserContext } from '../contexts/userContext'


export const ProtectedRoute = ({ children }) => {
    const {
        user,
    } = useContext(UserContext)

    const location = useLocation()

    const loginParams = new URLSearchParams([
        ["redirect", location.pathname + location.search],
    ])
    const redirectTo = '/login?' + loginParams.toString()
    let navigate = <Navigate to={redirectTo} replace />

    if (location.pathname.startsWith('/maps/')) {
        navigate = <Navigate to={`/public${location.pathname}`} />
    }

    return user
        ? children
        : navigate

}
