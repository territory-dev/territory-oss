import { Navbar } from './Navbar'
import { LegalFooter } from './LegalFooter'
import classNames from 'classnames'

import styles from './Layout.module.css'

export const Layout = ({
    children,
    shareButton = null,
    searchBar = null,
    scrollable = false,
    userMenu = null,
    routeInfo,
    legalFooter = false,
 }) => (
    <div className={classNames(styles.Layout, scrollable && styles.Scrollable)}>
        <div className={styles.Background}/>
        <Navbar shareButton={shareButton} routeInfo={routeInfo} userMenu={userMenu}>
            {searchBar}
        </Navbar>
        {children}
        <div className={styles.FooterOverlay}>
            Talk to us on <a target="_blank" href="https://discord.gg/34FmMARaKP">Discord</a> and <a target="_blank" href="https://x.com/territory_dev">X.com</a>
        </div>
        {legalFooter && <LegalFooter />}
    </div>
)
