export const checkIfScrollable = (element) => {
    const css = window.getComputedStyle(element)
    const overflow = css.getPropertyValue('overflow-y')
    if (overflow === 'scroll') {
        const { scrollHeight, clientHeight } = element
        return scrollHeight > clientHeight
    } else {
        return false
    }

}