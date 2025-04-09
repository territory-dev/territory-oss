import React from 'react'
import classnames from 'classnames'

import styles from './Loader.module.css'

export const Loader = ({ size = 'md', className = '' }) => (
    <div className={classnames(styles.wrapper, className)}>
        <div
            className={classnames(
                styles.loader,
                size === 'sm' && styles.sm,
                size === 'xs' && styles.xs,
            )}
        />
    </div>
)
