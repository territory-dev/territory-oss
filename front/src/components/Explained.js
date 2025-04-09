import Stack from '@mui/material/Stack'


import styles from './Explained.module.css'


export const Explained = ({children, explanation}) =>
    <div className={styles.Explained}>
        <div className={styles.content}>
            <Stack spacing={2}>
                {children}
            </Stack>
        </div>
        <div className={styles.explanation}>{explanation}</div>
    </div>
