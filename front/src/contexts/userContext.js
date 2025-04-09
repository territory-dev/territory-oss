import { createContext, useState, useEffect } from 'react'
import { onAuthStateChanged } from 'firebase/auth'
import { Loader } from '../components/Loader'
import { auth } from '../firebase'
import { maps } from '../api/api'

export const UserContext = createContext(undefined)

export const UserContextProvider = ({
    children,
}) => {
    const [user, setUser] = useState(null)
    const [isUserLoading, setIsUserLoading] = useState(true)

    useEffect(() => {
        onAuthStateChanged(auth, (newUser) => {
            if (newUser) {
                maps.getAccount()
                .then((account) => {
                    if (isUserLoading) setIsUserLoading(false)
                    setUser({account, ...newUser})
                }, (err) => {
                    if (err.response?.status === 403) {
                        setIsUserLoading(false)
                        setUser(null)
                        return auth.signOut()
                    } else {
                        throw err
                    }
                })
            } else {
                if (isUserLoading) setIsUserLoading(false)
                setUser(null)
            }
        })
    }, [])

    return (
        <UserContext.Provider value={{
            user,
            setUser,
            isUserLoading,
            setIsUserLoading,
            authDisabled: auth.disabled,
        }}>
            {isUserLoading ? <Loader /> : children}
        </UserContext.Provider>
    )
}
