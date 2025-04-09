import { useContext, useState, useRef, useEffect, useMemo } from 'react'
import classnames from 'classnames'
import { signOut } from 'firebase/auth'
import { useNavigate } from 'react-router-dom'
import Button from '@mui/material/Button'

import { UserContext } from '../contexts/userContext'
import { auth } from '../firebase'
import { UserAvatar } from './UserAvatar'

import styles from './UserMenu.module.css'

export const UserMenu = ({ loginRedirect, onLogin }) => {
    const {
        user,
        setUser,
        authDisabled,
    } = useContext(UserContext)

    const navigate = useNavigate()

    const [isDropdownOpen, setIsDropdownOpen] = useState(false)
    const wrapperRef = useRef()

    const logout = () => { authDisabled || signOut(auth) }
    const goToMyRepos = () => { navigate('/repos/new') }
    const goToSettings = () => { navigate('/account') }

    const handleBlur = (e) => {
        if (!wrapperRef?.current?.contains(e.target)) {
            setIsDropdownOpen(false)
        }
    }

    useEffect(() => {
        document.addEventListener('click', handleBlur)

        return () => document.removeEventListener('click', handleBlur)
    })

    const loginCallback = useMemo(() => (
        onLogin
        || (() => navigate('/login', { state: { redirect: loginRedirect }}))
    ), [navigate, onLogin])

    if (authDisabled) return null;

    if (!user) {
        return (
            <Button
                onClick={loginCallback}
            >
                Log In
            </Button>
        )
    }

    return (
        <div
            className={classnames(
                styles.wrapper,
                isDropdownOpen ? styles.opened : styles.closed
            )}
            ref={wrapperRef}
            onClick={() => setIsDropdownOpen(isOpen => !isOpen)}
        >
            <UserAvatar url={user.photoURL} />
            {isDropdownOpen && (
                <div
                    className={styles.dropdown}
                >
                    {user?.displayName && (
                        <div className={styles.username}>{user.displayName}</div>
                    )}
                    <div
                        className={styles.link}
                        onClick={goToMyRepos}
                    >
                        My repositories
                    </div>
                    <div
                        className={classnames(styles.link, 'tAccountSettings')}
                        onClick={goToSettings}
                    >
                        Settings
                    </div>
                    <div
                        className={styles.link}
                        onClick={logout}
                    >
                        Log out
                    </div>
                </div>
            )}
        </div>
    )
}
