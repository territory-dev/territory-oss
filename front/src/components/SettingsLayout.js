import { useState, useEffect, useContext, useMemo, useCallback } from 'react'

import { maps } from '../api/api'
import { UserContext } from '../contexts/userContext'

import { Layout } from './Layout'
import { SettingsSidebar } from './SettingsSidebar'

import styles from './SettingsLayout.module.css'
import { LegalFooter } from './LegalFooter'


export const SettingsLayout = ({children, selectedRoute, selectedRepo }) => {
    const {
        user,
    } = useContext(UserContext)

    const [repos, setRepos] = useState([])
    useEffect(() => {
        maps.getOwnedRepos(user.uid).then((resp) => setRepos(resp))
    }, [])

return <Layout scrollable={true} routeInfo={<strong className={styles.settingsRouteInfo}>Settings</strong>}>
        <div className={styles.SettingsLayout}>
            <div className={styles.Sidebar}>
                <SettingsSidebar
                    repoConfigEnabled={user.account.canCreateRepos}
                    repos={repos}
                    selectedRepo={selectedRepo}
                    selectedRoute={selectedRoute} />
            </div>
            <div className={styles.Content}>
                {children}
                <LegalFooter />
            </div>

        </div>
    </Layout>
}
