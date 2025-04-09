import moment from 'moment'
import Tooltip from '@mui/material/Tooltip';

import styles from './BuildInfo.module.css'


export const BuildInfo = ({
    repo,
    branch,
    build,
}) => {

    if (!repo || !branch || !build) return null;

    const tooltipContent = <div>
        {repo.user_name
            ? <div>{repo.user_name} / {repo.name} / {branch}</div>
            : <div>{repo.name} / {branch}</div>}
        <div>commit {build.commit} indexed {moment(build.ended).fromNow()}</div>
        <div className={styles.commitMessage}>{build.commit_message}</div>
    </div>
    return (
        <Tooltip title={tooltipContent} placement='right'>
            <div className={styles.repoName}>{repo.name}</div>
        </Tooltip>
    )
}

