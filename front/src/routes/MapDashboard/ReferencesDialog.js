import { useCallback, useContext } from 'react'
import { useQuery } from '@tanstack/react-query'
import classNames from 'classnames'
import CloseIcon from '@mui/icons-material/Close';

import { GraphContext } from '../../contexts/graphContext'
import { getReferences } from '../../api/api'
import { Loader } from '../../components/Loader'
import { useKey } from '../../hooks/useKey'

import styles from './ReferencesDialog.module.css'

const ZERO_WIDTH_SPACE = '\u200b'

const mapData = (data, addNode, id) => {
    if (!data) return <div></div>

    return (
        <ul className='tReferences'>
            {data.refs.map(({ href, context, use_path, use_location }) => (
                <li
                    key={use_path + ':' + use_location.line + ':' + use_location.column}
                    onClick={() => addNode({
                        href,
                        opener: id,
                        relation: 'reference',
                    })}
                    className={styles.listItem}
                >
                    <div className={styles.filePath}>{use_path.replaceAll('/', '/'+ZERO_WIDTH_SPACE)}</div>
                    {use_location && (
                        <div className={styles.location}>
                            Line <span className={styles.lineNo}>{use_location.line}</span>,
                            Column <span className={styles.colNo}>{use_location.column}</span>
                        </div>
                    )}
                    {context && <div className={styles.context}>{context}</div>}
                </li>
            ))}
        </ul>
    )
}

export const ReferencesDialog = () => {
    const {
        showRefs,
        setShowRefs,
        addNode,
        codeStorageConfig,
    } = useContext(GraphContext)

    const {
        data,
        isLoading,
    } = useQuery(
        ['references', showRefs],
        () => getReferences(codeStorageConfig, showRefs?.r),
        {
            enabled: !!showRefs?.r,
        }
    )

    const goToDefinitionClicked = useCallback(() => {
        addNode({
            href: showRefs.h,
            opener: showRefs.id,
            relation: 'container',
        })
    }, [showRefs, addNode])

    useKey('Escape', useCallback(() => { setShowRefs(null) }, [setShowRefs]))

    return (
        <div
            className={classNames(
                styles.dialog,
                showRefs && styles.open,
            )}
        >
            <div className={styles.header}>
                <h3>Edges</h3>
                <button
                    className={styles.closeButton}
                    onClick={() => setShowRefs(null)}
                >
                    <CloseIcon size={24} />
                </button>
            </div>

            <div className={styles.list}>
                <h4>Definition</h4>
                <ul className={styles.list}>
                    <li className={styles.listItem} onClick={goToDefinitionClicked}>
                        <div className={styles.goToDefinition}>Go to definition</div>
                    </li>
                </ul>

                <h4>References</h4>
                {isLoading ? <Loader /> : mapData(data, addNode, showRefs?.id)}
            </div>
        </div>
    )
}
