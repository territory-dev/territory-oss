import { cloneElement } from 'react'

const nbsp = String.fromCharCode(0xa0)

export const textLayout = (line, column) =>
    (text) => {
        const components = []
        let token = ''

        for (let i = 0; i < text.length; i++) {
            if (['\n', '\t', ' '].includes(text[i])) {
                if (token) components.push(<span key={i}>{token}</span>)
                token = ''
                if (text[i] === '\n') {
                    components.push(<br key={`${i}br`} />)
                    line++
                    column = 1
                }
                if (text[i] === '\t') {
                    let spaces = nbsp.repeat(8 - column % 8);
                    token += spaces
                    column += spaces.length
                }
                if (text[i] === ' ') {
                    token += nbsp
                    column++
                }
            } else {
                token += text[i]
                column++
            }
        }
        if (token.length) {
            components.push(<span key="last">{token}</span>)
        }

        return components
    }

export const getTextLines = (text, start) => {
    const lines = []
    let maxLength = 0
    let { line: realLineNum, col } = start
    let displayLineNum = 1
    let column = col

    let currLine = {tokens: [], lineNum: realLineNum, displayLineNum }
    let currLength = col-1

    // indent first line if it doesn't start at column 1
    if (col > 0) {
        const preSpace = nbsp.repeat(col)
        currLine.tokens.push({
            component: <span key="preSpace">{preSpace}</span>,
            id: null, t: preSpace, T: 'WS', h: null, r: null,
        })
    }

    for (let i = 0; i < text.length; i++) {
        const { id, t, T, h, ht, s, r, N } = text[i]

        realLineNum = N;
        let token = ''
        for (let j = 0; j < t.length; j++) {
            if (['\n', '\t', ' '].includes(t[j])) {
                if (token) currLine.tokens.push({
                    component: <span key={`${i}-${j}`}>{token}</span>,
                    id, t: token, T, h, ht, s, r, N: realLineNum,
                })
                token = ''
                if (t[j] === '\n') {
                    lines.push(currLine)
                    realLineNum++
                    displayLineNum++
                    currLine = {tokens: [], lineNum: realLineNum, displayLineNum}
                    column = 1
                    maxLength = Math.max(currLength, maxLength)
                    currLength = 0
                }
                if (t[j] === '\t') {
                    let spaces = nbsp.repeat(8 - column % 8);
                    token += spaces
                    currLength += spaces.length
                    column += spaces.length
                }
                if (t[j] === ' ') {
                    token += nbsp
                    currLength++
                    column++
                }
            } else {
                token += t[j]
                currLength++
                column++
            }
        }
        if (token) currLine.tokens.push({
            component: <span key={`${i}-last`}>{token}</span>,
            id, t: token, T, h, ht, s, r, N: realLineNum,
        })
    }

    lines.push(currLine)
    maxLength = Math.max(currLength, maxLength)

    return {
        list: lines,
        maxLength,
    }
}
