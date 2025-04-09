import { useContext } from 'react'
import { Link } from 'react-router-dom'

import { GraphContext } from '../contexts/graphContext'
import { UserMenu } from './UserMenu'

import styles from './Navbar.module.css'
import { PrimaryTip, PrimaryTipWrapper } from './PrimaryTip'
import { ActivityIndicatorLogo } from './ActivityIndicatorLogo'

export const Navbar = ({
    children,
    shareButton,
    routeInfo,
    userMenu,
}) => {
    const graphContext = useContext(GraphContext)
    return (
        <div className={styles.Navbar}>
            <div className={styles.nav}>
                <PrimaryTipWrapper>
                    <Link to="/">
                        <ActivityIndicatorLogo />
                    </Link>
                    {graphContext?.empty  &&
                        <PrimaryTip shiftLeft="7px" shiftDown="8px">
                            home
                        </PrimaryTip>}
                </PrimaryTipWrapper>
                {routeInfo || <div />}
            </div>
            {children}
            <div className={styles.rightIcons}>
                {shareButton}
                {userMenu || <UserMenu />}
            </div>
        </div>
    )
}
