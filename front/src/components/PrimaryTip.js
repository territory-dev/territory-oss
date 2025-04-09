import styles from './PrimaryTip.module.css'


export const PrimaryTipWrapper = ({children}) =>
    <div className={styles.PrimaryTipWrapper}>
        {children}
    </div>


export const PrimaryTip = ({children, shiftLeft, shiftDown}) =>
    <div className={styles.PrimaryTip} style={{ marginLeft: shiftLeft, marginTop: shiftDown }}>
        {children}
    </div>


export const Shortcut = ({children}) =>
    <span className={styles.Shortcut}>{children}</span>
