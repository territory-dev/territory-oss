import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import DoneIcon from '@mui/icons-material/Done';
import classNames from "classnames";

import { createMap, TRACK_ENDPOINT } from '../../api/api';
import { Loader } from "../../components/Loader";
import { QuickBuildFailureMessage } from "./QuickBuildFailureMessage";

import styles from './QuickBuildState.module.css'
import { createMapWithRetry } from "../../utils/newMapUtils";


export const QuickBuildState = ({buildRequestId, repoId, repoData}) => {
    const [state, setState] = useState({
        status: "Waiting for a worker",
        ready: false,
        failed: false,
    });
    const [error, setError] = useState(null);

    const [log, setLog] = useState('');

    const navigate = useNavigate()

    const newMapFromState = useCallback((repoId, state) => {
        createMapWithRetry(
            {
                repoId,
                branchId: state.branch,
                buildId: state.build_id,
                display_name: `${state.repo_name} Quick Map`,
                public: false,
            },
            (newMapId) => {
                if (!newMapId) return;
                navigate(`/maps/${newMapId}`)
            });
    });

    useEffect(() => {
        const eventSource = new EventSource(
            `${TRACK_ENDPOINT}/build-request-immediate/${repoId}/${buildRequestId}`);

        let lastLog = '';
        let prevState = null;
        let state = null;


        eventSource.onmessage = (event) => {
            prevState = state;
            state = JSON.parse(event.data);
            setState(state);

            if (state.log) {
                if (state.log != lastLog) {
                    lastLog = state.log;
                    setLog(log => log + lastLog);
                }
            } else {
                setLog('');
                lastLog = '';
            }

            if (state.ready && !prevState?.ready) {
                newMapFromState(repoId, state);
            }
        }

        eventSource.onerror = (ev) => {
            setError(ev.target)
        }

        return () => {
            eventSource.close();
        };

    }, [buildRequestId, repoId]);

    if (error) {
        return <Wrap>Connection error</Wrap>
    } else if (state.ready) {
        return <Wrap>
            <div className={styles.statusBlock}>
                <DoneIcon />
            <div className={classNames("tQuickBuildStatus", styles.statusText)}>Repository indexed.</div>
            </div>
            <button onClick={() => newMapFromState(repoId, state)}>New map</button>
        </Wrap>
    } else if (state.failed) {
        return <Wrap><QuickBuildFailureMessage repoId={repoId} /></Wrap>
    } else {
        return <Wrap>
            <div className={styles.statusBlock}>
                <Loader size="sm" className={styles.loader} /> <div className={classNames("tQuickBuildStatus", styles.statusText)}>{ state.status }</div>
            </div>
            {log && <div className={classNames(styles.log, "tLog")}>
                <div>{log}</div>
            </div>}
        </Wrap>
    }
}


export const Wrap = ({children}) => <div className={classNames("tQuickBuildState", styles.QuickBuildState)}>
    {children}
</div>
