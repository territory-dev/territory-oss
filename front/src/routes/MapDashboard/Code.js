import { useCallback, useContext, useMemo } from 'react'
import classnames from 'classnames'

import { mapTokenTypeToClass } from './mapTokenToClass'
import { getTextLines } from '../../utils/mapTextToComponents'
import { GraphContext } from '../../contexts/graphContext'

import { List } from './List'
import styles from './Code.module.css'


const calibrateFontWidth = () => {
    let e = document.createElement('span')
    e.textContent = 'a'
    e.style = "font-family: monospace, monospace; font-size: 16px;"

    document.body.append(e)
    const aWidth = e.getBoundingClientRect().width
    /*
    console.log('using width', aWidth)

    e.textContent = 'l'.repeat(99)
    const lsWidth = e.getBoundingClientRect().width
    console.log('l error', 99 * aWidth / lsWidth - 1)

    e.textContent = String.fromCharCode(0xa0).repeat(101)
    const spsWidth = e.getBoundingClientRect().width
    console.log('nbsp error', aWidth * 101 / spsWidth - 1)
    */
    e.remove()

    return aWidth
}

const W = calibrateFontWidth()

const shouldHighlight = (highlight, myHref, myHrefTok, mySym) => {
    if (!highlight) return false
    const {href, href_tok, sym} = highlight
    return (mySym && (href === mySym)) || (mySym && (sym === mySym)) || (myHref && (href == myHref) && (href_tok == myHrefTok))
}

export const Code = ({
    text,
    id,
    start,
    addNode,
    showLines,
    onLineNumMouseOver,
    onLineNumMouseOut,
}) => {

    const {
        setShowRefs,
        setIsDisabled,
        highlight,
        setHighlight,
    } = useContext(GraphContext)

    const getClickHandler = useCallback((h, r) => {
        if (r) {
            return () => setShowRefs({ r, h, id })
        } else if (h) {
            return () => addNode({
                href: h,
                opener: id,
                relation: 'container',
            })
        } else {
            return null
        }
    }, [setShowRefs, addNode, id])

    const textLines = useMemo(() => getTextLines(text, start), [text, start])

    const renderItem = useCallback(({ tokens, lineNum, displayLineNum }) => {
        return <div className={styles.line} key={displayLineNum}>
                    <span
                        className={styles.lineNum}
                        onMouseOver={onLineNumMouseOver}
                        onMouseLeave={onLineNumMouseOut}
                    >{lineNum} </span>
                    {tokens.map(({ component, h, ht, s, r, t, T, id }, index) => (
                        <span
                            key={`${id}-${index}`}
                            className={classnames(
                                mapTokenTypeToClass(T),
                                styles.token,
                                h && styles.clickable,
                                r && styles.underline,
                                shouldHighlight(highlight, h, ht, s) && styles.highlight,
                            )}
                            onClick={getClickHandler(h, r)}
                            onMouseOver={() => setHighlight({href: h, href_tok: ht, sym: s})}
                            onMouseLeave={() => setHighlight(null)}
                        >
                            {component}
                        </span>
                    ))}
                    &nbsp;
                </div>
    }, [highlight])


    const showLinesClass = showLines ? styles.showLines : styles.noShowLines;

    return (
        <List
            renderItem={renderItem}
            className={classnames(styles.code, 'tCode', showLinesClass)}
            list={textLines.list}
            maxWidth={`${textLines.maxLength * W + 2}px`}
            setIsDisabled={setIsDisabled}
            estimatedHeight={19}
        />
    )
}
