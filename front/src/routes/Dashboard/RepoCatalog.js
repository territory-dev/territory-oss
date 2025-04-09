import _ from 'lodash'
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faGlobe } from '@fortawesome/free-solid-svg-icons'
import { useState, useEffect, useContext, useMemo, useCallback } from 'react'

import { maps } from '../../api/api'
import { UserContext } from '../../contexts/userContext'

import styles from './RepoCatalog.module.css'
import { Link } from 'react-router-dom'
import { Loader } from '../../components/Loader'
import { DashboardHeader } from '../../components/DashboardHeader'


export const RepoCatalog = ({createMap}) => {
    const {
        user,
        authDisabled,
    } = useContext(UserContext)

    const [repos, setRepos] = useState(null)

    const reposLists = useMemo(() => {
        const result = {
            my: [],
            sharedWithMe: [],
            public: [],
        }

        if (!repos) return result;

        for (let doc of repos) {
            let data = doc;
            let dest;
            if (user && (data.owner == user.uid))
                dest = result.my;
            else if (user && _.includes(data.sharedWithUsers, user.uid))
                dest = result.sharedWithMe;
            else if (data.public)
                dest = result.public;
            else
                console.error("received a repo that we shouldn't see", doc, data);

            dest.push(doc)
        }
        return result
    }, [repos])

    useEffect(() => {
        maps.getReposWithBuilds().then((resp) => setRepos(resp))
    }, [])

    const items = (key, header, list) => <div key={key}>
        <DashboardHeader>{header}</DashboardHeader>
        <div className={styles.list} >
            {list.map(repo => <RepoCatalogItem key={repo.id} repo={repo} createMap={createMap} />)}
        </div>
    </div>

    if (repos === null) return <Loader />
    return <>
        <div className={styles.repoCatalog}>
            {(reposLists.my?.length > 0) && items('my', 'My repositories', reposLists.my)}
            {(reposLists.sharedWithMe?.length > 0) && items('shared', 'Shared with me', reposLists.sharedWithMe)}
            {items('public', 'Recently indexed', reposLists.public)}
        </div>

        {authDisabled || <div className={styles.buildsNote}>
            Don't see the repo you are looking for? <Link to="/repos/new">Import it here</Link>.
        </div>}
    </>
}


const RepoCatalogItem = ({repo, createMap}) => {
    return <div className={styles.item}>
        <a href="#" onClick={() => createMap(repo.id, repo)} >
            {repo.public && <div className={styles.shareIcon} >
                <FontAwesomeIcon size="xs" icon={faGlobe}/>
            </div>}
            <div className={styles.name}>
                {repo.name}
            </div>
            <div className={styles.origin}>{repo.origin}</div>

            {repo.user_name
                ? <div className={styles.by}>indexed by <span className={styles.byName}>{repo.user_name}</span></div>
                : <div className={styles.by}>indexed by Territory</div>
            }
        </a>
    </div>
}
