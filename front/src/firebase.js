// Import the functions you need from the SDKs you need
import firebase from 'firebase/compat/app'
import { getAuth, connectAuthEmulator } from 'firebase/auth'

const {
    REACT_APP_USE_FIRESTORE_EMULATOR: USE_FIRESTORE_EMULATOR,
} = process.env

const {
    REACT_APP_FIREBASE_API_KEY,
    REACT_APP_FIREBASE_AUTH_DOMAIN,
    REACT_APP_FIREBASE_PROJECT_ID,
    REACT_APP_FIREBASE_STORAGE_BUCKET,
    REACT_APP_FIREBASE_MESSAGING_SENDER_ID,
    REACT_APP_FIREBASE_APP_ID,
} = process.env

const firebaseConfig = {
    apiKey: REACT_APP_FIREBASE_API_KEY,
    authDomain: REACT_APP_FIREBASE_AUTH_DOMAIN,
    projectId: REACT_APP_FIREBASE_PROJECT_ID,
    storageBucket: REACT_APP_FIREBASE_STORAGE_BUCKET,
    messagingSenderId: REACT_APP_FIREBASE_MESSAGING_SENDER_ID,
    appId: REACT_APP_FIREBASE_APP_ID,
  };

// Initialize Firebase
export let app
export let auth


if (firebaseConfig.apiKey) {
  app = firebase.initializeApp(firebaseConfig)
  auth = getAuth(app)
  if (USE_FIRESTORE_EMULATOR)
    connectAuthEmulator(auth, 'http://127.0.0.1:9099')
} else {
  auth = {
    disabled: true,
    onAuthStateChanged(cb) { cb(null); },
  }
}
