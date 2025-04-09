import styles from './Tokens.module.css'

const classes = {
    Comment: styles.comment,
    Identifier: styles.identifier,
    Keyword: styles.keyword,
    Literal: styles.literal,
}

export const mapTokenTypeToClass = (type) => classes[type] || styles.other
