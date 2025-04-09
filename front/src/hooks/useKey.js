import { useEffect } from "react"


export function useKey(
  key,
  onKeyPressed,
) {
  useEffect(() => {
    function keyDownHandler(e) {
      if (e.key === key) {
        onKeyPressed(e)
      }
    }

    document.addEventListener("keydown", keyDownHandler)

    return () => {
      document.removeEventListener("keydown", keyDownHandler)
    };
  }, [key, onKeyPressed])
}
