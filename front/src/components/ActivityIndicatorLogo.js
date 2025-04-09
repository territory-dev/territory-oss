import { useEffect, useState } from 'react'
import classnames from 'classnames'

import { ApiEvents, isActive } from '../api/api'

import { ReactComponent as NavbarLogo } from './NavbarLogo.svg'
import styles from './ActivityIndicatorLogo.module.css'


export const ActivityIndicatorLogo = () => {
    const [activity, setActivity] = useState(isActive());

    useEffect(() => {
        const onActivityStarted = () => {
            setActivity(true)
        }

        const onActivityCeased = () => {
            setActivity(false)
        }

        ApiEvents.on('activityStarted', onActivityStarted)
        ApiEvents.on('activityCeased', onActivityCeased)

        return () => {
            ApiEvents.off('activityStarted', onActivityStarted)
            ApiEvents.off('activityCeased', onActivityCeased)
        }
    })

    const classNames = activity
        ? classnames(styles.logo, 'tHome', styles.activity, 'tActivitiy')
        : classnames(styles.logo, 'tHome');
    return <NavbarLogo className={classNames} />
}
