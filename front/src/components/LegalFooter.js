import styles from './LegalFooter.module.css'

export const {
    REACT_APP_DATA_POLICY_URL,
    REACT_APP_TOS_URL,
} = process.env

export const LegalFooter = () =>
    <div className={styles.LegalFooter}>
        Copyright 2024  Territory.dev
        {REACT_APP_DATA_POLICY_URL &&
            <span>| <a href={REACT_APP_DATA_POLICY_URL}>Data Policy</a></span>}
        {REACT_APP_TOS_URL &&
            <span>| <a href={REACT_APP_TOS_URL}>Terms of Service</a></span>}
    </div>
