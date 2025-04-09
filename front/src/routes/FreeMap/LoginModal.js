import { useContext, useMemo, useEffect, useState } from "react"
import { useSearchParams } from 'react-router-dom'
import CloseIcon from '@mui/icons-material/Close';
import IconButton from '@mui/material/IconButton';

import { GraphContext } from "../../contexts/graphContext"
import { renderLogin } from "../Login/renderLogin";

import styles from './LoginModal.module.css'

const THRESHOLD = 3;

export const LoginModal = ({ isOpened, setIsOpened, repoId, branch, buildId }) => {
    const [thresholdPassed, setThresholdPassed] = useState(false);
    const { nodes, relations } = useContext(GraphContext);
    const [numberOfNodes, setNumberOfNodes] = useState(!Object.values(nodes).length)

    const saveMapPath = useMemo(() =>
        `/maps/local/${repoId}/${encodeURIComponent(branch)}/${buildId}?showLogin=1`
    , [repoId, branch, buildId, nodes, relations])

    useEffect(() => {
        const currentNodesNumber = Object.values(nodes).length;
        if (!thresholdPassed && currentNodesNumber >= THRESHOLD) {
            setThresholdPassed(true)
            setIsOpened(true)
        }
        if (thresholdPassed && currentNodesNumber > numberOfNodes) {
            setIsOpened(true)
        }
        setNumberOfNodes(currentNodesNumber)
    }, [nodes])



    useEffect(() => {
        if (isOpened) renderLogin({ redirectUrl: saveMapPath })
    }, [isOpened])

    if (!isOpened) return null

    return (
        <div className={styles.LoginModalWrapper}>

            <div className={styles.LoginModal}>
                <div className={styles.IconRow}>
                    <IconButton
                        onClick={() => setIsOpened(false)}
                    >
                        <CloseIcon size={24} />
                    </IconButton>
                </div>
                <div className={styles.header}>
                    <div>Don't loose your map!</div>
                    <div>Log in to store and share maps, add repositories and get the latest features.</div>
                </div>
                <div id="render-login-ui"></div>
            </div>
        </div>
    )
}
