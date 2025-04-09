import { useContext, useEffect } from 'react'
import { useLocation, useNavigate, useSearchParams } from 'react-router-dom'
import 'firebaseui/dist/firebaseui.css'

import { ReactComponent as Logo } from '../../Logo.svg'

import styles from './Login.module.css'
import { renderLogin } from './renderLogin'
import { UserContext } from '../../contexts/userContext'

export const Login = () => {
    const {
        user,
    } = useContext(UserContext)
    const navigate = useNavigate()

    const { state } = useLocation()
    const [searchParams] = useSearchParams()

    const redirectUrl = searchParams.get('redirect') || state?.redirect || '/'

    useEffect(() => {
        if (user) navigate(redirectUrl)
        else renderLogin({ redirectUrl })
    }, [])

    return (
        <div className={styles.page}>
            <div className={styles.title}>
                <Logo />
            </div>
            <div id="render-login-ui"></div>
        </div>
    )
}
