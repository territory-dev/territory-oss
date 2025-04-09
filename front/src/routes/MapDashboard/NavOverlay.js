import { useContext } from 'react'
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faLocationCrosshairs } from '@fortawesome/free-solid-svg-icons'

import { GraphContext } from '../../contexts/graphContext'

import styles from './NavOverlay.module.css'
import { NavOverlayButton } from './NavOverlayButton'

export const NavOverlay = () => {
    const {
        requestCenter
    } = useContext(GraphContext)

    return <div className={styles.NavOverlay}>
        <NavOverlayButton onClick={requestCenter}>
            <FontAwesomeIcon icon={faLocationCrosshairs} size="lg" />
        </NavOverlayButton>
    </div>
}
