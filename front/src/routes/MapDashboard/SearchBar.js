import React, { useState, useCallback, useContext, useRef, useEffect } from 'react'
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faFolder } from '@fortawesome/free-regular-svg-icons'
import classnames from 'classnames'
import { Loader } from '../../components/Loader'
import { GraphContext } from '../../contexts/graphContext'
import { SearchContext } from '../../contexts/searchContext'
import { useKey } from '../../hooks/useKey'

import styles from './SearchBar.module.css'
import { useQuery } from '@tanstack/react-query'
import { PrimaryTip, PrimaryTipWrapper, Shortcut } from '../../components/PrimaryTip'

const Suggestions = ({suggestions, handleClick, isFetching, selectedIndex, setSelectedIndex}) => {
    if (!suggestions?.length) {
        return isFetching ? null : (
            <div className={styles.option}>
                Nothing found :(
            </div>
        )
    }

    const items = suggestions.map(
        ({ kind, href, key, type, path, positions }, idx) => {
            const selected = (idx == selectedIndex)

            let keyChars;
            if (positions) {
                let positionsStack = positions.toReversed()
                keyChars = key.split('').map((c, i) => {
                    if (i === positionsStack[positionsStack.length-1]) {
                        positionsStack.pop()
                        return <em key={i}>{c}</em>
                    } else {
                        return <span key={i}>{c}</span>
                    }
                })
            } else {
                keyChars = key;
            }
            return (
                <div
                    className={selected ? classnames(styles.option, styles.selected) : styles.option}
                    key={idx}
                    onClick={() => handleClick(href)}
                    onMouseEnter={() => setSelectedIndex(idx)}
                    tabIndex={-1}
                >
                    {keyChars}
                    <div className={styles.suggestionDetails}>
                        {type ? <span>{type}</span> : null}
                        {path ? <span> in {path}</span> : null}
                    </div>
                </div>
            )
        }
    )

    return <div>{items}</div>
}

export const SearchBar = () => {
    const [selectedIndex, setSelectedIndex] = useState(0)
    const [query, setQuery] = useState('')
    const [limit, setLimit] = useState(10)
    const wrapperRef = useRef()
    const inputRef = useRef()
    const dropdownRef = useRef()
    const [isDropdownOpen, setIsDropdownOpen] = useState(false)
    const { addNodeQuery, rootId, empty  } = useContext(GraphContext)
    const { search, isIndexLoading} = useContext(SearchContext)

    const addNode = addNodeQuery.mutate

    const searchQuery = useQuery(
        ['search', query, limit],
        () => search(query, limit),
        { enabled: !!query, keepPreviousData: !!query }
    )

    const handleBlur = (e) => {
        if (!wrapperRef.current.contains(e.target) && !(e.target.id === 'load-more')) {
            setIsDropdownOpen(false)
            inputRef?.current.blur()
        }
    }
    const handleBlurEsc = (e) => {
        setIsDropdownOpen(false)
        inputRef?.current.blur()
    }

    const handleClick = useCallback((href) => {
        addNode({ href })
        setIsDropdownOpen(false)
        inputRef?.current.blur()
    })

    useEffect(() => {
        document.addEventListener('click', handleBlur)

        return () => document.removeEventListener('click', handleBlur)
    }, [])

    useEffect(() => {
        setLimit(10)
    }, [query, isDropdownOpen])

    const focusInput = useCallback((e) => {
        if (inputRef?.current && inputRef.current !== document.activeElement) {
            e.preventDefault()
            setSelectedIndex(0)
            inputRef.current.focus()
        }
    }, [])

    const selectNextSuggestion = useCallback((e) => {
        if (!searchQuery.data) return
        e.preventDefault()
        if (selectedIndex < searchQuery.data.length - 1)
            setSelectedIndex(selectedIndex + 1)
        else
            setLimit(l => Math.max(l, l*2))
    }, [selectedIndex, searchQuery.data?.length])

    const selectPrevSuggestion = useCallback((e) => {
        if (!searchQuery.data) return
        e.preventDefault()
        setSelectedIndex(Math.max( selectedIndex - 1, 0 ))
    }, [selectedIndex, searchQuery.data?.length])

    const confirmSelection = useCallback(() => {
        if (!searchQuery.data) return
        if (!searchQuery.data[selectedIndex]) return
        handleClick(searchQuery.data[selectedIndex].href)
    }, [selectedIndex, searchQuery.data])

    useKey('/', focusInput)

    useKey('ArrowUp', selectPrevSuggestion)

    useKey('ArrowDown', selectNextSuggestion)

    useKey('Enter', confirmSelection)

    useKey('Escape', handleBlurEsc)

    return (
        <div
            ref={wrapperRef}
            className={styles.searchbar}
        >
            {isIndexLoading && (
                <div className={styles.loaderOverlay}>
                    <Loader size="sm" />
                </div>
            )}
            <PrimaryTipWrapper>
                <input
                    placeholder={isIndexLoading ? '' : 'Search for symbols, files and directories...'}
                    onFocus={() => setIsDropdownOpen(true)}
                    onChange={(e) => setQuery(e.target.value)}
                    ref={inputRef}
                />
                {empty &&
                    <PrimaryTip shiftLeft="10px">
                        <Shortcut>/</Shortcut>
                        <div>start searching</div>
                    </PrimaryTip>}
            </PrimaryTipWrapper>
            {isDropdownOpen && query && (
                <div ref={dropdownRef} className={styles.dropdown + ' tSearchResults'}>
                    <Suggestions
                        suggestions={searchQuery.data}
                        handleClick={handleClick}
                        isFetching={searchQuery.isFetching}
                        selectedIndex={selectedIndex}
                        setSelectedIndex={setSelectedIndex}
                    />
                    {searchQuery.isFetching ? <Loader /> : (
                        (searchQuery.data?.length == limit) && <div
                            className={classnames(styles.loadMore, styles.option)}
                            key="load-more"
                            id="load-more"
                            tabIndex={-1}
                            onClick={() => setLimit(l => l * 2)}
                        >
                            Load more
                        </div>
                    )}
                </div>
            )}
            {rootId && !isIndexLoading
                ? (
                    <PrimaryTipWrapper>
                        <button
                            className={styles.rootButton}
                            onClick={() => addNode({ href: rootId })}
                        >
                            <FontAwesomeIcon
                                icon={faFolder}
                                size="xs"
                            />
                        </button>

                        {empty &&
                            <PrimaryTip shiftLeft="15px">
                                repository root
                            </PrimaryTip>}
                    </PrimaryTipWrapper>
                ) : null
            }
        </div>
    )
}
