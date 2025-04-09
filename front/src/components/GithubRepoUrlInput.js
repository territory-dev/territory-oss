import { useCallback, useState } from "react";
import { decodeGithubUrl } from 'territory-schema-maps/github';

import styles from './GithubRepoUrlInput.module.css'

export const GithubRepoUrlInput = ({value, onChange}) => {
    const [innerValue, setInnerValue] = useState(value || "");

    const onUrlPaste = useCallback((ev) => {
        const pastedText = ev.clipboardData.getData("text")
        const decoded = decodeGithubUrl(pastedText)
        let v;
        if (decoded) {
            v=(`${decoded.owner}/${decoded.repository}`)
        } else {
            v=(pastedText)
        }
        ev.preventDefault();
        setInnerValue(v)
        onChange && onChange(v)
    }, [onChange]);

    const innerOnChange = useCallback((ev) => {
        setInnerValue(ev.target.value)
        onChange && onChange(ev.target.value)
    }, [onChange]);

    return <div className={styles.GithubRepoUrlInput}>
        github.com/<input name="repoUrl" value={innerValue} onChange={innerOnChange} onPaste={onUrlPaste}></input>
    </div>
}

