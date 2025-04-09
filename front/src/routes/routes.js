import { Navigate } from 'react-router-dom'

import { Dashboard } from './Dashboard/Dashboard'
import { PublicMap } from './PublicMap/PublicMap'
import { MapDashboard } from './MapDashboard/MapDashboard'
import { NewMapByLink } from './NewMapByLink'
import { Jobs } from './Jobs/Jobs'
import { Account } from './Settings/Account'
import { RepoConfig, NewRepo } from './Settings/RepoConfig'
import { UploadTokensDashboard } from './Settings/UploadTokensDashboard'
import { AuthorizeLocal } from './Settings/AuthorizeLocal'
import { Login } from './Login/Login'
import { ProtectedRoute } from './ProtectedRoute'
import { PublicOnly } from './PublicOnly'
import { FreeMap } from './FreeMap/FreeMap'
import { FreeMapForRepo } from './FreeMapForRepo'
import { QuickBuildStateView } from './QuickBuildState/QuickBuildStateView'

export const routes = [
    { path: '/', element: <Dashboard /> },
    { path: '/maps/:mapId', element: <ProtectedRoute><MapDashboard /></ProtectedRoute>},
    { path: '/account', element: <ProtectedRoute><Account /></ProtectedRoute>},
    { path: '/repos/new', element: <ProtectedRoute><NewRepo /></ProtectedRoute>},
    { path: '/repos/:repoId/config', element: <ProtectedRoute><RepoConfig /></ProtectedRoute>},
    { path: '/repos/:repoId/jobs', element: <ProtectedRoute><Jobs /></ProtectedRoute>},
    { path: '/repos/:repoId/buildreq/:buildRequestId', element: <ProtectedRoute><QuickBuildStateView /></ProtectedRoute>},
    { path: '/upload-tokens', element: <ProtectedRoute><UploadTokensDashboard /></ProtectedRoute>},
    { path: '/upload-tokens/authorize-local', element: <ProtectedRoute><AuthorizeLocal /></ProtectedRoute>},
    { path: '/login', element: <Login /> },
    { path: '/public/maps/:mapId', element: <PublicOnly><PublicMap /></PublicOnly>},
    { path: '/public/maps/new', element: <ProtectedRoute><NewMapByLink /></ProtectedRoute>},
    { path: '/maps/local/:repoId/:branch/:buildId', element: <FreeMap />},
    { path: '/maps/local/:repoId', element: <FreeMapForRepo />},
    { path: '*', element: <Navigate to="/" /> }
]
