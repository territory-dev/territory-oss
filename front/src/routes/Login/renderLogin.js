import firebase from 'firebase/compat/app'
import * as firebaseui from 'firebaseui'
import 'firebaseui/dist/firebaseui.css'

import { auth } from '../../firebase'


const PASSWORD_LOGIN = process.env.REACT_APP_PASSWORD_LOGIN


export const renderLogin = ({ redirectUrl } = {}) => {
    if (auth.disabled) return;

    const ui = firebaseui.auth.AuthUI.getInstance() || new firebaseui.auth.AuthUI(auth)
    const signInOptions = [
        {
            provider: firebase.auth.EmailAuthProvider.PROVIDER_ID,
            signInMethod: PASSWORD_LOGIN ?
                firebase.auth.EmailAuthProvider.EMAIL_PASSWORD_SIGN_IN_METHOD :
                firebase.auth.EmailAuthProvider.EMAIL_LINK_SIGN_IN_METHOD,
            requireDisplayName: false
        },
        firebase.auth.GithubAuthProvider.PROVIDER_ID,
        firebase.auth.GoogleAuthProvider.PROVIDER_ID,
    ];
    ui.start('#render-login-ui', {
        signInSuccessUrl: redirectUrl,
        signInOptions,
        tosUrl: 'https://territory.dev/terms-of-service.html',
        privacyPolicyUrl: 'https://territory.dev/data.html',

    })
}

