import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faXmark } from '@fortawesome/free-solid-svg-icons'

import styles from './Modal.module.css'


export const Modal = ({ closeModal, style = {}, children }) => {

    return (
        <>
            <div className={styles.overlay} />
            <div className={styles.modal} style={style}>
                <FontAwesomeIcon
                    icon={faXmark}
                    size="xl"
                    onClick={closeModal}
                    className={styles.closeIcon}
                />
                {children}
            </div>
        </>
    )
}
