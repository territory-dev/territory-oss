import { useCallback, useContext, useState } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";

import { maps } from '../../api/api';
import { Loader } from "../../components/Loader";
import { UserContext } from '../../contexts/userContext'
import { QuickBuildFailureMessage } from "../QuickBuildState/QuickBuildFailureMessage";
import { GithubRepoUrlInput } from "../../components/GithubRepoUrlInput";
import { DashboardHeader } from "../../components/DashboardHeader";

import styles from './QuickBuildForm.module.css'
import { Button } from "@mui/material";
import classNames from "classnames";
import { createMapWithDefaults } from "../../utils/newMapUtils";


export const QuickBuildForm = ({createMap}) => {
    const [requestState, setRequestState] = useState(null);
    const [searchParams] = useSearchParams()
    const [repoUrl, setRepoUrl] = useState(searchParams.get('qmRepoUrl') || '');

    const {user} = useContext(UserContext);
    const navigate = useNavigate();

    const onSubmit = useCallback(async (ev) => {
        ev.preventDefault();

        if (!user) {
            const uspLayerOne = new URLSearchParams();
            uspLayerOne.append('qmRepoUrl', repoUrl);

            const uspLayerTwo = new URLSearchParams();
            uspLayerTwo.append('redirect', '/?' + uspLayerOne.toString());

            navigate('/login?' + uspLayerTwo.toString());
            return;
        }

        setRequestState('pending')
        const result = await maps.requestImmediateBuild({ url: repoUrl });
        setRequestState(result);

        if (result.ok)
            navigate(`/repos/${result.repo_id}/buildreq/${result.build_request_id}`);
        if ((result.error == 'exists') && createMap) {
            createMapWithDefaults(result.repo_id).then(newMapId => {
                navigate(`/maps/${newMapId}`)
            });
        }
    }, [repoUrl]);


    const form = <form onSubmit={onSubmit}>
        <GithubRepoUrlInput value={repoUrl} onChange={setRepoUrl} />
        <Button variant="contained" onClick={onSubmit} size="large">Go</Button>
        <div className={styles.customBuild}>
            or make a <Link to="/repos/new">custom build</Link>
        </div>
    </form>;

    if (requestState === null) {
        return <Wrap>{form}</Wrap>;
    } else if (requestState === 'pending') {
        return <Wrap><Loader /></Wrap>
    } else {
        let err;
        switch (requestState.error) {
            case 'badurl':
                err = <div>Incorrect Github URL.</div>;
                break;
            case 'noauto':
                err = <QuickBuildFailureMessage repoId={requestState.repo_id} />
                break;
            case 'exists':
                err = '';
                break;
            default:
                err = <div>{requestState.error}</div>
        }

        return <Wrap>
            {form}
            {err}
        </Wrap>;
    }
}


const Wrap = ({children}) => <div className={classNames(styles.wrap, 'tQuickBuild')}>
    <DashboardHeader>Quick map</DashboardHeader>
    {children}
</div>;
