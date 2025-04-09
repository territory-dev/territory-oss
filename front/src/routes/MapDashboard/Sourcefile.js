import { useContext, useMemo, useCallback } from 'react'

import { GraphContext } from '../../contexts/graphContext'

import { List } from './List'
import { mapTokenTypeToClass } from './mapTokenToClass'
import styles from './Sourcefile.module.css'

const mapText = (text) => {
    const textList = []
    let currentLine = []

    text.forEach(({
        id, h, t, T,
    }) => {
        currentLine.push({
            id, t, T,
        })

        if (t.endsWith('\n')) {
            textList.push({ tokens: currentLine, href: h })
            currentLine = []
        }

    })
    return textList
}

export const Sourcefile = ({
    text,
    id,
    addNode,
}) => {

    const {
        setIsDisabled,
    } = useContext(GraphContext)

    const linesList = useMemo(() => mapText(text), [text])

    const maxLineLength = useMemo(() => {
        let max = 0
        linesList.forEach(({ tokens }) => {
            let lineLength = 0
            tokens.forEach(({ t }) => {
                lineLength += t.length
            })
            max = Math.max(max, lineLength)
        })

        return max
    }, [linesList])

    const renderItem = useCallback(({ href, tokens }, lineIdx) => {
        let [nodeId, tokenId] = [null, null]

        if (href) {
            [nodeId, tokenId] = href.split('#')
        }

        const clickHandler = nodeId
            ? () => addNode({
                href: nodeId,
                opener: id,
                relation: 'container',
            }) : () => {}
        
        return (
            <div className={styles.listItem} onClick={clickHandler} key={lineIdx}>
                {tokens.map(({ id, t, T }) => (
                    <span key={id} className={mapTokenTypeToClass(T)}>
                        {t}
                    </span>
                ))}
            </div>
        )
    }, [addNode, id])

    return (
        <List
            list={linesList}
            className={styles.list}
            renderItem={renderItem}
            setIsDisabled={setIsDisabled}
            estimatedHeight={30}
            maxWidth={`${maxLineLength * 16 * 0.6}px`}
        />
    )
}
