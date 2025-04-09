import { BuildInfo } from '../MapDashboard/BuildInfo'

import styles from './UnsavedMapInfo.module.css'


export const UnsavedMapInfo = ({
    repo,
    branch,
    build,
    onSavePropmptClick,
}) => {
    return (
        <div className={styles.mapData}>
            <h1 onClick={() => {onSavePropmptClick && onSavePropmptClick()}}>Unsaved Map</h1>
            <BuildInfo
                repo={repo}
                branch={branch}
                build={build}
            />
        </div>
    )
}
