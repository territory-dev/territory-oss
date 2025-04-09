import { useRef, useState, useCallback } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import classNames from 'classnames'

import { checkIfScrollable } from '../../utils/checkIfScrollable'

import styles from './List.module.css'

const VIRTUALISATION_THRESHOLD = 1000

const VirtualisedList = ({ renderItem, list, maxWidth, estimatedHeight, setIsDisabled, className }) => {
    const parentRef = useRef()

    const virtualizer = useVirtualizer({
        count: list.length,
        getScrollElement: () => parentRef.current,
        estimateSize: () => estimatedHeight,
    })

    return (
        <div
            ref={parentRef}
            className={classNames(className, styles.virtualList)}
            onMouseEnter={() => setIsDisabled(true)}
            onMouseLeave={() => setIsDisabled(false)}
        >
            <div
                className={styles.virtualListInnerWrapper}
                style={{
                    width: maxWidth,
                    height: `${virtualizer.getTotalSize()}px`,
                }}
            >
                {virtualizer.getVirtualItems().map(({ key, index, size, start }) => (
                    <div
                        key={key}
                        className={styles.virtualItemWrapper}
                        style={{
                            height: `${size}px`,
                            transform: `translateY(${start}px)`,
                        }}
                    >
                        {renderItem(list[index], index)}
                    </div>
                ))}
            </div>
        </div>
    )
}

export const List = ({
    renderItem,
    list,
    maxWidth,
    setIsDisabled,
    className,
    estimatedHeight,
}) => {
    const [isScrollable, setIsScrollable] = useState(list.length >= VIRTUALISATION_THRESHOLD)

    const refCallback = useCallback(
        (node) => {
            if (node && checkIfScrollable(node)) {
                setIsScrollable(true)
            } else {
                setIsScrollable(false)
            }
        },
        [setIsDisabled],
    )

    const blockIfScrollable = useCallback(() => {
        if (isScrollable) setIsDisabled(true)
    }, [isScrollable, setIsScrollable])


    if (!list) return
    return (list.length >= VIRTUALISATION_THRESHOLD)
        ? (
            <VirtualisedList
                renderItem={renderItem}
                list={list}
                maxWidth={maxWidth}
                setIsDisabled={setIsDisabled}
                className={className}
                estimatedHeight={estimatedHeight}
            />
        )
        : (
            <div
                className={classNames(className, styles.list)}
                onMouseEnter={blockIfScrollable}
                onMouseLeave={() => setIsDisabled(false)}
                ref={refCallback}
            >
                {list.map((item, index) => renderItem(item, index))}
            </div>
        )
}
