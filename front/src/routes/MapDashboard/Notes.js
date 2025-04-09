import { Note } from "./Note";

export const Notes = ({notes}) => Object.keys(notes).map((k) => {
    return <Note id={k} key={k} />
})
