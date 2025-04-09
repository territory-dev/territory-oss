import styles from './NavOverlayButton.module.css'

export const NavOverlayButton = ({children, onClick}) =>
    <button className={styles.NavOverlayButton} onClick={onClick}>
        {children}
    </button>
