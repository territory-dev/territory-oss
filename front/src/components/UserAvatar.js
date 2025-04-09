import classnames from 'classnames'

import styles from './UserAvatar.module.css'

export const UserAvatar = ({ url }) => {
    return (
        <div className={classnames(styles.avatar, 'tAvatar')}>
            {url && <img src={url} />}
        </div>
    )

}
