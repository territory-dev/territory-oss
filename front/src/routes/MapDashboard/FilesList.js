import { useContext, useCallback, useState, useMemo } from 'react'

import { GraphContext } from '../../contexts/graphContext'
import { checkIfScrollable } from '../../utils/checkIfScrollable'

import { List } from './List'
import styles from './FilesList.module.css'

export const FilesList = ({
    text,
    id,
    addNode,
}) => {

    const {
        setIsDisabled,
    } = useContext(GraphContext)

    const renderItem = useCallback((element) => {
        let [nodeId, tokenId] = [null, null]

        if (element.h) {
            [nodeId, tokenId] = element.h.split('#')
        }
        
        return (
            <div
                onClick={() => {
                    if (nodeId) {
                        addNode({
                            href: nodeId,
                            opener: id,
                            relation: 'container',
                        })
                    }
                }}
                className={styles.listItem}
                key={element.id}
            >
                {element.t}
            </div>
        )
    }, [id, addNode])


    return (
        <List
            renderItem={renderItem}
            className={styles.list}
            list={text}
            maxWidth={300}
            setIsDisabled={setIsDisabled}
        />
    )
}
