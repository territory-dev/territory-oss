import { useState } from 'react';

import Accordion from '@mui/material/Accordion';
import AccordionActions from '@mui/material/AccordionActions';
import AccordionSummary from '@mui/material/AccordionSummary';
import AccordionDetails from '@mui/material/AccordionDetails';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import RefreshIcon from '@mui/icons-material/Refresh';
import Button from '@mui/material/Button';
import Grid from '@mui/material/Grid';

import moment from 'moment'

import { useParams } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'

import { getBuildJobLog, getBuildJobs, getRepos, maps } from "../../api/api"

import styles from './Jobs.module.css'
import { SettingsLayout } from '../../components/SettingsLayout';
import { Loader } from '../../components/Loader';


const JobLogs = ({ job }) => {
    const [loadingLog, setLoadingLog] = useState()

    const getLog = (logUrl) => {
        getBuildJobLog(logUrl)
        .then(
            (resp) => {
                setLoadingLog(null);
                if (resp.url) window.open(resp.url, '_blank').focus()
            },
            () => {
                setLoadingLog(null);
            })
    }

    return <>
        {job.logs.map(log => {
            return <Button
                key={log.name}
                disabled={log.name == loadingLog}
                onClick={() => {
                    setLoadingLog(log.name)
                    getLog(log.url)
                }}
            >
                {log.name}
            </Button>
        })}
    </>
}


const RepoJobs = ({ repoId }) => {
    const { status, data, error, refetch, isFetching } = useQuery(
        ['repoBuildJobs', repoId],
        () => getBuildJobs(repoId),
        {
            retry: 0,
        }
    )

    const refresh = () => {
        refetch();
    }

    const refreshBtn = isFetching
        ? <Loader />
        : <Button className={styles.refresh} onClick={refresh}>
            <RefreshIcon/>
            Refresh
        </Button>

    if (status == 'loading') {
        return <div>Loading jobs...</div>
    } else if (data) {
        const jobs = data.jobs.toReversed()

        if (jobs.length == 0) {
            return <div>{refreshBtn}No jobs yet</div>
        }

        return <div>
            {refreshBtn}

            {data.build_requests_count > 0 &&
            <div>Pending build requests: {data.build_requests_count}</div>}

            <div className={styles.jobsHeader}>
                <Grid container spacing={2}>
                    <Grid item xs={1}>

                    </Grid>
                    <Grid item xs={3}>
                        Started
                    </Grid>
                    <Grid item xs={3}>
                        Run time
                    </Grid>
                    <Grid item xs={5}>
                        Status
                    </Grid>
                </Grid>
            </div>

            {jobs.map((r, i) => <Accordion defaultExpanded={i==0}>

                <AccordionSummary
                    expandIcon={<ExpandMoreIcon />}
                    aria-controls="panel1-content"
                    id="panel1-header"
                >
                    <Grid container spacing={2}>
                        <Grid item xs={1}>
                            {r.ready ? '🟢' : r.failed ? '🔴' : r.running ? '🏃🏻‍♀️' : null}
                        </Grid>
                        <Grid item xs={3}>
                            {moment(r.started).fromNow()}
                        </Grid>
                        <Grid item xs={3}>
                            {moment.duration(r.runtime, 'seconds').humanize()}
                        </Grid>
                        <Grid item xs={5}>
                            {r.status}
                        </Grid>
                    </Grid>
                </AccordionSummary>
                <AccordionDetails>
                    <Grid container spacing={2}>
                        <Grid item xs={2}>Job</Grid>
                        <Grid item xs={10}>{r.id}</Grid>
                        <Grid item xs={2}>Started</Grid>
                        <Grid item xs={10}>{r.started}</Grid>
                        {r.branch && <>
                        <Grid item xs={2}>Branch</Grid>
                        <Grid item xs={10}>{r.branch}</Grid>
                        </>}
                        {r.commit && <>
                            <Grid item xs={2}>Commit</Grid>
                            <Grid item xs={10}>{r.commit}</Grid>
                            <Grid item xs={2}></Grid>
                            <Grid item xs={10} className={styles.commitMessage}>{r.commit_message}</Grid>
                        </>}
                        <Grid item xs={2}>Logs</Grid>
                        <Grid item xs={10}>
                            <JobLogs job={r} />
                        </Grid>
                    </Grid>
                </AccordionDetails>
                <AccordionActions>
                </AccordionActions>
            </Accordion>)}
        </div>
    } else if (error) {
        return <div>
            {refreshBtn}
            Could not load jobs at this time
        </div>
    }
}


export const Jobs = () => {
    const { repoId } = useParams()

    const { status, data, error, refetch } = useQuery(
        ['repo', repoId],
        () => maps.getRepo(repoId),
        { retry: 0, }
    )

    let langInstructions
    if ((data?.lang == 'go') || (data?.lang == 'python'))
        langInstructions = <li>
            Run the uploader in the repo directory:
            <pre className="tUploadCommand">territory upload --repo-id {repoId} -l {data?.lang}</pre>
        </li>
    else
        langInstructions = <li>
            In the directory containing <code>compile_commands.json</code> run
            the uploader:
            <pre className="tUploadCommand">territory upload --repo-id {repoId} -l c</pre>
        </li>

    return <SettingsLayout selectedRoute="jobs" selectedRepo={repoId}>
        <h1>Recent {data?.name} builds</h1>

        {data?.manual && <div className={styles.uploadInstructions}>
            This repo has no tracking configured. To index code,&nbsp;
            <a href="https://github.com/territory-dev/cli?tab=readme-ov-file#uploading-sources-with-the-territorydev-cli-client">
            upload it first using the command line client
            </a>.

            <ol>
                <li>
                    Install the client using pip:
                    <pre>pip install territory</pre>
                </li>
                {langInstructions}
            </ol>

        </div>}

        <div className={styles.content}>
            {repoId ? <RepoJobs repoId={repoId} /> : null}
        </div>
    </SettingsLayout>
}
